use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use zyne_math::{Camera3d, Transform3d};

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
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

const CUBE_FACES: [([f32; 3], [f32; 3], [[f32; 3]; 4]); 6] = [
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
    vertex_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    checker_texture: CheckerTexture,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
}

impl CubePipeline {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ZYNE 3D Cube Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic_3d.wgsl").into()),
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ZYNE Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ZYNE Camera Uniform"),
            contents: bytemuck::bytes_of(&CameraUniform {
                view_projection: identity_matrix(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ZYNE Camera Bind Group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let checker_texture = create_checker_texture(device, queue);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ZYNE 3D Cube Pipeline Layout"),
            bind_group_layouts: &[&camera_layout, &checker_texture.layout],
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
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
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
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub(super) fn update_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        transforms: Option<&[Transform3d]>,
    ) -> Result<u32, crate::GraphicsError> {
        let instances: Vec<InstanceRaw> = match transforms {
            Some(transforms) => transforms
                .iter()
                .copied()
                .map(InstanceRaw::from_transform)
                .collect(),
            None => {
                let model = rotation_y(0.15);
                vec![InstanceRaw {
                    model,
                    normal_matrix: normal_matrix_from_model(model, [1.0; 3], model),
                }]
            },
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

    pub(super) const fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(super) const fn camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    pub(super) const fn texture_bind_group(&self) -> &wgpu::BindGroup {
        &self.checker_texture.bind_group
    }

    pub(super) const fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
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

#[cfg(test)]
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
    let singular = scale.iter().any(|component| component.abs() < MIN_ABS_SCALE);
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
            assert!(face_vertices.iter().all(|vertex| vertex.normal == face_vertices[0].normal));
            assert!((face_vertices[0].normal.iter().map(|n| n * n).sum::<f32>() - 1.0).abs() < 1.0e-6);
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
            [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]
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

        assert!(instance.normal_matrix.iter().flatten().all(|value| value.is_finite()));
        assert_eq!(
            instance.normal_matrix,
            [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]
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
