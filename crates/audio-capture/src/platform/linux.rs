//! Reserved for PipeWire monitor-node capture, with portal/session handling.
//! Microphone/ALSA devices must never stand in for system audio.
use crate::{AudioCapture, CaptureConfig, FrameReceiver};
pub(crate) fn system(
    _: CaptureConfig,
) -> audio_core::Result<(Box<dyn AudioCapture>, FrameReceiver)> {
    Err(audio_core::AudioError::Unsupported(
        "Linux system capture requires a future PipeWire backend",
    ))
}
