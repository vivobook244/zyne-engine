use crate::{PlatformError, PlatformEvent};

/// Window dimensions expressed in logical pixels (DPI-independent points).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Window width.
    pub width: u32,
    /// Window height.
    pub height: u32,
}

impl WindowSize {
    /// Creates a logical window size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Initial window mode supported by the first desktop implementation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowMode {
    /// A normal decorated desktop window.
    #[default]
    Windowed,
    /// Borderless fullscreen on the primary monitor selected by the backend.
    BorderlessFullscreen,
}

/// Configuration used when creating the initial desktop window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    /// Window title shown by the operating system.
    pub title: String,
    /// Initial logical dimensions.
    pub size: WindowSize,
    /// Optional minimum logical dimensions.
    pub min_size: Option<WindowSize>,
    /// Optional maximum logical dimensions.
    pub max_size: Option<WindowSize>,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Whether the window should be visible immediately after creation.
    pub visible: bool,
    /// Initial window mode.
    pub mode: WindowMode,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "ZYNE Application".to_owned(),
            size: WindowSize::new(1280, 720),
            min_size: None,
            max_size: None,
            resizable: true,
            visible: true,
            mode: WindowMode::Windowed,
        }
    }
}

impl WindowConfig {
    /// Validates dimensions and size constraints before calling the OS backend.
    ///
    /// # Errors
    ///
    /// Returns an error if any configured dimension is zero or if the initial
    /// size falls outside the configured minimum/maximum bounds.
    pub fn validate(&self) -> Result<(), PlatformError> {
        if !self.size.is_valid() {
            return Err(PlatformError::InvalidWindowConfig(
                "initial width and height must be greater than zero",
            ));
        }

        if self.min_size.is_some_and(|size| !size.is_valid()) {
            return Err(PlatformError::InvalidWindowConfig(
                "minimum width and height must be greater than zero",
            ));
        }

        if self.max_size.is_some_and(|size| !size.is_valid()) {
            return Err(PlatformError::InvalidWindowConfig(
                "maximum width and height must be greater than zero",
            ));
        }

        if let (Some(minimum), Some(maximum)) = (self.min_size, self.max_size) {
            if minimum.width > maximum.width || minimum.height > maximum.height {
                return Err(PlatformError::InvalidWindowConfig(
                    "minimum dimensions must not exceed maximum dimensions",
                ));
            }
        }

        if self.max_size.is_some_and(|maximum| {
            self.size.width > maximum.width || self.size.height > maximum.height
        }) {
            return Err(PlatformError::InvalidWindowConfig(
                "initial dimensions must not exceed maximum dimensions",
            ));
        }

        if self.min_size.is_some_and(|minimum| {
            self.size.width < minimum.width || self.size.height < minimum.height
        }) {
            return Err(PlatformError::InvalidWindowConfig(
                "initial dimensions must not be smaller than minimum dimensions",
            ));
        }

        Ok(())
    }
}

/// Events exposed by the initial single-window platform API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowEvent {
    /// The client area changed size, in physical pixels.
    Resized { width: u32, height: u32 },
    /// The window received or lost keyboard focus.
    Focused(bool),
    /// The window's DPI scale factor changed.
    ScaleFactorChanged { scale_factor: f64 },
    /// The operating system requested that the window close.
    CloseRequested,
    /// The window needs to be redrawn.
    RedrawRequested,
}

/// The result of handling a platform window event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControl {
    /// Continue running the application.
    Continue,
    /// Exit the native event loop.
    Exit,
}

/// A platform-owned window wrapper. Native backend types remain private.
pub struct PlatformWindow {
    pub(crate) inner: winit::window::Window,
}

impl PlatformWindow {
    pub(crate) fn new(inner: winit::window::Window) -> Self {
        Self { inner }
    }

    /// Returns the current client-area size in physical pixels.
    #[must_use]
    pub fn inner_size(&self) -> WindowSize {
        let size = self.inner.inner_size();
        WindowSize::new(size.width, size.height)
    }

    /// Returns the current DPI scale factor.
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.inner.scale_factor()
    }

    /// Changes the window title.
    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    /// Requests a new client-area size in logical pixels.
    pub fn request_inner_size(&self, size: WindowSize) {
        let logical_size =
            winit::dpi::LogicalSize::new(f64::from(size.width), f64::from(size.height));
        let _ = self.inner.request_inner_size(logical_size);
    }

    /// Sets whether the window can be resized by the user.
    pub fn set_resizable(&self, resizable: bool) {
        self.inner.set_resizable(resizable);
    }

    /// Sets window visibility.
    pub fn set_visible(&self, visible: bool) {
        self.inner.set_visible(visible);
    }

    /// Requests a redraw from the operating system.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }
}

/// Receives events from the single-window desktop runner.
///
/// Implementations run on the event-loop thread. Avoid blocking callbacks.
pub trait WindowEventHandler {
    /// Called once after the native window has been created.
    ///
    /// # Errors
    ///
    /// Return an error to stop the event loop and report initialization failure.
    fn on_window_created(&mut self, _window: &PlatformWindow) -> Result<(), String> {
        Ok(())
    }

    /// Called for every platform lifecycle or window event.
    ///
    /// The default implementation forwards window events to `on_window_event`
    /// and ignores lifecycle notifications.
    ///
    /// # Errors
    ///
    /// Return an error to stop the event loop and report handler failure.
    fn on_platform_event(
        &mut self,
        event: PlatformEvent,
        window: &PlatformWindow,
    ) -> Result<WindowControl, String> {
        match event {
            PlatformEvent::Window(event) => self.on_window_event(event, window),
            PlatformEvent::Resumed | PlatformEvent::Suspended | PlatformEvent::Input(_) => {\n                Ok(WindowControl::Continue)\n            },
        }
    }

    /// Called when a supported window event is received.
    ///
    /// Closing the window exits by default. Override this method to customize
    /// close handling.
    ///
    /// # Errors
    ///
    /// Return an error to stop the event loop and report handler failure.
    fn on_window_event(
        &mut self,
        event: WindowEvent,
        _window: &PlatformWindow,
    ) -> Result<WindowControl, String> {
        Ok(if event == WindowEvent::CloseRequested {
            WindowControl::Exit
        } else {
            WindowControl::Continue
        })
    }
}

/// Runs the native event loop and manages the initial desktop window.
///
/// Engine startup/update/shutdown policy remains the responsibility of runtime.
pub struct WindowRunner;

impl WindowRunner {
    /// Creates and runs a single desktop window.
    ///
    /// The window is created from the active `resumed` callback as required by
    /// winit 0.30. Call this method from the process main thread; winit event
    /// loops are main-thread-bound and only one event loop is supported per process.
    ///
    /// # Errors
    ///
    /// Returns a typed error for invalid configuration, event-loop/window
    /// creation failure, handler failure, or event-loop failure.
    pub fn run<H: WindowEventHandler>(
        config: WindowConfig,
        handler: H,
    ) -> Result<(), PlatformError> {
        config.validate()?;
        crate::backend::winit::run_window(config, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowConfig, WindowSize};
    use crate::PlatformError;

    #[test]
    fn default_window_config_is_valid() {
        assert!(WindowConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_zero_initial_dimensions() {
        let config = WindowConfig {
            size: WindowSize::new(0, 720),
            ..WindowConfig::default()
        };

        assert!(matches!(
            config.validate(),
            Err(PlatformError::InvalidWindowConfig(_))
        ));
    }

    #[test]
    fn rejects_minimum_dimensions_larger_than_maximum() {
        let config = WindowConfig {
            min_size: Some(WindowSize::new(1920, 1080)),
            max_size: Some(WindowSize::new(1280, 720)),
            ..WindowConfig::default()
        };

        assert!(matches!(
            config.validate(),
            Err(PlatformError::InvalidWindowConfig(_))
        ));
    }

    #[test]
    fn rejects_initial_dimensions_outside_constraints() {
        let config = WindowConfig {
            size: WindowSize::new(640, 480),
            min_size: Some(WindowSize::new(800, 600)),
            ..WindowConfig::default()
        };

        assert!(matches!(
            config.validate(),
            Err(PlatformError::InvalidWindowConfig(_))
        ));
    }
}
