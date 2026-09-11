//! CPAL's WASAPI build_input_stream detects eRender endpoints and sets
//! AUDCLNT_STREAMFLAGS_LOOPBACK. No microphone fallback is used.
use crate::{AudioCapture, CaptureConfig, FrameReceiver, MicrophoneCapture};
pub(crate) fn system(
    config: CaptureConfig,
) -> audio_core::Result<(Box<dyn AudioCapture>, FrameReceiver)> {
    let (capture, receiver) =
        MicrophoneCapture::build(config, audio_core::AudioSourceKind::System)?;
    Ok((Box::new(capture), receiver))
}
