use zyne_runtime::{
    Application, ApplicationConfig, Camera3d, Engine, EngineContext, InputState, KeyCode,
    OrbitCameraController, Scene, Transform3d, ZyneError, ZyneResult,
};

struct SmokeEngine {
    camera_controller: OrbitCameraController,
    scene: Scene,
    frames: u64,
    fixed_updates: u64,
}

impl Engine for SmokeEngine {
    fn on_start(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!("application smoke engine started");
        Ok(())
    }

    fn on_fixed_update(
        &mut self,
        _context: &EngineContext,
        fixed_delta_seconds: f32,
    ) -> ZyneResult<()> {
        self.fixed_updates += 1;

        // Log once per second so fixed-step execution is visible without
        // flooding the terminal with one message per simulation tick.
        if self.fixed_updates == 1 || self.fixed_updates % 60 == 0 {
            tracing::info!(
                fixed_update = self.fixed_updates,
                fixed_delta_seconds,
                "application fixed update"
            );
        }

        Ok(())
    }

    fn on_update(&mut self, _context: &EngineContext, delta_seconds: f32) -> ZyneResult<()> {
        self.frames += 1;
        tracing::info!(frame = self.frames, delta_seconds, "application frame");
        Ok(())
    }

    fn on_update_with_input(
        &mut self,
        _context: &EngineContext,
        delta_seconds: f32,
        input: &InputState,
    ) -> ZyneResult<()> {
        self.frames += 1;
        if input.just_key_pressed(KeyCode::Escape) {
            tracing::info!("escape key pressed");
        }
        self.camera_controller
            .update(input)
            .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
        tracing::info!(frame = self.frames, delta_seconds, "application frame");
        Ok(())
    }

    fn render_transforms(&self) -> Option<Vec<Transform3d>> {
        Some(self.scene.entity_transforms())
    }

    fn camera(&self, _context: &EngineContext, aspect_ratio: f32) -> Option<Camera3d> {
        self.camera_controller
            .camera_with_aspect_ratio(aspect_ratio)
            .ok()
    }

    fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!(
            frames = self.frames,
            fixed_updates = self.fixed_updates,
            "application smoke engine shutdown"
        );
        Ok(())
    }
}

fn main() -> ZyneResult<()> {
    zyne_runtime::diagnostics::init_logging()?;
    let application = Application::new(ApplicationConfig::default())?;
    let camera_controller = OrbitCameraController::new([0.0; 3], 6.0, 16.0 / 9.0)
        .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
    let mut scene = Scene::new("ZYNE Multi-Entity Smoke Scene")
        .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
    scene
        .spawn_entity(
            "Left Cube",
            Transform3d::new([-1.5, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3])
                .map_err(|error| ZyneError::Subsystem(error.to_string()))?,
        )
        .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
    scene
        .spawn_entity("Center Cube", Transform3d::identity())
        .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
    scene
        .spawn_entity(
            "Right Cube",
            Transform3d::new(
                [1.5, 0.0, 0.0],
                [0.0, 0.258_819_04, 0.0, 0.965_925_8],
                [0.8; 3],
            )
            .map_err(|error| ZyneError::Subsystem(error.to_string()))?,
        )
        .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
    application.run(SmokeEngine {
        camera_controller,
        scene,
        frames: 0,
        fixed_updates: 0,
    })
}
