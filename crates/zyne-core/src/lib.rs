//! Foundational contracts shared across ZYNE.
//!
//! This crate must remain independent from rendering, gameplay, and editor code.

mod config;
mod context;
mod engine;
mod error;
mod version;

pub use config::EngineConfig;
pub use context::EngineContext;
pub use engine::Engine;
pub use error::{ZyneError, ZyneResult};
pub use version::EngineVersion;
