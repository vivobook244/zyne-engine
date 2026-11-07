use std::sync::Arc;

use wgpu::{
    Adapter, AdapterInfo, CompositeAlphaMode, Device, DeviceDescriptor, Features, Instance, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages,
};
use zyne_platform::PlatformWindow;

use crate::GraphicsError;

/// Owns the GPU connection and the initial presentation surface for one window.
///
/// This phase initializes the GPU and configures the surface only. Frame
/// acquisition, resize handling, command encoding, and presentation are added
/// by later rendering phases.
pub struct GraphicsContext {
    _instance: Instance,
    adapter: Adapter,
    adapter_info: AdapterInfo,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
}

impl GraphicsContext {
    /// Creates a compatible GPU device and configures the initial window surface.
    ///
    /// The surface owns a clone of the platform window handle, so its lifetime
    /// is independent of the temporary callback borrow.
    ///
    /// # Errors
    ///
    /// Returns a descriptive graphics error if surface, adapter, device, or
    /// surface configuration initialization fails.
    pub fn new(window: PlatformWindow) -> Result<Self, GraphicsError> {
        let instance = Instance::default();
        let surface = instance
            .create_surface(Arc::new(window.clone()))
            .map_err(|error| GraphicsError::SurfaceCreation(error.to_string()))?;

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|error| GraphicsError::AdapterRequest(error.to_string()))?;

        let adapter_info = adapter.get_info();
        let descriptor = DeviceDescriptor {
            label: Some("ZYNE GPU Device"),
            required_features: Features::empty(),
            required_limits: Limits::downlevel_defaults(),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
            .map_err(|error| GraphicsError::DeviceRequest(error.to_string()))?;

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err(GraphicsError::ZeroSizedSurface);
        }

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(GraphicsError::NoSurfaceFormats)?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == PresentMode::Fifo)
            .or_else(|| capabilities.present_modes.first().copied())
            .ok_or(GraphicsError::NoPresentModes)?;
        let alpha_mode = capabilities
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == CompositeAlphaMode::Auto)
            .or_else(|| capabilities.alpha_modes.first().copied())
            .ok_or(GraphicsError::NoAlphaModes)?;

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);

        tracing::info!(
            adapter = %adapter_info.name,
            backend = ?adapter_info.backend,
            device_type = ?adapter_info.device_type,
            driver = %adapter_info.driver,
            width = surface_config.width,
            height = surface_config.height,
            format = ?surface_config.format,
            present_mode = ?surface_config.present_mode,
            "graphics context initialized"
        );

        Ok(Self {
            _instance: instance,
            adapter,
            adapter_info,
            device,
            queue,
            surface,
            surface_config,
        })
    }

    /// Returns information about the selected physical GPU adapter.
    #[must_use]
    pub const fn adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    /// Returns the logical GPU device.
    #[must_use]
    pub const fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the command submission queue for the logical GPU device.
    #[must_use]
    pub const fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Returns the configured color format for the window surface.
    #[must_use]
    pub const fn surface_format(&self) -> TextureFormat {
        self.surface_config.format
    }

    /// Returns the configured physical surface dimensions.
    #[must_use]
    pub const fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// Returns the selected adapter.
    #[must_use]
    pub const fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    /// Returns the window presentation surface.
    ///
    /// Surface acquisition and presentation are intentionally not part of
    /// Phase 2.1.
    #[must_use]
    pub const fn surface(&self) -> &Surface<'static> {
        &self.surface
    }
}
