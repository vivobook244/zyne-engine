use serde::{Deserialize, Serialize};

use crate::{ZyneError, ZyneResult};

/// Initial engine-level settings.
///
/// Project files use TOML. Window-specific settings will be added with the
/// platform phase rather than coupling core configuration to a window API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// Human-readable application name used by diagnostics and platform UI.
    pub application_name: String,
    /// Target fixed-update frequency in ticks per second.
    pub fixed_timestep_hz: u32,
    /// Maximum frame delta accepted by the runtime, in seconds.
    pub max_frame_delta_seconds: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            application_name: "ZYNE Application".to_owned(),
            fixed_timestep_hz: 60,
            max_frame_delta_seconds: 0.25,
        }
    }
}

impl EngineConfig {
    /// Parse a project configuration document from TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when TOML parsing fails or a parsed value is invalid.
    pub fn from_toml(source: &str) -> ZyneResult<Self> {
        let config: Self =
            toml::from_str(source).map_err(|error| ZyneError::ConfigParse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Serialize this configuration as a TOML document.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration is invalid or serialization fails.
    pub fn to_toml(&self) -> ZyneResult<String> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|error| ZyneError::ConfigParse(error.to_string()))
    }

    /// Validate values before the runtime consumes them.
    ///
    /// # Errors
    ///
    /// Returns an error when a field is outside its supported range.
    pub fn validate(&self) -> ZyneResult<()> {
        if self.application_name.trim().is_empty() {
            return Err(ZyneError::ConfigValidation(
                "application_name must not be empty".to_owned(),
            ));
        }

        if !(1..=1000).contains(&self.fixed_timestep_hz) {
            return Err(ZyneError::ConfigValidation(
                "fixed_timestep_hz must be between 1 and 1000".to_owned(),
            ));
        }

        if !self.max_frame_delta_seconds.is_finite()
            || self.max_frame_delta_seconds <= 0.0
            || self.max_frame_delta_seconds > 1.0
        {
            return Err(ZyneError::ConfigValidation(
                "max_frame_delta_seconds must be finite and in the range (0, 1]".to_owned(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EngineConfig;
    use crate::ZyneError;

    #[test]
    fn default_configuration_is_valid() {
        assert!(EngineConfig::default().validate().is_ok());
    }

    #[test]
    fn parses_toml_and_applies_defaults() {
        let result = EngineConfig::from_toml("application_name = \"Test Game\"");
        assert!(result.is_ok());
        let config = result.unwrap_or_default();

        assert_eq!(config.application_name, "Test Game");
        assert_eq!(config.fixed_timestep_hz, 60);
    }

    #[test]
    fn rejects_empty_application_name() {
        let result = EngineConfig::from_toml("application_name = \"\"");

        assert!(matches!(result, Err(ZyneError::ConfigValidation(_))));
    }

    #[test]
    fn rejects_invalid_fixed_timestep() {
        let mut config = EngineConfig::default();
        config.fixed_timestep_hz = 0;

        assert!(matches!(
            config.validate(),
            Err(ZyneError::ConfigValidation(_))
        ));
    }

    #[test]
    fn serializes_and_round_trips() {
        let config = EngineConfig::default();
        let serialized = config.to_toml();
        assert!(serialized.is_ok());
        let parsed = serialized
            .ok()
            .and_then(|source| EngineConfig::from_toml(&source).ok());

        assert_eq!(parsed, Some(config));
    }
}
