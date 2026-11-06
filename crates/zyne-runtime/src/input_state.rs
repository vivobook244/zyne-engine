use std::collections::HashSet;

use zyne_platform::{ButtonState, InputEvent, KeyCode, MouseButton, PlatformEvent, ScrollDelta, WindowEvent};

/// Per-frame keyboard and mouse state maintained by the runtime.
///
/// Call `begin_frame` once before processing the next frame's events. Then
/// feed platform events to `process_platform_event`. The `just_*` queries
/// remain true for that frame, while held queries reflect current state.
#[derive(Debug, Default)]
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    cursor_position: Option<(f64, f64)>,
    scroll_delta: (f32, f32),
}

impl InputState {
    /// Creates an empty input state.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Clears frame-transient states. Call once at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.scroll_delta = (0.0, 0.0);
    }

    /// Applies a platform event, including focus-loss cleanup.
    pub fn process_platform_event(&mut self, event: PlatformEvent) {
        match event {
            PlatformEvent::Input(input) => self.process_event(input),
            PlatformEvent::Window(WindowEvent::Focused(false)) => self.reset(),
            _ => {}
        }
    }

    /// Applies one normalized keyboard or mouse event.
    pub fn process_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { code, state: ButtonState::Pressed, repeat } => {
                if !repeat && self.keys_down.insert(code) { self.keys_pressed.insert(code); }
            }
            InputEvent::Key { code, state: ButtonState::Released, .. } => {
                if self.keys_down.remove(&code) { self.keys_released.insert(code); }
            }
            InputEvent::MouseButton { button, state: ButtonState::Pressed } => {
                if self.mouse_down.insert(button) { self.mouse_pressed.insert(button); }
            }
            InputEvent::MouseButton { button, state: ButtonState::Released } => {
                if self.mouse_down.remove(&button) { self.mouse_released.insert(button); }
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

    /// Releases all held inputs, intended for window focus loss.
    pub fn reset(&mut self) {
        self.keys_released.extend(self.keys_down.drain());
        self.mouse_released.extend(self.mouse_down.drain());
    }

    #[must_use]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool { self.keys_down.contains(&key) }
    #[must_use]
    pub fn just_key_pressed(&self, key: KeyCode) -> bool { self.keys_pressed.contains(&key) }
    #[must_use]
    pub fn just_key_released(&self, key: KeyCode) -> bool { self.keys_released.contains(&key) }
    #[must_use]
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool { self.mouse_down.contains(&button) }
    #[must_use]
    pub fn just_mouse_pressed(&self, button: MouseButton) -> bool { self.mouse_pressed.contains(&button) }
    #[must_use]
    pub fn just_mouse_released(&self, button: MouseButton) -> bool { self.mouse_released.contains(&button) }
    #[must_use]
    pub fn cursor_position(&self) -> Option<(f64, f64)> { self.cursor_position }
    #[must_use]
    pub fn scroll_delta(&self) -> (f32, f32) { self.scroll_delta }
}

#[cfg(test)]
mod tests {
    use super::InputState;
    use zyne_platform::{ButtonState, InputEvent, KeyCode, MouseButton, PlatformEvent, ScrollDelta, WindowEvent};

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
    fn focus_loss_releases_held_keyboard_and_mouse_inputs() {
        let mut input = InputState::new();
        input.process_event(InputEvent::Key { code: KeyCode::A, state: ButtonState::Pressed, repeat: false });
        input.process_event(InputEvent::MouseButton { button: MouseButton::Right, state: ButtonState::Pressed });
        input.process_platform_event(PlatformEvent::Window(WindowEvent::Focused(false)));
        assert!(!input.is_key_pressed(KeyCode::A));
        assert!(!input.is_mouse_pressed(MouseButton::Right));
        assert!(input.just_key_released(KeyCode::A));
        assert!(input.just_mouse_released(MouseButton::Right));
    }
}
