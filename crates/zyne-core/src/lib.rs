//! Foundational types and services shared across ZYNE.
//! This crate must remain independent from rendering, gameplay, and editor code.

/// Engine-wide result type.
pub type ZyneResult<T> = Result<T, ZyneError>;

/// Error type for foundational engine operations.
#[derive(Debug, thiserror::Error)]
pub enum ZyneError {
    /// An operation received invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// A requested resource could not be found.
    #[error("resource not found: {0}")]
    NotFound(String),
    /// An operation failed in an underlying subsystem.
    #[error("subsystem error: {0}")]
    Subsystem(String),
}
