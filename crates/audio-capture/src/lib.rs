//! Native capture adapters. All sample delivery is bounded and nonblocking at
//! the producer. Call `FrameReceiver::recv_timeout` from a consumer thread.
mod config;
mod devices;
mod microphone;
mod platform;
mod queue;
mod system;
pub use audio_core;
use audio_core::Result;
pub use config::CaptureConfig;
pub use devices::*;
pub use microphone::MicrophoneCapture;
pub use queue::{CaptureMetrics, FrameReceiver};
pub use system::SystemAudioCapture;

pub trait AudioCapture {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn is_running(&self) -> bool;
    fn metrics(&self) -> CaptureMetrics;
}
pub(crate) fn backend_error(error: impl std::fmt::Display) -> audio_core::AudioError {
    audio_core::AudioError::BackendError {
        backend: "CPAL",
        message: error.to_string(),
    }
}
