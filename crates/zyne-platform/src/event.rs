/// Events translated from the host operating system into ZYNE-owned types.
use crate::{input::InputEvent, window::WindowEvent};

/// A platform lifecycle, window, or input event delivered to the runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformEvent {
    /// The host application became active or resumed.
    Resumed,
    /// The host application was suspended by the operating system.
    Suspended,
    /// An event associated with the native window.
    Window(WindowEvent),
    /// A normalized keyboard or mouse event.
    Input(InputEvent),
}
