//! Scene-to-GPU mesh synchronization and render extraction.

use std::collections::HashMap;
use std::fmt;

use zyne_assets::{AssetCache, AssetRegistry, BlinnPhongMaterial, ResourceKind, TextureData};
use zyne_core::Engine;
use zyne_core::ResourceId;
use zyne_graphics::{
    DirectionalLightData, FrameOutcome, GpuMaterial, GpuMesh, GpuTexture, GraphicsContext,
    GraphicsError, LightingData, MeshData, MeshRenderBatch, PointLightData, SpotLightData,
    TextureColorSpace,
};
use zyne_math::Camera3d;
use zyne_scene::{Scene, SceneEntityId};

use crate::{
    SceneMaterialResolutionIssue, SceneTextureResolutionIssue, resolve_scene_materials,
    resolve_scene_textures,
};

/// Engine contract for applications that render a live ECS scene.
pub trait SceneEngine: Engine {
    /// Returns the mutable game scene as a read-only render snapshot.
    fn scene(&self) -> &Scene;

    /// Returns persistent metadata for the scene's asset references.
    fn asset_registry(&self) -> &AssetRegistry;

    /// Returns loaded CPU mesh data referenced by the scene.
    fn mesh_cache(&self) -> &AssetCache<MeshData>;
}

/// Extended scene-engine contract for applications that render assigned materials.
pub trait SceneMaterialEngine: SceneEngine {
    /// Returns loaded CPU material data referenced by the scene.
    fn material_cache(&self) -> &AssetCache<BlinnPhongMaterial>;
}

/// Extended scene-engine contract for materials with diffuse texture references.
pub trait SceneTextureEngine: SceneMaterialEngine {
    /// Returns loaded CPU texture data referenced by scene materials.
    fn texture_cache(&self) -> &AssetCache<TextureData>;
}

/// Owns GPU mesh uploads referenced by a scene.
#[derive(Default)]
pub struct SceneMeshRenderer {
    meshes: HashMap<ResourceId, GpuMesh>,
}

/// Owns GPU material uploads referenced by a scene.
#[derive(Default)]
pub struct SceneMaterialRenderer {
    materials: HashMap<ResourceId, GpuMaterial>,
    textures: HashMap<ResourceId, GpuTexture>,
    material_textures: HashMap<ResourceId, Option<ResourceId>>,
}

impl SceneMaterialRenderer {
    /// Creates an empty scene material renderer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads referenced material assets that are not resident on the GPU yet.
    ///
    /// Shared material references are uploaded only once and reused by all entities.
    ///
    /// # Errors
    ///
    /// Returns an error if a scene assignment cannot be resolved or GPU material
    /// creation fails in the graphics backend.
    pub fn sync_materials(
        &mut self,
        graphics: &GraphicsContext,
        scene: &Scene,
        registry: &AssetRegistry,
        material_cache: &AssetCache<BlinnPhongMaterial>,
    ) -> Result<usize, SceneRenderError> {
        let report = resolve_scene_materials(scene, registry, material_cache);
        if let Some(unresolved) = report.unresolved().first() {
            return Err(SceneRenderError::UnresolvedMaterial {
                entity_id: unresolved.entity_id(),
                resource_id: unresolved.resource_id(),
                issue: unresolved.issue(),
            });
        }

        let mut uploaded = 0;
        for resolved in report.resolved() {
            self.material_textures.insert(
                resolved.resource_id(),
                resolved.material().diffuse_texture(),
            );
            if self.materials.contains_key(&resolved.resource_id()) {
                continue;
            }
            let gpu_material = graphics.create_material(&resolved.render_data());
            self.materials.insert(resolved.resource_id(), gpu_material);
            uploaded += 1;
        }
        Ok(uploaded)
    }

    /// Uploads referenced diffuse textures once and reuses shared GPU allocations.
    ///
    /// Call after material synchronization so assignments are up to date.
    ///
    /// # Errors
    ///
    /// Returns an error when a diffuse reference cannot be resolved or GPU upload fails.
    pub fn sync_textures(
        &mut self,
        graphics: &GraphicsContext,
        scene: &Scene,
        registry: &AssetRegistry,
        material_cache: &AssetCache<BlinnPhongMaterial>,
        texture_cache: &AssetCache<TextureData>,
    ) -> Result<usize, SceneRenderError> {
        let report = resolve_scene_textures(scene, registry, material_cache, texture_cache);
        if let Some(unresolved) = report.unresolved().first() {
            return Err(SceneRenderError::UnresolvedTexture {
                entity_id: unresolved.entity_id(),
                material_id: unresolved.material_id(),
                resource_id: unresolved.texture_id(),
                issue: unresolved.issue(),
            });
        }

        let mut uploaded = 0;
        for resolved in report.resolved() {
            if self.textures.contains_key(&resolved.texture_id()) {
                continue;
            }
            let data = texture_cache.get(resolved.texture_id()).ok_or(
                SceneRenderError::MissingCpuTexture {
                    resource_id: resolved.texture_id(),
                },
            )?;
            let color_space = match data.color_space() {
                zyne_assets::TextureColorSpace::Srgb => TextureColorSpace::Srgb,
                zyne_assets::TextureColorSpace::Linear => TextureColorSpace::Linear,
            };
            let texture = graphics
                .create_texture(data.width(), data.height(), color_space, data.pixels())
                .map_err(SceneRenderError::Graphics)?;
            self.textures.insert(resolved.texture_id(), texture);
            uploaded += 1;
        }
        Ok(uploaded)
    }

    /// Removes a GPU diffuse texture upload and returns whether one was resident.
    pub fn unload_texture(&mut self, resource_id: ResourceId) -> bool {
        self.textures.remove(&resource_id).is_some()
    }

    /// Removes a GPU material upload and returns whether one was resident.
    pub fn unload_material(&mut self, resource_id: ResourceId) -> bool {
        self.material_textures.remove(&resource_id);
        self.materials.remove(&resource_id).is_some()
    }

    /// Returns the number of GPU material uploads owned by this renderer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Returns whether this renderer has no uploaded materials.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

impl SceneMeshRenderer {
    /// Creates an empty scene mesh renderer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads referenced mesh assets that are not resident on the GPU yet.
    ///
    /// Only references registered as `ResourceKind::Mesh` are considered.
    /// Existing GPU uploads are retained and reused.
    ///
    /// # Errors
    ///
    /// Returns an error when a registered mesh has no loaded CPU data or GPU
    /// upload fails.
    pub fn sync_meshes(
        &mut self,
        graphics: &GraphicsContext,
        scene: &Scene,
        registry: &AssetRegistry,
        mesh_cache: &AssetCache<MeshData>,
    ) -> Result<usize, SceneRenderError> {
        let mut uploaded = 0;
        for (resource_id, _) in collect_scene_mesh_instances(scene, registry) {
            if self.meshes.contains_key(&resource_id) {
                continue;
            }

            let data = mesh_cache
                .get(resource_id)
                .ok_or(SceneRenderError::MissingCpuMesh { resource_id })?;
            let gpu_mesh = graphics
                .create_mesh(data)
                .map_err(SceneRenderError::Graphics)?;
            self.meshes.insert(resource_id, gpu_mesh);
            uploaded += 1;
        }
        Ok(uploaded)
    }

    /// Draws scene mesh instances using their persistent material assignments.
    ///
    /// Instances are grouped by mesh and material so shared combinations use
    /// one instanced indexed draw while different materials remain independent.
    ///
    /// # Errors
    ///
    /// Returns an error when a mesh or material has not been uploaded or the
    /// graphics context cannot render the frame.
    pub fn render_scene_with_materials(
        &self,
        materials: &SceneMaterialRenderer,
        graphics: &mut GraphicsContext,
        scene: &Scene,
        registry: &AssetRegistry,
        camera: Option<&Camera3d>,
    ) -> Result<FrameOutcome, SceneRenderError> {
        let mut draw_instances = Vec::new();
        for (mesh_id, material_id, transform) in
            collect_scene_mesh_material_instances(scene, registry)
        {
            if !self.meshes.contains_key(&mesh_id) {
                return Err(SceneRenderError::MissingGpuMesh {
                    resource_id: mesh_id,
                });
            }
            if let Some(resource_id) = material_id {
                if !materials.materials.contains_key(&resource_id) {
                    return Err(SceneRenderError::MissingGpuMaterial { resource_id });
                }
            }
            let texture_id =
                material_id.and_then(|id| materials.material_textures.get(&id).copied().flatten());
            if let Some(resource_id) = texture_id {
                if !materials.textures.contains_key(&resource_id) {
                    return Err(SceneRenderError::MissingGpuTexture { resource_id });
                }
            }
            draw_instances.push((mesh_id, material_id, texture_id, transform));
        }
        let grouped = group_scene_draw_instances(draw_instances);

        let mut groups: Vec<_> = grouped.into_iter().collect();
        groups.sort_unstable_by_key(|((mesh_id, material_id, texture_id), _)| {
            (
                *mesh_id.as_uuid().as_bytes(),
                material_id.map_or([0; 16], |id| *id.as_uuid().as_bytes()),
                texture_id.map_or([0; 16], |id| *id.as_uuid().as_bytes()),
            )
        });
        let batches: Vec<_> = groups
            .iter()
            .map(|((mesh_id, material_id, texture_id), transforms)| {
                let mesh = self
                    .meshes
                    .get(mesh_id)
                    .ok_or(SceneRenderError::MissingGpuMesh {
                        resource_id: *mesh_id,
                    })?;
                let material = material_id
                    .map(|resource_id| {
                        materials
                            .materials
                            .get(&resource_id)
                            .ok_or(SceneRenderError::MissingGpuMaterial { resource_id })
                    })
                    .transpose()?;
                let texture = texture_id
                    .map(|resource_id| {
                        materials
                            .textures
                            .get(&resource_id)
                            .ok_or(SceneRenderError::MissingGpuTexture { resource_id })
                    })
                    .transpose()?;
                match (material, texture) {
                    (Some(material), Some(texture)) => {
                        Ok(MeshRenderBatch::with_material_and_texture(
                            mesh, transforms, material, texture,
                        ))
                    }
                    (Some(material), None) => {
                        Ok(MeshRenderBatch::with_material(mesh, transforms, material))
                    }
                    (None, Some(texture)) => {
                        Ok(MeshRenderBatch::with_texture(mesh, transforms, texture))
                    }
                    (None, None) => Ok(MeshRenderBatch::new(mesh, transforms)),
                }
            })
            .collect::<Result<_, SceneRenderError>>()?;

        let lighting = collect_scene_lighting(scene);
        graphics
            .render_frame_with_mesh_batches_and_lighting(camera, &batches, &lighting)
            .map_err(SceneRenderError::Graphics)
    }

    /// Removes a GPU mesh upload and returns whether one was resident.
    pub fn unload_mesh(&mut self, resource_id: ResourceId) -> bool {
        self.meshes.remove(&resource_id).is_some()
    }

    /// Returns the number of GPU mesh uploads owned by this renderer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    /// Returns whether this renderer has no uploaded meshes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Draws every scene entity that references a resident mesh asset.
    ///
    /// Entities sharing one mesh are grouped into one instanced indexed draw.
    /// Meshes are submitted in deterministic resource-ID order.
    ///
    /// # Errors
    ///
    /// Returns an error when a referenced mesh has not been uploaded or the
    /// graphics context cannot render the frame.
    pub fn render_scene(
        &self,
        graphics: &mut GraphicsContext,
        scene: &Scene,
        registry: &AssetRegistry,
        camera: Option<&Camera3d>,
    ) -> Result<FrameOutcome, SceneRenderError> {
        let mut grouped: HashMap<ResourceId, Vec<zyne_math::Transform3d>> = HashMap::new();
        for (resource_id, transform) in collect_scene_mesh_instances(scene, registry) {
            if !self.meshes.contains_key(&resource_id) {
                return Err(SceneRenderError::MissingGpuMesh { resource_id });
            }
            grouped.entry(resource_id).or_default().push(transform);
        }

        let mut groups: Vec<_> = grouped.into_iter().collect();
        groups.sort_unstable_by_key(|(id, _)| *id.as_uuid().as_bytes());
        let batches: Vec<_> = groups
            .iter()
            .map(|(resource_id, transforms)| {
                self.meshes
                    .get(resource_id)
                    .map(|mesh| MeshRenderBatch::new(mesh, transforms))
                    .ok_or(SceneRenderError::MissingGpuMesh {
                        resource_id: *resource_id,
                    })
            })
            .collect::<Result<_, _>>()?;

        let lighting = collect_scene_lighting(scene);
        graphics
            .render_frame_with_mesh_batches_and_lighting(camera, &batches, &lighting)
            .map_err(SceneRenderError::Graphics)
    }
}

/// Errors produced while synchronizing or rendering scene meshes.
#[derive(Debug)]
pub enum SceneRenderError {
    /// A referenced mesh is registered but its CPU cache value is not loaded.
    MissingCpuMesh { resource_id: ResourceId },
    /// A referenced mesh has not been uploaded to this renderer.
    MissingGpuMesh { resource_id: ResourceId },
    /// A scene material reference could not be resolved from the CPU asset cache.
    UnresolvedMaterial {
        entity_id: SceneEntityId,
        resource_id: ResourceId,
        issue: SceneMaterialResolutionIssue,
    },
    /// A referenced material has not been uploaded to this renderer.
    MissingGpuMaterial { resource_id: ResourceId },
    /// A referenced texture has no loaded CPU payload.
    MissingCpuTexture { resource_id: ResourceId },
    /// A referenced texture has not been uploaded to the GPU.
    MissingGpuTexture { resource_id: ResourceId },
    /// A diffuse texture reference could not be resolved.
    UnresolvedTexture {
        entity_id: SceneEntityId,
        material_id: ResourceId,
        resource_id: ResourceId,
        issue: SceneTextureResolutionIssue,
    },
    /// The graphics backend rejected an upload or frame.
    Graphics(GraphicsError),
}

impl fmt::Display for SceneRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCpuMesh { resource_id } => {
                write!(
                    formatter,
                    "mesh asset {resource_id} is not loaded in the CPU cache"
                )
            }
            Self::MissingGpuMesh { resource_id } => {
                write!(
                    formatter,
                    "mesh asset {resource_id} has not been uploaded to the GPU"
                )
            }
            Self::UnresolvedMaterial {
                entity_id,
                resource_id,
                issue,
            } => write!(
                formatter,
                "material {resource_id} assigned to entity {entity_id:?} could not be resolved: {issue:?}"
            ),
            Self::MissingGpuMaterial { resource_id } => write!(
                formatter,
                "material asset {resource_id} has not been uploaded to the GPU"
            ),
            Self::MissingCpuTexture { resource_id } => write!(
                formatter,
                "texture asset {resource_id} is not loaded in the CPU cache"
            ),
            Self::MissingGpuTexture { resource_id } => write!(
                formatter,
                "texture asset {resource_id} has not been uploaded to the GPU"
            ),
            Self::UnresolvedTexture {
                entity_id,
                material_id,
                resource_id,
                issue,
            } => write!(
                formatter,
                "diffuse texture {resource_id} referenced by material {material_id} on entity {entity_id:?} could not be resolved: {issue:?}"
            ),
            Self::Graphics(error) => write!(formatter, "scene rendering failed: {error}"),
        }
    }
}

impl std::error::Error for SceneRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Graphics(error) => Some(error),
            Self::MissingCpuMesh { .. }
            | Self::MissingGpuMesh { .. }
            | Self::UnresolvedMaterial { .. }
            | Self::MissingGpuMaterial { .. }
            | Self::MissingCpuTexture { .. }
            | Self::MissingGpuTexture { .. }
            | Self::UnresolvedTexture { .. } => None,
        }
    }
}

fn collect_scene_lighting(scene: &Scene) -> LightingData {
    let mut lighting = LightingData::with_ambient([0.30, 0.30, 0.32]);
    for entity_id in scene.entity_ids() {
        let Some(transform) = scene.entity_transform(entity_id) else {
            continue;
        };
        let rotation = zyne_math::Transform3d::new([0.0; 3], transform.rotation(), [1.0; 3])
            .expect("scene transforms contain validated rotations")
            .to_matrix();

        if let Some(light) = scene.entity_directional_light(entity_id) {
            lighting.directional_lights.push(DirectionalLightData {
                direction: transform_direction(rotation, light.direction()),
                color: light.color(),
                intensity: light.intensity(),
            });
        }
        if let Some(light) = scene.entity_point_light(entity_id) {
            lighting.point_lights.push(PointLightData {
                position: transform.translation(),
                color: light.color(),
                intensity: light.intensity(),
                range: light.range(),
            });
        }
        if let Some(light) = scene.entity_spot_light(entity_id) {
            lighting.spot_lights.push(SpotLightData {
                position: transform.translation(),
                direction: transform_direction(rotation, light.direction()),
                color: light.color(),
                intensity: light.intensity(),
                range: light.range(),
                inner_angle: light.inner_angle(),
                outer_angle: light.outer_angle(),
            });
        }
    }
    lighting
}

fn transform_direction(matrix: [[f32; 4]; 4], direction: [f32; 3]) -> [f32; 3] {
    let transformed = [
        matrix[0][0] * direction[0] + matrix[1][0] * direction[1] + matrix[2][0] * direction[2],
        matrix[0][1] * direction[0] + matrix[1][1] * direction[1] + matrix[2][1] * direction[2],
        matrix[0][2] * direction[0] + matrix[1][2] * direction[1] + matrix[2][2] * direction[2],
    ];
    let length = transformed
        .iter()
        .map(|component| component * component)
        .sum::<f32>()
        .sqrt();
    if length > f32::EPSILON {
        transformed.map(|component| component / length)
    } else {
        direction
    }
}

fn collect_scene_mesh_instances(
    scene: &Scene,
    registry: &AssetRegistry,
) -> Vec<(ResourceId, zyne_math::Transform3d)> {
    let mut instances = Vec::new();
    for entity_id in scene.entity_ids() {
        let Some(transform) = scene.entity_transform(entity_id) else {
            continue;
        };
        let Some(references) = scene.entity_asset_references(entity_id) else {
            continue;
        };
        for resource_id in references {
            if registry
                .find(*resource_id)
                .is_some_and(|metadata| metadata.kind() == ResourceKind::Mesh)
            {
                instances.push((*resource_id, transform));
            }
        }
    }
    instances.sort_unstable_by_key(|(id, _)| *id.as_uuid().as_bytes());
    instances
}

fn group_scene_draw_instances(
    instances: impl IntoIterator<
        Item = (
            ResourceId,
            Option<ResourceId>,
            Option<ResourceId>,
            zyne_math::Transform3d,
        ),
    >,
) -> HashMap<
    (ResourceId, Option<ResourceId>, Option<ResourceId>),
    Vec<zyne_math::Transform3d>,
> {
    let mut grouped = HashMap::new();
    for (mesh_id, material_id, texture_id, transform) in instances {
        grouped
            .entry((mesh_id, material_id, texture_id))
            .or_insert_with(Vec::new)
            .push(transform);
    }
    grouped
}

fn collect_scene_mesh_material_instances(
    scene: &Scene,
    registry: &AssetRegistry,
) -> Vec<(ResourceId, Option<ResourceId>, zyne_math::Transform3d)> {
    let mut instances = Vec::new();
    for entity_id in scene.entity_ids() {
        let Some(transform) = scene.entity_transform(entity_id) else {
            continue;
        };
        let Some(references) = scene.entity_asset_references(entity_id) else {
            continue;
        };
        let material_id = scene.entity_material_reference(entity_id);
        for resource_id in references {
            if registry
                .find(*resource_id)
                .is_some_and(|metadata| metadata.kind() == ResourceKind::Mesh)
            {
                instances.push((*resource_id, material_id, transform));
            }
        }
    }
    instances.sort_unstable_by_key(|(mesh_id, material_id, _)| {
        (
            *mesh_id.as_uuid().as_bytes(),
            material_id.map_or([0; 16], |id| *id.as_uuid().as_bytes()),
        )
    });
    instances
}

#[cfg(test)]
mod tests {
    use zyne_assets::{
        AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;
    use zyne_scene::Scene;

    use super::{collect_scene_mesh_instances, collect_scene_mesh_material_instances};

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

    #[test]
    fn extracts_mesh_instances_and_ignores_non_mesh_references() {
        let mut scene = Scene::new("Mesh Extraction").unwrap();
        let first = scene
            .spawn_entity(
                "First",
                Transform3d::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]).unwrap(),
            )
            .unwrap();
        let second = scene
            .spawn_entity(
                "Second",
                Transform3d::new([2.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]).unwrap(),
            )
            .unwrap();

        let mesh_id = ResourceId::new();
        let texture_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(&mut registry, mesh_id, ResourceKind::Mesh, "mesh.glb");
        register(
            &mut registry,
            texture_id,
            ResourceKind::Texture,
            "texture.png",
        );

        scene
            .set_entity_asset_references(first, &[mesh_id, texture_id])
            .unwrap();
        scene
            .set_entity_asset_references(second, &[mesh_id])
            .unwrap();

        let instances = collect_scene_mesh_instances(&scene, &registry);
        assert_eq!(instances.len(), 2);
        assert!(instances.iter().all(|(id, _)| *id == mesh_id));
        assert!(instances.iter().any(|(_, transform)| {
            transform
                .translation()
                .iter()
                .zip([1.0, 0.0, 0.0])
                .all(|(actual, expected)| (actual - expected).abs() < 1.0e-6)
        }));
        assert!(instances.iter().any(|(_, transform)| {
            transform
                .translation()
                .iter()
                .zip([2.0, 0.0, 0.0])
                .all(|(actual, expected)| (actual - expected).abs() < 1.0e-6)
        }));
    }

    #[test]
    fn extracts_mesh_instances_with_distinct_material_assignments() {
        let mut scene = Scene::new("Mesh Material Extraction").unwrap();
        let first = scene
            .spawn_entity("First", Transform3d::identity())
            .unwrap();
        let second = scene
            .spawn_entity("Second", Transform3d::identity())
            .unwrap();
        let mesh_id = ResourceId::new();
        let first_material = ResourceId::new();
        let second_material = ResourceId::new();
        let mut registry = AssetRegistry::new();
        register(&mut registry, mesh_id, ResourceKind::Mesh, "shared.mesh");
        scene
            .set_entity_asset_references(first, &[mesh_id])
            .unwrap();
        scene
            .set_entity_asset_references(second, &[mesh_id])
            .unwrap();
        scene.set_entity_material_reference(first, Some(first_material));
        scene.set_entity_material_reference(second, Some(second_material));

        let instances = collect_scene_mesh_material_instances(&scene, &registry);

        assert_eq!(instances.len(), 2);
        assert!(instances.iter().all(|(id, _, _)| *id == mesh_id));
        let materials: Vec<_> = instances
            .iter()
            .filter_map(|(_, material_id, _)| *material_id)
            .collect();
        assert_eq!(materials.len(), 2);
        assert!(materials.contains(&first_material));
        assert!(materials.contains(&second_material));
    }

    #[test]
    fn draw_grouping_keeps_distinct_texture_bindings_separate() {
        use super::group_scene_draw_instances;

        let mesh_id = ResourceId::new();
        let material_id = ResourceId::new();
        let first_texture = ResourceId::new();
        let second_texture = ResourceId::new();
        let first_transform = Transform3d::new(
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0; 3],
        )
        .unwrap();
        let second_transform = Transform3d::new(
            [2.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0; 3],
        )
        .unwrap();

        let grouped = group_scene_draw_instances([
            (mesh_id, Some(material_id), Some(first_texture), first_transform),
            (mesh_id, Some(material_id), Some(first_texture), second_transform),
            (mesh_id, Some(material_id), Some(second_texture), Transform3d::identity()),
        ]);

        assert_eq!(grouped.len(), 2);
        assert_eq!(
            grouped
                .get(&(mesh_id, Some(material_id), Some(first_texture)))
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            grouped
                .get(&(mesh_id, Some(material_id), Some(second_texture)))
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn draw_grouping_preserves_checker_fallback_as_distinct_binding() {
        use super::group_scene_draw_instances;

        let mesh_id = ResourceId::new();
        let material_id = ResourceId::new();
        let texture_id = ResourceId::new();
        let grouped = group_scene_draw_instances([
            (mesh_id, Some(material_id), None, Transform3d::identity()),
            (
                mesh_id,
                Some(material_id),
                Some(texture_id),
                Transform3d::identity(),
            ),
        ]);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key(&(mesh_id, Some(material_id), None)));
        assert!(grouped.contains_key(&(mesh_id, Some(material_id), Some(texture_id))));
    }

    #[test]
    fn directional_shadow_owner_follows_scene_creation_order() {
        use zyne_scene::DirectionalLight;

        let mut scene = Scene::new("Directional Shadow Order").unwrap();
        let key = scene
            .spawn_entity("Key Light", Transform3d::identity())
            .unwrap();
        let fill = scene
            .spawn_entity("Fill Light", Transform3d::identity())
            .unwrap();
        scene.set_entity_directional_light(
            key,
            Some(DirectionalLight::new([0.0, -1.0, 0.0], [1.0; 3], 2.0).unwrap()),
        );
        scene.set_entity_directional_light(
            fill,
            Some(DirectionalLight::new([1.0, -1.0, 0.0], [1.0; 3], 0.5).unwrap()),
        );

        let lighting = super::collect_scene_lighting(&scene);

        assert_eq!(lighting.directional_lights.len(), 2);
        assert_eq!(lighting.directional_lights[0].direction, [0.0, -1.0, 0.0]);
        let expected_fill_direction = [
            std::f32::consts::FRAC_1_SQRT_2,
            -std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        ];
        assert!(
            lighting.directional_lights[1]
                .direction
                .iter()
                .zip(expected_fill_direction)
                .all(|(actual, expected)| (actual - expected).abs() < 1.0e-6)
        );
    }

    #[test]
    fn extracts_world_space_directional_point_and_spot_lights() {
        use zyne_scene::{DirectionalLight, PointLight, SpotLight};

        let mut scene = Scene::new("Lighting Extraction").unwrap();
        let entity = scene
            .spawn_entity(
                "Light",
                Transform3d::new(
                    [3.0, 4.0, 5.0],
                    [
                        0.0,
                        std::f32::consts::FRAC_1_SQRT_2,
                        0.0,
                        std::f32::consts::FRAC_1_SQRT_2,
                    ],
                    [1.0; 3],
                )
                .unwrap(),
            )
            .unwrap();
        scene.set_entity_directional_light(
            entity,
            Some(DirectionalLight::new([0.0, 0.0, -1.0], [1.0; 3], 2.0).unwrap()),
        );
        scene.set_entity_point_light(entity, Some(PointLight::new([1.0; 3], 3.0, 8.0).unwrap()));
        scene.set_entity_spot_light(
            entity,
            Some(
                SpotLight::new(
                    [0.0, 0.0, -1.0],
                    [1.0; 3],
                    4.0,
                    10.0,
                    15.0_f32.to_radians(),
                    25.0_f32.to_radians(),
                )
                .unwrap(),
            ),
        );

        let lighting = super::collect_scene_lighting(&scene);
        assert_eq!(lighting.directional_lights.len(), 1);
        assert_eq!(lighting.point_lights.len(), 1);
        assert_eq!(lighting.spot_lights.len(), 1);
        assert_eq!(lighting.point_lights[0].position, [3.0, 4.0, 5.0]);
        assert!((lighting.directional_lights[0].direction[0] + 1.0).abs() < 1.0e-5);
        assert!(lighting.directional_lights[0].direction[2].abs() < 1.0e-5);
        assert!((lighting.spot_lights[0].direction[0] + 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn scene_without_lights_uses_ambient_only_lighting_snapshot() {
        let scene = Scene::new("Ambient Only").unwrap();
        let lighting = super::collect_scene_lighting(&scene);
        assert_eq!(lighting.ambient_color, [0.30, 0.30, 0.32]);
        assert_eq!(lighting.directional_lights.as_slice(), &[]);
        assert_eq!(lighting.point_lights.as_slice(), &[]);
        assert_eq!(lighting.spot_lights.as_slice(), &[]);
    }

    #[test]
    fn ignores_unregistered_scene_asset_references() {
        let mut scene = Scene::new("Unregistered Mesh").unwrap();
        let entity = scene.spawn_entity("Prop", Transform3d::identity()).unwrap();
        let missing_id = ResourceId::new();
        scene
            .set_entity_asset_references(entity, &[missing_id])
            .unwrap();

        assert_eq!(
            collect_scene_mesh_instances(&scene, &AssetRegistry::new()),
            Vec::new()
        );
    }
}
