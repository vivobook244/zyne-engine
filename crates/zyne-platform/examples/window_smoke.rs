use zyne_platform::{
    PlatformWindow, WindowConfig, WindowControl, WindowEvent, WindowEventHandler, WindowRunner,
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

    fn on_window_event(
        &mut self,
        event: WindowEvent,
        _window: &PlatformWindow,
    ) -> Result<WindowControl, String> {
        println!("Window event: {event:?}");
        Ok(if event == WindowEvent::CloseRequested {
            WindowControl::Exit
        } else {
            WindowControl::Continue
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
