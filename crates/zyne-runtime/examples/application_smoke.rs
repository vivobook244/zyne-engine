use zyne_runtime::{Application, ApplicationConfig, Engine, EngineContext, ZyneResult};

struct SmokeEngine {
    frames: u64,
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
        tracing::trace!(fixed_delta_seconds, "fixed simulation update");
        Ok(())
    }

    fn on_update(
        &mut self,
        _context: &EngineContext,
        delta_seconds: f32,
    ) -> ZyneResult<()> {
        self.frames += 1;
        tracing::info!(frame = self.frames, delta_seconds, "application frame");
        Ok(())
    }

    fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!(frames = self.frames, "application smoke engine stopped");
        Ok(())
    }
}

fn main() -> ZyneResult<()> {
    zyne_runtime::diagnostics::init_logging();
    let application = Application::new(ApplicationConfig::default())?;
    application.run(SmokeEngine { frames: 0 })
}
