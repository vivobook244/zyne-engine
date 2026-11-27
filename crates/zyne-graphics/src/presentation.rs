//! Linear HDR scene target and final surface presentation pipeline.

use wgpu::util::DeviceExt;

pub(super) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Owns the linear floating-point color target used by the forward scene pass.
pub(super) struct HdrRenderTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl HdrRenderTarget {
    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ZYNE Linear HDR Scene Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }

    pub(super) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

pub(super) const MIN_EXPOSURE_EV: f32 = -16.0;
pub(super) const MAX_EXPOSURE_EV: f32 = 16.0;

pub(super) fn is_valid_exposure_ev(value: f32) -> bool {
    value.is_finite() && (MIN_EXPOSURE_EV..=MAX_EXPOSURE_EV).contains(&value)
}

/// Presents the HDR scene texture through exposure compensation and tone mapping.
///
/// Scene-linear values are multiplied by the configured exposure scale before
/// the fitted ACES operator maps them into the display range.
pub(super) struct PresentationPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    exposure_buffer: wgpu::Buffer,
    exposure_ev: f32,
    encode_srgb_in_shader: bool,
}

impl PresentationPipeline {
    pub(super) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ZYNE HDR Presentation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/presentation.wgsl").into()),
        });
        let exposure_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ZYNE Presentation Exposure Uniform"),
            contents: bytemuck::cast_slice(&[
                0.0_f32,
                if surface_format.is_srgb() { 0.0 } else { 1.0 },
                0.0,
                0.0,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ZYNE HDR Presentation Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ZYNE HDR Presentation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ZYNE HDR Presentation Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            exposure_buffer,
            exposure_ev: 0.0,
            encode_srgb_in_shader: !surface_format.is_srgb(),
        }
    }

    pub(super) fn create_bind_group(
        &self,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZYNE HDR Presentation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.exposure_buffer.as_entire_binding(),
                },
            ],
        })
    }

    pub(super) fn set_exposure_ev(&mut self, queue: &wgpu::Queue, exposure_ev: f32) {
        self.exposure_ev = exposure_ev;
        queue.write_buffer(
            &self.exposure_buffer,
            0,
            bytemuck::cast_slice(&[
                exposure_ev,
                if self.encode_srgb_in_shader { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ]),
        );
    }

    pub(super) const fn exposure_ev(&self) -> f32 {
        self.exposure_ev
    }

    pub(super) const fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HDR_FORMAT, MAX_EXPOSURE_EV, MIN_EXPOSURE_EV, PresentationPipeline, is_valid_exposure_ev,
    };

    #[test]
    fn scene_target_uses_half_precision_floating_point_color() {
        assert_eq!(HDR_FORMAT, wgpu::TextureFormat::Rgba16Float);
    }

    #[test]
    fn exposure_ev_accepts_supported_finite_values() {
        assert!(is_valid_exposure_ev(MIN_EXPOSURE_EV));
        assert!(is_valid_exposure_ev(0.0));
        assert!(is_valid_exposure_ev(MAX_EXPOSURE_EV));
    }

    #[test]
    fn exposure_ev_rejects_non_finite_and_out_of_range_values() {
        assert!(!is_valid_exposure_ev(f32::NAN));
        assert!(!is_valid_exposure_ev(f32::INFINITY));
        assert!(!is_valid_exposure_ev(f32::NEG_INFINITY));
        assert!(!is_valid_exposure_ev(MIN_EXPOSURE_EV - 0.01));
        assert!(!is_valid_exposure_ev(MAX_EXPOSURE_EV + 0.01));
    }

    #[test]
    fn presentation_gpu_readback_matches_srgb_display_output_for_both_surface_formats() {
        use std::sync::mpsc;
        use wgpu::{
            Color, DeviceDescriptor, Instance, LoadOp, MapMode, Operations, PollType,
            RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, StoreOp,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            TextureViewDescriptor,
        };

        const WIDTH: u32 = 1;
        const HEIGHT: u32 = 1;
        const BYTES_PER_ROW: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

        pollster::block_on(async {
            let instance = Instance::default();
            let adapter = match instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
            {
                Ok(adapter) => adapter,
                Err(error) => {
                    eprintln!("GPU TEST BLOCKED: no headless adapter available: {error}");
                    return;
                }
            };
            let (device, queue) = adapter
                .request_device(&DeviceDescriptor {
                    label: Some("ZYNE Presentation Readback Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                })
                .await
                .expect("presentation test device should initialize");

            let hdr_texture = device.create_texture(&TextureDescriptor {
                label: Some("ZYNE Presentation Test HDR Input"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: HDR_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let hdr_view = hdr_texture.create_view(&TextureViewDescriptor::default());
            let mut clear_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ZYNE Presentation HDR Clear Encoder"),
                });
            {
                let attachments = [Some(RenderPassColorAttachment {
                    view: &hdr_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.18,
                            g: 0.18,
                            b: 0.18,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })];
                let _pass = clear_encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("ZYNE Presentation HDR Clear Pass"),
                    color_attachments: &attachments,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            queue.submit([clear_encoder.finish()]);

            for surface_format in [TextureFormat::Rgba8Unorm, TextureFormat::Rgba8UnormSrgb] {
                let output = device.create_texture(&TextureDescriptor {
                    label: Some("ZYNE Presentation Test Surface"),
                    size: wgpu::Extent3d {
                        width: WIDTH,
                        height: HEIGHT,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: surface_format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let output_view = output.create_view(&TextureViewDescriptor::default());
                let presentation = PresentationPipeline::new(&device, surface_format);
                let bind_group = presentation.create_bind_group(&device, &hdr_view);
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ZYNE Presentation Pixel Readback"),
                    size: u64::from(BYTES_PER_ROW),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ZYNE Presentation Render Encoder"),
                });
                {
                    let attachments = [Some(RenderPassColorAttachment {
                        view: &output_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    })];
                    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("ZYNE HDR Presentation Pass"),
                        color_attachments: &attachments,
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(presentation.pipeline());
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.draw(0..3, 0..1);
                }
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &output,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(BYTES_PER_ROW),
                            rows_per_image: Some(HEIGHT),
                        },
                    },
                    wgpu::Extent3d {
                        width: WIDTH,
                        height: HEIGHT,
                        depth_or_array_layers: 1,
                    },
                );
                queue.submit([encoder.finish()]);

                let slice = readback.slice(..);
                let (sender, receiver) = mpsc::channel();
                slice.map_async(MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
                device
                    .poll(PollType::Wait)
                    .expect("presentation GPU work should complete");
                receiver
                    .recv()
                    .expect("presentation map callback should run")
                    .expect("presentation output should map");
                let pixel = {
                    let mapped = slice.get_mapped_range();
                    [mapped[0], mapped[1], mapped[2], mapped[3]]
                };
                readback.unmap();

                assert!(
                    (132..=150).contains(&pixel[0]),
                    "0.18 linear gray should tone-map and encode to about 141 sRGB, got {pixel:?} for {surface_format:?}"
                );
                assert!(
                    pixel[0].abs_diff(pixel[1]) <= 1
                        && pixel[1].abs_diff(pixel[2]) <= 1
                        && pixel[3] >= 254,
                    "presentation should preserve neutral gray and opaque alpha, got {pixel:?} for {surface_format:?}"
                );
            }
        });
    }

    #[test]
    fn presentation_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(include_str!("shaders/presentation.wgsl"))
            .expect("presentation shader should parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("presentation shader should validate");
    }
}
