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
    FrameOutcome, GpuMaterial, GpuTexture, GraphicsError, LightingData, MaterialData,
    mesh::{GpuMesh, MeshBounds, MeshData, MeshRenderBatch},
    pipeline::CubePipeline,
    presentation::{HDR_FORMAT, HdrRenderTarget, PresentationPipeline, is_valid_exposure_ev},
    texture::TextureColorSpace,
};

fn merge_transformed_bounds(
    target: &mut Option<MeshBounds>,
    local_bounds: MeshBounds,
    transforms: &[Transform3d],
) {
    for transform in transforms {
        let world_bounds = local_bounds.transformed(*transform);
        *target = Some((*target).map_or(world_bounds, |current| current.union(world_bounds)));
    }
}

const DEFAULT_CLEAR_COLOR: Color = Color {
    r: 0.055,
    g: 0.065,
    b: 0.085,
    a: 1.0,
};

fn select_texture_binding<T>(texture: Option<&T>, fallback: &T) -> &T {
    texture.unwrap_or(fallback)
}

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
    hdr_target: HdrRenderTarget,
    presentation_pipeline: PresentationPipeline,
    presentation_bind_group: wgpu::BindGroup,
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
        let hdr_format_features = adapter.get_texture_format_features(HDR_FORMAT);
        if !hdr_format_features
            .allowed_usages
            .contains(TextureUsages::RENDER_ATTACHMENT)
            || !hdr_format_features
                .allowed_usages
                .contains(TextureUsages::TEXTURE_BINDING)
        {
            return Err(GraphicsError::HdrFormatUnsupported);
        }

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
        let cube_pipeline = CubePipeline::new(&device, &queue, HDR_FORMAT);
        let hdr_target = HdrRenderTarget::new(&device, surface_config.width, surface_config.height);
        let presentation_pipeline = PresentationPipeline::new(&device, surface_config.format);
        let presentation_bind_group =
            presentation_pipeline.create_bind_group(&device, hdr_target.view());
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
            hdr_target,
            presentation_pipeline,
            presentation_bind_group,
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
        let vertex_count =
            u64::try_from(data.vertices().len()).map_err(|_| GraphicsError::MeshBufferTooLarge)?;
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

        Ok(GpuMesh::new(
            vertex_buffer,
            index_buffer,
            index_count,
            data.bounds(),
        ))
    }

    /// Uploads tightly packed RGBA8 pixels into a reusable sampled GPU texture.
    ///
    /// Only linear and sRGB RGBA8 formats are accepted. The texture is owned by
    /// the returned handle and uses repeat addressing with linear filtering.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported formats, zero or oversized dimensions,
    /// or a pixel payload whose length does not exactly match RGBA8 dimensions.
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        color_space: TextureColorSpace,
        pixels: &[u8],
    ) -> Result<GpuTexture, GraphicsError> {
        crate::texture::create_gpu_texture(
            &self.device,
            &self.queue,
            self.cube_pipeline.texture_layout(),
            width,
            height,
            color_space,
            pixels,
        )
    }

    /// Uploads renderer-facing material parameters for reuse across draws.
    #[must_use]
    pub fn create_material(&self, data: &MaterialData) -> GpuMaterial {
        let (buffer, bind_group) = self
            .cube_pipeline
            .create_material_bind_group(&self.device, data);
        GpuMaterial::new(buffer, bind_group)
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
        self.render_frame_internal(None, None, None, None, None)
    }

    /// Sets presentation exposure compensation in EV stops.
    ///
    /// Zero EV preserves the scene exposure. Positive values brighten the
    /// scene before tone mapping; negative values darken it. The supported
    /// range is -16 to +16 EV.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is non-finite or outside the supported range.
    pub fn set_exposure_ev(&mut self, exposure_ev: f32) -> Result<(), GraphicsError> {
        if !is_valid_exposure_ev(exposure_ev) {
            return Err(GraphicsError::InvalidExposure);
        }

        self.presentation_pipeline
            .set_exposure_ev(&self.queue, exposure_ev);
        Ok(())
    }

    /// Returns the current presentation exposure compensation in EV stops.
    #[must_use]
    pub const fn exposure_ev(&self) -> f32 {
        self.presentation_pipeline.exposure_ev()
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
        self.render_frame_internal(Some(camera), None, None, None, None)
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
        self.render_frame_internal(camera, Some(transforms), None, None, None)
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
        self.render_frame_internal(camera, Some(transforms), Some(mesh), None, None)
    }

    /// Renders multiple uploaded meshes in one frame using instanced indexed draws.
    ///
    /// Each batch is drawn with its own contiguous range in the shared instance
    /// buffer. Empty batches are ignored; an empty batch list clears the frame.
    ///
    /// # Errors
    ///
    /// Returns an error if the combined instance buffer exceeds device limits
    /// or frame acquisition/presentation fails.
    pub fn render_frame_with_mesh_batches(
        &mut self,
        camera: Option<&Camera3d>,
        batches: &[MeshRenderBatch<'_>],
    ) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(camera, None, None, Some(batches), None)
    }

    /// Renders scene mesh batches using the supplied Blinn–Phong lighting snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if instance upload or frame acquisition/presentation fails.
    pub fn render_frame_with_mesh_batches_and_lighting(
        &mut self,
        camera: Option<&Camera3d>,
        batches: &[MeshRenderBatch<'_>],
        lighting: &LightingData,
    ) -> Result<FrameOutcome, GraphicsError> {
        self.render_frame_internal(camera, None, None, Some(batches), Some(lighting))
    }

    // Keep frame acquisition, resource updates, command encoding, and presentation
    // together: splitting this lifecycle would obscure its ordering guarantees.
    #[allow(clippy::too_many_lines)]
    fn render_frame_internal(
        &mut self,
        camera: Option<&Camera3d>,
        transforms: Option<&[Transform3d]>,
        mesh: Option<&GpuMesh>,
        mesh_batches: Option<&[MeshRenderBatch<'_>]>,
        lighting: Option<&LightingData>,
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

        self.cube_pipeline.update_lighting(&self.queue, lighting);

        let batch_transforms = mesh_batches.map(|batches| {
            batches
                .iter()
                .flat_map(|batch| batch.transforms().iter().copied())
                .collect::<Vec<_>>()
        });
        let frame_transforms = batch_transforms.as_deref().or(transforms);
        let shadow_bounds = if let Some(batches) = mesh_batches {
            let mut bounds = None;
            for batch in batches {
                merge_transformed_bounds(&mut bounds, batch.mesh().bounds(), batch.transforms());
            }
            bounds
        } else if let Some(mesh) = mesh {
            let mut bounds = None;
            merge_transformed_bounds(&mut bounds, mesh.bounds(), frame_transforms.unwrap_or(&[]));
            bounds
        } else {
            let cube_bounds = MeshBounds::new([-0.7; 3], [0.7; 3]);
            let mut bounds = None;
            if let Some(transforms) = frame_transforms {
                merge_transformed_bounds(&mut bounds, cube_bounds, transforms);
            } else {
                bounds = Some(cube_bounds);
            }
            bounds
        };
        self.cube_pipeline
            .update_shadow_camera(&self.queue, lighting, shadow_bounds);
        let instance_count =
            self.cube_pipeline
                .update_instances(&self.device, &self.queue, frame_transforms)?;
        let mut batch_ranges = Vec::new();
        if let Some(batches) = mesh_batches {
            let mut first_instance = 0_u32;
            for batch in batches {
                let count = u32::try_from(batch.transforms().len())
                    .map_err(|_| GraphicsError::TooManyInstances)?;
                let end_instance = first_instance
                    .checked_add(count)
                    .ok_or(GraphicsError::TooManyInstances)?;
                if count > 0 {
                    batch_ranges.push((
                        batch.mesh(),
                        batch.material(),
                        batch.texture(),
                        first_instance..end_instance,
                    ));
                }
                first_instance = end_instance;
            }
            if first_instance != instance_count {
                return Err(GraphicsError::TooManyInstances);
            }
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ZYNE Frame Command Encoder"),
            });

        {
            let mut shadow_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("ZYNE Directional Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.cube_pipeline.shadow_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            shadow_pass.set_pipeline(self.cube_pipeline.shadow_pipeline());
            shadow_pass.set_bind_group(0, self.cube_pipeline.shadow_camera_bind_group(), &[]);
            shadow_pass.set_vertex_buffer(1, self.cube_pipeline.instance_buffer().slice(..));
            if !batch_ranges.is_empty() {
                for (mesh, _, _, instances) in &batch_ranges {
                    shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
                    shadow_pass
                        .set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                    shadow_pass.draw_indexed(0..mesh.index_count(), 0, instances.clone());
                }
            } else if let Some(mesh) = mesh {
                shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
                shadow_pass
                    .set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.index_count(), 0, 0..instance_count);
            } else if mesh_batches.is_none() {
                shadow_pass.set_vertex_buffer(0, self.cube_pipeline.vertex_buffer().slice(..));
                shadow_pass.draw(0..36, 0..instance_count);
            }
        }

        {
            let color_attachments = [Some(RenderPassColorAttachment {
                view: self.hdr_target.view(),
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
            render_pass.set_bind_group(2, self.cube_pipeline.lighting_bind_group(), &[]);
            render_pass.set_vertex_buffer(1, self.cube_pipeline.instance_buffer().slice(..));
            if !batch_ranges.is_empty() {
                for (mesh, material, texture, instances) in &batch_ranges {
                    render_pass.set_bind_group(
                        1,
                        select_texture_binding(
                            texture.map(GpuTexture::bind_group),
                            self.cube_pipeline.texture_bind_group(),
                        ),
                        &[],
                    );
                    render_pass.set_bind_group(
                        3,
                        material.map_or_else(
                            || self.cube_pipeline.default_material_bind_group(),
                            |value| value.bind_group(),
                        ),
                        &[],
                    );
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
                    render_pass
                        .set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..mesh.index_count(), 0, instances.clone());
                }
            } else if let Some(mesh) = mesh {
                render_pass.set_bind_group(
                    3,
                    self.cube_pipeline.default_material_bind_group(),
                    &[],
                );
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count(), 0, 0..instance_count);
            } else if mesh_batches.is_none() {
                render_pass.set_bind_group(
                    3,
                    self.cube_pipeline.default_material_bind_group(),
                    &[],
                );
                render_pass.set_vertex_buffer(0, self.cube_pipeline.vertex_buffer().slice(..));
                render_pass.draw(0..36, 0..instance_count);
            }
        }

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
            let mut presentation_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("ZYNE HDR Presentation Pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            presentation_pass.set_pipeline(self.presentation_pipeline.pipeline());
            presentation_pass.set_bind_group(0, &self.presentation_bind_group, &[]);
            presentation_pass.draw(0..3, 0..1);
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
        self.hdr_target = HdrRenderTarget::new(
            &self.device,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.presentation_bind_group = self
            .presentation_pipeline
            .create_bind_group(&self.device, self.hdr_target.view());
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
    fn textured_batch_selects_its_texture_binding() {
        let texture = "material texture bind group";
        let fallback = "checker fallback bind group";

        let selected = super::select_texture_binding(Some(&texture), &fallback);

        assert_eq!(selected, &texture);
    }

    #[test]
    fn untextured_batch_selects_checker_fallback_binding() {
        let fallback = "checker fallback bind group";

        let selected: &&str = super::select_texture_binding(None, &fallback);

        assert_eq!(selected, &fallback);
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
