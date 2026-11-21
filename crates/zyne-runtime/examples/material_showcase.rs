use zyne_assets::{
    AssetCache, AssetMetadata, AssetRegistry, BlinnPhongMaterial, ImporterSettings,
    ProjectRelativePath, ResourceKind, TextureColorSpace, TextureData,
};
use zyne_core::ResourceId;
use zyne_graphics::{MeshData, MeshVertex};
use zyne_runtime::{
    Application, ApplicationConfig, Camera3d, Engine, EngineContext, InputState,
    OrbitCameraController, Scene, SceneEngine, SceneMaterialEngine, SceneTextureEngine, Transform3d, ZyneError,
    ZyneResult,
};
use zyne_scene::DirectionalLight;

struct MaterialShowcaseEngine {
    camera_controller: OrbitCameraController,
    scene: Scene,
    asset_registry: AssetRegistry,
    mesh_cache: AssetCache<MeshData>,
    material_cache: AssetCache<BlinnPhongMaterial>,
    texture_cache: AssetCache<TextureData>,
    exposure_ev: f32,
}

impl Engine for MaterialShowcaseEngine {
    fn on_start(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!("ZYNE material showcase started");
        tracing::info!("Mouse: orbit camera; scroll: zoom");
        tracing::info!("Exposure: Arrow Up/Down adjusts by 0.5 EV; Digit0 resets");
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
        if input.just_key_pressed(zyne_runtime::KeyCode::ArrowUp) {
            self.exposure_ev = (self.exposure_ev + 0.5).min(4.0);
            tracing::info!(exposure_ev = self.exposure_ev, "showcase exposure changed");
        } else if input.just_key_pressed(zyne_runtime::KeyCode::ArrowDown) {
            self.exposure_ev = (self.exposure_ev - 0.5).max(-4.0);
            tracing::info!(exposure_ev = self.exposure_ev, "showcase exposure changed");
        } else if input.just_key_pressed(zyne_runtime::KeyCode::Digit0) {
            self.exposure_ev = 0.0;
            tracing::info!(exposure_ev = self.exposure_ev, "showcase exposure reset");
        }

        self.camera_controller
            .update(input)
            .map(|_| ())
            .map_err(|error| ZyneError::Subsystem(error.to_string()))
    }

    fn presentation_exposure_ev(&self) -> Option<f32> {
        Some(self.exposure_ev)
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

impl SceneTextureEngine for MaterialShowcaseEngine {
    fn texture_cache(&self) -> &AssetCache<TextureData> {
        &self.texture_cache
    }
}

fn main() -> ZyneResult<()> {
    zyne_runtime::diagnostics::init_logging()?;
    let application = Application::new(ApplicationConfig::default())?;
    application.run_scene_with_textures(build_engine()?)
}

struct ShowcaseAssets {
    registry: AssetRegistry,
    mesh_cache: AssetCache<MeshData>,
    material_cache: AssetCache<BlinnPhongMaterial>,
    texture_cache: AssetCache<TextureData>,
    mesh_id: ResourceId,
    matte_id: ResourceId,
    glossy_id: ResourceId,
    metal_like_id: ResourceId,
}

fn build_engine() -> ZyneResult<MaterialShowcaseEngine> {
    let assets = build_assets()?;
    let scene = build_scene(
        assets.mesh_id,
        assets.matte_id,
        assets.glossy_id,
        assets.metal_like_id,
    )?;
    let camera_controller =
        OrbitCameraController::new([0.0, 0.0, 0.0], 8.5, 16.0 / 9.0).map_err(subsystem_error)?;

    Ok(MaterialShowcaseEngine {
        camera_controller,
        scene,
        asset_registry: assets.registry,
        mesh_cache: assets.mesh_cache,
        material_cache: assets.material_cache,
        texture_cache: assets.texture_cache,
        exposure_ev: 0.0,
    })
}

fn build_assets() -> ZyneResult<ShowcaseAssets> {
    let mut registry = AssetRegistry::new();
    let mut mesh_cache = AssetCache::<MeshData>::new(ResourceKind::Mesh);
    let mut material_cache = AssetCache::<BlinnPhongMaterial>::new_material();
    let mut texture_cache = AssetCache::<TextureData>::new(ResourceKind::Texture);

    let mesh_id = ResourceId::new();
    register_asset(
        &mut registry,
        mesh_id,
        "Showcase Cube",
        ResourceKind::Mesh,
        "meshes/showcase_cube.mesh",
    )?;
    mesh_cache
        .begin_loading(&registry, mesh_id)
        .map_err(subsystem_error)?;
    mesh_cache
        .complete_loading(&registry, mesh_id, cube_mesh()?)
        .map_err(subsystem_error)?;

    let blue_texture_id = load_texture(
        &mut registry,
        &mut texture_cache,
        "Blue Ceramic Pattern",
        "textures/blue_ceramic.rgba8",
        patterned_texture([28, 96, 210], [120, 205, 255], false)?,
    )?;
    let amber_texture_id = load_texture(
        &mut registry,
        &mut texture_cache,
        "Amber Metal Pattern",
        "textures/amber_metal.rgba8",
        patterned_texture([170, 72, 18], [255, 190, 72], true)?,
    )?;

    let matte_id = load_material(
        &mut registry,
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
    let mut glossy_material = BlinnPhongMaterial::new(
        [0.025, 0.08, 0.16],
        [0.72, 0.82, 1.0],
        [0.95, 0.95, 1.0],
        112.0,
    )
    .map_err(subsystem_error)?;
    glossy_material.set_diffuse_texture(Some(blue_texture_id));
    let glossy_id = load_material(
        &mut registry,
        &mut material_cache,
        "Glossy Textured",
        "materials/glossy.ron",
        glossy_material,
    )?;
    let mut metal_like_material = BlinnPhongMaterial::new(
        [0.12, 0.075, 0.025],
        [0.82, 0.72, 0.55],
        [1.4, 1.15, 0.72],
        144.0,
    )
    .map_err(subsystem_error)?;
    metal_like_material.set_diffuse_texture(Some(amber_texture_id));
    let metal_like_id = load_material(
        &mut registry,
        &mut material_cache,
        "Metal Like Textured",
        "materials/metal_like.ron",
        metal_like_material,
    )?;

    Ok(ShowcaseAssets {
        registry,
        mesh_cache,
        material_cache,
        texture_cache,
        mesh_id,
        matte_id,
        glossy_id,
        metal_like_id,
    })
}

fn build_scene(
    mesh_id: ResourceId,
    matte_id: ResourceId,
    glossy_id: ResourceId,
    metal_like_id: ResourceId,
) -> ZyneResult<Scene> {
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
    let ground = scene
        .spawn_entity(
            "Shadow Receiver Ground",
            Transform3d::new([0.0, -0.75, 0.0], [0.0, 0.0, 0.0, 1.0], [8.0, 0.1, 8.0])
                .map_err(subsystem_error)?,
        )
        .map_err(subsystem_error)?;

    for entity in [matte, glossy, metal_like, ground] {
        scene
            .set_entity_asset_references(entity, &[mesh_id])
            .map_err(subsystem_error)?;
    }
    scene.set_entity_material_reference(matte, Some(matte_id));
    scene.set_entity_material_reference(glossy, Some(glossy_id));
    scene.set_entity_material_reference(metal_like, Some(metal_like_id));
    scene.set_entity_material_reference(ground, Some(matte_id));

    let light = scene
        .spawn_entity("Showcase Key Light", Transform3d::identity())
        .map_err(subsystem_error)?;
    let directional_light = DirectionalLight::new([0.35, -0.8, -0.55], [1.0, 0.96, 0.88], 2.2)
        .map_err(subsystem_error)?;
    scene.set_entity_directional_light(light, Some(directional_light));

    // Scene creation order is preserved during light extraction. The key light
    // therefore owns the shadow map; this fill light remains unshadowed.
    let fill_light = scene
        .spawn_entity("Showcase Fill Light (Unshadowed)", Transform3d::identity())
        .map_err(subsystem_error)?;
    let fill_directional_light =
        DirectionalLight::new([-0.65, -0.25, 0.7], [0.55, 0.68, 1.0], 0.45)
            .map_err(subsystem_error)?;
    scene.set_entity_directional_light(fill_light, Some(fill_directional_light));

    Ok(scene)
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

fn load_texture(
    registry: &mut AssetRegistry,
    cache: &mut AssetCache<TextureData>,
    name: &str,
    source_path: &str,
    data: TextureData,
) -> ZyneResult<ResourceId> {
    let id = ResourceId::new();
    register_asset(registry, id, name, ResourceKind::Texture, source_path)?;
    cache.begin_loading(registry, id).map_err(subsystem_error)?;
    cache.complete_loading(registry, id, data).map_err(subsystem_error)?;
    Ok(id)
}

fn patterned_texture(
    first: [u8; 3],
    second: [u8; 3],
    diagonal: bool,
) -> ZyneResult<TextureData> {
    let mut pixels = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let alternate = if diagonal { (x + y) % 4 < 2 } else { (x / 2 + y / 2) % 2 == 0 };
            let color = if alternate { first } else { second };
            pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
    TextureData::new_rgba8(8, 8, TextureColorSpace::Srgb, pixels).map_err(subsystem_error)
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
