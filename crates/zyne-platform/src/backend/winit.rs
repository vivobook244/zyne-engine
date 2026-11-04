use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent as WinitWindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowAttributes, WindowId},
};

use crate::{
    PlatformError,
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

    fn dispatch_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        match self.handler.on_window_event(event, window) {
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
            _ => None,
        };

        if let Some(event) = translated {
            self.dispatch_window_event(event_loop, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
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
