use std::fmt;

/// Errors produced by the platform layer.
#[derive(Debug)]
pub enum PlatformError {
    /// The operating system event loop could not be initialized.
    EventLoopCreation(String),
    /// A native window could not be created.
    WindowCreation(String),
    /// The requested window configuration is invalid.
    InvalidWindowConfig(&'static str),
    /// The application-provided window handler returned an error.
    Handler(String),
    /// The native event loop failed while running.
    EventLoopRun(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoopCreation(message) => write!(formatter, "failed to create platform event loop: {message}"),
            Self::WindowCreation(message) => write!(formatter, "failed to create platform window: {message}"),
            Self::InvalidWindowConfig(message) => write!(formatter, "invalid window configuration: {message}"),
            Self::Handler(message) => write!(formatter, "window event handler failed: {message}"),
            Self::EventLoopRun(message) => write!(formatter, "platform event loop failed: {message}"),
        }
    }
}

impl std::error::Error for PlatformError {}
