use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use zyne_core::{Engine, EngineContext, ZyneError, ZyneResult};
use zyne_platform::{
    PlatformEvent, PlatformWindow, WindowControl, WindowEvent, WindowEventHandler, WindowRunner,
};

use crate::{ApplicationConfig, EventQueue, InputState};

const MAX_FIXED_UPDATES_PER_FRAME: u32 = 8;

/// Owns the desktop application configuration and starts the runtime loop.
#[derive(Debug, Clone)]
pub struct Application {
    config: ApplicationConfig,
}

impl Application {
    /// Creates an application after validating engine and window settings.
    ///
    /// # Errors
    ///
    /// Returns a configuration error before creating any operating-system
    /// resources.
    pub fn new(config: ApplicationConfig) -> ZyneResult<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Runs the native desktop event loop and manages the engine lifecycle.
    ///
    /// The engine is started after the native window is created. If startup
    /// succeeds, shutdown is attempted exactly once after the event loop exits,
    /// including when a later update callback fails.
    ///
    /// # Errors
    ///
    /// Returns the first runtime or platform error. A shutdown error is
    /// returned only when no earlier error occurred.
    pub fn run<E: Engine>(self, engine: E) -> ZyneResult<()> {
        let context = EngineContext::new(self.config.engine().clone())?;
        let fixed_delta = 1.0 / f64::from(self.config.engine().fixed_timestep_hz);
        let frame_interval = Duration::from_secs_f64(fixed_delta);
        let state = Rc::new(RefCell::new(RuntimeState {
            engine,
            context,
            events: EventQueue::new(),
            input: InputState::new(),
            started: false,
            suspended: false,
            failure: None,
            last_frame: None,
            next_frame: None,
            accumulator: 0.0,
            fixed_delta,
            frame_interval,
            max_frame_delta: f64::from(self.config.engine().max_frame_delta_seconds),
        }));

        let runner = ApplicationRunner {
            state: Rc::clone(&state),
        };
        let platform_result = WindowRunner::run(self.config.window().clone(), runner);

        let mut state = state.borrow_mut();
        let mut failure = state.failure.take();

        if state.started {
            let shutdown_result = {
                let RuntimeState { engine, context, .. } = &mut *state;
                engine.on_shutdown(context)
            };
            if let Err(error) = shutdown_result {
                tracing::error!(%error, "engine shutdown callback failed");
                if failure.is_none() {
                    failure = Some(error);
                }
            }
            state.started = false;
        }

        drop(state);

        if let Some(error) = failure {
            return Err(error);
        }

        platform_result.map_err(|error| ZyneError::Subsystem(error.to_string()))
    }
}

struct RuntimeState<E> {
    engine: E,
    context: EngineContext,
    events: EventQueue,
    input: InputState,
    started: bool,
    suspended: bool,
    failure: Option<ZyneError>,
    last_frame: Option<Instant>,
    next_frame: Option<Instant>,
    accumulator: f64,
    fixed_delta: f64,
    frame_interval: Duration,
    max_frame_delta: f64,
}

struct ApplicationRunner<E> {
    state: Rc<RefCell<RuntimeState<E>>>,
}

impl<E: Engine> WindowEventHandler for ApplicationRunner<E> {
    fn on_window_created(&mut self, _window: &PlatformWindow) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        let startup_result = {
            let RuntimeState { engine, context, .. } = &mut *state;
            engine.on_start(context)
        };
        match startup_result {
            Ok(()) => {
                state.started = true;
                let now = Instant::now();
                state.last_frame = Some(now);
                state.next_frame = Some(now);
                tracing::info!("engine startup completed");
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                state.failure = Some(error);
                Err(message)
            }
        }
    }

    fn on_platform_event(
        &mut self,
        event: PlatformEvent,
        _window: &PlatformWindow,
    ) -> Result<WindowControl, String> {
        let should_exit = matches!(&event, PlatformEvent::Window(WindowEvent::CloseRequested));
        self.state.borrow_mut().events.push(event);
        Ok(if should_exit {
            WindowControl::Exit
        } else {
            WindowControl::Continue
        })
    }

    fn on_about_to_wait(&mut self) -> Result<Option<Instant>, String> {
        let mut state = self.state.borrow_mut();

        if let Err(error) = update_runtime(&mut state) {
            let message = error.to_string();
            state.failure = Some(error);
            return Err(message);
        }

        if state.suspended {
            return Ok(None);
        }

        Ok(state.next_frame)
    }
}

fn update_runtime<E: Engine>(state: &mut RuntimeState<E>) -> ZyneResult<()> {
    state.input.begin_frame();

    for event in state.events.drain() {
        match event {
            PlatformEvent::Resumed => {
                state.suspended = false;
                let now = Instant::now();
                state.last_frame = Some(now);
                state.next_frame = Some(now);
                state.accumulator = 0.0;
            }
            PlatformEvent::Suspended => {
                state.suspended = true;
                state.last_frame = None;
                state.next_frame = None;
                state.accumulator = 0.0;
            }
            _ => {}
        }
        state.input.process_platform_event(event);
    }

    if state.suspended || !state.started {
        return Ok(());
    }

    let now = Instant::now();
    let Some(deadline) = state.next_frame else {
        state.next_frame = Some(now);
        return Ok(());
    };

    if now < deadline {
        return Ok(());
    }

    let elapsed = state
        .last_frame
        .map_or(0.0, |last_frame| now.duration_since(last_frame).as_secs_f64());
    let delta = elapsed.min(state.max_frame_delta);
    state.last_frame = Some(now);
    state.accumulator += delta;

    let fixed_delta = state.fixed_delta;
    // The engine callback contract uses f32 seconds; timing remains f64 internally.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let fixed_delta_seconds = fixed_delta as f32;
    let mut fixed_updates = 0;

    while state.accumulator >= fixed_delta
        && fixed_updates < MAX_FIXED_UPDATES_PER_FRAME
    {
        let fixed_update_result = {
            let RuntimeState { engine, context, .. } = state;
            engine.on_fixed_update(context, fixed_delta_seconds)
        };
        fixed_update_result?;
        state.accumulator -= fixed_delta;
        fixed_updates += 1;
    }

    if state.accumulator >= fixed_delta {
        tracing::warn!(
            remaining_seconds = state.accumulator,
            "fixed update catch-up limit reached; dropping excess simulation time"
        );
        state.accumulator %= fixed_delta;
    }

    // The engine callback contract uses f32 seconds; timing remains f64 internally.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let frame_delta_seconds = delta as f32;
    let update_result = {
        let RuntimeState { engine, context, .. } = state;
        engine.on_update(context, frame_delta_seconds)
    };
    update_result?;
    state.next_frame = Some(now + state.frame_interval);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::Rc,
        time::{Duration, Instant},
    };

    use super::{
        Application, ApplicationConfig, MAX_FIXED_UPDATES_PER_FRAME, RuntimeState, update_runtime,
    };
    use crate::{EventQueue, InputState};
    use zyne_core::{Engine, EngineConfig, EngineContext, ZyneError, ZyneResult};
    use zyne_platform::{WindowConfig, WindowSize};
    
    struct RecordingEngine {
        callbacks: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Engine for RecordingEngine {
        fn on_start(&mut self, _context: &EngineContext) -> ZyneResult<()> {
            Ok(())
        }

        fn on_fixed_update(
            &mut self,
            _context: &EngineContext,
            _fixed_delta_seconds: f32,
        ) -> ZyneResult<()> {
            self.callbacks.borrow_mut().push("fixed");
            Ok(())
        }

        fn on_update(
            &mut self,
            _context: &EngineContext,
            _delta_seconds: f32,
        ) -> ZyneResult<()> {
            self.callbacks.borrow_mut().push("variable");
            Ok(())
        }

        fn on_shutdown(&mut self, _context: &EngineContext) -> ZyneResult<()> {
            Ok(())
        }
    }

    #[test]
    fn rejects_invalid_configuration_before_running() {
        let config = ApplicationConfig::with_window_config(
            EngineConfig::default(),
            WindowConfig {
                size: WindowSize::new(0, 720),
                ..WindowConfig::default()
            },
        );

        assert!(matches!(
            Application::new(config),
            Err(ZyneError::Subsystem(_))
        ));
    }

    #[test]
    fn accepts_default_configuration() {
        assert!(Application::new(ApplicationConfig::default()).is_ok());
    }
    
    #[test]
    fn fixed_updates_are_bounded_and_run_before_variable_update() {
        let callbacks = Rc::new(RefCell::new(Vec::new()));
        let context = EngineContext::new(EngineConfig {
            fixed_timestep_hz: 100,
            max_frame_delta_seconds: 0.5,
            ..EngineConfig::default()
        })
        .expect("test engine configuration should be valid");
        let now = Instant::now();
        let mut state = RuntimeState {
            engine: RecordingEngine {
                callbacks: Rc::clone(&callbacks),
            },
            context,
            events: EventQueue::new(),
            input: InputState::new(),
            started: true,
            suspended: false,
            failure: None,
            last_frame: Some(now - Duration::from_millis(500)),
            next_frame: Some(now - Duration::from_millis(1)),
            accumulator: 0.0,
            fixed_delta: 0.01,
            frame_interval: Duration::from_millis(10),
            max_frame_delta: 0.5,
        };

        update_runtime(&mut state).expect("runtime update should succeed");

        let callbacks = callbacks.borrow();
        let fixed_count = callbacks.iter().filter(|&&name| name == "fixed").count();
        assert_eq!(fixed_count, MAX_FIXED_UPDATES_PER_FRAME as usize);
        assert_eq!(callbacks.last(), Some(&"variable"));
    }
}
