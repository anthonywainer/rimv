use crate::{
    AudioCapture, CaptureConfig, CaptureMetrics, FrameReceiver, backend_error,
    devices::select,
    queue::{Shared, pcm_queue},
};
use audio_core::{AudioError, AudioFormat, AudioSourceKind, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, atomic::Ordering};

pub struct MicrophoneCapture {
    stream: cpal::Stream,
    shared: Arc<Shared>,
    active: bool,
}
impl MicrophoneCapture {
    pub fn new(config: CaptureConfig) -> Result<(Self, FrameReceiver)> {
        Self::build(config, AudioSourceKind::Microphone)
    }
    pub(crate) fn build(
        config: CaptureConfig,
        source: AudioSourceKind,
    ) -> Result<(Self, FrameReceiver)> {
        config.validate()?;
        let input = source == AudioSourceKind::Microphone;
        let device = select(config.device_id.as_deref(), input)?;
        let native = choose_config(&device, &config, input)?;
        let format = AudioFormat::new(native.sample_rate().0, native.channels())?;
        let shared = Shared::new();
        let (mut producer, receiver) = pcm_queue(&config, format, source, shared.clone());
        let error_shared = shared.clone();
        let on_error = move |error: cpal::StreamError| {
            let error = match error {
                cpal::StreamError::DeviceNotAvailable => {
                    AudioError::DeviceUnavailable("active audio device".into())
                }
                other => backend_error(other),
            };
            error_shared.fail(error);
        };
        // On WASAPI, build_input_stream on an output endpoint is true loopback.
        let stream = match native.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &native.config(),
                move |d: &[f32], _| producer.push(d, audio_core::normalize_f32),
                on_error,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &native.config(),
                move |d: &[i16], _| producer.push(d, audio_core::normalize_i16),
                on_error,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &native.config(),
                move |d: &[u16], _| producer.push(d, audio_core::normalize_u16),
                on_error,
                None,
            ),
            other => return Err(AudioError::UnsupportedSampleFormat(format!("{other:?}"))),
        }
        .map_err(|e| match e {
            cpal::BuildStreamError::DeviceNotAvailable => {
                AudioError::DeviceUnavailable("selected device".into())
            }
            cpal::BuildStreamError::StreamConfigNotSupported => {
                AudioError::InvalidConfiguration("device rejected stream configuration")
            }
            other => backend_error(other),
        })?;
        tracing::info!(backend=?cpal::default_host().id(),device=?device.name(),source=?source,sample_rate=format.sample_rate(),channels=format.channels(),native_format=?native.sample_format(),"capture configured");
        Ok((
            Self {
                stream,
                shared,
                active: false,
            },
            receiver,
        ))
    }
}
fn supported(format: cpal::SampleFormat) -> bool {
    matches!(
        format,
        cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
    )
}
fn choose_config(
    device: &cpal::Device,
    config: &CaptureConfig,
    input: bool,
) -> Result<cpal::SupportedStreamConfig> {
    let default = if input {
        device.default_input_config()
    } else {
        device.default_output_config()
    }
    .map_err(backend_error)?;
    if supported(default.sample_format())
        && config
            .preferred_sample_rate
            .is_none_or(|r| r == default.sample_rate().0)
        && config
            .preferred_channels
            .is_none_or(|c| c == default.channels())
    {
        return Ok(default);
    }
    let ranges: Vec<_> = if input {
        device
            .supported_input_configs()
            .map_err(backend_error)?
            .collect()
    } else {
        device
            .supported_output_configs()
            .map_err(backend_error)?
            .collect()
    };
    for range in ranges {
        if !supported(range.sample_format())
            || config
                .preferred_channels
                .is_some_and(|c| c != range.channels())
        {
            continue;
        }
        let rate = config.preferred_sample_rate.unwrap_or(
            default
                .sample_rate()
                .0
                .clamp(range.min_sample_rate().0, range.max_sample_rate().0),
        );
        if rate >= range.min_sample_rate().0 && rate <= range.max_sample_rate().0 {
            return Ok(range.with_sample_rate(cpal::SampleRate(rate)));
        }
    }
    Err(AudioError::InvalidConfiguration(
        "no supported f32/i16/u16 device configuration matches preferences",
    ))
}
impl AudioCapture for MicrophoneCapture {
    fn start(&mut self) -> Result<()> {
        if self.active {
            return Err(AudioError::CaptureAlreadyRunning);
        }
        self.shared.running.store(true, Ordering::Release);
        if let Err(error) = self.stream.play() {
            self.shared.running.store(false, Ordering::Release);
            return Err(backend_error(error));
        }
        self.active = true;
        tracing::info!("CPAL capture started");
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        if !self.active {
            return Err(AudioError::CaptureNotRunning);
        }
        self.stream.pause().map_err(backend_error)?;
        self.active = false;
        self.shared.running.store(false, Ordering::Release);
        tracing::info!(metrics=?self.metrics(),"CPAL capture stopped");
        Ok(())
    }
    fn is_running(&self) -> bool {
        self.active && self.shared.running.load(Ordering::Acquire)
    }
    fn metrics(&self) -> CaptureMetrics {
        self.shared.metrics()
    }
}
impl Drop for MicrophoneCapture {
    fn drop(&mut self) {
        if self.active {
            let _ = self.stop();
        }
    }
}
