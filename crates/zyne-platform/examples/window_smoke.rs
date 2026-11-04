use zyne_platform::{
    PlatformEvent, PlatformWindow, WindowConfig, WindowControl, WindowEvent, WindowEventHandler,
    WindowRunner,
};

struct WindowSmoke;

impl WindowEventHandler for WindowSmoke {
    fn on_window_created(&mut self, window: &PlatformWindow) -> Result<(), String> {
        println!(
            "ZYNE window created: {}x{}, scale factor {}",
            window.inner_size().width,
            window.inner_size().height,
            window.scale_factor()
        );
        Ok(())
    }

    fn on_platform_event(
        &mut self,
        event: PlatformEvent,
        _window: &PlatformWindow,
    ) -> Result<WindowControl, String> {
        println!("Platform event: {event:?}");
        Ok(match event {
            PlatformEvent::Window(WindowEvent::CloseRequested) => WindowControl::Exit,
            _ => WindowControl::Continue,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    WindowRunner::run(
        WindowConfig {
            title: "ZYNE — Window Smoke Test".to_owned(),
            ..WindowConfig::default()
        },
        WindowSmoke,
    )?;

    Ok(())
}
