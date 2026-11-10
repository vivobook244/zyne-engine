use crate::{EngineContext, ZyneResult};
use zyne_math::Camera3d;

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

    /// Called at the fixed simulation frequency with read-only per-frame input.
    ///
    /// Override this callback when simulation logic needs keyboard or mouse
    /// state. The default delegates to `on_fixed_update` for compatibility.
    ///
    /// # Errors
    ///
    /// Returns an error if the fixed update cannot complete.
    fn on_fixed_update_with_input(
        &mut self,
        context: &EngineContext,
        fixed_delta_seconds: f32,
        _input: &crate::InputState,
    ) -> ZyneResult<()> {
        self.on_fixed_update(context, fixed_delta_seconds)
    }

    /// Called once for each runtime frame with the clamped elapsed frame time.
    ///
    /// # Errors
    ///
    /// Returns an error if the update cannot complete.
    fn on_update(&mut self, context: &EngineContext, delta_seconds: f32) -> ZyneResult<()>;

    /// Called once per runtime frame with read-only keyboard and mouse state.
    ///
    /// Override this callback to consume input without depending on the
    /// platform crate. The default delegates to `on_update`, preserving
    /// existing engine implementations.
    ///
    /// # Errors
    ///
    /// Returns an error if the update cannot complete.
    fn on_update_with_input(
        &mut self,
        context: &EngineContext,
        delta_seconds: f32,
        _input: &crate::InputState,
    ) -> ZyneResult<()> {
        self.on_update(context, delta_seconds)
    }

    /// Returns an optional camera for the current viewport.
    ///
    /// The runtime supplies the current surface aspect ratio so cameras can
    /// update their projection after window resizes. Returning `None` keeps the
    /// renderer's built-in default camera.
    fn camera(&self, _context: &EngineContext, _aspect_ratio: f32) -> Option<Camera3d> {
        None
    }

    /// Called once during graceful shutdown, but only if startup succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown or resource cleanup fails.
    fn on_shutdown(&mut self, context: &EngineContext) -> ZyneResult<()>;
}
