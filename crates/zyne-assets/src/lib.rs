//! Asset import, persistent metadata, and validated CPU-side representations for ZYNE.
//!
//! This crate does not depend on ECS or graphics APIs. Importers produce owned
//! CPU data, while the registry persists stable metadata independently of
//! runtime handles and GPU allocations.

mod cache;
mod gltf_import;
mod material;
mod registry;
mod texture;

pub use cache::{AssetCache, AssetCacheError, AssetCacheStatus, AssetState};
pub use gltf_import::{
    ImportError, ImportedAsset, ImportedImage, ImportedMesh, ImportedPrimitive, import_gltf,
};
pub use material::{BlinnPhongMaterial, MaterialError};
pub use texture::{TextureColorSpace, TextureData, TextureDataError};

/// Typed CPU-side cache for registered Blinn–Phong material assets.
pub type MaterialCache = AssetCache<BlinnPhongMaterial>;
/// Typed CPU-side cache for registered validated texture assets.
pub type TextureCache = AssetCache<TextureData>;

pub use registry::{
    AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, RegistryError,
    RegistryLoadError, ResourceKind,
};
