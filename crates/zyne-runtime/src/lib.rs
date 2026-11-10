//! Public runtime facade used by games built with ZYNE.
//! Game projects should depend on this crate rather than renderer internals.

mod application;
mod application_config;
pub mod diagnostics;
mod event_queue;

pub use application::Application;
pub use application_config::ApplicationConfig;
pub use event_queue::EventQueue;

pub use zyne_core::{
    ButtonState, Engine, EngineConfig, EngineContext, EngineVersion, InputEvent, InputState,
    KeyCode, MouseButton, ScrollDelta, ZyneError, ZyneResult,
};
pub use zyne_platform::PlatformEvent;
