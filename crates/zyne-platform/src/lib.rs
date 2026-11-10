//! OS integration for ZYNE desktop applications.
//!
//! This crate owns native window and operating-system event integration.
//! Engine lifecycle policy and frame scheduling belong to `zyne-runtime`.

mod backend;
mod error;
mod event;
mod input;
mod window;

pub use error::PlatformError;
pub use event::PlatformEvent;
pub use input::{ButtonState, InputEvent, KeyCode, MouseButton, ScrollDelta};
pub use window::{
    PlatformWindow, WindowConfig, WindowControl, WindowEvent, WindowEventHandler, WindowMode,
    WindowRunner, WindowSize,
};
