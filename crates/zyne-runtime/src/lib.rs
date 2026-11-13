//! Public runtime facade used by games built with ZYNE.
//! Game projects should depend on this crate rather than renderer internals.

mod application;
mod application_config;
mod camera_controller;
pub mod diagnostics;
mod event_queue;
mod mesh_conversion;
mod scene_assets;
mod scene_loading;
mod scene_renderer;

pub use application::Application;
pub use application_config::ApplicationConfig;
pub use camera_controller::OrbitCameraController;
pub use event_queue::EventQueue;
pub use mesh_conversion::{PrimitiveMeshError, imported_primitive_to_mesh_data};
pub use scene_assets::{
    MissingAssetReference, ResolvedSceneAssetReference, SceneAssetResolverError,
    SceneAssetRuntimeStatus, SceneAssetStatusReport, SceneAssetValidationReport,
    resolve_scene_asset_statuses, validate_scene_asset_references,
};
pub use scene_loading::{LoadedScene, SceneAssetLoadError, load_scene_with_asset_diagnostics};
pub use scene_renderer::{SceneMeshRenderer, SceneRenderError};

pub use zyne_assets::AssetRegistry;
pub use zyne_core::{
    ButtonState, Engine, EngineConfig, EngineContext, EngineVersion, InputEvent, InputState,
    KeyCode, MouseButton, ScrollDelta, ZyneError, ZyneResult,
};
pub use zyne_math::{Camera3d, CameraError, Transform3d};
pub use zyne_platform::PlatformEvent;
pub use zyne_scene::{
    Scene, SceneDocument, SceneEntityDocument, SceneEntityId, SceneError, SceneId, SceneLoadError,
    SceneSaveError,
};
