#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
pub(crate) use linux::system;
#[cfg(target_os = "macos")]
pub(crate) use macos::system;
#[cfg(target_os = "windows")]
pub(crate) use windows::system;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(crate) fn system(
    _: crate::CaptureConfig,
) -> audio_core::Result<(Box<dyn crate::AudioCapture>, crate::FrameReceiver)> {
    Err(audio_core::AudioError::UnsupportedPlatform(
        std::env::consts::OS,
    ))
}
