//! Rendering API and GPU implementation.
//! The renderer consumes extracted scene data; it does not own gameplay or ECS policy.

mod context;
mod error;

pub use context::GraphicsContext;
pub use error::GraphicsError;
