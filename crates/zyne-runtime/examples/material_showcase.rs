use zyne_assets::{
    AssetCache, AssetMetadata, AssetRegistry, BlinnPhongMaterial, ImporterSettings,
    ProjectRelativePath, ResourceKind,
};
use zyne_core::ResourceId;
use zyne_graphics::{MeshData, MeshVertex};
use zyne_runtime::{
    Application, ApplicationConfig, Camera3d, Engine, EngineContext, InputState,
    OrbitCameraController, Scene, SceneEngine, SceneMaterialEngine, Transform3d, ZyneError,
    ZyneResult,
};
use zyne_scene::DirectionalLight;

struct MaterialShowcaseEngine {
    camera_controller: OrbitCameraController,
    scene: Scene,
    asset_registry: AssetRegistry,
    mesh_cache: AssetCache<MeshData>,
    material_cache: AssetCache<BlinnPhongMaterial>,
}

impl Engine for MaterialShowcaseEngine {
    fn on_start(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!("ZYNE material showcase started");
        tracing::info!("Use the mouse to orbit the camera and scroll to zoom");
        Ok(())
    }

    fn on_update(&mut self, _context: &EngineContext, _delta_seconds: f32) -> ZyneResult<()> {
        Ok(())
    }

    fn on_update_with_input(
        &mut self,
        _context: &EngineContext,
        _delta_seconds: f32,
        input: &InputState,
    ) -> ZyneResult<()> {
        self.camera_controller
            .update(input)
            .map_err(|error| ZyneError::Subsystem(error.to_string()))
    }

    fn camera(&self, _context: &EngineContext, aspect_ratio: f32) -> Option<Camera3d> {
        self.camera_controller
            .camera_with_aspect_ratio(aspect_ratio)
            .ok()
    }

    fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!("ZYNE material showcase stopped");
        Ok(())
    }
}

impl SceneEngine for MaterialShowcaseEngine {
    fn scene(&self) -> &Scene {
        &self.scene
    }

    fn asset_registry(&self) -> &AssetRegistry {
        &self.asset_registry
    }

    fn mesh_cache(&self) -> &AssetCache<MeshData> {
        &self.mesh_cache
    }
}

impl SceneMaterialEngine for MaterialShowcaseEngine {
    fn material_cache(&self) -> &AssetCache<BlinnPhongMaterial> {
        &self.material_cache
    }
}

fn main() -> ZyneResult<()> {
    zyne_runtime::diagnostics::init_logging()?;

    let application = Application::new(ApplicationConfig::default())?;
    let mut asset_registry = AssetRegistry::new();
    let mut mesh_cache = AssetCache::<MeshData>::new(ResourceKind::Mesh);
    let mut material_cache = AssetCache::<BlinnPhongMaterial>::new_material();

    let mesh_id = ResourceId::new();
    register_asset(
        &mut asset_registry,
        mesh_id,
        "Showcase Cube",
        ResourceKind::Mesh,
        "meshes/showcase_cube.mesh",
    )?;
    mesh_cache
        .begin_loading(&asset_registry, mesh_id)
        .map_err(subsystem_error)?;
    mesh_cache
        .complete_loading(&asset_registry, mesh_id, cube_mesh()?)
        .map_err(subsystem_error)?;

    let matte_id = load_material(
        &mut asset_registry,
        &mut material_cache,
        "Matte",
        "materials/matte.ron",
        BlinnPhongMaterial::new(
            [0.18, 0.035, 0.025],
            [0.82, 0.16, 0.10],
            [0.06, 0.06, 0.06],
            8.0,
        )
        .map_err(subsystem_error)?,
    )?;
    let glossy_id = load_material(
        &mut asset_registry,
        &mut material_cache,
        "Glossy",
        "materials/glossy.ron",
        BlinnPhongMaterial::new(
            [0.025, 0.08, 0.16],
            [0.12, 0.38, 0.88],
            [0.95, 0.95, 1.0],
            112.0,
        )
        .map_err(subsystem_error)?,
    )?;
    let metal_like_id = load_material(
        &mut asset_registry,
        &mut material_cache,
        "Metal Like",
        "materials/metal_like.ron",
        BlinnPhongMaterial::new(
            [0.12, 0.075, 0.025],
            [0.62, 0.34, 0.09],
            [1.4, 1.15, 0.72],
            144.0,
        )
        .map_err(subsystem_error)?,
    )?;

    let mut scene = Scene::new("ZYNE Material Visual Showcase").map_err(subsystem_error)?;

    let matte = scene
        .spawn_entity(
            "Matte - Low Specular",
            Transform3d::new([-1.8, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.35; 3])
                .map_err(subsystem_error)?,
        )
        .map_err(subsystem_error)?;
    let glossy = scene
        .spawn_entity(
            "Glossy - Tight Highlight",
            Transform3d::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.35; 3])
                .map_err(subsystem_error)?,
        )
        .map_err(subsystem_error)?;
    let metal_like = scene
        .spawn_entity(
            "Metal Like - Warm Specular",
            Transform3d::new([1.8, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.35; 3])
                .map_err(subsystem_error)?,
        )
        .map_err(subsystem_error)?;

    for entity in [matte, glossy, metal_like] {
        scene
            .set_entity_asset_references(entity, &[mesh_id])
            .map_err(subsystem_error)?;
    }
    scene.set_entity_material_reference(matte, Some(matte_id));
    scene.set_entity_material_reference(glossy, Some(glossy_id));
    scene.set_entity_material_reference(metal_like, Some(metal_like_id));

    let light = scene
        .spawn_entity("Showcase Key Light", Transform3d::identity())
        .map_err(subsystem_error)?;
    let directional_light = DirectionalLight::new([0.35, -0.8, -0.55], [1.0, 0.96, 0.88], 2.2)
        .map_err(subsystem_error)?;
    scene.set_entity_directional_light(light, Some(directional_light));

    let camera_controller =
        OrbitCameraController::new([0.0, 0.0, 0.0], 8.5, 16.0 / 9.0).map_err(subsystem_error)?;

    application.run_scene_with_materials(MaterialShowcaseEngine {
        camera_controller,
        scene,
        asset_registry,
        mesh_cache,
        material_cache,
    })
}

fn register_asset(
    registry: &mut AssetRegistry,
    id: ResourceId,
    name: &str,
    kind: ResourceKind,
    source_path: &str,
) -> ZyneResult<()> {
    let metadata = AssetMetadata::new(
        id,
        name,
        kind,
        ProjectRelativePath::new(source_path).map_err(subsystem_error)?,
        ImporterSettings::new("zyne-material-showcase", 1).map_err(subsystem_error)?,
    )
    .map_err(subsystem_error)?;
    registry.register(metadata).map_err(subsystem_error)
}

fn load_material(
    registry: &mut AssetRegistry,
    cache: &mut AssetCache<BlinnPhongMaterial>,
    name: &str,
    source_path: &str,
    material: BlinnPhongMaterial,
) -> ZyneResult<ResourceId> {
    let id = ResourceId::new();
    register_asset(registry, id, name, ResourceKind::Material, source_path)?;
    cache.begin_loading(registry, id).map_err(subsystem_error)?;
    cache
        .complete_loading(registry, id, material)
        .map_err(subsystem_error)?;
    Ok(id)
}

fn cube_mesh() -> ZyneResult<MeshData> {
    let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
        (
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [0.0, 0.0, 1.0],
        ),
        (
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
            [0.0, 0.0, -1.0],
        ),
        (
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
            [1.0, 0.0, 0.0],
        ),
        (
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [-1.0, 0.0, 0.0],
        ),
        (
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
            [0.0, 1.0, 0.0],
        ),
        (
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            [0.0, -1.0, 0.0],
        ),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (face_index, (positions, normal)) in faces.into_iter().enumerate() {
        let base = u32::try_from(face_index * 4)
            .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        for (position, uv) in positions.into_iter().zip(uvs) {
            vertices.push(MeshVertex::new(position, [1.0; 3], uv, normal));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    MeshData::new(vertices, indices).map_err(subsystem_error)
}

fn subsystem_error(error: impl std::fmt::Display) -> ZyneError {
    ZyneError::Subsystem(error.to_string())
}
