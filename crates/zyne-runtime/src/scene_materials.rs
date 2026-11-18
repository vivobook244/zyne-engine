//! Resolution of persistent scene material assignments into CPU render snapshots.

use zyne_assets::{AssetCache, AssetRegistry, BlinnPhongMaterial, ResourceKind};
use zyne_core::ResourceId;

use crate::{Scene, SceneEntityId};

/// A material assignment resolved for one scene entity.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSceneMaterial {
    entity_id: SceneEntityId,
    resource_id: ResourceId,
    material: BlinnPhongMaterial,
}

impl ResolvedSceneMaterial {
    /// Returns the entity that owns this material assignment.
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the persistent material resource identity.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the renderer-independent material parameters.
    #[must_use]
    pub const fn material(&self) -> &BlinnPhongMaterial {
        &self.material
    }
}

/// Why a scene material assignment could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneMaterialResolutionIssue {
    /// The referenced ID is absent from the asset registry.
    NotRegistered,
    /// The referenced ID belongs to another asset category.
    WrongResourceKind { actual: ResourceKind },
    /// The material is registered but has not been loaded into the CPU cache.
    NotLoaded,
}

/// One unresolved material assignment, with its owning entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedSceneMaterial {
    entity_id: SceneEntityId,
    resource_id: ResourceId,
    issue: SceneMaterialResolutionIssue,
}

impl UnresolvedSceneMaterial {
    /// Returns the entity that owns this unresolved assignment.
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the referenced persistent material identity.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the reason resolution failed.
    #[must_use]
    pub const fn issue(&self) -> SceneMaterialResolutionIssue {
        self.issue
    }
}

/// Deterministic result of resolving every material assignment in a scene.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneMaterialResolutionReport {
    resolved: Vec<ResolvedSceneMaterial>,
    unresolved: Vec<UnresolvedSceneMaterial>,
}

impl SceneMaterialResolutionReport {
    /// Returns all successfully resolved material assignments.
    #[must_use]
    pub fn resolved(&self) -> &[ResolvedSceneMaterial] {
        &self.resolved
    }

    /// Returns all assignments that could not be resolved.
    #[must_use]
    pub fn unresolved(&self) -> &[UnresolvedSceneMaterial] {
        &self.unresolved
    }

    /// Returns true when every material assignment was resolved.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

/// Resolves each entity's material reference against registry metadata and the
/// typed CPU material cache.
///
/// The returned snapshot owns cloned material values, so it contains no ECS
/// borrows or runtime handles and can be safely passed to later render
/// extraction stages. Entities and results follow persistent entity-ID order.
#[must_use]
pub fn resolve_scene_materials(
    scene: &Scene,
    registry: &AssetRegistry,
    material_cache: &AssetCache<BlinnPhongMaterial>,
) -> SceneMaterialResolutionReport {
    let mut report = SceneMaterialResolutionReport::default();

    for entity_id in scene.entity_ids() {
        let Some(resource_id) = scene.entity_material_reference(entity_id) else {
            continue;
        };

        let Some(metadata) = registry.find(resource_id) else {
            report.unresolved.push(UnresolvedSceneMaterial {
                entity_id,
                resource_id,
                issue: SceneMaterialResolutionIssue::NotRegistered,
            });
            continue;
        };

        if metadata.kind() != ResourceKind::Material {
            report.unresolved.push(UnresolvedSceneMaterial {
                entity_id,
                resource_id,
                issue: SceneMaterialResolutionIssue::WrongResourceKind {
                    actual: metadata.kind(),
                },
            });
            continue;
        }

        let Some(material) = material_cache.get(resource_id) else {
            report.unresolved.push(UnresolvedSceneMaterial {
                entity_id,
                resource_id,
                issue: SceneMaterialResolutionIssue::NotLoaded,
            });
            continue;
        };

        report.resolved.push(ResolvedSceneMaterial {
            entity_id,
            resource_id,
            material: material.clone(),
        });
    }

    report
}

#[cfg(test)]
mod tests {
    use zyne_assets::{
        AssetCache, AssetMetadata, AssetRegistry, BlinnPhongMaterial, ImporterSettings,
        ProjectRelativePath, ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;
    use zyne_scene::Scene;

    use super::{SceneMaterialResolutionIssue, resolve_scene_materials};

    fn register(registry: &mut AssetRegistry, id: ResourceId, kind: ResourceKind, path: &str) {
        registry
            .register(
                AssetMetadata::new(
                    id,
                    path,
                    kind,
                    ProjectRelativePath::new(path).unwrap(),
                    ImporterSettings::new("test", 1).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn load_material(
        registry: &AssetRegistry,
        cache: &mut AssetCache<BlinnPhongMaterial>,
        id: ResourceId,
        material: BlinnPhongMaterial,
    ) {
        cache.begin_loading(registry, id).unwrap();
        cache.complete_loading(registry, id, material).unwrap();
    }

    #[test]
    fn resolves_loaded_material_assignments_for_multiple_entities() {
        let mut scene = Scene::new("Shared Material").unwrap();
        let first = scene
            .spawn_entity("First", Transform3d::identity())
            .unwrap();
        let second = scene
            .spawn_entity("Second", Transform3d::identity())
            .unwrap();
        let material_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            material_id,
            ResourceKind::Material,
            "materials/stone.ron",
        );
        scene.set_entity_material_reference(first, Some(material_id));
        scene.set_entity_material_reference(second, Some(material_id));

        let material =
            BlinnPhongMaterial::new([0.05, 0.05, 0.05], [0.6, 0.7, 0.8], [0.2, 0.2, 0.2], 48.0)
                .unwrap();
        let mut cache = AssetCache::new_material();
        load_material(&registry, &mut cache, material_id, material.clone());

        let report = resolve_scene_materials(&scene, &registry, &cache);

        assert!(report.is_complete());
        assert_eq!(report.resolved().len(), 2);
        assert_eq!(report.resolved()[0].resource_id(), material_id);
        assert_eq!(report.resolved()[1].resource_id(), material_id);
        assert_eq!(report.resolved()[0].material(), &material);
        assert_eq!(report.resolved()[1].material(), &material);
    }

    #[test]
    fn reports_unregistered_material_references() {
        let mut scene = Scene::new("Missing Material").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let missing_id = ResourceId::new();
        scene.set_entity_material_reference(entity, Some(missing_id));

        let report =
            resolve_scene_materials(&scene, &AssetRegistry::new(), &AssetCache::new_material());

        assert!(report.resolved().is_empty());
        assert_eq!(report.unresolved().len(), 1);
        assert_eq!(
            report.unresolved()[0].issue(),
            SceneMaterialResolutionIssue::NotRegistered
        );
    }

    #[test]
    fn rejects_registered_resources_that_are_not_materials() {
        let mut scene = Scene::new("Wrong Material Kind").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let mesh_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            mesh_id,
            ResourceKind::Mesh,
            "meshes/prop.mesh",
        );
        scene.set_entity_material_reference(entity, Some(mesh_id));

        let report = resolve_scene_materials(&scene, &registry, &AssetCache::new_material());

        assert_eq!(
            report.unresolved()[0].issue(),
            SceneMaterialResolutionIssue::WrongResourceKind {
                actual: ResourceKind::Mesh
            }
        );
    }

    #[test]
    fn reports_registered_materials_that_are_not_loaded() {
        let mut scene = Scene::new("Unloaded Material").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let material_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            material_id,
            ResourceKind::Material,
            "materials/wood.ron",
        );
        scene.set_entity_material_reference(entity, Some(material_id));

        let report = resolve_scene_materials(&scene, &registry, &AssetCache::new_material());

        assert_eq!(
            report.unresolved()[0].issue(),
            SceneMaterialResolutionIssue::NotLoaded
        );
    }

    #[test]
    fn ignores_entities_without_material_assignments() {
        let mut scene = Scene::new("No Material").unwrap();
        scene.spawn_entity("Prop", Transform3d::identity()).unwrap();

        let report =
            resolve_scene_materials(&scene, &AssetRegistry::new(), &AssetCache::new_material());

        assert!(report.is_complete());
        assert!(report.resolved().is_empty());
        assert!(report.unresolved().is_empty());
    }
}
