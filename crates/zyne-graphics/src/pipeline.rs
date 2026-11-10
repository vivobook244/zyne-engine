use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use zyne_math::Camera3d;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

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
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

const CUBE_FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
    (
        [0.95, 0.28, 0.22],
        [
            [-0.7, -0.7, 0.7],
            [0.7, -0.7, 0.7],
            [0.7, 0.7, 0.7],
            [-0.7, 0.7, 0.7],
        ],
    ),
    (
        [0.22, 0.72, 0.95],
        [
            [0.7, -0.7, -0.7],
            [-0.7, -0.7, -0.7],
            [-0.7, 0.7, -0.7],
            [0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.30, 0.85, 0.48],
        [
            [0.7, -0.7, 0.7],
            [0.7, -0.7, -0.7],
            [0.7, 0.7, -0.7],
            [0.7, 0.7, 0.7],
        ],
    ),
    (
        [0.95, 0.72, 0.24],
        [
            [-0.7, -0.7, -0.7],
            [-0.7, -0.7, 0.7],
            [-0.7, 0.7, 0.7],
            [-0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.68, 0.42, 0.92],
        [
            [-0.7, 0.7, 0.7],
            [0.7, 0.7, 0.7],
            [0.7, 0.7, -0.7],
            [-0.7, 0.7, -0.7],
        ],
    ),
    (
        [0.24, 0.78, 0.82],
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
    }; 36];
    let triangles = [0, 1, 2, 0, 2, 3];

    for (face_index, (color, corners)) in CUBE_FACES.iter().enumerate() {
        for (vertex_index, triangle_corner) in triangles.iter().enumerate() {
            vertices[face_index * 6 + vertex_index] = Vertex {
                position: corners[*triangle_corner],
                color: *color,
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
}

impl CubePipeline {
    pub(super) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ZYNE 3D Cube Pipeline Layout"),
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ZYNE 3D Cube Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
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

        Self {
            pipeline,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub(super) fn update_camera(&self, queue: &wgpu::Queue, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        #[allow(clippy::cast_precision_loss)]
        let aspect = width as f32 / height as f32;
        let camera = Camera3d::new(
            [4.0, 2.8, 4.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            55.0_f32.to_radians(),
            aspect,
            0.1,
            100.0,
        )
        .expect("the built-in cube camera configuration must be valid");

        // Camera3d owns view/projection conventions. Keep only the cube's
        // local model transform in the graphics layer.
        let model = rotation_y(0.15);
        let view_projection = multiply_column_major(camera.view_projection_matrix(), model);
        let uniform = CameraUniform { view_projection };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
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

#[cfg(test)]
mod tests {
    use super::{Camera3d, cube_vertices, multiply_column_major, rotation_y};

    #[test]
    fn cube_has_twelve_triangles_and_valid_colors() {
        let vertices = cube_vertices();
        assert_eq!(vertices.len(), 36);
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.position.iter().all(|n| (-0.7..=0.7).contains(n)))
        );
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.color.iter().all(|n| (0.0..=1.0).contains(n)))
        );
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
