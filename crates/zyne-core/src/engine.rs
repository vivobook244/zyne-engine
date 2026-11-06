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

    /// Called at the configured fixed simulation frequency.
    ///
    /// The runtime may call this zero or more times per rendered frame to
    /// catch up with elapsed simulation time. The default is a no-op so
    /// existing engine implementations remain source-compatible.
    ///
    /// # Errors
    ///
    /// Returns an error if the fixed update cannot complete.
    fn on_fixed_update(
        &mut self,
        _context: &EngineContext,
        _fixed_delta_seconds: f32,
    ) -> ZyneResult<()> {
        Ok(())
    }

    /// Called once for each runtime frame with the clamped elapsed frame time.
    ///
    /// # Errors
    ///
    /// Returns an error if the update cannot complete.
    fn on_update(&mut self, context: &EngineContext, delta_seconds: f32) -> ZyneResult<()>;

    /// Called once during graceful shutdown, but only if startup succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown or resource cleanup fails.
    fn on_shutdown(&mut self, context: &EngineContext) -> ZyneResult<()>;
}
