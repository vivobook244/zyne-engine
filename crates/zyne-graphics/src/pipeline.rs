use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

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
        let projection = perspective_rh_wgpu(55.0_f32.to_radians(), aspect, 0.1, 100.0);
        let view = look_at_rh([4.0, 2.8, 4.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        // Keep a modest model rotation so the camera reveals three distinct faces.
        let model = rotation_y(0.15);
        let view_projection = multiply(projection, multiply(view, model));
        let uniform = CameraUniform {
            view_projection: to_columns(view_projection),
        };
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

fn to_columns(matrix: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0], matrix[3][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1], matrix[3][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]],
        [matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]],
    ]
}

#[allow(clippy::many_single_char_names)]
fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];

    for (row_index, row) in out.iter_mut().enumerate() {
        for (column_index, value) in row.iter_mut().enumerate() {
            for (inner_index, left) in a[row_index].iter().enumerate() {
                *value += left * b[inner_index][column_index];
            }
        }
    }

    out
}

fn rotation_y(angle: f32) -> [[f32; 4]; 4] {
    let (s, c) = angle.sin_cos();
    [
        [c, 0.0, s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn perspective_rh_wgpu(fov: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov * 0.5).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), (near * far) / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ]
}

fn look_at_rh(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize(sub(target, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], s[1], s[2], -dot(s, eye)],
        [u[0], u[1], u[2], -dot(u, eye)],
        [-f[0], -f[1], -f[2], dot(f, eye)],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

#[cfg(test)]
mod tests {
    use super::{cube_vertices, multiply, perspective_rh_wgpu, rotation_y};

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
    fn perspective_matrix_has_expected_clip_depth_terms() {
        let matrix = perspective_rh_wgpu(55.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        assert!(matrix[2][3] < 0.0);
        assert_eq!(matrix[3][2], -1.0);
        assert!(matrix[3][3].abs() < f32::EPSILON);
    }

    #[test]
    fn camera_target_is_inside_wgpu_clip_depth_range() {
        let projection = perspective_rh_wgpu(55.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        let view = super::look_at_rh([2.8, 2.1, 4.2], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let view_projection = multiply(projection, view);

        // The world-space origin is the camera target. For wgpu's 0..1 depth
        // convention, its projected depth must be between zero and clip W.
        let clip_z = view_projection[2][3];
        let clip_w = view_projection[3][3];
        assert!(clip_w > 0.0);
        assert!(clip_z >= 0.0 && clip_z <= clip_w);
    }

    #[test]
    fn view_matrix_places_the_camera_target_in_front_of_camera() {
        let view = super::look_at_rh([2.8, 2.1, 4.2], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);

        // Row-major view matrices store translation in the final column.
        // The origin must be in front of the camera (negative view-space Z).
        assert!(view[2][3] < 0.0);
        assert_eq!(view[3], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn matrix_multiplication_preserves_identity() {
        let identity = identity_for_test();
        let actual = multiply(identity, rotation_y(0.5));
        let expected = rotation_y(0.5);

        for row in 0..4 {
            for column in 0..4 {
                assert!((actual[row][column] - expected[row][column]).abs() < f32::EPSILON);
            }
        }
    }

    fn identity_for_test() -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
}
