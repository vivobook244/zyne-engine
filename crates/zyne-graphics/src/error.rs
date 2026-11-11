use std::{error::Error, fmt};

/// Errors produced while initializing and using the graphics device and surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsError {
    /// The native window could not be converted into a graphics surface.
    SurfaceCreation(String),
    /// No adapter could render to the requested window surface.
    AdapterRequest(String),
    /// The selected adapter could not provide a logical device and queue.
    DeviceRequest(String),
    /// The surface did not expose a usable color format.
    NoSurfaceFormats,
    /// The surface did not expose a usable presentation mode.
    NoPresentModes,
    /// The surface did not expose a usable alpha mode.
    NoAlphaModes,
    /// The native window currently has no drawable physical area.
    ZeroSizedSurface,
    /// Surface frame acquisition failed after the recovery retry.
    SurfaceAcquisition(String),
    /// The GPU ran out of memory while acquiring a surface frame.
    SurfaceOutOfMemory,
    /// Scene instance count exceeds the supported GPU buffer or draw range.
    TooManyInstances,
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(message) => {
                write!(formatter, "failed to create graphics surface: {message}")
            }
            Self::AdapterRequest(message) => {
                write!(
                    formatter,
                    "failed to request a compatible GPU adapter: {message}"
                )
            }
            Self::DeviceRequest(message) => {
                write!(
                    formatter,
                    "failed to create GPU device and queue: {message}"
                )
            }
            Self::NoSurfaceFormats => {
                formatter.write_str("the window surface exposes no supported color formats")
            }
            Self::NoPresentModes => {
                formatter.write_str("the window surface exposes no presentation modes")
            }
            Self::NoAlphaModes => formatter.write_str("the window surface exposes no alpha modes"),
            Self::ZeroSizedSurface => {
                formatter.write_str("cannot configure a graphics surface with zero dimensions")
            }
            Self::SurfaceAcquisition(message) => {
                write!(formatter, "failed to acquire a surface frame: {message}")
            }
            Self::SurfaceOutOfMemory => {
                formatter.write_str("GPU ran out of memory while acquiring a surface frame")
            }
            Self::TooManyInstances => {
                formatter.write_str("scene contains too many render instances for this GPU")
            }
        }
    }
}

impl Error for GraphicsError {}
