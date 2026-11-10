use zyne_runtime::{
    Application, ApplicationConfig, Engine, EngineContext, InputState, KeyCode, ZyneResult,
};

struct SmokeEngine {
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
        tracing::info!(frame = self.frames, delta_seconds, "application frame");
        Ok(())
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
    application.run(SmokeEngine {
        frames: 0,
        fixed_updates: 0,
    })
}
