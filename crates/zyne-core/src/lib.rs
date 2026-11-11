//! Foundational contracts shared across ZYNE.
//!
//! This crate must remain independent from rendering, gameplay, and editor code.

mod config;
mod context;
mod engine;
mod error;
mod input;
mod resource;
mod version;

pub use config::EngineConfig;
pub use context::EngineContext;
pub use engine::Engine;
pub use error::{ZyneError, ZyneResult};
pub use input::{ButtonState, InputEvent, InputState, KeyCode, MouseButton, ScrollDelta};
pub use resource::{Handle, ResourceId};
pub use version::EngineVersion;
pub use zyne_math::{Camera3d, Transform3d};
