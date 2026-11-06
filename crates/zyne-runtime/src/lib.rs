//! Public runtime facade used by games built with ZYNE.
//! Game projects should depend on this crate rather than renderer internals.

pub mod diagnostics;
mod application_config;
mod event_queue;
mod input_state;

pub use application_config::ApplicationConfig;
pub use event_queue::EventQueue;
pub use input_state::InputState;

pub use zyne_core::{Engine, EngineConfig, EngineContext, EngineVersion, ZyneError, ZyneResult};
pub use zyne_platform::PlatformEvent;
