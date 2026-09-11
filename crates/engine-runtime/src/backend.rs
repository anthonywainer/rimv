//! Native adapter plus an injection boundary for hardware-free tests and future
//! local backends. Streams are created/used/dropped on the runtime worker.
use crate::{AudioSource, EngineCapabilities, EngineError, EngineErrorCode, Result, SourceConfig};
use audio_capture::{
    AudioCapture, CaptureConfig, CaptureMetrics, FrameReceiver, MicrophoneCapture,
    SystemAudioCapture,
};
use audio_core::{AudioError, AudioFrame};
use std::time::Duration;

/// The factory crosses into the worker; a native stream does not have to be Send.
pub trait CaptureBackend: Send + 'static {
    fn capabilities(&self) -> EngineCapabilities;
    fn open(
        &mut self,
        source: AudioSource,
        config: &SourceConfig,
    ) -> Result<Box<dyn CaptureStream>>;
}
pub trait CaptureStream {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn try_recv(&mut self) -> Result<Option<AudioFrame>>;
    fn is_running(&self) -> bool;
    fn metrics(&self) -> CaptureMetrics;
}

pub(crate) fn native_config(config: &SourceConfig) -> CaptureConfig {
    CaptureConfig {
        device_id: config.device_id.clone(),
        preferred_sample_rate: config.preferred_sample_rate,
        preferred_channels: config.preferred_channels,
        ..Default::default()
    }
}

pub struct NativeBackend;
impl CaptureBackend for NativeBackend {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            microphone_capture: cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            )),
            system_audio_capture: cfg!(any(target_os = "macos", target_os = "windows")),
            transcription: false,
        }
    }
    fn open(
        &mut self,
        source: AudioSource,
        config: &SourceConfig,
    ) -> Result<Box<dyn CaptureStream>> {
        let (capture, receiver): (Box<dyn AudioCapture>, _) = match source {
            AudioSource::Microphone => {
                let (capture, receiver) = MicrophoneCapture::new(native_config(config))
                    .map_err(|e| native_error(source, e))?;
                (Box::new(capture), receiver)
            }
            AudioSource::System => {
                let (capture, receiver) = SystemAudioCapture::new(native_config(config))
                    .map_err(|e| native_error(source, e))?;
                (Box::new(capture), receiver)
            }
        };
        Ok(Box::new(NativeStream {
            capture,
            receiver,
            source,
        }))
    }
}
struct NativeStream {
    capture: Box<dyn AudioCapture>,
    receiver: FrameReceiver,
    source: AudioSource,
}
impl CaptureStream for NativeStream {
    fn start(&mut self) -> Result<()> {
        self.capture
            .start()
            .map_err(|e| native_error(self.source, e))
    }
    fn stop(&mut self) -> Result<()> {
        self.capture
            .stop()
            .map_err(|e| native_error(self.source, e))
    }
    fn try_recv(&mut self) -> Result<Option<AudioFrame>> {
        self.receiver
            .recv_timeout(Duration::ZERO)
            .map_err(|e| native_error(self.source, e))
    }
    fn is_running(&self) -> bool {
        self.capture.is_running()
    }
    fn metrics(&self) -> CaptureMetrics {
        self.capture.metrics()
    }
}
fn native_error(source: AudioSource, error: AudioError) -> EngineError {
    let code = match error {
        AudioError::PermissionDenied(_) => EngineErrorCode::PermissionDenied,
        AudioError::Unsupported(_) | AudioError::UnsupportedPlatform(_) => {
            EngineErrorCode::Unsupported
        }
        _ => EngineErrorCode::CaptureFailed,
    };
    EngineError::new(code, error.to_string()).for_source(source)
}
