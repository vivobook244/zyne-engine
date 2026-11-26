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
    use super::{HDR_FORMAT, MAX_EXPOSURE_EV, MIN_EXPOSURE_EV, is_valid_exposure_ev};

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
