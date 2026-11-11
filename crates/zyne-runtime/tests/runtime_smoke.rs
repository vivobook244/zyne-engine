use zyne_runtime::{Engine, EngineConfig, EngineContext, ZyneResult};

#[derive(Default)]
struct LifecycleProbe {
    calls: Vec<&'static str>,
    last_delta_seconds: Option<f32>,
}

impl Engine for LifecycleProbe {
    fn on_start(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        self.calls.push("start");
        Ok(())
    }

    fn on_update(&mut self, _context: &EngineContext, delta_seconds: f32) -> ZyneResult<()> {
        self.calls.push("update");
        self.last_delta_seconds = Some(delta_seconds);
        Ok(())
    }

    fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
        self.calls.push("shutdown");
        Ok(())
    }
}

#[test]
fn runtime_exposes_a_usable_engine_lifecycle_contract() {
    let context = EngineContext::new(EngineConfig::default())
        .unwrap_or_else(|error| panic!("default engine configuration should be valid: {error}"));
    let mut engine = LifecycleProbe::default();
    let delta_seconds = 1.0 / 60.0;

    assert_eq!(engine.render_transforms(), None);
    assert!(engine.on_start(&context).is_ok());
    assert!(engine.on_update(&context, delta_seconds).is_ok());
    assert!(engine.on_shutdown(&context).is_ok());

    assert_eq!(engine.calls, vec!["start", "update", "shutdown"]);
    assert!(matches!(
        engine.last_delta_seconds,
        Some(actual) if (actual - delta_seconds).abs() < f32::EPSILON
    ));
}
