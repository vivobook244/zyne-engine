use zyne_core::{EngineConfig, ZyneError, ZyneResult};
use zyne_platform::WindowConfig;

/// Combined engine and desktop-window configuration for one application.
///
/// Engine-wide settings remain owned by `zyne-core`; native window settings
/// remain owned by `zyne-platform`. This runtime type only composes them.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationConfig {
    engine: EngineConfig,
    window: WindowConfig,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl ApplicationConfig {
    /// Creates an application configuration and uses the engine application
    /// name as the initial native window title.
    #[must_use]
    pub fn new(engine: EngineConfig) -> Self {
        let window = WindowConfig {
            title: engine.application_name.clone(),
            ..WindowConfig::default()
        };

        Self { engine, window }
    }

    /// Creates an application configuration with explicit window settings.
    #[must_use]
    pub fn with_window_config(engine: EngineConfig, window: WindowConfig) -> Self {
        Self { engine, window }
    }

    /// Returns the engine-wide configuration.
    #[must_use]
    pub const fn engine(&self) -> &EngineConfig {
        &self.engine
    }

    /// Returns the native window configuration.
    #[must_use]
    pub const fn window(&self) -> &WindowConfig {
        &self.window
    }

    /// Validates both engine and window settings before application startup.
    ///
    /// # Errors
    ///
    /// Returns an engine configuration error or wraps a platform window
    /// configuration error with its original message.
    pub fn validate(&self) -> ZyneResult<()> {
        self.engine.validate()?;
        self.window
            .validate()
            .map_err(|error| ZyneError::Subsystem(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ApplicationConfig;
    use zyne_core::{EngineConfig, ZyneError};
    use zyne_platform::{WindowConfig, WindowSize};

    #[test]
    fn default_configuration_is_valid_and_uses_application_name_as_title() {
        let config = ApplicationConfig::default();

        assert!(config.validate().is_ok());
        assert_eq!(config.window().title, config.engine().application_name);
        assert_eq!(config.window().size, WindowSize::new(1280, 720));
    }

    #[test]
    fn custom_engine_name_becomes_default_window_title() {
        let engine = EngineConfig {
            application_name: "My ZYNE Game".to_owned(),
            ..EngineConfig::default()
        };
        let config = ApplicationConfig::new(engine);

        assert_eq!(config.window().title, "My ZYNE Game");
    }

    #[test]
    fn explicit_window_configuration_is_preserved() {
        let engine = EngineConfig::default();
        let window = WindowConfig {
            title: "Custom Window Title".to_owned(),
            ..WindowConfig::default()
        };
        let config = ApplicationConfig::with_window_config(engine, window);

        assert_eq!(config.window().title, "Custom Window Title");
    }

    #[test]
    fn rejects_invalid_engine_configuration() {
        let engine = EngineConfig {
            fixed_timestep_hz: 0,
            ..EngineConfig::default()
        };
        let config = ApplicationConfig::new(engine);

        assert!(matches!(
            config.validate(),
            Err(ZyneError::ConfigValidation(_))
        ));
    }

    #[test]
    fn rejects_invalid_window_configuration() {
        let engine = EngineConfig::default();
        let window = WindowConfig {
            size: WindowSize::new(0, 720),
            ..WindowConfig::default()
        };
        let config = ApplicationConfig::with_window_config(engine, window);

        assert!(matches!(config.validate(), Err(ZyneError::Subsystem(_))));
    }
}
