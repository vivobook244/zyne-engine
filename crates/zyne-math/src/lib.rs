//! Math types and conventions used by ZYNE.

mod camera;
mod transform;

pub use camera::{Camera3d, CameraError};
pub use transform::{Transform3d, TransformError};

/// World-space distance in engine units.
pub type WorldUnit = f32;
