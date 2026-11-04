use crate::{EngineConfig, EngineVersion, ZyneResult};

/// Immutable initialization context shared with engine subsystems.
#[derive(Debug, Clone)]
pub struct EngineContext {
    config: EngineConfig,
    version: EngineVersion,
}

impl EngineContext {
    /// Create a context after validating the supplied configuration.
    pub fn new(config: EngineConfig) -> ZyneResult<Self> {
        config.validate()?;

        Ok(Self {
            config,
            version: EngineVersion::CURRENT,
        })
    }

    /// Validated engine configuration.
    #[must_use]
    pub const fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Engine version associated with this context.
    #[must_use]
    pub const fn version(&self) -> EngineVersion {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::EngineContext;
    use crate::EngineConfig;

    #[test]
    fn creates_context_with_current_version() {
        let context = EngineContext::new(EngineConfig::default()).unwrap();

        assert_eq!(context.version().major, 0);
        assert_eq!(context.config().fixed_timestep_hz, 60);
    }
}
