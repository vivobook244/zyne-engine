//! Public runtime facade used by games built with ZYNE.
//! Game projects should depend on this crate rather than renderer internals.

pub mod diagnostics;
mod event_queue;

pub use event_queue::EventQueue;

pub use zyne_core::{Engine, EngineConfig, EngineContext, EngineVersion, ZyneError, ZyneResult};
pub use zyne_platform::PlatformEvent;
