use zyne_runtime::{diagnostics, Engine, EngineConfig, EngineContext, ZyneResult};

struct SmokeEngine;

impl Engine for SmokeEngine {
    fn on_start(&mut self, context: &EngineContext) -> ZyneResult<()> {
        tracing::info!(
            application = %context.config().application_name,
            version = %context.version(),
            "headless smoke engine started"
        );
        Ok(())
    }

    fn on_update(&mut self, _context: &EngineContext, delta_seconds: f32) -> ZyneResult<()> {
        tracing::debug!(delta_seconds, "headless smoke update");
        Ok(())
    }

    fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        tracing::info!("headless smoke engine stopped");
        Ok(())
    }
}

fn main() -> ZyneResult<()> {
    diagnostics::init_logging()?;

    let config = EngineConfig {
        application_name: "ZYNE Headless Smoke".to_owned(),
        ..EngineConfig::default()
    };
    let context = EngineContext::new(config)?;
    let mut engine = SmokeEngine;

    engine.on_start(&context)?;
    engine.on_update(&context, 1.0 / 60.0)?;
    engine.on_shutdown(&context)?;

    Ok(())
}
