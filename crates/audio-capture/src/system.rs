use crate::{AudioCapture, CaptureConfig, CaptureMetrics, FrameReceiver};
use audio_core::Result;
/// True system playback capture. Platform code chooses the native backend.
pub struct SystemAudioCapture {
    inner: Box<dyn AudioCapture>,
}
impl SystemAudioCapture {
    pub fn new(config: CaptureConfig) -> Result<(Self, FrameReceiver)> {
        config.validate()?;
        let (inner, receiver) = crate::platform::system(config)?;
        Ok((Self { inner }, receiver))
    }
}
impl AudioCapture for SystemAudioCapture {
    fn start(&mut self) -> Result<()> {
        self.inner.start()
    }
    fn stop(&mut self) -> Result<()> {
        self.inner.stop()
    }
    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
    fn metrics(&self) -> CaptureMetrics {
        self.inner.metrics()
    }
}
