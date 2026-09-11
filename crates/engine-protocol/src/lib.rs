//! UI-independent contracts. No native capture, transport, runtime, or storage
//! dependencies. Enable `serde` for JSON or other serialization adapters.
mod command;
mod error;
mod event;
mod state;
mod status;

pub use command::*;
pub use error::*;
pub use event::*;
pub use state::*;
pub use status::*;
