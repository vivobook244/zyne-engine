use winit::{
    keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent as WinitWindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowAttributes, WindowId},
};

use crate::{
    ButtonState, InputEvent, KeyCode, MouseButton, PlatformError, PlatformEvent, ScrollDelta,
    window::{
        PlatformWindow, WindowConfig, WindowControl, WindowEvent, WindowEventHandler, WindowMode,
    },
};

struct WinitApplication<H> {
    config: WindowConfig,
    handler: H,
    window: Option<PlatformWindow>,
    failure: Option<PlatformError>,
}

impl<H: WindowEventHandler> WinitApplication<H> {
    fn new(config: WindowConfig, handler: H) -> Self {
        Self {
            config,
            handler,
            window: None,
            failure: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.failure.is_some() {
            return;
        }

        let mut attributes: WindowAttributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.size.width),
                f64::from(self.config.size.height),
            ))
            .with_resizable(self.config.resizable)
            .with_visible(self.config.visible);

        if let Some(size) = self.config.min_size {
            attributes = attributes.with_min_inner_size(LogicalSize::new(
                f64::from(size.width),
                f64::from(size.height),
            ));
        }

        if let Some(size) = self.config.max_size {
            attributes = attributes.with_max_inner_size(LogicalSize::new(
                f64::from(size.width),
                f64::from(size.height),
            ));
        }

        if self.config.mode == WindowMode::BorderlessFullscreen {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = PlatformWindow::new(window);
                match self.handler.on_window_created(&window) {
                    Ok(()) => {
                        tracing::info!(
                            title = %self.config.title,
                            width = self.config.size.width,
                            height = self.config.size.height,
                            "platform window created"
                        );
                        self.window = Some(window);
                    }
                    Err(message) => {
                        self.failure = Some(PlatformError::Handler(message));
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                self.failure = Some(PlatformError::WindowCreation(error.to_string()));
                event_loop.exit();
            }
        }
    }

    fn dispatch_platform_event(&mut self, event_loop: &ActiveEventLoop, event: PlatformEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        match self.handler.on_platform_event(event, window) {
            Ok(WindowControl::Continue) => {}
            Ok(WindowControl::Exit) => event_loop.exit(),
            Err(message) => {
                self.failure = Some(PlatformError::Handler(message));
                event_loop.exit();
            }
        }
    }
}

impl<H: WindowEventHandler> ApplicationHandler for WinitApplication<H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
        if self.window.is_some() {
            self.dispatch_platform_event(event_loop, PlatformEvent::Resumed);
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch_platform_event(event_loop, PlatformEvent::Suspended);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.inner.id() != window_id {
            return;
        }

        let translated = match event {
            WinitWindowEvent::Resized(size) => Some(WindowEvent::Resized {
                width: size.width,
                height: size.height,
            }),
            WinitWindowEvent::Focused(focused) => Some(WindowEvent::Focused(focused)),
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Some(WindowEvent::ScaleFactorChanged { scale_factor })
            }
            WinitWindowEvent::CloseRequested => Some(WindowEvent::CloseRequested),
            WinitWindowEvent::RedrawRequested => Some(WindowEvent::RedrawRequested),
            WinitWindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return; };
                let Some(code) = map_key_code(code) else { return; };
                self.dispatch_platform_event(event_loop, PlatformEvent::Input(InputEvent::Key {
                    code,
                    state: map_button_state(event.state),
                    repeat: event.repeat,
                }));
                None
            }
            WinitWindowEvent::CursorMoved { position, .. } => {
                self.dispatch_platform_event(event_loop, PlatformEvent::Input(InputEvent::CursorMoved {
                    x: position.x, y: position.y,
                }));
                None
            }
            WinitWindowEvent::MouseInput { state, button, .. } => {
                self.dispatch_platform_event(event_loop, PlatformEvent::Input(InputEvent::MouseButton {
                    button: map_mouse_button(button),
                    state: map_button_state(state),
                }));
                None
            }
            WinitWindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines { x: f64::from(x), y: f64::from(y) },
                    MouseScrollDelta::PixelDelta(position) => ScrollDelta::Pixels {
                        x: position.x, y: position.y,
                    },
                };
                self.dispatch_platform_event(event_loop, PlatformEvent::Input(InputEvent::MouseWheel(delta)));
                None
            }
            _ => None,
        };

        if let Some(event) = translated {
            self.dispatch_platform_event(event_loop, PlatformEvent::Window(event));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn map_button_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

fn map_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(value) => MouseButton::Other(value),
    }
}

fn map_key_code(code: WinitKeyCode) -> Option<KeyCode> {
    Some(match code {
        WinitKeyCode::KeyA => KeyCode::A, WinitKeyCode::KeyB => KeyCode::B,
        WinitKeyCode::KeyC => KeyCode::C, WinitKeyCode::KeyD => KeyCode::D,
        WinitKeyCode::KeyE => KeyCode::E, WinitKeyCode::KeyF => KeyCode::F,
        WinitKeyCode::KeyG => KeyCode::G, WinitKeyCode::KeyH => KeyCode::H,
        WinitKeyCode::KeyI => KeyCode::I, WinitKeyCode::KeyJ => KeyCode::J,
        WinitKeyCode::KeyK => KeyCode::K, WinitKeyCode::KeyL => KeyCode::L,
        WinitKeyCode::KeyM => KeyCode::M, WinitKeyCode::KeyN => KeyCode::N,
        WinitKeyCode::KeyO => KeyCode::O, WinitKeyCode::KeyP => KeyCode::P,
        WinitKeyCode::KeyQ => KeyCode::Q, WinitKeyCode::KeyR => KeyCode::R,
        WinitKeyCode::KeyS => KeyCode::S, WinitKeyCode::KeyT => KeyCode::T,
        WinitKeyCode::KeyU => KeyCode::U, WinitKeyCode::KeyV => KeyCode::V,
        WinitKeyCode::KeyW => KeyCode::W, WinitKeyCode::KeyX => KeyCode::X,
        WinitKeyCode::KeyY => KeyCode::Y, WinitKeyCode::KeyZ => KeyCode::Z,
        WinitKeyCode::Digit0 => KeyCode::Digit0, WinitKeyCode::Digit1 => KeyCode::Digit1,
        WinitKeyCode::Digit2 => KeyCode::Digit2, WinitKeyCode::Digit3 => KeyCode::Digit3,
        WinitKeyCode::Digit4 => KeyCode::Digit4, WinitKeyCode::Digit5 => KeyCode::Digit5,
        WinitKeyCode::Digit6 => KeyCode::Digit6, WinitKeyCode::Digit7 => KeyCode::Digit7,
        WinitKeyCode::Digit8 => KeyCode::Digit8, WinitKeyCode::Digit9 => KeyCode::Digit9,
        WinitKeyCode::Escape => KeyCode::Escape, WinitKeyCode::Enter => KeyCode::Enter,
        WinitKeyCode::Space => KeyCode::Space, WinitKeyCode::Tab => KeyCode::Tab,
        WinitKeyCode::Backspace => KeyCode::Backspace, WinitKeyCode::Delete => KeyCode::Delete,
        WinitKeyCode::Insert => KeyCode::Insert, WinitKeyCode::Home => KeyCode::Home,
        WinitKeyCode::End => KeyCode::End, WinitKeyCode::PageUp => KeyCode::PageUp,
        WinitKeyCode::PageDown => KeyCode::PageDown, WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown, WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight, WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
        WinitKeyCode::ShiftRight => KeyCode::ShiftRight, WinitKeyCode::ControlLeft => KeyCode::ControlLeft,
        WinitKeyCode::ControlRight => KeyCode::ControlRight, WinitKeyCode::AltLeft => KeyCode::AltLeft,
        WinitKeyCode::AltRight => KeyCode::AltRight, WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::F2 => KeyCode::F2, WinitKeyCode::F3 => KeyCode::F3,
        WinitKeyCode::F4 => KeyCode::F4, WinitKeyCode::F5 => KeyCode::F5,
        WinitKeyCode::F6 => KeyCode::F6, WinitKeyCode::F7 => KeyCode::F7,
        WinitKeyCode::F8 => KeyCode::F8, WinitKeyCode::F9 => KeyCode::F9,
        WinitKeyCode::F10 => KeyCode::F10, WinitKeyCode::F11 => KeyCode::F11,
        WinitKeyCode::F12 => KeyCode::F12,
        _ => return None,
    })
}

pub(crate) fn run_window<H: WindowEventHandler>(
    config: WindowConfig,
    handler: H,
) -> Result<(), PlatformError> {
    let event_loop =
        EventLoop::new().map_err(|error| PlatformError::EventLoopCreation(error.to_string()))?;
    let mut application = WinitApplication::new(config, handler);

    let run_result = event_loop.run_app(&mut application);
    if let Some(error) = application.failure {
        return Err(error);
    }

    run_result.map_err(|error| PlatformError::EventLoopRun(error.to_string()))
}
