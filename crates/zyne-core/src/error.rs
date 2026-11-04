use thiserror::Error;

/// Engine-wide result type.
pub type ZyneResult<T> = Result<T, ZyneError>;

/// Errors that can cross ZYNE crate boundaries.
#[derive(Debug, Error)]
pub enum ZyneError {
    /// An operation received invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// A requested resource could not be found.
    #[error("resource not found: {0}")]
    NotFound(String),
    /// A configuration document could not be parsed.
    #[error("configuration parse failed: {0}")]
    ConfigParse(String),
    /// A parsed configuration did not satisfy engine requirements.
    #[error("invalid engine configuration: {0}")]
    ConfigValidation(String),
    /// An operation failed in an underlying subsystem.
    #[error("subsystem error: {0}")]
    Subsystem(String),
}

#[cfg(test)]
mod tests {
    use super::ZyneError;

    #[test]
    fn formats_invalid_input_error() {
        let error = ZyneError::InvalidInput("missing name".to_owned());
        assert_eq!(error.to_string(), "invalid input: missing name");
    }

    #[test]
    fn formats_not_found_error() {
        let error = ZyneError::NotFound("texture/brick".to_owned());
        assert_eq!(error.to_string(), "resource not found: texture/brick");
    }

    #[test]
    fn formats_configuration_errors() {
        let parse = ZyneError::ConfigParse("unexpected token".to_owned());
        let validation = ZyneError::ConfigValidation("invalid timestep".to_owned());

        assert_eq!(
            parse.to_string(),
            "configuration parse failed: unexpected token"
        );
        assert_eq!(
            validation.to_string(),
            "invalid engine configuration: invalid timestep"
        );
    }

    #[test]
    fn formats_subsystem_error() {
        let error = ZyneError::Subsystem("device unavailable".to_owned());
        assert_eq!(error.to_string(), "subsystem error: device unavailable");
    }
}
