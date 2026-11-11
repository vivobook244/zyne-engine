//! Asset import and validated CPU-side asset representations for ZYNE.
//!
//! This crate deliberately does not depend on ECS, runtime, or graphics APIs.
//! Imported data is owned by the engine and can later be uploaded by zyne-graphics.

mod gltf_import;

pub use gltf_import::{
    ImportedAsset, ImportedImage, ImportedMesh, ImportedPrimitive, ImportError, import_gltf,
};
