/// Backend-independent keyboard and mouse input types.
 
/// Physical keyboard keys represented by the platform abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,
    Escape, Enter, Space, Tab, Backspace, Delete, Insert,
    Home, End, PageUp, PageDown,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    ShiftLeft, ShiftRight, ControlLeft, ControlRight,
    AltLeft, AltRight,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

/// Press/release state for a button-like input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

/// Mouse buttons supported by the portable input API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// Scroll amount reported by the platform, preserving its native unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    Lines { x: f64, y: f64 },
    Pixels { x: f64, y: f64 },
}

/// A normalized keyboard or mouse event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Key { code: KeyCode, state: ButtonState, repeat: bool },
    MouseButton { button: MouseButton, state: ButtonState },
    CursorMoved { x: f64, y: f64 },
    MouseWheel(ScrollDelta),
}
