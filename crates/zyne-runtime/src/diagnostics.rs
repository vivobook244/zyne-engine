use tracing_subscriber::{fmt, EnvFilter};

use zyne_core::{ZyneError, ZyneResult};

/// Initialize the process-wide tracing subscriber.
///
/// Libraries and engine subsystems should only emit tracing events. The
/// executable/game host owns subscriber initialization and output policy.
///
/// # Errors
///
/// Returns an error if another global subscriber has already been installed.
pub fn init_logging() -> ZyneResult<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|error| ZyneError::Subsystem(format!("logging initialization failed: {error}")))
}
