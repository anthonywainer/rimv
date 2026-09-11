use thiserror::Error;
pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("no input device available")]
    NoInputDevice,
    #[error("no output device available")]
    NoOutputDevice,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(&'static str),
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("unsupported sample format: {0}")]
    UnsupportedSampleFormat(String),
    #[error("device unavailable: {0}")]
    DeviceUnavailable(String),
    #[error("capture already running")]
    CaptureAlreadyRunning,
    #[error("capture not running")]
    CaptureNotRunning,
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("invalid audio frame: {0}")]
    InvalidFrame(&'static str),
    #[error("{backend} backend error: {message}")]
    BackendError {
        backend: &'static str,
        message: String,
    },
}
