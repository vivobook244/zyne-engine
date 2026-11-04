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
