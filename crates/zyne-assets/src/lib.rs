//! Asset import, persistent metadata, and validated CPU-side representations for ZYNE.
//!
//! This crate does not depend on ECS or graphics APIs. Importers produce owned
//! CPU data, while the registry persists stable metadata independently of
//! runtime handles and GPU allocations.

mod cache;
mod gltf_import;
mod registry;

pub use cache::{AssetCache, AssetCacheError, AssetCacheStatus, AssetState};
pub use gltf_import::{
    ImportError, ImportedAsset, ImportedImage, ImportedMesh, ImportedPrimitive, import_gltf,
};
pub use registry::{
    AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, RegistryError,
    RegistryLoadError, ResourceKind,
};
