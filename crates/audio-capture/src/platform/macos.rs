//! ScreenCaptureKit audio-only delivery. Its maintained Rust bindings isolate
//! the Swift/Objective-C bridge. No video output handler is registered.
use crate::{
    AudioCapture, CaptureConfig, CaptureMetrics, FrameReceiver,
    queue::{ReceiverKind, Shared},
};
use audio_core::{AudioError, AudioFormat, AudioFrame, AudioSourceKind, Result};
use crossbeam_queue::ArrayQueue;
use screencapturekit::{
    cm::CMSampleBufferExt,
    error::{SCError, SCStreamErrorCode},
    prelude::*,
    stream::delegate_trait::ErrorHandler,
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

fn error(e: SCError) -> AudioError {
    if matches!(e, SCError::PermissionDenied(_))
        || e.stream_error_code() == Some(SCStreamErrorCode::UserDeclined)
        // The binding's shareable-content API erases NSError codes into text.
        // Classify the observed TCC denial, but preserve unrelated failures.
        || matches!(&e, SCError::NoShareableContent(message) if message.contains("declined TCC"))
    {
        AudioError::PermissionDenied(e.to_string())
    } else {
        AudioError::BackendError {
            backend: "ScreenCaptureKit",
            message: e.to_string(),
        }
    }
}
struct MacCapture {
    stream: SCStream,
    shared: Arc<Shared>,
    active: bool,
}
pub(crate) struct MacReceiver {
    queue: Arc<ArrayQueue<CMSampleBuffer>>,
    origin: Option<f64>,
    max_samples: usize,
}
impl MacReceiver {
    pub(crate) fn pop(&mut self) -> Result<Option<AudioFrame>> {
        let Some(sample) = self.queue.pop() else {
            return Ok(None);
        };
        let desc = sample
            .format_description()
            .ok_or(AudioError::InvalidFrame("missing system audio format"))?;
        if !desc.is_pcm() || !desc.audio_is_float() || desc.audio_bits_per_channel() != Some(32) {
            return Err(AudioError::UnsupportedSampleFormat(
                "ScreenCaptureKit must deliver float32 PCM".into(),
            ));
        }
        let rate = desc
            .audio_sample_rate()
            .ok_or(AudioError::InvalidFrame("missing sample rate"))?;
        let channels = desc
            .audio_channel_count()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or(AudioError::InvalidFrame("invalid channels"))?;
        if !rate.is_finite() || rate <= 0.0 || rate > u32::MAX as f64 || rate.fract() != 0.0 {
            return Err(AudioError::InvalidFrame("invalid sample rate"));
        }
        let format = AudioFormat::new(rate as u32, channels)?;
        let pts = sample
            .presentation_timestamp()
            .as_seconds()
            .filter(|t| t.is_finite())
            .ok_or(AudioError::InvalidFrame("invalid presentation timestamp"))?;
        let origin = *self.origin.get_or_insert(pts);
        if pts < origin {
            return Err(AudioError::InvalidFrame("system timestamp moved backwards"));
        }
        let buffers = sample
            .audio_buffer_list()
            .ok_or(AudioError::InvalidFrame("missing system audio buffers"))?;
        let frames = usize::try_from(sample.num_samples())
            .map_err(|_| AudioError::InvalidFrame("negative sample count"))?;
        let count = frames
            .checked_mul(channels as usize)
            .filter(|n| *n <= self.max_samples)
            .ok_or(AudioError::InvalidFrame(
                "system audio block exceeds configured limit",
            ))?;
        let samples = decode_buffers(
            frames,
            channels as usize,
            count,
            buffers
                .iter()
                .map(|buffer| (buffer.number_channels() as usize, buffer.data())),
            desc.audio_is_big_endian(),
        )?;
        Ok(Some(AudioFrame::new(
            AudioSourceKind::System,
            Duration::from_secs_f64(pts - origin),
            format,
            samples,
        )?))
    }
}
pub(crate) fn system(config: CaptureConfig) -> Result<(Box<dyn AudioCapture>, FrameReceiver)> {
    if config.device_id.is_some() {
        return Err(AudioError::Unsupported(
            "ScreenCaptureKit captures application playback globally, not a selected output device",
        ));
    }
    let rate = config.preferred_sample_rate.unwrap_or(48000);
    let channels = config.preferred_channels.unwrap_or(2);
    if !matches!(rate, 8000 | 16000 | 24000 | 48000) || !matches!(channels, 1 | 2) {
        return Err(AudioError::InvalidConfiguration(
            "ScreenCaptureKit supports 8000/16000/24000/48000 Hz and 1 or 2 channels",
        ));
    }
    let content = SCShareableContent::get().map_err(error)?;
    let displays = content.displays();
    let display = displays.first().ok_or(AudioError::Unsupported(
        "ScreenCaptureKit needs a shareable display in a logged-in graphical session",
    ))?;
    let filter = SCContentFilter::create()
        .with_display(display)
        .with_excluding_windows(&[])
        .build();
    let native = SCStreamConfiguration::new()
        .with_width(2)
        .with_height(2)
        .with_captures_audio(true)
        .with_sample_rate(rate as i32)
        .with_channel_count(channels as i32)
        .with_excludes_current_process_audio(false);
    let shared = Shared::new();
    let errors = shared.clone();
    let mut stream = SCStream::new_with_delegate(
        &filter,
        &native,
        ErrorHandler::new(move |e| errors.fail(error(e))),
    );
    let queue = Arc::new(ArrayQueue::new(config.buffer_capacity));
    let target = queue.clone();
    let state = shared.clone();
    let max_samples = config.max_samples_per_frame;
    let handler = move |sample: CMSampleBuffer, kind: SCStreamOutputType| {
        if kind != SCStreamOutputType::Audio {
            return;
        }
        // Transfer the retained native buffer. Decode/interleave on the consumer.
        if sample.num_samples() <= 0 {
            return;
        }
        if sample.num_samples() as u64 * channels as u64 > max_samples as u64
            || target.push(sample).is_err()
        {
            state.dropped.fetch_add(1, Ordering::Relaxed);
        }
    };
    if stream
        .add_output_handler(handler, SCStreamOutputType::Audio)
        .is_none()
    {
        return Err(AudioError::BackendError {
            backend: "ScreenCaptureKit",
            message: "audio handler registration failed".into(),
        });
    }
    tracing::info!(
        backend = "ScreenCaptureKit",
        sample_rate = rate,
        channels,
        native_format = "f32",
        "system capture configured"
    );
    let receiver = FrameReceiver {
        kind: ReceiverKind::Mac(MacReceiver {
            queue,
            origin: None,
            max_samples,
        }),
        shared: shared.clone(),
    };
    Ok((
        Box::new(MacCapture {
            stream,
            shared,
            active: false,
        }),
        receiver,
    ))
}
impl AudioCapture for MacCapture {
    fn start(&mut self) -> Result<()> {
        if self.active {
            return Err(AudioError::CaptureAlreadyRunning);
        }
        self.shared.running.store(true, Ordering::Release);
        if let Err(e) = self.stream.start_capture() {
            self.shared.running.store(false, Ordering::Release);
            return Err(error(e));
        }
        self.active = true;
        tracing::info!("ScreenCaptureKit capture started");
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        if !self.active {
            return Err(AudioError::CaptureNotRunning);
        }
        self.stream.stop_capture().map_err(error)?;
        self.active = false;
        self.shared.running.store(false, Ordering::Release);
        tracing::info!(metrics=?self.metrics(),"ScreenCaptureKit capture stopped");
        Ok(())
    }
    fn is_running(&self) -> bool {
        self.active && self.shared.running.load(Ordering::Acquire)
    }
    fn metrics(&self) -> CaptureMetrics {
        self.shared.metrics()
    }
}
impl Drop for MacCapture {
    fn drop(&mut self) {
        if self.active {
            let _ = self.stop();
        }
    }
}

/// Accept planar, interleaved, or grouped channels while validating byte sizes.
fn decode_buffers<'a>(
    frames: usize,
    channels: usize,
    count: usize,
    buffers: impl Iterator<Item = (usize, &'a [u8])>,
    big_endian: bool,
) -> Result<Vec<f32>> {
    let mut samples = vec![0.0; count];
    let mut channel_offset = 0usize;
    for (n, bytes) in buffers {
        if n == 0 || channel_offset + n > channels || bytes.len() != frames * n * 4 {
            return Err(AudioError::InvalidFrame(
                "inconsistent system audio buffer layout",
            ));
        }
        for frame in 0..frames {
            for channel in 0..n {
                let offset = (frame * n + channel) * 4;
                let raw = [
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ];
                let value = if big_endian {
                    f32::from_be_bytes(raw)
                } else {
                    f32::from_le_bytes(raw)
                };
                samples[frame * channels + channel_offset + channel] =
                    audio_core::normalize_f32(value);
            }
        }
        channel_offset += n;
    }
    if channel_offset != channels {
        return Err(AudioError::InvalidFrame("incomplete system audio channels"));
    }
    Ok(samples)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_permission_and_backend_failures() {
        assert!(matches!(
            error(SCError::NoShareableContent("The user declined TCCs".into())),
            AudioError::PermissionDenied(_)
        ));
        assert!(matches!(
            error(SCError::NoShareableContent("display unavailable".into())),
            AudioError::BackendError { .. }
        ));
        assert!(matches!(
            error(SCError::from_stream_error_code(
                SCStreamErrorCode::UserDeclined
            )),
            AudioError::PermissionDenied(_)
        ));
    }
    #[test]
    fn planar_and_interleaved_float_audio() {
        let left: Vec<u8> = [0.25f32, 0.5]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        let right: Vec<u8> = [-0.25f32, -0.5]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        assert_eq!(
            decode_buffers(
                2,
                2,
                4,
                [(1, left.as_slice()), (1, right.as_slice())].into_iter(),
                false
            )
            .unwrap(),
            vec![0.25, -0.25, 0.5, -0.5]
        );
        let packed: Vec<u8> = [0.25f32, -0.25, 0.5, -0.5]
            .into_iter()
            .flat_map(f32::to_be_bytes)
            .collect();
        assert_eq!(
            decode_buffers(2, 2, 4, [(2, packed.as_slice())].into_iter(), true).unwrap(),
            vec![0.25, -0.25, 0.5, -0.5]
        );
        assert!(decode_buffers(2, 2, 4, [(2, left.as_slice())].into_iter(), false).is_err());
        assert!(decode_buffers(2, 2, 4, [(1, left.as_slice())].into_iter(), false).is_err());
    }
    #[test]
    fn rejects_system_device_and_invalid_format_without_hardware() {
        assert!(matches!(
            system(CaptureConfig {
                device_id: Some("output".into()),
                ..Default::default()
            }),
            Err(AudioError::Unsupported(_))
        ));
        assert!(matches!(
            system(CaptureConfig {
                preferred_sample_rate: Some(44100),
                ..Default::default()
            }),
            Err(AudioError::InvalidConfiguration(_))
        ));
    }
}
