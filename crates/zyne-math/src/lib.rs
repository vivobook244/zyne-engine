//! Math types and conventions used by ZYNE.
//! The public facade will be backed by glam when math implementation begins.

mod camera;

pub use camera::{Camera3d, CameraError};

/// World-space distance in engine units.
pub type WorldUnit = f32;
