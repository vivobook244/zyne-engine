use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use zyne_math::{Camera3d, Transform3d};

use crate::MaterialData;
use crate::lighting::{
    DirectionalLightData, LightingData, MAX_DIRECTIONAL_LIGHTS, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS,
    PointLightData, SpotLightData,
};
use crate::mesh::MeshBounds;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CHECKER_TEXTURE_SIZE: u32 = 64;
const CHECKER_TILE_SIZE: u32 = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
    normal: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x3];

    const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 3],
}

impl InstanceRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4
    ];

    const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }

    fn from_transform(transform: Transform3d) -> Self {
        let model = transform.to_matrix();
        let rotation = Transform3d::new([0.0; 3], transform.rotation(), [1.0; 3])
            .expect("a validated transform rotation must remain valid")
            .to_matrix();
        Self {
            model,
            normal_matrix: normal_matrix_from_model(model, transform.scale(), rotation),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
}

impl MaterialUniform {
    fn from_data(data: &MaterialData) -> Self {
        let valid = |value: f32| {
            if value.is_finite() && value >= 0.0 {
                value
            } else {
                0.0
            }
        };
        Self {
            ambient: [
                valid(data.ambient()[0]),
                valid(data.ambient()[1]),
                valid(data.ambient()[2]),
                1.0,
            ],
            diffuse: [
                valid(data.diffuse()[0]),
                valid(data.diffuse()[1]),
                valid(data.diffuse()[2]),
                1.0,
            ],
            specular: [
                valid(data.specular()[0]),
                valid(data.specular()[1]),
                valid(data.specular()[2]),
                if data.shininess().is_finite() {
                    data.shininess().clamp(1.0, 256.0)
                } else {
                    32.0
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ShadowCameraUniform {
    view_projection: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct DirectionalLightRaw {
    direction: [f32; 4],
    color_intensity: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct PointLightRaw {
    position_range: [f32; 4],
    color_intensity: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SpotLightRaw {
    position_range: [f32; 4],
    direction_inner_cos: [f32; 4],
    color_intensity: [f32; 4],
    outer_cos_padding: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct LightingUniform {
    ambient_color: [f32; 4],
    counts: [u32; 4],
    directional_lights: [DirectionalLightRaw; MAX_DIRECTIONAL_LIGHTS],
    point_lights: [PointLightRaw; MAX_POINT_LIGHTS],
    spot_lights: [SpotLightRaw; MAX_SPOT_LIGHTS],
}

impl LightingUniform {
    fn from_data(data: &LightingData) -> Self {
        let mut uniform = Self::zeroed();
        uniform.ambient_color = [
            valid_color_component(data.ambient_color[0]),
            valid_color_component(data.ambient_color[1]),
            valid_color_component(data.ambient_color[2]),
            1.0,
        ];

        let mut directional_count = 0;
        for light in &data.directional_lights {
            if directional_count == MAX_DIRECTIONAL_LIGHTS {
                break;
            }
            if valid_directional_light(*light) {
                uniform.directional_lights[directional_count] = directional_light_raw(*light);
                directional_count += 1;
            }
        }

        let mut point_count = 0;
        for light in &data.point_lights {
            if point_count == MAX_POINT_LIGHTS {
                break;
            }
            if valid_position(light.position)
                && valid_color(light.color)
                && valid_intensity(light.intensity)
                && valid_range(light.range)
            {
                uniform.point_lights[point_count] = point_light_raw(*light);
                point_count += 1;
            }
        }

        let mut spot_count = 0;
        for light in &data.spot_lights {
            if spot_count == MAX_SPOT_LIGHTS {
                break;
            }
            if valid_position(light.position)
                && valid_direction(light.direction)
                && valid_color(light.color)
                && valid_intensity(light.intensity)
                && valid_range(light.range)
                && valid_spot_angles(light.inner_angle, light.outer_angle)
            {
                uniform.spot_lights[spot_count] = spot_light_raw(*light);
                spot_count += 1;
            }
        }

        uniform.counts = [
            u32::try_from(directional_count).unwrap_or(u32::MAX),
            u32::try_from(point_count).unwrap_or(u32::MAX),
            u32::try_from(spot_count).unwrap_or(u32::MAX),
            0,
        ];
        uniform
    }
}

fn valid_color_component(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn valid_color(color: [f32; 3]) -> bool {
    color
        .iter()
        .all(|component| component.is_finite() && *component >= 0.0)
}

fn valid_intensity(intensity: f32) -> bool {
    intensity.is_finite() && intensity >= 0.0
}

fn valid_range(range: f32) -> bool {
    range.is_finite() && range > 0.0
}

fn valid_position(position: [f32; 3]) -> bool {
    position.iter().all(|component| component.is_finite())
}

fn valid_direction(direction: [f32; 3]) -> bool {
    if !valid_position(direction) {
        return false;
    }
    direction
        .iter()
        .map(|component| f64::from(*component) * f64::from(*component))
        .sum::<f64>()
        > f64::from(f32::EPSILON)
}

fn valid_directional_light(light: DirectionalLightData) -> bool {
    valid_direction(light.direction) && valid_color(light.color) && valid_intensity(light.intensity)
}

fn normalized_direction(direction: [f32; 3]) -> [f32; 3] {
    // Scale before measuring length so large finite inputs cannot overflow
    // and normalization stays entirely in f32 without lossy casts.
    let scale = direction
        .iter()
        .map(|component| component.abs())
        .fold(0.0_f32, f32::max);
    let scaled = direction.map(|component| component / scale);
    let length = scaled
        .iter()
        .map(|component| component * component)
        .sum::<f32>()
        .sqrt();
    scaled.map(|component| component / length)
}

fn valid_spot_angles(inner_angle: f32, outer_angle: f32) -> bool {
    inner_angle.is_finite()
        && outer_angle.is_finite()
        && inner_angle >= 0.0
        && outer_angle >= inner_angle
        && outer_angle <= std::f32::consts::FRAC_PI_2
}

fn directional_light_raw(light: DirectionalLightData) -> DirectionalLightRaw {
    let direction = normalized_direction(light.direction);
    DirectionalLightRaw {
        direction: [direction[0], direction[1], direction[2], 0.0],
        color_intensity: [
            light.color[0],
            light.color[1],
            light.color[2],
            light.intensity,
        ],
    }
}

fn point_light_raw(light: PointLightData) -> PointLightRaw {
    PointLightRaw {
        position_range: [
            light.position[0],
            light.position[1],
            light.position[2],
            light.range,
        ],
        color_intensity: [
            light.color[0],
            light.color[1],
            light.color[2],
            light.intensity,
        ],
    }
}

fn spot_light_raw(light: SpotLightData) -> SpotLightRaw {
    let direction = normalized_direction(light.direction);
    SpotLightRaw {
        position_range: [
            light.position[0],
            light.position[1],
            light.position[2],
            light.range,
        ],
        direction_inner_cos: [
            direction[0],
            direction[1],
            direction[2],
            light.inner_angle.cos(),
        ],
        color_intensity: [
            light.color[0],
            light.color[1],
            light.color[2],
            light.intensity,
        ],
        outer_cos_padding: [light.outer_angle.cos(), 0.0, 0.0, 0.0],
    }
}

type CubeFace = ([f32; 3], [f32; 3], [[f32; 3]; 4]);

const CUBE_FACES: [CubeFace; 6] = [
    (
        [0.95, 0.28, 0.22],
        [0.0, 0.0, 1.0],
        [
            [-0.7, -0.7, 0.7],
            [0.7, -0.7, 0.7],
            [0.7, 0.7, 0.7],
            [-0.7, 0.7, 0.7],
        ],
    ),
    (
        [0.22, 0.72, 0.95],
        [0.0, 0.0, -1.0],
        [
            [0.7, -0.7, -0.7],
            [-0.7, -0.7, -0.7],
            [-0.7, 0.7, -0.7],
            [0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.30, 0.85, 0.48],
        [1.0, 0.0, 0.0],
        [
            [0.7, -0.7, 0.7],
            [0.7, -0.7, -0.7],
            [0.7, 0.7, -0.7],
            [0.7, 0.7, 0.7],
        ],
    ),
    (
        [0.95, 0.72, 0.24],
        [-1.0, 0.0, 0.0],
        [
            [-0.7, -0.7, -0.7],
            [-0.7, -0.7, 0.7],
            [-0.7, 0.7, 0.7],
            [-0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.68, 0.42, 0.92],
        [0.0, 1.0, 0.0],
        [
            [-0.7, 0.7, 0.7],
            [0.7, 0.7, 0.7],
            [0.7, 0.7, -0.7],
            [-0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.24, 0.78, 0.82],
        [0.0, -1.0, 0.0],
        [
            [-0.7, -0.7, -0.7],
            [0.7, -0.7, -0.7],
            [0.7, -0.7, 0.7],
            [-0.7, -0.7, 0.7],
        ],
    ),
];

fn cube_vertices() -> [Vertex; 36] {
    let mut vertices = [Vertex {
        position: [0.0; 3],
        color: [0.0; 3],
        uv: [0.0; 2],
        normal: [0.0, 0.0, 1.0],
    }; 36];
    let face_uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let triangles = [0, 1, 2, 0, 2, 3];

    for (face_index, (color, normal, corners)) in CUBE_FACES.iter().enumerate() {
        for (vertex_index, triangle_corner) in triangles.iter().enumerate() {
            vertices[face_index * 6 + vertex_index] = Vertex {
                position: corners[*triangle_corner],
                color: *color,
                uv: face_uvs[*triangle_corner],
                normal: *normal,
            };
        }
    }

    vertices
}

/// A perspective-projected, depth-tested cube pipeline.
pub(super) struct CubePipeline {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_view: wgpu::TextureView,
    _shadow_texture: wgpu::Texture,
    _shadow_sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    shadow_camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    shadow_camera_bind_group: wgpu::BindGroup,
    lighting_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,
    material_layout: wgpu::BindGroupLayout,
    _default_material_buffer: wgpu::Buffer,
    default_material_bind_group: wgpu::BindGroup,
    checker_texture: CheckerTexture,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
}

impl CubePipeline {
    #[allow(clippy::too_many_lines)]
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ZYNE 3D Cube Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic_3d.wgsl").into()),
        });
        let shadow_resources = create_shadow_resources(device);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ZYNE Camera And Shadow Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ZYNE Camera Uniform"),
            contents: bytemuck::bytes_of(&CameraUniform {
                view_projection: identity_matrix(),
                camera_position: [4.0, 2.8, 4.0, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZYNE Camera And Shadow Bind Group"),
            layout: &camera_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_resources.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_resources.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: shadow_resources.camera_buffer.as_entire_binding(),
                },
            ],
        });

        let shadow_camera_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ZYNE Shadow Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let shadow_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZYNE Shadow Camera Bind Group"),
            layout: &shadow_camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 3,
                resource: shadow_resources.camera_buffer.as_entire_binding(),
            }],
        });
        let lighting_resources = create_lighting_resources(device);
        let material_resources = create_material_resources(device, &MaterialData::default());
        let checker_texture = create_checker_texture(device, queue);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ZYNE 3D Cube Pipeline Layout"),
            bind_group_layouts: &[
                &camera_layout,
                &checker_texture.layout,
                &lighting_resources.layout,
                &material_resources.layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ZYNE 3D Cube Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout(), InstanceRaw::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ZYNE Directional Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_camera_layout],
                push_constant_ranges: &[],
            });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ZYNE Directional Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout(), InstanceRaw::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ZYNE Cube Vertices"),
            contents: bytemuck::cast_slice(&cube_vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ZYNE Scene Instance Buffer"),
            size: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            shadow_pipeline,
            shadow_view: shadow_resources.view,
            _shadow_texture: shadow_resources.texture,
            _shadow_sampler: shadow_resources.sampler,
            vertex_buffer,
            camera_buffer,
            shadow_camera_buffer: shadow_resources.camera_buffer,
            camera_bind_group,
            shadow_camera_bind_group,
            lighting_buffer: lighting_resources.buffer,
            lighting_bind_group: lighting_resources.bind_group,
            material_layout: material_resources.layout,
            _default_material_buffer: material_resources.buffer,
            default_material_bind_group: material_resources.bind_group,
            checker_texture,
            instance_buffer,
            instance_capacity: 1,
        }
    }

    pub(super) fn update_camera(
        &self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        camera: Option<&Camera3d>,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        let default_camera;
        let camera = if let Some(camera) = camera {
            camera
        } else {
            #[allow(clippy::cast_precision_loss)]
            let aspect = width as f32 / height as f32;
            default_camera = Camera3d::new(
                [4.0, 2.8, 4.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                55.0_f32.to_radians(),
                aspect,
                0.1,
                100.0,
            )
            .expect("the built-in cube camera configuration must be valid");
            &default_camera
        };

        let uniform = CameraUniform {
            view_projection: camera.view_projection_matrix(),
            camera_position: [
                camera.position()[0],
                camera.position()[1],
                camera.position()[2],
                1.0,
            ],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(super) fn update_shadow_camera(
        &self,
        queue: &wgpu::Queue,
        lighting: Option<&LightingData>,
        bounds: Option<MeshBounds>,
    ) {
        let default_lighting;
        let lighting = if let Some(lighting) = lighting {
            lighting
        } else {
            default_lighting = LightingData::default();
            &default_lighting
        };
        let direction = directional_shadow_direction(lighting);
        let uniform = ShadowCameraUniform {
            view_projection: directional_shadow_matrix_for_bounds(direction, bounds),
        };
        queue.write_buffer(&self.shadow_camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(super) fn update_lighting(&self, queue: &wgpu::Queue, lighting: Option<&LightingData>) {
        let default_lighting;
        let lighting = if let Some(lighting) = lighting {
            lighting
        } else {
            default_lighting = LightingData::default();
            &default_lighting
        };
        let uniform = LightingUniform::from_data(lighting);
        queue.write_buffer(&self.lighting_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(super) fn update_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        transforms: Option<&[Transform3d]>,
    ) -> Result<u32, crate::GraphicsError> {
        let instances: Vec<InstanceRaw> = if let Some(transforms) = transforms {
            transforms
                .iter()
                .copied()
                .map(InstanceRaw::from_transform)
                .collect()
        } else {
            let model = rotation_y(0.15);
            vec![InstanceRaw {
                model,
                normal_matrix: normal_matrix_from_model(model, [1.0; 3], model),
            }]
        };
        let instance_count =
            u32::try_from(instances.len()).map_err(|_| crate::GraphicsError::TooManyInstances)?;
        if instances.is_empty() {
            return Ok(0);
        }

        if instances.len() > self.instance_capacity {
            let capacity = instances
                .len()
                .checked_next_power_of_two()
                .ok_or(crate::GraphicsError::TooManyInstances)?;
            let capacity =
                u64::try_from(capacity).map_err(|_| crate::GraphicsError::TooManyInstances)?;
            let buffer_size = capacity
                .checked_mul(std::mem::size_of::<InstanceRaw>() as u64)
                .ok_or(crate::GraphicsError::TooManyInstances)?;
            if buffer_size > device.limits().max_buffer_size {
                return Err(crate::GraphicsError::TooManyInstances);
            }
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ZYNE Scene Instance Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity =
                usize::try_from(capacity).map_err(|_| crate::GraphicsError::TooManyInstances)?;
        }

        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        Ok(instance_count)
    }

    pub(super) const fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub(super) const fn shadow_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.shadow_pipeline
    }

    pub(super) const fn shadow_camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.shadow_camera_bind_group
    }

    pub(super) const fn shadow_view(&self) -> &wgpu::TextureView {
        &self.shadow_view
    }

    pub(super) const fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(super) const fn camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    pub(super) const fn texture_bind_group(&self) -> &wgpu::BindGroup {
        &self.checker_texture.bind_group
    }

    pub(super) const fn lighting_bind_group(&self) -> &wgpu::BindGroup {
        &self.lighting_bind_group
    }

    pub(super) fn create_material_bind_group(
        &self,
        device: &wgpu::Device,
        data: &MaterialData,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform = MaterialUniform::from_data(data);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ZYNE Material Uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZYNE Material Bind Group"),
            layout: &self.material_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }

    pub(super) const fn default_material_bind_group(&self) -> &wgpu::BindGroup {
        &self.default_material_bind_group
    }

    pub(super) const fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }
}

struct ShadowResources {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    camera_buffer: wgpu::Buffer,
}

const SHADOW_MAP_SIZE: u32 = 2048;
const SHADOW_MAP_SIZE_F32: f32 = 2048.0;

fn create_shadow_resources(device: &wgpu::Device) -> ShadowResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ZYNE Directional Shadow Map"),
        size: wgpu::Extent3d {
            width: SHADOW_MAP_SIZE,
            height: SHADOW_MAP_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ZYNE Directional Shadow Comparison Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ZYNE Shadow Camera Uniform"),
        contents: bytemuck::bytes_of(&ShadowCameraUniform {
            view_projection: directional_shadow_matrix([-0.4, -1.0, -0.3]),
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    ShadowResources {
        texture,
        view,
        sampler,
        camera_buffer,
    }
}

fn directional_shadow_direction(lighting: &LightingData) -> [f32; 3] {
    // The first light accepted by the GPU upload owns the single shadow map.
    // Keep this predicate identical to LightingUniform::from_data so CPU shadow
    // camera selection and shader directional_lights[0] cannot diverge.
    lighting
        .directional_lights
        .iter()
        .find(|light| valid_directional_light(**light))
        .map_or([-0.4, -1.0, -0.3], |light| {
            normalized_direction(light.direction)
        })
}

fn directional_shadow_matrix(direction: [f32; 3]) -> [[f32; 4]; 4] {
    directional_shadow_matrix_for_bounds(direction, None)
}

fn directional_shadow_matrix_for_bounds(
    direction: [f32; 3],
    bounds: Option<MeshBounds>,
) -> [[f32; 4]; 4] {
    let direction = normalized_direction(direction);
    let (center, radius) = bounds.map_or(([0.0; 3], 10.0), |bounds| {
        let center = [
            f32::midpoint(bounds.min[0], bounds.max[0]),
            f32::midpoint(bounds.min[1], bounds.max[1]),
            f32::midpoint(bounds.min[2], bounds.max[2]),
        ];
        let extent = [
            (bounds.max[0] - bounds.min[0]) * 0.5,
            (bounds.max[1] - bounds.min[1]) * 0.5,
            (bounds.max[2] - bounds.min[2]) * 0.5,
        ];
        let radius = dot_vector(extent, extent).sqrt().max(1.0);
        (center, radius)
    });
    let margin = (radius * 0.1).max(0.5);
    let near = 0.1;
    let light_distance = radius + margin + near;
    let far = light_distance + radius + margin;
    let half_extent = radius + margin;
    let world_up = if direction[1].abs() > 0.95 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = right_vector(direction, world_up);
    let up = cross_vector(right, direction);
    let light_position = [
        center[0] - direction[0] * light_distance,
        center[1] - direction[1] * light_distance,
        center[2] - direction[2] * light_distance,
    ];
    let texel_world_size = (2.0 * half_extent) / SHADOW_MAP_SIZE_F32;
    let snapped_right = snap_to_shadow_texel(dot_vector(right, center), texel_world_size);
    let snapped_up = snap_to_shadow_texel(dot_vector(up, center), texel_world_size);
    let view = [
        [right[0], up[0], -direction[0], 0.0],
        [right[1], up[1], -direction[1], 0.0],
        [right[2], up[2], -direction[2], 0.0],
        [
            -snapped_right,
            -snapped_up,
            dot_vector(direction, light_position),
            1.0,
        ],
    ];
    let projection = [
        [1.0 / half_extent, 0.0, 0.0, 0.0],
        [0.0, 1.0 / half_extent, 0.0, 0.0],
        [0.0, 0.0, 1.0 / (near - far), 0.0],
        [0.0, 0.0, near / (near - far), 1.0],
    ];
    multiply_column_major(projection, view)
}

fn right_vector(direction: [f32; 3], world_up: [f32; 3]) -> [f32; 3] {
    normalize_vector(cross_vector(direction, world_up))
}

fn snap_to_shadow_texel(value: f32, texel_world_size: f32) -> f32 {
    (value / texel_world_size).round() * texel_world_size
}

fn normalize_vector(value: [f32; 3]) -> [f32; 3] {
    let length = dot_vector(value, value).sqrt().max(1.0e-8);
    value.map(|component| component / length)
}

fn dot_vector(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross_vector(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

struct MaterialResources {
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

fn create_material_resources(device: &wgpu::Device, data: &MaterialData) -> MaterialResources {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ZYNE Material Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let uniform = MaterialUniform::from_data(data);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ZYNE Default Material Uniform"),
        contents: bytemuck::bytes_of(&uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ZYNE Default Material Bind Group"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    MaterialResources {
        layout,
        buffer,
        bind_group,
    }
}

struct LightingResources {
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

fn create_lighting_resources(device: &wgpu::Device) -> LightingResources {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ZYNE Lighting Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ZYNE Lighting Uniform"),
        contents: bytemuck::bytes_of(&LightingUniform::from_data(&LightingData::default())),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ZYNE Lighting Bind Group"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    LightingResources {
        layout,
        buffer,
        bind_group,
    }
}

struct CheckerTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

fn create_checker_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> CheckerTexture {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ZYNE Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ZYNE Embedded Checker Texture"),
        size: wgpu::Extent3d {
            width: CHECKER_TEXTURE_SIZE,
            height: CHECKER_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let pixels = checker_texture_pixels();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(CHECKER_TEXTURE_SIZE * 4),
            rows_per_image: Some(CHECKER_TEXTURE_SIZE),
        },
        wgpu::Extent3d {
            width: CHECKER_TEXTURE_SIZE,
            height: CHECKER_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ZYNE Checker Texture Sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ZYNE Checker Texture Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    CheckerTexture {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        layout,
        bind_group,
    }
}

fn checker_texture_pixels() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((CHECKER_TEXTURE_SIZE * CHECKER_TEXTURE_SIZE * 4) as usize);

    for y in 0..CHECKER_TEXTURE_SIZE {
        for x in 0..CHECKER_TEXTURE_SIZE {
            let is_light = (x / CHECKER_TILE_SIZE + y / CHECKER_TILE_SIZE) % 2 == 0;
            let value = if is_light { 235 } else { 42 };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }

    pixels
}

fn identity_matrix() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn multiply_column_major(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];

    for column in 0..4 {
        for row in 0..4 {
            for inner in 0..4 {
                result[column][row] += a[inner][row] * b[column][inner];
            }
        }
    }

    result
}

fn rotation_y(angle: f32) -> [[f32; 4]; 4] {
    let (sine, cosine) = angle.sin_cos();
    [
        [cosine, 0.0, -sine, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Builds an inverse-transpose normal matrix from a column-major model matrix.
///
/// Singular or near-singular scales use the rotation-only basis so the GPU
/// never receives infinities or NaNs. Lighting for such collapsed geometry is
/// necessarily approximate because a singular transform has no unique normal matrix.
fn normal_matrix_from_model(
    model: [[f32; 4]; 4],
    scale: [f32; 3],
    rotation: [[f32; 4]; 4],
) -> [[f32; 4]; 3] {
    const MIN_ABS_SCALE: f32 = 1.0e-6;
    let singular = scale
        .iter()
        .any(|component| component.abs() < MIN_ABS_SCALE);
    let source = if singular { rotation } else { model };

    let mut normal = [[0.0; 4]; 3];
    for column in 0..3 {
        for row in 0..3 {
            normal[column][row] = if singular {
                source[column][row]
            } else {
                // Divide sequentially to avoid overflowing scale * scale.
                source[column][row] / scale[column] / scale[column]
            };
        }
    }
    normal
}

#[cfg(test)]
mod tests {
    use super::{
        CHECKER_TEXTURE_SIZE, Camera3d, InstanceRaw, Transform3d, checker_texture_pixels,
        cube_vertices, multiply_column_major, rotation_y,
    };

    #[test]
    fn shadow_camera_is_stable_under_sub_texel_scene_motion() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let bounds = MeshBounds::new([-1.0; 3], [1.0; 3]);
        let tiny_motion = MeshBounds::new([-0.9999, -1.0, -1.0], [1.0001, 1.0, 1.0]);
        let stable = directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(bounds));
        let after_tiny_motion =
            directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(tiny_motion));

        assert_eq!(
            stable, after_tiny_motion,
            "sub-texel movement should not shift the shadow projection"
        );
    }

    #[test]
    fn shadow_camera_is_stable_under_sub_texel_motion_on_both_light_plane_axes() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let bounds = MeshBounds::new([-1.0; 3], [1.0; 3]);
        // With a vertical light, the shadow-map plane spans world X and Z.
        let tiny_right_motion = MeshBounds::new([-1.0001, -1.0, -1.0], [0.9999, 1.0, 1.0]);
        let tiny_up_motion = MeshBounds::new([-1.0, -1.0, -0.9999], [1.0, 1.0, 1.0001]);
        let baseline = directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(bounds));

        assert_eq!(
            baseline,
            directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(tiny_right_motion)),
            "sub-texel movement along shadow right must not shift the projection"
        );
        assert_eq!(
            baseline,
            directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(tiny_up_motion)),
            "sub-texel movement along shadow up must not shift the projection"
        );
    }

    #[test]
    fn shadow_camera_tracks_scene_motion_along_light_depth_without_losing_coverage() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let direction = [-0.4, -1.0, -0.3];
        let original = MeshBounds::new([-2.0, -1.0, -3.0], [2.0, 3.0, 1.0]);
        let moved = MeshBounds::new(
            [
                original.min[0] + direction[0] * 25.0,
                original.min[1] + direction[1] * 25.0,
                original.min[2] + direction[2] * 25.0,
            ],
            [
                original.max[0] + direction[0] * 25.0,
                original.max[1] + direction[1] * 25.0,
                original.max[2] + direction[2] * 25.0,
            ],
        );
        let first = directional_shadow_matrix_for_bounds(direction, Some(original));
        let second = directional_shadow_matrix_for_bounds(direction, Some(moved));

        assert_ne!(
            first, second,
            "moving the scene along the light depth must update the shadow camera"
        );
        assert!(second.iter().flatten().all(|value| value.is_finite()));

        for bounds in [original, moved] {
            let matrix = directional_shadow_matrix_for_bounds(direction, Some(bounds));
            for x in [bounds.min[0], bounds.max[0]] {
                for y in [bounds.min[1], bounds.max[1]] {
                    for z in [bounds.min[2], bounds.max[2]] {
                        let point = [x, y, z, 1.0];
                        let clip = std::array::from_fn::<_, 4, _>(|row| {
                            (0..4)
                                .map(|column| matrix[column][row] * point[column])
                                .sum::<f32>()
                        });
                        assert!(clip[0].abs() <= 1.001, "x clip coordinate: {}", clip[0]);
                        assert!(clip[1].abs() <= 1.001, "y clip coordinate: {}", clip[1]);
                        assert!(
                            (0.0..=1.001).contains(&clip[2]),
                            "z clip coordinate: {}",
                            clip[2]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn shadow_camera_updates_after_scene_crosses_shadow_texel() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let bounds = MeshBounds::new([-1.0; 3], [1.0; 3]);
        let moved = MeshBounds::new([-0.99, -1.0, -1.0], [1.01, 1.0, 1.0]);
        let first = directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(bounds));
        let second = directional_shadow_matrix_for_bounds([0.0, -1.0, 0.0], Some(moved));

        assert_ne!(
            first, second,
            "shadow projection should advance when the scene crosses a texel boundary"
        );
    }

    #[test]
    fn shadow_camera_fits_scene_bounds() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let bounds = MeshBounds::new([-4.0, -2.0, -3.0], [5.0, 3.0, 6.0]);
        let matrix = directional_shadow_matrix_for_bounds([-0.4, -1.0, -0.3], Some(bounds));
        for x in [bounds.min[0], bounds.max[0]] {
            for y in [bounds.min[1], bounds.max[1]] {
                for z in [bounds.min[2], bounds.max[2]] {
                    let point = [x, y, z, 1.0];
                    let clip = std::array::from_fn::<_, 4, _>(|row| {
                        (0..4)
                            .map(|column| matrix[column][row] * point[column])
                            .sum::<f32>()
                    });
                    assert!(clip[0].abs() <= 1.001, "x clip coordinate: {}", clip[0]);
                    assert!(clip[1].abs() <= 1.001, "y clip coordinate: {}", clip[1]);
                    assert!(
                        (0.0..=1.001).contains(&clip[2]),
                        "z clip coordinate: {}",
                        clip[2]
                    );
                }
            }
        }
    }

    #[test]
    fn shadow_camera_tracks_scene_bounds_center_and_scale() {
        use super::directional_shadow_matrix_for_bounds;
        use crate::mesh::MeshBounds;

        let first = directional_shadow_matrix_for_bounds(
            [-0.4, -1.0, -0.3],
            Some(MeshBounds::new([-1.0; 3], [1.0; 3])),
        );
        let second = directional_shadow_matrix_for_bounds(
            [-0.4, -1.0, -0.3],
            Some(MeshBounds::new([19.0, -1.0, 4.0], [31.0, 5.0, 16.0])),
        );
        assert_ne!(first, second);
        assert!(second.iter().flatten().all(|value| value.is_finite()));
    }

    #[test]
    fn shadow_camera_tracks_directional_light_changes() {
        use super::{directional_shadow_direction, directional_shadow_matrix};
        use crate::lighting::{DirectionalLightData, LightingData};

        let mut lighting = LightingData::default();
        lighting.directional_lights.push(DirectionalLightData {
            direction: [0.0, -1.0, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
        });
        let first = directional_shadow_matrix(directional_shadow_direction(&lighting));

        lighting.directional_lights[0].direction = [1.0, -1.0, 0.5];
        let second = directional_shadow_matrix(directional_shadow_direction(&lighting));

        assert_ne!(
            first, second,
            "moving the light must update the shadow camera"
        );
        assert!(second.iter().flatten().all(|value| value.is_finite()));
    }

    #[test]
    fn shadow_camera_skips_invalid_directional_lights() {
        use super::{directional_shadow_direction, directional_shadow_matrix};
        use crate::lighting::{DirectionalLightData, LightingData};

        let lighting = LightingData {
            directional_lights: vec![
                DirectionalLightData {
                    direction: [0.0; 3],
                    color: [1.0; 3],
                    intensity: 1.0,
                },
                DirectionalLightData {
                    direction: [1.0, -2.0, 0.5],
                    color: [1.0; 3],
                    intensity: 1.0,
                },
            ],
            ..Default::default()
        };

        let selected = directional_shadow_direction(&lighting);
        let expected = directional_shadow_matrix([1.0, -2.0, 0.5]);
        assert_eq!(directional_shadow_matrix(selected), expected);
    }

    #[test]
    fn shadow_camera_matches_first_directional_light_uploaded_to_gpu() {
        use super::{LightingUniform, directional_shadow_direction, directional_shadow_matrix};
        use crate::lighting::{DirectionalLightData, LightingData};

        let lighting = LightingData {
            directional_lights: vec![
                DirectionalLightData {
                    direction: [0.0, -1.0, 0.0],
                    color: [f32::NAN, 1.0, 1.0],
                    intensity: 1.0,
                },
                DirectionalLightData {
                    direction: [1.0, -2.0, 0.5],
                    color: [1.0; 3],
                    intensity: 1.0,
                },
                DirectionalLightData {
                    direction: [0.0, 0.0, 1.0],
                    color: [1.0; 3],
                    intensity: f32::INFINITY,
                },
            ],
            ..Default::default()
        };

        let uniform = LightingUniform::from_data(&lighting);
        let selected = directional_shadow_direction(&lighting);
        let expected_direction = [1.0, -2.0, 0.5];

        assert_eq!(uniform.counts[0], 1);
        assert_eq!(
            uniform.directional_lights[0].direction[..3],
            super::normalized_direction(expected_direction)
        );
        assert_eq!(
            directional_shadow_matrix(selected),
            directional_shadow_matrix(expected_direction),
            "the shadow camera must follow the first light accepted by GPU upload"
        );
    }

    #[test]
    fn shadow_camera_selection_preserves_gpu_light_order() {
        use super::{LightingUniform, directional_shadow_direction, directional_shadow_matrix};
        use crate::lighting::{DirectionalLightData, LightingData};

        let first = DirectionalLightData {
            direction: [0.0, -1.0, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
        };
        let second = DirectionalLightData {
            direction: [1.0, 0.0, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
        };
        let lighting = LightingData {
            directional_lights: vec![first, second],
            ..Default::default()
        };

        let uniform = LightingUniform::from_data(&lighting);
        assert_eq!(uniform.counts[0], 2);
        assert_eq!(
            directional_shadow_matrix(directional_shadow_direction(&lighting)),
            directional_shadow_matrix(first.direction),
            "the first valid uploaded directional light must retain shadow ownership"
        );
    }

    #[test]
    fn shadow_camera_matrix_remains_finite_for_vertical_light_directions() {
        use super::directional_shadow_matrix;

        for direction in [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]] {
            let matrix = directional_shadow_matrix(direction);
            assert!(
                matrix.iter().flatten().all(|value| value.is_finite()),
                "shadow matrix must remain finite for {direction:?}"
            );
        }
    }

    #[test]
    fn basic_3d_shader_parses_and_passes_wgsl_validation() {
        let source = include_str!("shaders/basic_3d.wgsl");
        let module = naga::front::wgsl::parse_str(source)
            .expect("basic_3d.wgsl should parse as valid WGSL syntax");

        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("basic_3d.wgsl should pass Naga semantic validation");
    }

    #[test]
    fn lighting_uniform_uploads_supported_light_counts_and_parameters() {
        use super::LightingUniform;
        use crate::lighting::{DirectionalLightData, LightingData, PointLightData, SpotLightData};

        let mut lighting = LightingData::with_ambient([0.1, 0.2, 0.3]);
        lighting.directional_lights.push(DirectionalLightData {
            direction: [0.0, -1.0, 0.0],
            color: [1.0, 0.8, 0.6],
            intensity: 2.0,
        });
        lighting.point_lights.push(PointLightData {
            position: [1.0, 2.0, 3.0],
            color: [0.5, 0.6, 0.7],
            intensity: 3.0,
            range: 12.0,
        });
        lighting.spot_lights.push(SpotLightData {
            position: [4.0, 5.0, 6.0],
            direction: [0.0, 0.0, -1.0],
            color: [0.9, 0.8, 0.7],
            intensity: 4.0,
            range: 20.0,
            inner_angle: 15.0_f32.to_radians(),
            outer_angle: 30.0_f32.to_radians(),
        });

        let uniform = LightingUniform::from_data(&lighting);
        assert_eq!(uniform.ambient_color, [0.1, 0.2, 0.3, 1.0]);
        assert_eq!(uniform.counts, [1, 1, 1, 0]);
        assert_eq!(
            uniform.directional_lights[0].direction,
            [0.0, -1.0, 0.0, 0.0]
        );
        assert_eq!(
            uniform.directional_lights[0].color_intensity,
            [1.0, 0.8, 0.6, 2.0]
        );
        assert_eq!(
            uniform.point_lights[0].position_range,
            [1.0, 2.0, 3.0, 12.0]
        );
        assert_eq!(uniform.spot_lights[0].position_range, [4.0, 5.0, 6.0, 20.0]);
        assert!(
            (uniform.spot_lights[0].direction_inner_cos[3] - 15.0_f32.to_radians().cos()).abs()
                < 1.0e-6
        );
        assert!(
            (uniform.spot_lights[0].outer_cos_padding[0] - 30.0_f32.to_radians().cos()).abs()
                < 1.0e-6
        );
    }

    #[test]
    fn lighting_uniform_layout_matches_wgsl_uniform_buffer_contract() {
        use super::{
            DirectionalLightRaw, LightingUniform, PointLightRaw, SpotLightRaw,
        };
        use std::mem::{offset_of, size_of};

        // WGSL lays out every vec4 and fixed-size light array on 16-byte
        // boundaries. Keep the CPU Pod representation byte-for-byte aligned
        // with the shader's Lighting struct.
        assert_eq!(size_of::<DirectionalLightRaw>(), 32);
        assert_eq!(size_of::<PointLightRaw>(), 32);
        assert_eq!(size_of::<SpotLightRaw>(), 64);
        assert_eq!(offset_of!(LightingUniform, ambient_color), 0);
        assert_eq!(offset_of!(LightingUniform, counts), 16);
        assert_eq!(offset_of!(LightingUniform, directional_lights), 32);
        assert_eq!(offset_of!(LightingUniform, point_lights), 160);
        assert_eq!(offset_of!(LightingUniform, spot_lights), 672);
        assert_eq!(size_of::<LightingUniform>(), 1184);
        assert_eq!(bytemuck::bytes_of(&LightingUniform::zeroed()).len(), 1184);
    }

    #[test]
    fn lighting_uniform_caps_light_counts_to_gpu_capacity() {
        use super::LightingUniform;
        use crate::lighting::{
            DirectionalLightData, LightingData, MAX_DIRECTIONAL_LIGHTS, MAX_POINT_LIGHTS,
            MAX_SPOT_LIGHTS, PointLightData, SpotLightData,
        };

        let mut lighting = LightingData::with_ambient([0.1; 3]);
        lighting.directional_lights = vec![
            DirectionalLightData {
                direction: [0.0, -1.0, 0.0],
                color: [1.0; 3],
                intensity: 1.0,
            };
            MAX_DIRECTIONAL_LIGHTS + 2
        ];
        lighting.point_lights = vec![
            PointLightData {
                position: [0.0, 1.0, 0.0],
                color: [1.0; 3],
                intensity: 1.0,
                range: 10.0,
            };
            MAX_POINT_LIGHTS + 2
        ];
        lighting.spot_lights = vec![
            SpotLightData {
                position: [0.0, 1.0, 0.0],
                direction: [0.0, -1.0, 0.0],
                color: [1.0; 3],
                intensity: 1.0,
                range: 10.0,
                inner_angle: 15.0_f32.to_radians(),
                outer_angle: 30.0_f32.to_radians(),
            };
            MAX_SPOT_LIGHTS + 2
        ];

        let uniform = LightingUniform::from_data(&lighting);
        assert_eq!(
            uniform.counts,
            [
                u32::try_from(MAX_DIRECTIONAL_LIGHTS).unwrap_or(u32::MAX),
                u32::try_from(MAX_POINT_LIGHTS).unwrap_or(u32::MAX),
                u32::try_from(MAX_SPOT_LIGHTS).unwrap_or(u32::MAX),
                0,
            ]
        );
    }

    #[test]
    fn lighting_uniform_normalizes_large_finite_directions_without_overflow() {
        use super::LightingUniform;
        use crate::lighting::{DirectionalLightData, LightingData, SpotLightData};

        let mut lighting = LightingData::with_ambient([0.1; 3]);
        lighting.directional_lights.push(DirectionalLightData {
            direction: [f32::MAX, f32::MAX, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
        });
        lighting.spot_lights.push(SpotLightData {
            position: [0.0; 3],
            direction: [f32::MAX, 0.0, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
            range: 5.0,
            inner_angle: 0.2,
            outer_angle: 0.4,
        });

        let uniform = LightingUniform::from_data(&lighting);
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(uniform.counts, [1, 0, 1, 0]);
        assert!((uniform.directional_lights[0].direction[0] - diagonal).abs() < 1.0e-6);
        assert!((uniform.directional_lights[0].direction[1] - diagonal).abs() < 1.0e-6);
        assert_eq!(
            uniform.spot_lights[0].direction_inner_cos[..3],
            [1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn lighting_uniform_skips_invalid_values_before_gpu_upload() {
        use super::LightingUniform;
        use crate::lighting::{DirectionalLightData, LightingData, PointLightData, SpotLightData};

        let mut lighting = LightingData::with_ambient([f32::NAN, -1.0, 0.25]);
        lighting.directional_lights = vec![
            DirectionalLightData {
                direction: [0.0; 3],
                color: [1.0; 3],
                intensity: 1.0,
            },
            DirectionalLightData {
                direction: [0.0, -1.0, 0.0],
                color: [1.0; 3],
                intensity: 1.0,
            },
        ];
        lighting.point_lights = vec![PointLightData {
            position: [f32::INFINITY, 0.0, 0.0],
            color: [1.0; 3],
            intensity: 1.0,
            range: 5.0,
        }];
        lighting.spot_lights = vec![SpotLightData {
            position: [0.0; 3],
            direction: [0.0, 0.0, -1.0],
            color: [1.0; 3],
            intensity: 1.0,
            range: 5.0,
            inner_angle: 0.8,
            outer_angle: 0.4,
        }];

        let uniform = LightingUniform::from_data(&lighting);
        assert_eq!(uniform.ambient_color, [0.0, 0.0, 0.25, 1.0]);
        assert_eq!(uniform.counts, [1, 0, 0, 0]);
    }

    #[test]
    fn cube_has_twelve_triangles_and_valid_colors() {
        let vertices = cube_vertices();
        assert_eq!(vertices.len(), 36);
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.position.iter().all(|n| (-0.7..=0.7).contains(n)))
        );
        for face in 0..6 {
            let face_vertices = &vertices[(face * 6)..(face * 6 + 6)];
            assert!(face_vertices.iter().all(|vertex| {
                vertex
                    .normal
                    .iter()
                    .zip(face_vertices[0].normal)
                    .all(|(actual, expected)| (actual - expected).abs() < 1.0e-6)
            }));
            assert!(
                (face_vertices[0].normal.iter().map(|n| n * n).sum::<f32>() - 1.0).abs() < 1.0e-6
            );
        }
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.color.iter().all(|n| (0.0..=1.0).contains(n)))
        );
    }

    #[test]
    fn cube_vertices_include_face_uv_coordinates() {
        let vertices = cube_vertices();

        assert_eq!(vertices.len(), 36);
        assert!(vertices.iter().all(|vertex| {
            vertex
                .uv
                .iter()
                .all(|coordinate| (0.0..=1.0).contains(coordinate))
        }));
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn checker_texture_has_expected_rgba_size_and_alternating_tiles() {
        let pixels = checker_texture_pixels();
        let expected_bytes = (CHECKER_TEXTURE_SIZE * CHECKER_TEXTURE_SIZE * 4) as usize;

        assert_eq!(pixels.len(), expected_bytes);
        assert_eq!(&pixels[0..4], &[235, 235, 235, 255]);
        assert_eq!(&pixels[(8 * 4)..(8 * 4 + 4)], &[42, 42, 42, 255]);
        assert_eq!(&pixels[(8 * 64 * 4)..(8 * 64 * 4 + 4)], &[42, 42, 42, 255]);
    }

    #[test]
    fn model_transform_composes_after_camera_view_projection() {
        let camera = Camera3d::new(
            [4.0, 2.8, 4.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            55.0_f32.to_radians(),
            16.0 / 9.0,
            0.1,
            100.0,
        )
        .expect("camera configuration should be valid");
        let model = rotation_y(0.15);
        let combined = multiply_column_major(camera.view_projection_matrix(), model);

        assert!(combined.iter().flatten().all(|value| value.is_finite()));
        assert_ne!(combined, camera.view_projection_matrix());

        let instance = InstanceRaw::from_transform(Transform3d::identity());
        assert_eq!(instance.model, Transform3d::identity().to_matrix());
        assert_eq!(
            instance.normal_matrix,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0]
            ]
        );
    }

    #[test]
    fn shadow_instances_use_each_objects_world_transform() {
        let first_transform = Transform3d::new([2.0, 0.0, -1.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3])
            .expect("first object transform should be valid");
        let second_transform = Transform3d::new(
            [-3.0, 1.5, 4.0],
            [
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
            [2.0, 0.5, 1.5],
        )
        .expect("second object transform should be valid");

        // The shadow vertex entry point consumes the same per-instance model
        // attributes as the lit vertex entry point. Keep each object's world
        // transform intact when building the shared instance stream.
        let instances = [first_transform, second_transform].map(InstanceRaw::from_transform);

        assert_eq!(instances[0].model, first_transform.to_matrix());
        assert_eq!(instances[1].model, second_transform.to_matrix());
        assert_ne!(instances[0].model, instances[1].model);
        assert!(
            instances
                .iter()
                .flat_map(|instance| instance.model.iter().flatten())
                .all(|value| value.is_finite())
        );
        assert!(
            instances
                .iter()
                .flat_map(|instance| instance.normal_matrix.iter().flatten())
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn normal_matrix_uses_inverse_transpose_for_non_uniform_scale() {
        let transform = Transform3d::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [2.0, 4.0, 0.5])
            .expect("non-uniform scale should be valid");
        let instance = InstanceRaw::from_transform(transform);

        assert_eq!(instance.normal_matrix[0][0], 0.5);
        assert_eq!(instance.normal_matrix[1][1], 0.25);
        assert_eq!(instance.normal_matrix[2][2], 2.0);
    }

    #[test]
    fn singular_scale_falls_back_to_finite_rotation_basis() {
        let transform = Transform3d::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.0, 2.0, 3.0])
            .expect("zero scale is currently a valid transform");
        let instance = InstanceRaw::from_transform(transform);

        assert!(
            instance
                .normal_matrix
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
        assert_eq!(
            instance.normal_matrix,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0]
            ]
        );
    }

    #[test]
    fn column_major_multiplication_preserves_identity() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let model = rotation_y(0.5);
        let actual = multiply_column_major(identity, model);

        assert_eq!(actual, model);
    }
}
