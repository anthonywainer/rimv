//! Portable audio domain. Samples are interleaved normalized PCM f32.
//! No device, threading, filesystem, or operating-system capture dependencies.
mod buffer;
mod error;
mod format;
mod frame;
mod source;
pub use buffer::*;
pub use error::*;
pub use format::*;
pub use frame::*;
pub use source::*;
