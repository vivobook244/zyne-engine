//! Backend-independent keyboard and mouse input contracts.
//!
//! These types live in core so game code can consume input without depending
//! on a particular windowing backend.

use std::collections::HashSet;

/// Physical keyboard keys represented by the platform abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    Escape, Enter, Space, Tab, Backspace, Delete, Insert, Home, End, PageUp, PageDown,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight, ShiftLeft, ShiftRight, ControlLeft,
    ControlRight, AltLeft, AltRight, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
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

/// Read-only per-frame keyboard and mouse state exposed to game callbacks.
///
/// The runtime owns and updates this state. Game code receives only a shared
/// reference, so it can query input but cannot mutate the platform state.
#[derive(Debug, Default)]
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    cursor_position: Option<(f64, f64)>,
    scroll_delta: (f64, f64),
}

impl InputState {
    /// Creates an empty input state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears transient states before events for the next frame are processed.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.scroll_delta = (0.0, 0.0);
    }

    /// Applies one normalized keyboard or mouse event.
    pub fn process_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { code, state: ButtonState::Pressed, repeat } => {
                if !repeat && self.keys_down.insert(code) {
                    self.keys_pressed.insert(code);
                }
            }
            InputEvent::Key { code, state: ButtonState::Released, .. } => {
                if self.keys_down.remove(&code) {
                    self.keys_released.insert(code);
                }
            }
            InputEvent::MouseButton { button, state: ButtonState::Pressed } => {
                if self.mouse_down.insert(button) {
                    self.mouse_pressed.insert(button);
                }
            }
            InputEvent::MouseButton { button, state: ButtonState::Released } => {
                if self.mouse_down.remove(&button) {
                    self.mouse_released.insert(button);
                }
            }
            InputEvent::CursorMoved { x, y } => self.cursor_position = Some((x, y)),
            InputEvent::MouseWheel(delta) => {
                let (x, y) = match delta {
                    ScrollDelta::Lines { x, y } | ScrollDelta::Pixels { x, y } => (x, y),
                };
                self.scroll_delta.0 += x;
                self.scroll_delta.1 += y;
            }
        }
    }

    /// Releases every held key and mouse button, for example after focus loss.
    pub fn reset(&mut self) {
        self.keys_released.extend(self.keys_down.drain());
        self.mouse_released.extend(self.mouse_down.drain());
    }

    /// Returns whether a key is currently held.
    #[must_use]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns whether a key became pressed during this frame.
    #[must_use]
    pub fn just_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Returns whether a key became released during this frame.
    #[must_use]
    pub fn just_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Returns whether a mouse button is currently held.
    #[must_use]
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    /// Returns whether a mouse button became pressed during this frame.
    #[must_use]
    pub fn just_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    /// Returns whether a mouse button became released during this frame.
    #[must_use]
    pub fn just_mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_released.contains(&button)
    }

    /// Returns the latest cursor position in window coordinates, if received.
    #[must_use]
    pub fn cursor_position(&self) -> Option<(f64, f64)> {
        self.cursor_position
    }

    /// Returns accumulated scroll movement for this frame.
    #[must_use]
    pub fn scroll_delta(&self) -> (f64, f64) {
        self.scroll_delta
    }
}

#[cfg(test)]
mod tests {
    use super::{ButtonState, InputEvent, InputState, KeyCode, MouseButton, ScrollDelta};

    #[test]
    fn tracks_key_press_hold_and_release() {
        let mut input = InputState::new();
        input.process_event(InputEvent::Key { code: KeyCode::W, state: ButtonState::Pressed, repeat: false });
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(input.just_key_pressed(KeyCode::W));
        input.begin_frame();
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(!input.just_key_pressed(KeyCode::W));
        input.process_event(InputEvent::Key { code: KeyCode::W, state: ButtonState::Released, repeat: false });
        assert!(!input.is_key_pressed(KeyCode::W));
        assert!(input.just_key_released(KeyCode::W));
    }

    #[test]
    fn ignores_repeated_key_down_for_just_pressed() {
        let mut input = InputState::new();
        input.process_event(InputEvent::Key { code: KeyCode::Space, state: ButtonState::Pressed, repeat: false });
        input.begin_frame();
        input.process_event(InputEvent::Key { code: KeyCode::Space, state: ButtonState::Pressed, repeat: true });
        assert!(!input.just_key_pressed(KeyCode::Space));
        assert!(input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn tracks_mouse_buttons_cursor_and_scroll() {
        let mut input = InputState::new();
        input.process_event(InputEvent::MouseButton { button: MouseButton::Left, state: ButtonState::Pressed });
        input.process_event(InputEvent::CursorMoved { x: 12.0, y: 24.0 });
        input.process_event(InputEvent::MouseWheel(ScrollDelta::Lines { x: 0.0, y: 1.0 }));
        assert!(input.just_mouse_pressed(MouseButton::Left));
        assert_eq!(input.cursor_position(), Some((12.0, 24.0)));
        assert_eq!(input.scroll_delta(), (0.0, 1.0));
    }

    #[test]
    fn reset_releases_held_keyboard_and_mouse_inputs() {
        let mut input = InputState::new();
        input.process_event(InputEvent::Key { code: KeyCode::A, state: ButtonState::Pressed, repeat: false });
        input.process_event(InputEvent::MouseButton { button: MouseButton::Right, state: ButtonState::Pressed });
        input.reset();
        assert!(!input.is_key_pressed(KeyCode::A));
        assert!(!input.is_mouse_pressed(MouseButton::Right));
        assert!(input.just_key_released(KeyCode::A));
        assert!(input.just_mouse_released(MouseButton::Right));
    }
}
