//! Validation of persistent scene asset references against a project registry.

use zyne_assets::AssetRegistry;
use zyne_core::ResourceId;

use crate::{Scene, SceneEntityId};

/// One scene reference whose resource identity is absent from the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingAssetReference {
    entity_id: SceneEntityId,
    resource_id: ResourceId,
}

impl MissingAssetReference {
    /// Returns the persistent identity of the referencing scene entity.
    #[must_use]
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the unresolved persistent resource identity.
    #[must_use]
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }
}

/// Non-mutating result of validating all asset references in a scene.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SceneAssetValidationReport {
    missing_references: Vec<MissingAssetReference>,
}

impl SceneAssetValidationReport {
    /// Returns true when every scene asset reference exists in the registry.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.missing_references.is_empty()
    }

    /// Returns the number of references missing from the registry.
    #[must_use]
    pub fn missing_count(&self) -> usize {
        self.missing_references.len()
    }

    /// Returns missing references in deterministic entity/reference order.
    #[must_use]
    pub fn missing_references(&self) -> &[MissingAssetReference] {
        &self.missing_references
    }
}

/// Checks every persistent asset reference in a scene against an asset registry.
///
/// This operation is read-only: it does not modify the scene or registry. The
/// report is ordered by the scene's deterministic entity order and then by the
/// order in which references were attached to each entity.
#[must_use]
pub fn validate_scene_asset_references(
    scene: &Scene,
    registry: &AssetRegistry,
) -> SceneAssetValidationReport {
    let mut report = SceneAssetValidationReport::default();

    for entity_id in scene.entity_ids() {
        let Some(references) = scene.entity_asset_references(entity_id) else {
            continue;
        };

        for &resource_id in references {
            if registry.find(resource_id).is_none() {
                report.missing_references.push(MissingAssetReference {
                    entity_id,
                    resource_id,
                });
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::validate_scene_asset_references;
    use crate::Scene;
    use zyne_assets::{
        AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;

    fn register_asset(registry: &mut AssetRegistry, id: ResourceId, path: &str) {
        let metadata = AssetMetadata::new(
            id,
            "Test Asset",
            ResourceKind::Model,
            ProjectRelativePath::new(path).unwrap(),
            ImporterSettings::new("gltf", 1).unwrap(),
        )
        .unwrap();
        registry.register(metadata).unwrap();
    }

    #[test]
    fn accepts_scene_references_registered_in_asset_registry() {
        let mut scene = Scene::new("Registered Assets").unwrap();
        let entity_id = scene
            .spawn_entity("Player", Transform3d::identity())
            .unwrap();
        let registered_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register_asset(&mut registry, registered_id, "models/player.glb");
        scene
            .set_entity_asset_references(entity_id, &[registered_id])
            .unwrap();

        let report = validate_scene_asset_references(&scene, &registry);

        assert!(report.is_valid());
        assert_eq!(report.missing_count(), 0);
        assert_eq!(report.missing_references(), []);
    }

    #[test]
    fn reports_missing_resource_with_entity_and_resource_identity() {
        let mut scene = Scene::new("Missing Asset").unwrap();
        let entity_id = scene
            .spawn_entity("Player", Transform3d::identity())
            .unwrap();
        let missing_id = ResourceId::new();
        scene
            .set_entity_asset_references(entity_id, &[missing_id])
            .unwrap();

        let report = validate_scene_asset_references(&scene, &AssetRegistry::new());

        assert!(!report.is_valid());
        assert_eq!(report.missing_count(), 1);
        assert_eq!(report.missing_references()[0].entity_id(), entity_id);
        assert_eq!(report.missing_references()[0].resource_id(), missing_id);
    }

    #[test]
    fn reports_all_missing_references_and_does_not_mutate_inputs() {
        let mut scene = Scene::new("Multiple References").unwrap();
        let first_entity = scene
            .spawn_entity("First", Transform3d::identity())
            .unwrap();
        let second_entity = scene
            .spawn_entity("Second", Transform3d::identity())
            .unwrap();

        let registered_id = ResourceId::new();
        let first_missing_id = ResourceId::new();
        let second_missing_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register_asset(&mut registry, registered_id, "models/registered.glb");
        scene
            .set_entity_asset_references(
                first_entity,
                &[registered_id, first_missing_id, second_missing_id],
            )
            .unwrap();
        scene
            .set_entity_asset_references(second_entity, &[first_missing_id])
            .unwrap();

        let report = validate_scene_asset_references(&scene, &registry);

        assert_eq!(report.missing_count(), 3);
        let expected = scene
            .entity_ids()
            .into_iter()
            .flat_map(|entity_id| {
                let resource_ids = if entity_id == first_entity {
                    vec![first_missing_id, second_missing_id]
                } else {
                    vec![first_missing_id]
                };
                resource_ids
                    .into_iter()
                    .map(move |resource_id| (entity_id, resource_id))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            report
                .missing_references()
                .iter()
                .map(|missing| (missing.entity_id(), missing.resource_id()))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(registry.len(), 1);
        assert_eq!(
            scene.entity_asset_references(first_entity),
            Some(&[registered_id, first_missing_id, second_missing_id][..])
        );
        assert_eq!(
            scene.entity_asset_references(second_entity),
            Some(&[first_missing_id][..])
        );
    }
}

/// Runtime state of one persistent asset reference in a scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneAssetRuntimeStatus {
    /// The scene references an ID that is absent from the project registry.
    Unregistered,
    /// The registry knows the asset kind, but no matching typed cache was supplied.
    CacheUnavailable { kind: zyne_assets::ResourceKind },
    /// The asset is registered but has not started loading in its cache.
    Unloaded,
    /// The asset is currently being loaded.
    Loading,
    /// The asset has a resident runtime value.
    Loaded,
    /// The latest load attempt failed.
    Failed { reason: String },
}

/// Runtime status for one scene entity's asset reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSceneAssetReference {
    entity_id: SceneEntityId,
    resource_id: ResourceId,
    status: SceneAssetRuntimeStatus,
}

impl ResolvedSceneAssetReference {
    /// Returns the persistent identity of the referencing scene entity.
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the persistent identity of the referenced asset.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the resolved runtime status.
    #[must_use]
    pub const fn status(&self) -> &SceneAssetRuntimeStatus {
        &self.status
    }
}

/// Deterministic, read-only runtime status snapshot for a scene's asset references.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SceneAssetStatusReport {
    references: Vec<ResolvedSceneAssetReference>,
}

impl SceneAssetStatusReport {
    /// Returns every reference in deterministic entity/reference order.
    #[must_use]
    pub fn references(&self) -> &[ResolvedSceneAssetReference] {
        &self.references
    }

    /// Returns true only when every reference has a resident runtime value.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.references
            .iter()
            .all(|reference| reference.status == SceneAssetRuntimeStatus::Loaded)
    }

    /// Returns the number of references whose assets are loaded.
    #[must_use]
    pub fn loaded_count(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| reference.status == SceneAssetRuntimeStatus::Loaded)
            .count()
    }

    /// Returns the number of references whose latest load attempt failed.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| {
                matches!(
                    reference.status,
                    SceneAssetRuntimeStatus::Failed { .. }
                )
            })
            .count()
    }

    /// Returns the number of registered references that are not yet loaded.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.references
            .iter()
            .filter(|reference| {
                matches!(
                    reference.status,
                    SceneAssetRuntimeStatus::Unloaded | SceneAssetRuntimeStatus::Loading
                )
            })
            .count()
    }
}

/// Errors encountered while resolving scene asset runtime status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneAssetResolverError {
    /// More than one cache was supplied for the same resource kind.
    DuplicateCacheKind { kind: zyne_assets::ResourceKind },
}

impl std::fmt::Display for SceneAssetResolverError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateCacheKind { kind } => {
                write!(formatter, "multiple asset caches supplied for {kind:?}")
            }
        }
    }
}

impl std::error::Error for SceneAssetResolverError {}

/// Resolves every scene asset reference against the registry and typed runtime caches.
///
/// The supplied caches must contain at most one cache per resource kind.
/// This function does not mutate the scene, registry, or caches. Results follow
/// deterministic scene entity order and each entity's reference insertion order.
///
/// # Errors
///
/// Returns DuplicateCacheKind when multiple caches are supplied for one resource kind.
#[must_use]
pub fn resolve_scene_asset_statuses(
    scene: &Scene,
    registry: &AssetRegistry,
    caches: &[&dyn zyne_assets::AssetCacheStatus],
) -> Result<SceneAssetStatusReport, SceneAssetResolverError> {
    let mut cache_kinds = Vec::with_capacity(caches.len());
    for cache in caches {
        let kind = cache.kind();
        if cache_kinds.contains(&kind) {
            return Err(SceneAssetResolverError::DuplicateCacheKind { kind });
        }
        cache_kinds.push(kind);
    }

    let mut report = SceneAssetStatusReport::default();
    for entity_id in scene.entity_ids() {
        let Some(references) = scene.entity_asset_references(entity_id) else {
            continue;
        };

        for &resource_id in references {
            let status = match registry.find(resource_id) {
                None => SceneAssetRuntimeStatus::Unregistered,
                Some(metadata) => {
                    match caches
                        .iter()
                        .find(|cache| cache.kind() == metadata.kind())
                    {
                        None => SceneAssetRuntimeStatus::CacheUnavailable {
                            kind: metadata.kind(),
                        },
                        Some(cache) => match cache.state(resource_id) {
                            zyne_assets::AssetState::Unloaded => {
                                SceneAssetRuntimeStatus::Unloaded
                            }
                            zyne_assets::AssetState::Loading => {
                                SceneAssetRuntimeStatus::Loading
                            }
                            zyne_assets::AssetState::Loaded => SceneAssetRuntimeStatus::Loaded,
                            zyne_assets::AssetState::Failed => {
                                SceneAssetRuntimeStatus::Failed {
                                    reason: cache
                                        .failure_reason(resource_id)
                                        .unwrap_or("asset load failed without a diagnostic")
                                        .to_owned(),
                                }
                            }
                        },
                    }
                }
            };

            report
                .references
                .push(ResolvedSceneAssetReference {
                    entity_id,
                    resource_id,
                    status,
                });
        }
    }

    Ok(report)
}

#[cfg(test)]
mod runtime_status_tests {
    use super::{SceneAssetResolverError, SceneAssetRuntimeStatus, resolve_scene_asset_statuses};
    use crate::Scene;
    use zyne_assets::{
        AssetCache, AssetCacheStatus, AssetMetadata, AssetRegistry, ImporterSettings,
        ProjectRelativePath, ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;

    struct MeshData;
    struct TextureData;
    struct AudioData;

    fn register(registry: &mut AssetRegistry, id: ResourceId, kind: ResourceKind, path: &str) {
        let metadata = AssetMetadata::new(
            id,
            path,
            kind,
            ProjectRelativePath::new(path).unwrap(),
            ImporterSettings::new("test", 1).unwrap(),
        )
        .unwrap();
        registry.register(metadata).unwrap();
    }

    #[test]
    fn resolves_all_runtime_states_across_multiple_typed_caches() {
        let mut scene = Scene::new("Runtime Asset States").unwrap();
        let entity = scene.spawn_entity("Actor", Transform3d::identity()).unwrap();
        let mesh_id = ResourceId::new();
        let texture_id = ResourceId::new();
        let audio_id = ResourceId::new();
        let no_cache_id = ResourceId::new();
        let unregistered_id = ResourceId::new();

        let mut registry = AssetRegistry::new();
        register(&mut registry, mesh_id, ResourceKind::Mesh, "mesh.glb");
        register(&mut registry, texture_id, ResourceKind::Texture, "texture.png");
        register(&mut registry, audio_id, ResourceKind::Audio, "audio.ogg");
        register(&mut registry, no_cache_id, ResourceKind::Model, "model.glb");
        scene.set_entity_asset_references(
            entity,
            &[mesh_id, texture_id, audio_id, no_cache_id, unregistered_id],
        ).unwrap();

        let mut meshes = AssetCache::<MeshData>::new(ResourceKind::Mesh);
        meshes.begin_loading(&registry, mesh_id).unwrap();
        meshes.complete_loading(&registry, mesh_id, MeshData).unwrap();

        let mut textures = AssetCache::<TextureData>::new(ResourceKind::Texture);
        textures.begin_loading(&registry, texture_id).unwrap();

        let mut audio = AssetCache::<AudioData>::new(ResourceKind::Audio);
        audio.begin_loading(&registry, audio_id).unwrap();
        audio.fail_loading(audio_id, "unsupported audio codec").unwrap();

        let caches: [&dyn AssetCacheStatus; 3] = [&meshes, &textures, &audio];
        let report = resolve_scene_asset_statuses(&scene, &registry, &caches).unwrap();

        assert_eq!(report.references().len(), 5);
        assert_eq!(report.loaded_count(), 1);
        assert_eq!(report.pending_count(), 1);
        assert_eq!(report.failed_count(), 1);
        assert!(!report.is_ready());

        let statuses = report.references().iter().map(|reference| {
            (reference.resource_id(), reference.status().clone())
        }).collect::<Vec<_>>();
        assert_eq!(statuses, vec![
            (mesh_id, SceneAssetRuntimeStatus::Loaded),
            (texture_id, SceneAssetRuntimeStatus::Loading),
            (audio_id, SceneAssetRuntimeStatus::Failed {
                reason: "unsupported audio codec".to_owned(),
            }),
            (no_cache_id, SceneAssetRuntimeStatus::CacheUnavailable {
                kind: ResourceKind::Model,
            }),
            (unregistered_id, SceneAssetRuntimeStatus::Unregistered),
        ]);
    }

    #[test]
    fn reports_unloaded_state_when_matching_cache_has_not_seen_asset() {
        let mut scene = Scene::new("Unloaded Asset").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(&mut registry, id, ResourceKind::Mesh, "prop.glb");
        scene.set_entity_asset_references(entity, &[id]).unwrap();

        let meshes = AssetCache::<MeshData>::new(ResourceKind::Mesh);
        let caches: [&dyn AssetCacheStatus; 1] = [&meshes];
        let report = resolve_scene_asset_statuses(&scene, &registry, &caches).unwrap();

        assert_eq!(report.references()[0].status(), &SceneAssetRuntimeStatus::Unloaded);
        assert_eq!(report.pending_count(), 1);
    }

    #[test]
    fn rejects_duplicate_cache_kinds_instead_of_resolving_ambiguously() {
        let scene = Scene::new("Duplicate Cache Kinds").unwrap();
        let registry = AssetRegistry::new();
        let first = AssetCache::<MeshData>::new(ResourceKind::Mesh);
        let second = AssetCache::<MeshData>::new(ResourceKind::Mesh);
        let caches: [&dyn AssetCacheStatus; 2] = [&first, &second];

        assert_eq!(
            resolve_scene_asset_statuses(&scene, &registry, &caches),
            Err(SceneAssetResolverError::DuplicateCacheKind {
                kind: ResourceKind::Mesh,
            }),
        );
    }

    #[test]
    fn empty_scene_is_ready_without_asset_references() {
        let scene = Scene::new("No Assets").unwrap();
        let report = resolve_scene_asset_statuses(&scene, &AssetRegistry::new(), &[]).unwrap();

        assert!(report.is_ready());
        assert_eq!(report.references().len(), 0);
    }
}
