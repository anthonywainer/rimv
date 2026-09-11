use crate::CaptureConfig;
use audio_core::{AudioFormat, AudioFrame, AudioSourceKind, Result};
use crossbeam_queue::ArrayQueue;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CaptureMetrics {
    pub dropped_frames: u64,
    pub backend_errors: u64,
}
pub(crate) struct Block {
    samples: Vec<f32>,
    timestamp: Duration,
}
pub(crate) struct Shared {
    pub running: AtomicBool,
    pub dropped: AtomicU64,
    pub error_count: AtomicU64,
    pub errors: ArrayQueue<audio_core::AudioError>,
}
impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            running: AtomicBool::new(false),
            dropped: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            errors: ArrayQueue::new(8),
        })
    }
    pub fn fail(&self, error: audio_core::AudioError) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        self.running.store(false, Ordering::Release);
        let _ = self.errors.push(error);
    }
    pub fn metrics(&self) -> CaptureMetrics {
        CaptureMetrics {
            dropped_frames: self.dropped.load(Ordering::Relaxed),
            backend_errors: self.error_count.load(Ordering::Relaxed),
        }
    }
}
pub(crate) struct Producer {
    ready: Arc<ArrayQueue<Block>>,
    pool: Arc<ArrayQueue<Vec<f32>>>,
    shared: Arc<Shared>,
    format: AudioFormat,
    position: u64,
}
impl Producer {
    pub fn push<T: Copy>(&mut self, data: &[T], normalize: fn(T) -> f32) {
        let timestamp =
            Duration::from_secs_f64(self.position as f64 / self.format.sample_rate() as f64);
        self.position += (data.len() / self.format.channels() as usize) as u64;
        if data.is_empty() {
            return;
        }
        let Some(mut samples) = self.pool.pop() else {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if data.len() > samples.capacity()
            || !data.len().is_multiple_of(self.format.channels() as usize)
        {
            let _ = self.pool.push(samples);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        samples.extend(data.iter().copied().map(normalize));
        if let Err(mut block) = self.ready.push(Block { samples, timestamp }) {
            block.samples.clear();
            let _ = self.pool.push(block.samples);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
pub(crate) enum ReceiverKind {
    Pcm {
        ready: Arc<ArrayQueue<Block>>,
        pool: Arc<ArrayQueue<Vec<f32>>>,
        format: AudioFormat,
        source: AudioSourceKind,
    },
    #[cfg(target_os = "macos")]
    Mac(crate::platform::macos::MacReceiver),
}
/// Single consumer. Allocation and platform PCM conversion happen here, never
/// in the microphone callback. A 2 ms polling interval avoids callback locks.
pub struct FrameReceiver {
    pub(crate) kind: ReceiverKind,
    pub(crate) shared: Arc<Shared>,
}
impl FrameReceiver {
    /// Returns `None` on timeout, including when stopped and fully drained.
    /// Backend failures are returned as typed errors. Stop capture before draining.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<AudioFrame>> {
        let start = Instant::now();
        loop {
            if let Some(error) = self.shared.errors.pop() {
                return Err(error);
            }
            let frame = match &mut self.kind {
                ReceiverKind::Pcm {
                    ready,
                    pool,
                    format,
                    source,
                } => {
                    if let Some(mut block) = ready.pop() {
                        let samples = block.samples.clone();
                        block.samples.clear();
                        let _ = pool.push(block.samples);
                        Some(AudioFrame::new(*source, block.timestamp, *format, samples)?)
                    } else {
                        None
                    }
                }
                #[cfg(target_os = "macos")]
                ReceiverKind::Mac(receiver) => receiver.pop()?,
            };
            if frame.is_some() {
                return Ok(frame);
            }
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            std::thread::sleep(
                Duration::from_millis(2).min(timeout.saturating_sub(start.elapsed())),
            );
        }
    }
}
pub(crate) fn pcm_queue(
    config: &CaptureConfig,
    format: AudioFormat,
    source: AudioSourceKind,
    shared: Arc<Shared>,
) -> (Producer, FrameReceiver) {
    let ready = Arc::new(ArrayQueue::new(config.buffer_capacity));
    let pool = Arc::new(ArrayQueue::new(config.buffer_capacity + 1));
    for _ in 0..=config.buffer_capacity {
        let _ = pool.push(Vec::with_capacity(config.max_samples_per_frame));
    }
    (
        Producer {
            ready: ready.clone(),
            pool: pool.clone(),
            shared: shared.clone(),
            format,
            position: 0,
        },
        FrameReceiver {
            kind: ReceiverKind::Pcm {
                ready,
                pool,
                format,
                source,
            },
            shared,
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_is_bounded_and_timestamp_preserves_gap() {
        let config = CaptureConfig {
            buffer_capacity: 1,
            max_samples_per_frame: 4,
            ..Default::default()
        };
        let shared = Shared::new();
        let (mut p, mut r) = pcm_queue(
            &config,
            AudioFormat::new(10, 1).unwrap(),
            AudioSourceKind::Microphone,
            shared.clone(),
        );
        p.push(&[1i16; 2], audio_core::normalize_i16);
        p.push(&[2i16; 2], audio_core::normalize_i16);
        assert_eq!(shared.metrics().dropped_frames, 1);
        assert_eq!(
            r.recv_timeout(Duration::ZERO).unwrap().unwrap().timestamp(),
            Duration::ZERO
        );
        p.push(&[3i16; 2], audio_core::normalize_i16);
        assert_eq!(
            r.recv_timeout(Duration::ZERO).unwrap().unwrap().timestamp(),
            Duration::from_millis(400)
        );
        p.push(&[0i16; 5], audio_core::normalize_i16);
        assert_eq!(shared.metrics().dropped_frames, 2);
        shared.fail(audio_core::AudioError::DeviceUnavailable("test".into()));
        assert!(matches!(
            r.recv_timeout(Duration::ZERO),
            Err(audio_core::AudioError::DeviceUnavailable(_))
        ));
    }
}
