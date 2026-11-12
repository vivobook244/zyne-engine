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
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the unresolved persistent resource identity.
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
                report
                    .missing_references
                    .push(MissingAssetReference {
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
    use uuid::Uuid;
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
        assert!(report.missing_references().is_empty());
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
        let first_entity = crate::SceneEntityId::from_uuid(Uuid::from_u128(1));
        let second_entity = crate::SceneEntityId::from_uuid(Uuid::from_u128(2));
        scene
            .spawn_entity_with_id(first_entity, "First", Transform3d::identity())
            .unwrap();
        scene
            .spawn_entity_with_id(second_entity, "Second", Transform3d::identity())
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
        assert_eq!(
            report
                .missing_references()
                .iter()
                .map(|missing| (missing.entity_id(), missing.resource_id()))
                .collect::<Vec<_>>(),
            vec![
                (first_entity, first_missing_id),
                (first_entity, second_missing_id),
                (second_entity, first_missing_id),
            ]
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
