//! Rendering API and GPU implementation.
//! The renderer consumes extracted scene data; it does not own gameplay or ECS policy.

mod context;
mod error;
mod mesh;
mod pipeline;

pub use context::GraphicsContext;
pub use error::GraphicsError;
pub use mesh::{GpuMesh, MeshData, MeshError, MeshVertex};

/// Result of attempting to render one window frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    /// A frame was acquired, submitted, and presented.
    Presented,
    /// The surface was suspended or temporarily unavailable.
    Skipped,
}
