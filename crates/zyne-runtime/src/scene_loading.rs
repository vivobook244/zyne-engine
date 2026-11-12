//! Scene loading integration with persistent asset validation and runtime diagnostics.

use std::fmt;

use zyne_assets::{AssetCacheStatus, AssetRegistry};

use crate::{
    Scene, SceneAssetResolverError, SceneAssetStatusReport, SceneAssetValidationReport,
    SceneLoadError, resolve_scene_asset_statuses, validate_scene_asset_references,
};

/// A successfully decoded scene together with its asset diagnostics.
///
/// Missing, unavailable, unloaded, loading, or failed assets do not prevent
/// the scene itself from loading. Callers can inspect the reports and decide
/// whether the scene is ready for a particular runtime operation.
pub struct LoadedScene {
    scene: Scene,
    asset_validation: SceneAssetValidationReport,
    asset_status: SceneAssetStatusReport,
}

impl LoadedScene {
    /// Returns the loaded scene.
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Consumes the report and returns its loaded scene.
    #[must_use]
    pub fn into_scene(self) -> Scene {
        self.scene
    }

    /// Returns the persistent registry validation report.
    #[must_use]
    pub const fn asset_validation(&self) -> &SceneAssetValidationReport {
        &self.asset_validation
    }

    /// Returns the runtime status report for every scene asset reference.
    #[must_use]
    pub const fn asset_status(&self) -> &SceneAssetStatusReport {
        &self.asset_status
    }

    /// Returns true when every referenced asset is resident in a supplied cache.
    #[must_use]
    pub fn is_asset_ready(&self) -> bool {
        self.asset_validation.is_valid() && self.asset_status.is_ready()
    }
}

/// Errors that prevent a scene and its asset diagnostics from being produced.
#[derive(Debug)]
pub enum SceneAssetLoadError {
    /// The scene source could not be decoded or validated.
    Scene(SceneLoadError),
    /// The supplied runtime caches are ambiguous.
    Resolver(SceneAssetResolverError),
}

impl fmt::Display for SceneAssetLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scene(error) => write!(formatter, "failed to load scene: {error}"),
            Self::Resolver(error) => write!(formatter, "failed to resolve scene assets: {error}"),
        }
    }
}

impl std::error::Error for SceneAssetLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Scene(error) => Some(error),
            Self::Resolver(error) => Some(error),
        }
    }
}

/// Loads a RON scene and produces persistent and runtime asset diagnostics.
///
/// Asset problems are reported rather than treated as scene decoding errors.
/// The supplied caches must contain at most one cache for each resource kind.
///
/// # Errors
///
/// Returns Scene when the RON source or scene document is invalid, and
/// Resolver when multiple caches are supplied for the same resource kind.
pub fn load_scene_with_asset_diagnostics(
    source: &str,
    registry: &AssetRegistry,
    caches: &[&dyn AssetCacheStatus],
) -> Result<LoadedScene, SceneAssetLoadError> {
    let scene = Scene::from_ron(source).map_err(SceneAssetLoadError::Scene)?;
    let asset_validation = validate_scene_asset_references(&scene, registry);
    let asset_status = resolve_scene_asset_statuses(&scene, registry, caches)
        .map_err(SceneAssetLoadError::Resolver)?;

    Ok(LoadedScene {
        scene,
        asset_validation,
        asset_status,
    })
}

#[cfg(test)]
mod tests {
    use zyne_assets::{
        AssetCache, AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath,
        ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;

    use super::{SceneAssetLoadError, load_scene_with_asset_diagnostics};
    use crate::{Scene, SceneAssetRuntimeStatus};

    #[derive(Debug)]
    struct TestMesh;

    fn register_mesh(registry: &mut AssetRegistry, id: ResourceId) {
        let metadata = AssetMetadata::new(
            id,
            "Test Mesh",
            ResourceKind::Mesh,
            ProjectRelativePath::new("assets/test.mesh").unwrap(),
            ImporterSettings::new("test-importer", 1).unwrap(),
        )
        .unwrap();
        registry.register(metadata).unwrap();
    }

    fn scene_source(references: &[ResourceId]) -> String {
        let mut scene = Scene::new("Asset Diagnostic Scene").unwrap();
        let entity = scene
            .spawn_entity("Prop", Transform3d::identity())
            .unwrap();
        scene
            .set_entity_asset_references(entity, references)
            .unwrap();
        scene.to_ron().unwrap()
    }

    #[test]
    fn loads_scene_and_reports_loaded_asset_status() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register_mesh(&mut registry, id);
        let source = scene_source(&[id]);

        let mut cache = AssetCache::<TestMesh>::new(ResourceKind::Mesh);
        cache.begin_loading(&registry, id).unwrap();
        cache.complete_loading(&registry, id, TestMesh).unwrap();

        let loaded = load_scene_with_asset_diagnostics(&source, &registry, &[&cache]).unwrap();

        assert_eq!(loaded.scene().name(), "Asset Diagnostic Scene");
        assert!(loaded.asset_validation().is_valid());
        assert_eq!(loaded.asset_status().loaded_count(), 1);
        assert!(loaded.is_asset_ready());
    }

    #[test]
    fn keeps_scene_loadable_and_reports_missing_and_unavailable_assets() {
        let registered_id = ResourceId::new();
        let missing_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register_mesh(&mut registry, registered_id);
        let source = scene_source(&[registered_id, missing_id]);

        let loaded = load_scene_with_asset_diagnostics(&source, &registry, &[]).unwrap();

        assert_eq!(loaded.scene().entity_count(), 1);
        assert_eq!(loaded.asset_validation().missing_count(), 1);
        assert_eq!(
            loaded.asset_status().references()[0].status(),
            &SceneAssetRuntimeStatus::CacheUnavailable {
                kind: ResourceKind::Mesh,
            }
        );
        assert_eq!(
            loaded.asset_status().references()[1].status(),
            &SceneAssetRuntimeStatus::Unregistered
        );
        assert!(!loaded.is_asset_ready());
    }

    #[test]
    fn reports_scene_decode_errors_separately_from_asset_diagnostics() {
        let registry = AssetRegistry::new();
        assert!(matches!(
            load_scene_with_asset_diagnostics("not a scene", &registry, &[]),
            Err(SceneAssetLoadError::Scene(_))
        ));
    }

    #[test]
    fn rejects_ambiguous_runtime_cache_configuration() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register_mesh(&mut registry, id);
        let source = scene_source(&[id]);
        let first = AssetCache::<TestMesh>::new(ResourceKind::Mesh);
        let second = AssetCache::<TestMesh>::new(ResourceKind::Mesh);

        assert!(matches!(
            load_scene_with_asset_diagnostics(&source, &registry, &[&first, &second]),
            Err(SceneAssetLoadError::Resolver(_))
        ));
    }
}
