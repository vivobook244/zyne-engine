use crate::{EngineContext, ZyneResult};

/// Lifecycle contract implemented by a game or engine host.
///
/// The runtime owns the event loop and decides when these callbacks run.
/// Callbacks are intentionally independent of any window or renderer type.
pub trait Engine {
    /// Called once after the engine context and required subsystems are ready.
    ///
    /// # Errors
    ///
    /// Returns an error if game or subsystem initialization fails.
    fn on_start(&mut self, context: &EngineContext) -> ZyneResult<()>;

    /// Called for each variable-rate frame update.
    ///
    /// # Errors
    ///
    /// Returns an error if the update cannot complete.
    fn on_update(&mut self, context: &EngineContext, delta_seconds: f32) -> ZyneResult<()>;

    /// Called once during a graceful shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown or resource cleanup fails.
    fn on_shutdown(&mut self, context: &EngineContext) -> ZyneResult<()>;
}
