use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu::{
    Adapter, AdapterInfo, Color, CompositeAlphaMode, Device, DeviceDescriptor, Features, Instance,
    Limits, LoadOp, PowerPreference, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface, SurfaceCapabilities,
    SurfaceConfiguration, SurfaceError, TextureFormat, TextureUsages,
};
use zyne_math::{Camera3d, Transform3d};
use zyne_platform::PlatformWindow;

use crate::{
    FrameOutcome, GraphicsError,
    mesh::{GpuMesh, MeshData},
    pipeline::CubePipeline,
};

const DEFAULT_CLEAR_COLOR: Color = Color {
    r: 0.055,
    g: 0.065,
    b: 0.085,
    a: 1.0,
};

/// Owns the GPU connection, presentation surface, and frame lifecycle.
pub struct GraphicsContext {
    _instance: Instance,
    adapter: Adapter,
    adapter_info: AdapterInfo,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    cube_pipeline: CubePipeline,
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    surface_suspended: bool,
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
    pub fn new(window: &PlatformWindow) -> Result<Self, GraphicsError> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err(GraphicsError::ZeroSizedSurface);
        }

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

        let capabilities = surface.get_capabilities(&adapter);
        let surface_config = build_surface_configuration(&capabilities, size.width, size.height)?;
        surface.configure(&device, &surface_config);
        let cube_pipeline = CubePipeline::new(&device, &queue, surface_config.format);
        let (depth_texture, depth_view) =
            create_depth_texture(&device, surface_config.width, surface_config.height);

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
            cube_pipeline,
            depth_texture,
            depth_view,
            surface_suspended: false,
        })
    }

    /// Uploads validated CPU mesh data into reusable GPU vertex and index buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if a buffer exceeds device limits or the index count
    /// cannot be represented by the indexed draw API.
    pub fn create_mesh(&self, data: &MeshData) -> Result<GpuMesh, GraphicsError> {
        let vertex_count = u64::try_from(data.vertices().len())
            .map_err(|_| GraphicsError::MeshBufferTooLarge)?;
        let vertex_stride = u64::try_from(std::mem::size_of::<crate::MeshVertex>())
            .map_err(|_| GraphicsError::MeshBufferTooLarge)?;
        let vertex_size = vertex_count
            .checked_mul(vertex_stride)
            .ok_or(GraphicsError::MeshBufferTooLarge)?;
        let index_count =
            u32::try_from(data.indices().len()).map_err(|_| GraphicsError::MeshBufferTooLarge)?;
        let index_stride = u64::try_from(std::mem::size_of::<u32>())
            .map_err(|_| GraphicsError::MeshBufferTooLarge)?;
        let index_size = u64::try_from(data.indices().len())
            .map_err(|_| GraphicsError::MeshBufferTooLarge)?
            .checked_mul(index_stride)
            .ok_or(GraphicsError::MeshBufferTooLarge)?;

        if vertex_size > self.device.limits().max_buffer_size
            || index_size > self.device.limits().max_buffer_size
        {
            return Err(GraphicsError::MeshBufferTooLarge);
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ZYNE Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(data.vertices()),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ZYNE Mesh Index Buffer"),
                contents: bytemuck::cast_slice(data.indices()),
                usage: wgpu::BufferUsages::INDEX,
            });

        Ok(GpuMesh::new(vertex_buffer, index_buffer, index_count))
    }

    /// Acquires a surface frame, clears it, submits GPU work, and presents it.
    ///
    /// Lost and outdated surfaces are reconfigured and retried once. A timeout
    /// skips the current frame; an out-of-memory condition is fatal.
    ///
    /// # Errors
    ///
    /// Returns an error when frame acquisition cannot recover or GPU memory is
    /// exhausted.
    pub fn render_frame(&mut self) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(None, None, None)
    }

    /// Renders a frame using a caller-supplied camera.
    ///
    /// # Errors
    ///
    /// Returns an error when frame acquisition or GPU presentation fails.
    pub fn render_frame_with_camera(
        &mut self,
        camera: &Camera3d,
    ) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(Some(camera), None, None)
    }

    /// Renders all supplied transforms as instances of the built-in cube mesh.
    ///
    /// `None` camera uses the renderer's default camera. An empty transform
    /// slice clears the frame without drawing entities.
    ///
    /// # Errors
    ///
    /// Returns an error when the scene exceeds GPU instance-buffer limits or
    /// surface acquisition/presentation fails.
    pub fn render_frame_with_transforms(
        &mut self,
        camera: Option<&Camera3d>,
        transforms: &[Transform3d],
    ) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(camera, Some(transforms), None)
    }

    /// Renders an uploaded mesh with one or more transforms using indexed drawing.
    ///
    /// The mesh must have been uploaded by this graphics context. All supplied
    /// transforms are submitted in a single instanced indexed draw call.
    ///
    /// # Errors
    ///
    /// Returns an error if the instance buffer exceeds device limits or frame
    /// acquisition/presentation fails.
    pub fn render_frame_with_mesh(
        &mut self,
        camera: Option<&Camera3d>,
        mesh: &GpuMesh,
        transforms: &[Transform3d],
    ) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(camera, Some(transforms), Some(mesh))
    }

    fn render_frame_internal(
        &mut self,
        camera: Option<&Camera3d>,
        transforms: Option<&[Transform3d]>,
        mesh: Option<&GpuMesh>,
    ) -> Result<FrameOutcome, GraphicsError> {
        if self.surface_suspended {
            return Ok(FrameOutcome::Skipped);
        }

        let Some(frame) = self.acquire_frame()? else {
            return Ok(FrameOutcome::Skipped);
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.cube_pipeline.update_camera(
            &self.queue,
            self.surface_config.width,
            self.surface_config.height,
            camera,
        );

        let instance_count =
            self.cube_pipeline
                .update_instances(&self.device, &self.queue, transforms)?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ZYNE Frame Command Encoder"),
            });

        {
            let color_attachments = [Some(RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: LoadOp::Clear(DEFAULT_CLEAR_COLOR),
                    store: StoreOp::Store,
                },
            })];

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("ZYNE 3D Cube Render Pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(self.cube_pipeline.pipeline());
            render_pass.set_bind_group(0, self.cube_pipeline.camera_bind_group(), &[]);
            render_pass.set_bind_group(1, self.cube_pipeline.texture_bind_group(), &[]);
            render_pass.set_vertex_buffer(1, self.cube_pipeline.instance_buffer().slice(..));
            if let Some(mesh) = mesh {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count(), 0, 0..instance_count);
            } else {
                render_pass.set_vertex_buffer(0, self.cube_pipeline.vertex_buffer().slice(..));
                render_pass.draw(0..36, 0..instance_count);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(FrameOutcome::Presented)
    }

    fn acquire_frame(&mut self) -> Result<Option<wgpu::SurfaceTexture>, GraphicsError> {
        for attempt in 0..2 {
            match self.surface.get_current_texture() {
                Ok(frame) => return Ok(Some(frame)),
                Err(SurfaceError::Timeout) => {
                    tracing::debug!("surface acquisition timed out; skipping frame");
                    return Ok(None);
                }
                Err(SurfaceError::OutOfMemory) => {
                    return Err(GraphicsError::SurfaceOutOfMemory);
                }
                Err(error @ (SurfaceError::Lost | SurfaceError::Outdated)) => {
                    if attempt == 1 {
                        return Err(GraphicsError::SurfaceAcquisition(error.to_string()));
                    }

                    tracing::warn!(
                        %error,
                        "surface unavailable; reconfiguring before retry"
                    );
                    self.surface.configure(&self.device, &self.surface_config);
                }
                Err(error) => {
                    return Err(GraphicsError::SurfaceAcquisition(error.to_string()));
                }
            }
        }

        Err(GraphicsError::SurfaceAcquisition(
            "surface acquisition retry was exhausted".to_owned(),
        ))
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

    /// Updates the configured surface dimensions after a native window resize.
    ///
    /// A zero-sized window (for example, while minimized) suspends surface
    /// configuration. The previous valid dimensions are retained and the
    /// surface is configured again when a non-zero size is received.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            if !self.surface_suspended {
                tracing::debug!(
                    width,
                    height,
                    "graphics surface suspended for zero-sized window"
                );
            }
            self.surface_suspended = true;
            return;
        }

        if !surface_resize_requires_configuration(
            self.surface_config.width,
            self.surface_config.height,
            self.surface_suspended,
            width,
            height,
        ) {
            return;
        }

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        let (depth_texture, depth_view) = create_depth_texture(
            &self.device,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.surface_suspended = false;

        tracing::info!(width, height, "graphics surface reconfigured");
    }

    /// Marks the surface unavailable while the native application is suspended.
    ///
    /// The next non-zero resize (including a resize with unchanged dimensions)
    /// reconfigures the surface before it is used again.
    pub fn suspend(&mut self) {
        self.surface_suspended = true;
    }

    /// Returns whether the surface is currently suspended.
    #[must_use]
    pub const fn is_surface_suspended(&self) -> bool {
        self.surface_suspended
    }

    /// Returns the selected adapter.
    #[must_use]
    pub const fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    /// Returns the window presentation surface.
    #[must_use]
    pub const fn surface(&self) -> &Surface<'static> {
        &self.surface
    }
}

fn create_depth_texture(
    device: &Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ZYNE Main Depth Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn surface_resize_requires_configuration(
    current_width: u32,
    current_height: u32,
    surface_suspended: bool,
    width: u32,
    height: u32,
) -> bool {
    width > 0
        && height > 0
        && (surface_suspended || current_width != width || current_height != height)
}

fn build_surface_configuration(
    capabilities: &SurfaceCapabilities,
    width: u32,
    height: u32,
) -> Result<SurfaceConfiguration, GraphicsError> {
    if width == 0 || height == 0 {
        return Err(GraphicsError::ZeroSizedSurface);
    }

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

    Ok(SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode,
        desired_maximum_frame_latency: 2,
        alpha_mode,
        view_formats: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use wgpu::{
        CompositeAlphaMode, PresentMode, SurfaceCapabilities, TextureFormat, TextureUsages,
    };

    use super::build_surface_configuration;
    use crate::GraphicsError;

    fn sample_capabilities() -> SurfaceCapabilities {
        SurfaceCapabilities {
            usages: TextureUsages::RENDER_ATTACHMENT,
            formats: vec![TextureFormat::Bgra8Unorm, TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![PresentMode::Immediate, PresentMode::Fifo],
            alpha_modes: vec![CompositeAlphaMode::Opaque, CompositeAlphaMode::Auto],
        }
    }

    #[test]
    fn resize_requires_reconfiguration_when_dimensions_change() {
        assert!(super::surface_resize_requires_configuration(
            1280, 720, false, 1920, 1080
        ));
    }

    #[test]
    fn resize_skips_reconfiguration_when_dimensions_are_unchanged() {
        assert!(!super::surface_resize_requires_configuration(
            1280, 720, false, 1280, 720
        ));
    }

    #[test]
    fn resize_reconfigures_after_surface_suspension_even_at_same_dimensions() {
        assert!(super::surface_resize_requires_configuration(
            1280, 720, true, 1280, 720
        ));
    }

    #[test]
    fn resize_does_not_configure_zero_sized_surface() {
        assert!(!super::surface_resize_requires_configuration(
            1280, 720, false, 0, 720
        ));
        assert!(!super::surface_resize_requires_configuration(
            1280, 720, false, 1280, 0
        ));
    }

    #[test]
    fn prefers_srgb_fifo_and_automatic_alpha_when_available() {
        let config = build_surface_configuration(&sample_capabilities(), 1280, 720)
            .expect("sample surface capabilities should produce a valid configuration");

        assert_eq!(config.format, TextureFormat::Bgra8UnormSrgb);
        assert_eq!(config.present_mode, PresentMode::Fifo);
        assert_eq!(config.alpha_mode, CompositeAlphaMode::Auto);
        assert_eq!((config.width, config.height), (1280, 720));
        assert_eq!(config.desired_maximum_frame_latency, 2);
    }

    #[test]
    fn falls_back_to_first_supported_surface_capabilities() {
        let capabilities = SurfaceCapabilities {
            usages: TextureUsages::RENDER_ATTACHMENT,
            formats: vec![TextureFormat::Bgra8Unorm],
            present_modes: vec![PresentMode::Immediate],
            alpha_modes: vec![CompositeAlphaMode::Opaque],
        };

        let config = build_surface_configuration(&capabilities, 640, 480)
            .expect("non-empty surface capabilities should produce a valid configuration");

        assert_eq!(config.format, TextureFormat::Bgra8Unorm);
        assert_eq!(config.present_mode, PresentMode::Immediate);
        assert_eq!(config.alpha_mode, CompositeAlphaMode::Opaque);
    }

    #[test]
    fn rejects_zero_sized_surface() {
        let result = build_surface_configuration(&sample_capabilities(), 0, 720);

        assert_eq!(result, Err(GraphicsError::ZeroSizedSurface));
    }

    #[test]
    fn rejects_empty_surface_formats() {
        let mut capabilities = sample_capabilities();
        capabilities.formats.clear();

        let result = build_surface_configuration(&capabilities, 1280, 720);

        assert_eq!(result, Err(GraphicsError::NoSurfaceFormats));
    }

    #[test]
    fn rejects_empty_presentation_modes() {
        let mut capabilities = sample_capabilities();
        capabilities.present_modes.clear();

        let result = build_surface_configuration(&capabilities, 1280, 720);

        assert_eq!(result, Err(GraphicsError::NoPresentModes));
    }

    #[test]
    fn rejects_empty_alpha_modes() {
        let mut capabilities = sample_capabilities();
        capabilities.alpha_modes.clear();

        let result = build_surface_configuration(&capabilities, 1280, 720);

        assert_eq!(result, Err(GraphicsError::NoAlphaModes));
    }
}
