//! Public runtime facade used by games built with ZYNE.
//! Game projects should depend on this crate rather than renderer internals.

mod application;
mod application_config;
mod camera_controller;
pub mod diagnostics;
mod event_queue;

pub use application::Application;
pub use application_config::ApplicationConfig;
pub use camera_controller::OrbitCameraController;
pub use event_queue::EventQueue;

pub use zyne_core::{
    ButtonState, Engine, EngineConfig, EngineContext, EngineVersion, InputEvent, InputState,
    KeyCode, MouseButton, ScrollDelta, ZyneError, ZyneResult,
};
pub use zyne_math::{Camera3d, CameraError};
pub use zyne_platform::PlatformEvent;
