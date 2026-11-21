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

    /// Converts this asset material into renderer-facing CPU data.
    #[must_use]
    pub fn render_data(&self) -> zyne_graphics::MaterialData {
        zyne_graphics::MaterialData::new(
            self.material.ambient(),
            self.material.diffuse(),
            self.material.specular(),
            self.material.shininess(),
        )
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

/// A diffuse texture reference resolved for one scene entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSceneTexture {
    entity: SceneEntityId,
    material: ResourceId,
    texture: ResourceId,
}

impl ResolvedSceneTexture {
    /// Returns the entity that owns the material assignment.
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity
    }

    /// Returns the persistent material resource identity.
    #[must_use]
    pub const fn material_id(&self) -> ResourceId {
        self.material
    }

    /// Returns the persistent diffuse texture resource identity.
    #[must_use]
    pub const fn texture_id(&self) -> ResourceId {
        self.texture
    }
}

/// Why a material's diffuse texture reference could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTextureResolutionIssue {
    /// The referenced texture ID is absent from the asset registry.
    NotRegistered,
    /// The referenced ID belongs to another asset category.
    WrongResourceKind { actual: ResourceKind },
    /// The texture is registered but is not loaded in the CPU cache.
    NotLoaded,
}

/// One unresolved diffuse texture reference and its owning scene entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedSceneTexture {
    entity_id: SceneEntityId,
    material_id: ResourceId,
    texture_id: ResourceId,
    issue: SceneTextureResolutionIssue,
}

impl UnresolvedSceneTexture {
    /// Returns the entity that owns the material assignment.
    #[must_use]
    pub const fn entity_id(&self) -> SceneEntityId {
        self.entity_id
    }

    /// Returns the material containing the texture reference.
    #[must_use]
    pub const fn material_id(&self) -> ResourceId {
        self.material_id
    }

    /// Returns the unresolved texture resource identity.
    #[must_use]
    pub const fn texture_id(&self) -> ResourceId {
        self.texture_id
    }

    /// Returns why the texture could not be resolved.
    #[must_use]
    pub const fn issue(&self) -> SceneTextureResolutionIssue {
        self.issue
    }
}

/// Deterministic result of resolving diffuse texture references in scene materials.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneTextureResolutionReport {
    resolved: Vec<ResolvedSceneTexture>,
    unresolved: Vec<UnresolvedSceneTexture>,
}

impl SceneTextureResolutionReport {
    /// Returns all resolved diffuse texture assignments.
    #[must_use]
    pub fn resolved(&self) -> &[ResolvedSceneTexture] {
        &self.resolved
    }

    /// Returns all unresolved diffuse texture assignments.
    #[must_use]
    pub fn unresolved(&self) -> &[UnresolvedSceneTexture] {
        &self.unresolved
    }

    /// Returns true when every referenced diffuse texture is loaded and valid.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

/// Resolves optional diffuse texture IDs from loaded scene materials against
/// registry metadata and the typed CPU texture cache. Materials without a
/// diffuse texture are intentionally omitted so the renderer can use its
/// explicit fallback texture.
#[must_use]
pub fn resolve_scene_textures(
    scene: &Scene,
    registry: &AssetRegistry,
    material_cache: &AssetCache<BlinnPhongMaterial>,
    texture_cache: &AssetCache<zyne_assets::TextureData>,
) -> SceneTextureResolutionReport {
    let mut report = SceneTextureResolutionReport::default();

    for entity_id in scene.entity_ids() {
        let Some(material_id) = scene.entity_material_reference(entity_id) else {
            continue;
        };
        let Some(material) = material_cache.get(material_id) else {
            continue;
        };
        let Some(texture_id) = material.diffuse_texture() else {
            continue;
        };

        let issue = match registry.find(texture_id) {
            None => Some(SceneTextureResolutionIssue::NotRegistered),
            Some(metadata) if metadata.kind() != ResourceKind::Texture => {
                Some(SceneTextureResolutionIssue::WrongResourceKind {
                    actual: metadata.kind(),
                })
            }
            Some(_) if texture_cache.get(texture_id).is_none() => {
                Some(SceneTextureResolutionIssue::NotLoaded)
            }
            Some(_) => None,
        };

        if let Some(issue) = issue {
            report.unresolved.push(UnresolvedSceneTexture {
                entity_id,
                material_id,
                texture_id,
                issue,
            });
        } else {
            report.resolved.push(ResolvedSceneTexture {
                entity_id,
                material_id,
                texture_id,
            });
        }
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

    use super::{
        SceneMaterialResolutionIssue, SceneTextureResolutionIssue, resolve_scene_materials,
        resolve_scene_textures,
    };

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
        assert_eq!(
            report.resolved()[0].render_data(),
            zyne_graphics::MaterialData::new(
                [0.05, 0.05, 0.05],
                [0.6, 0.7, 0.8],
                [0.2, 0.2, 0.2],
                48.0,
            )
        );
    }

    #[test]
    fn reports_unregistered_material_references() {
        let mut scene = Scene::new("Missing Material").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let missing_id = ResourceId::new();
        scene.set_entity_material_reference(entity, Some(missing_id));

        let report =
            resolve_scene_materials(&scene, &AssetRegistry::new(), &AssetCache::new_material());

        assert_eq!(report.resolved(), &[]);
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
        assert_eq!(report.resolved(), &[]);
        assert_eq!(report.unresolved(), &[]);
    }

    #[test]
    fn resolves_loaded_diffuse_texture_reference() {
        use zyne_assets::{TextureColorSpace, TextureData};

        let mut scene = Scene::new("Textured Material").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let material_id = ResourceId::new();
        let texture_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            material_id,
            ResourceKind::Material,
            "materials/brick.ron",
        );
        register(
            &mut registry,
            texture_id,
            ResourceKind::Texture,
            "textures/brick.png",
        );
        scene.set_entity_material_reference(entity, Some(material_id));

        let mut material = BlinnPhongMaterial::default();
        material.set_diffuse_texture(Some(texture_id));
        let mut materials = AssetCache::new_material();
        load_material(&registry, &mut materials, material_id, material);

        let mut textures = AssetCache::new(ResourceKind::Texture);
        textures.begin_loading(&registry, texture_id).unwrap();
        textures
            .complete_loading(
                &registry,
                texture_id,
                TextureData::new_rgba8(1, 1, TextureColorSpace::Srgb, vec![255, 255, 255, 255])
                    .unwrap(),
            )
            .unwrap();

        let report = resolve_scene_textures(&scene, &registry, &materials, &textures);
        assert!(report.is_complete());
        assert_eq!(report.resolved().len(), 1);
        assert_eq!(report.resolved()[0].entity_id(), entity);
        assert_eq!(report.resolved()[0].material_id(), material_id);
        assert_eq!(report.resolved()[0].texture_id(), texture_id);
    }

    #[test]
    fn reports_missing_diffuse_texture_reference_without_silent_fallback() {
        let mut scene = Scene::new("Missing Diffuse Texture").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let material_id = ResourceId::new();
        let texture_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            material_id,
            ResourceKind::Material,
            "materials/missing.ron",
        );
        scene.set_entity_material_reference(entity, Some(material_id));

        let mut material = BlinnPhongMaterial::default();
        material.set_diffuse_texture(Some(texture_id));
        let mut materials = AssetCache::new_material();
        load_material(&registry, &mut materials, material_id, material);

        let report = resolve_scene_textures(
            &scene,
            &registry,
            &materials,
            &AssetCache::new(ResourceKind::Texture),
        );
        assert_eq!(report.resolved(), []);
        assert_eq!(report.unresolved().len(), 1);
        assert_eq!(report.unresolved()[0].texture_id(), texture_id);
        assert_eq!(
            report.unresolved()[0].issue(),
            SceneTextureResolutionIssue::NotRegistered
        );
    }

    #[test]
    fn ignores_materials_without_diffuse_texture_assignments() {
        let mut scene = Scene::new("Untextured Material").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let material_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(
            &mut registry,
            material_id,
            ResourceKind::Material,
            "materials/plain.ron",
        );
        scene.set_entity_material_reference(entity, Some(material_id));
        let mut materials = AssetCache::new_material();
        load_material(
            &registry,
            &mut materials,
            material_id,
            BlinnPhongMaterial::default(),
        );

        let report = resolve_scene_textures(
            &scene,
            &registry,
            &materials,
            &AssetCache::new(ResourceKind::Texture),
        );
        assert!(report.is_complete());
        assert!(report.resolved().is_empty());
        assert_eq!(report.unresolved(), []);
    }
}
