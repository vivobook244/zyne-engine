//! Rendering API and GPU implementation.
//! The renderer consumes extracted scene data; it does not own gameplay or ECS policy.

mod context;
mod error;
mod lighting;
mod material;
mod mesh;
mod pipeline;
mod presentation;
mod texture;

pub use context::GraphicsContext;
pub use error::GraphicsError;
pub use lighting::{
    DirectionalLightData, LightingData, MAX_DIRECTIONAL_LIGHTS, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS,
    PointLightData, SpotLightData,
};
pub use material::MaterialData;
pub use mesh::{GpuMaterial, GpuMesh, MeshData, MeshError, MeshRenderBatch, MeshVertex};
pub use texture::{GpuTexture, TextureColorSpace};

/// Result of attempting to render one window frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    /// A frame was acquired, submitted, and presented.
    Presented,
    /// The surface was suspended or temporarily unavailable.
    Skipped,
}
