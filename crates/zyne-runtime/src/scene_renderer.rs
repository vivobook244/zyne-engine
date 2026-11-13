//! Scene-to-GPU mesh synchronization and render extraction.

use std::collections::HashMap;
use std::fmt;

use zyne_assets::{AssetCache, AssetRegistry, ResourceKind};
use zyne_core::Engine;
use zyne_core::ResourceId;
use zyne_graphics::{
    DirectionalLightData, FrameOutcome, GpuMesh, GraphicsContext, GraphicsError, LightingData,
    MeshData, MeshRenderBatch, PointLightData, SpotLightData,
};
use zyne_math::Camera3d;
use zyne_scene::Scene;

/// Engine contract for applications that render a live ECS scene.
pub trait SceneEngine: Engine {
    /// Returns the mutable game scene as a read-only render snapshot.
    fn scene(&self) -> &Scene;

    /// Returns persistent metadata for the scene's asset references.
    fn asset_registry(&self) -> &AssetRegistry;

    /// Returns loaded CPU mesh data referenced by the scene.
    fn mesh_cache(&self) -> &AssetCache<MeshData>;
}

/// Owns GPU mesh uploads referenced by a scene.
#[derive(Default)]
pub struct SceneMeshRenderer {
    meshes: HashMap<ResourceId, GpuMesh>,
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
            Self::Graphics(error) => write!(formatter, "scene mesh rendering failed: {error}"),
        }
    }
}

impl std::error::Error for SceneRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Graphics(error) => Some(error),
            Self::MissingCpuMesh { .. } | Self::MissingGpuMesh { .. } => None,
        }
    }
}

fn collect_scene_lighting(scene: &Scene) -> LightingData {
    let mut lighting = LightingData::with_ambient([0.16, 0.16, 0.18]);
    for entity_id in scene.entity_ids() {
        let Some(transform) = scene.entity_transform(entity_id) else {
            continue;
        };
        let rotation = zyne_math::Transform3d::new(
            [0.0; 3],
            transform.rotation(),
            [1.0; 3],
        )
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
        matrix[0][0] * direction[0]
            + matrix[1][0] * direction[1]
            + matrix[2][0] * direction[2],
        matrix[0][1] * direction[0]
            + matrix[1][1] * direction[1]
            + matrix[2][1] * direction[2],
        matrix[0][2] * direction[0]
            + matrix[1][2] * direction[1]
            + matrix[2][2] * direction[2],
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

#[cfg(test)]
mod tests {
    use zyne_assets::{
        AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, ResourceKind,
    };
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;
    use zyne_scene::Scene;

    use super::collect_scene_mesh_instances;

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
    fn extracts_world_space_directional_point_and_spot_lights() {
        use zyne_scene::{DirectionalLight, PointLight, SpotLight};

        let mut scene = Scene::new("Lighting Extraction").unwrap();
        let entity = scene
            .spawn_entity(
                "Light",
                Transform3d::new(
                    [3.0, 4.0, 5.0],
                    [0.0, std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2],
                    [1.0; 3],
                )
                .unwrap(),
            )
            .unwrap();
        scene.set_entity_directional_light(
            entity,
            Some(DirectionalLight::new([0.0, 0.0, -1.0], [1.0; 3], 2.0).unwrap()),
        );
        scene.set_entity_point_light(
            entity,
            Some(PointLight::new([1.0; 3], 3.0, 8.0).unwrap()),
        );
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
        assert_eq!(lighting.ambient_color, [0.16, 0.16, 0.18]);
        assert!(lighting.directional_lights.is_empty());
        assert!(lighting.point_lights.is_empty());
        assert!(lighting.spot_lights.is_empty());
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
