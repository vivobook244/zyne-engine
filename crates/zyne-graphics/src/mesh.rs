use bytemuck::{Pod, Zeroable};
use std::{error::Error, fmt};
use zyne_math::Transform3d;

/// An axis-aligned mesh bound in local or world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MeshBounds {
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

impl MeshBounds {
    pub(crate) const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    pub(crate) fn union(self, other: Self) -> Self {
        let mut min = self.min;
        let mut max = self.max;
        for ((min_value, max_value), (other_min, other_max)) in min
            .iter_mut()
            .zip(max.iter_mut())
            .zip(other.min.iter().zip(other.max.iter()))
        {
            *min_value = (*min_value).min(*other_min);
            *max_value = (*max_value).max(*other_max);
        }
        Self { min, max }
    }

    /// Transforms all eight corners and returns a conservative world-space AABB.
    pub(crate) fn transformed(self, transform: zyne_math::Transform3d) -> Self {
        let matrix = transform.to_matrix();
        let mut result = Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        };
        for x in [self.min[0], self.max[0]] {
            for y in [self.min[1], self.max[1]] {
                for z in [self.min[2], self.max[2]] {
                    let point = [
                        matrix[0][0] * x + matrix[1][0] * y + matrix[2][0] * z + matrix[3][0],
                        matrix[0][1] * x + matrix[1][1] * y + matrix[2][1] * z + matrix[3][1],
                        matrix[0][2] * x + matrix[1][2] * y + matrix[2][2] * z + matrix[3][2],
                    ];
                    for ((min_value, max_value), value) in result
                        .min
                        .iter_mut()
                        .zip(result.max.iter_mut())
                        .zip(point)
                    {
                        *min_value = (*min_value).min(value);
                        *max_value = (*max_value).max(value);
                    }
                }
            }
        }
        result
    }

    fn from_vertices(vertices: &[MeshVertex]) -> Self {
        let mut result = Self {
            min: vertices[0].position,
            max: vertices[0].position,
        };
        for vertex in &vertices[1..] {
            for ((min_value, max_value), value) in result
                .min
                .iter_mut()
                .zip(result.max.iter_mut())
                .zip(vertex.position)
            {
                *min_value = (*min_value).min(value);
                *max_value = (*max_value).max(value);
            }
        }
        result
    }
}

/// A mesh uploaded to a specific graphics device.
///
/// GPU meshes are created by `GraphicsContext::create_mesh` and reused across frames.
pub struct GpuMesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    index_count: u32,
    bounds: MeshBounds,
}

impl GpuMesh {
    pub(crate) const fn new(
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        index_count: u32,
        bounds: MeshBounds,
    ) -> Self {
        Self {
            vertex_buffer,
            index_buffer,
            index_count,
            bounds,
        }
    }

    /// Returns the number of indices submitted by the indexed draw call.
    #[must_use]
    pub const fn index_count(&self) -> u32 {
        self.index_count
    }

    pub(crate) const fn bounds(&self) -> MeshBounds {
        self.bounds
    }

    pub(crate) const fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(crate) const fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }
}

/// A material uniform uploaded to a specific graphics device.
pub struct GpuMaterial {
    _uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GpuMaterial {
    pub(crate) const fn new(uniform_buffer: wgpu::Buffer, bind_group: wgpu::BindGroup) -> Self {
        Self {
            _uniform_buffer: uniform_buffer,
            bind_group,
        }
    }

    pub(crate) const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// One uploaded mesh and the transforms of its scene instances.
pub struct MeshRenderBatch<'a> {
    mesh: &'a GpuMesh,
    transforms: &'a [Transform3d],
    material: Option<&'a GpuMaterial>,
}

impl<'a> MeshRenderBatch<'a> {
    /// Creates a batch for one mesh and its instance transforms.
    #[must_use]
    pub const fn new(mesh: &'a GpuMesh, transforms: &'a [Transform3d]) -> Self {
        Self {
            mesh,
            transforms,
            material: None,
        }
    }

    /// Creates a batch using an explicit GPU material.
    #[must_use]
    pub const fn with_material(
        mesh: &'a GpuMesh,
        transforms: &'a [Transform3d],
        material: &'a GpuMaterial,
    ) -> Self {
        Self {
            mesh,
            transforms,
            material: Some(material),
        }
    }

    /// Returns the GPU material assigned to this batch, if any.
    #[must_use]
    pub const fn material(&self) -> Option<&'a GpuMaterial> {
        self.material
    }

    /// Returns the uploaded mesh used by this batch.
    #[must_use]
    pub const fn mesh(&self) -> &'a GpuMesh {
        self.mesh
    }

    /// Returns the transforms submitted for this mesh.
    #[must_use]
    pub const fn transforms(&self) -> &'a [Transform3d] {
        self.transforms
    }
}

/// One renderer-facing vertex with position, linear color, UV, and normal data.
///
/// The memory layout matches the vertex attributes consumed by the basic 3D
/// WGSL shader. Values are validated when placed in `MeshData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

impl MeshVertex {
    /// Creates a vertex. Attribute validation is performed by `MeshData::new`.
    #[must_use]
    pub const fn new(position: [f32; 3], color: [f32; 3], uv: [f32; 2], normal: [f32; 3]) -> Self {
        Self {
            position,
            color,
            uv,
            normal,
        }
    }
}

/// Validated, renderer-independent CPU mesh data.
///
/// This type deliberately contains no GPU handles. A graphics context can
/// upload it while asset importers and scene data remain CPU-side.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

impl MeshData {
    /// Creates a triangle-list mesh and normalizes its vertex normals.
    ///
    /// # Errors
    ///
    /// Returns an error for empty data, incomplete triangles, out-of-range
    /// indices, non-finite attributes, or zero-length normals.
    pub fn new(mut vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Result<Self, MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptyVertices);
        }
        if indices.is_empty() {
            return Err(MeshError::EmptyIndices);
        }
        if indices.len() % 3 != 0 {
            return Err(MeshError::InvalidTriangleIndexCount(indices.len()));
        }

        for (vertex_index, vertex) in vertices.iter_mut().enumerate() {
            validate_attribute(vertex.position, vertex_index, "position")?;
            validate_attribute(vertex.color, vertex_index, "color")?;
            validate_attribute(vertex.uv, vertex_index, "uv")?;
            validate_attribute(vertex.normal, vertex_index, "normal")?;

            let length_squared = vertex
                .normal
                .iter()
                .map(|component| f64::from(*component).powi(2))
                .sum::<f64>();
            if length_squared <= f64::MIN_POSITIVE {
                return Err(MeshError::ZeroLengthNormal(vertex_index));
            }
            let inverse_length = length_squared.sqrt().recip();
            for component in &mut vertex.normal {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *component = (f64::from(*component) * inverse_length) as f32;
                }
            }
        }

        let vertex_count = vertices.len();
        for &index in &indices {
            if !usize::try_from(index).is_ok_and(|value| value < vertex_count) {
                return Err(MeshError::IndexOutOfBounds {
                    index,
                    vertex_count,
                });
            }
        }
        Ok(Self { vertices, indices })
    }

    /// Returns the local-space bounds of the validated vertex data.
    #[must_use]
    pub(crate) fn bounds(&self) -> MeshBounds {
        MeshBounds::from_vertices(&self.vertices)
    }

    /// Returns the validated vertex data.
    #[must_use]
    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }

    /// Returns the validated triangle-list indices.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

fn validate_attribute<const N: usize>(
    values: [f32; N],
    vertex_index: usize,
    attribute: &'static str,
) -> Result<(), MeshError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MeshError::NonFiniteAttribute {
            vertex_index,
            attribute,
        });
    }
    Ok(())
}

/// Validation errors for renderer-facing mesh data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    EmptyVertices,
    EmptyIndices,
    InvalidTriangleIndexCount(usize),
    IndexOutOfBounds {
        index: u32,
        vertex_count: usize,
    },
    NonFiniteAttribute {
        vertex_index: usize,
        attribute: &'static str,
    },
    ZeroLengthNormal(usize),
}

impl fmt::Display for MeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVertices => formatter.write_str("mesh has no vertices"),
            Self::EmptyIndices => formatter.write_str("mesh has no triangle indices"),
            Self::InvalidTriangleIndexCount(count) => write!(
                formatter,
                "mesh has {count} indices; triangle lists require a multiple of three"
            ),
            Self::IndexOutOfBounds {
                index,
                vertex_count,
            } => write!(
                formatter,
                "mesh index {index} exceeds vertex count {vertex_count}"
            ),
            Self::NonFiniteAttribute {
                vertex_index,
                attribute,
            } => write!(
                formatter,
                "mesh vertex {vertex_index} has a non-finite {attribute} attribute"
            ),
            Self::ZeroLengthNormal(vertex_index) => write!(
                formatter,
                "mesh vertex {vertex_index} has a zero-length normal"
            ),
        }
    }
}

impl Error for MeshError {}

#[cfg(test)]
mod tests {
    use super::{MeshData, MeshError, MeshVertex};

    fn triangle() -> (Vec<MeshVertex>, Vec<u32>) {
        (
            vec![
                MeshVertex::new([0.0, 0.0, 0.0], [1.0; 3], [0.0, 0.0], [0.0, 0.0, 2.0]),
                MeshVertex::new([1.0, 0.0, 0.0], [1.0; 3], [1.0, 0.0], [0.0, 0.0, 1.0]),
                MeshVertex::new([0.0, 1.0, 0.0], [1.0; 3], [0.0, 1.0], [0.0, 0.0, 1.0]),
            ],
            vec![0, 1, 2],
        )
    }

    #[test]
    fn transformed_bounds_cover_rotated_non_uniformly_scaled_mesh() {
        use super::MeshBounds;
        use zyne_math::Transform3d;

        let local_bounds = MeshBounds::new([0.0; 3], [1.0, 1.0, 0.0]);
        let half_angle = std::f32::consts::FRAC_PI_4;
        let transform = Transform3d::new(
            [3.0, 4.0, 5.0],
            [0.0, 0.0, half_angle.sin(), half_angle.cos()],
            [2.0, 3.0, 1.0],
        )
        .expect("transform should be valid");

        let world_bounds = local_bounds.transformed(transform);
        for (actual, expected) in world_bounds.min.into_iter().zip([0.0, 4.0, 5.0]) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
        for (actual, expected) in world_bounds.max.into_iter().zip([3.0, 6.0, 5.0]) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn bounds_union_contains_both_mesh_regions() {
        use super::MeshBounds;

        let first = MeshBounds::new([-2.0, -1.0, 0.0], [1.0, 2.0, 3.0]);
        let second = MeshBounds::new([4.0, -3.0, -2.0], [5.0, 1.0, 6.0]);
        let combined = first.union(second);

        assert_eq!(combined.min, [-2.0, -3.0, -2.0]);
        assert_eq!(combined.max, [5.0, 2.0, 6.0]);
    }

    #[test]
    fn accepts_triangle_and_normalizes_vertex_normals() {
        let (vertices, indices) = triangle();
        let mesh = MeshData::new(vertices, indices).expect("triangle should be valid");

        assert_eq!(mesh.vertices().len(), 3);
        assert_eq!(mesh.indices(), [0, 1, 2]);
        assert_eq!(mesh.vertices()[0].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn rejects_empty_mesh_data() {
        assert_eq!(
            MeshData::new(Vec::new(), vec![0, 1, 2]),
            Err(MeshError::EmptyVertices)
        );
        let (vertices, _) = triangle();
        assert_eq!(
            MeshData::new(vertices, Vec::new()),
            Err(MeshError::EmptyIndices)
        );
    }

    #[test]
    fn rejects_incomplete_triangle_indices() {
        let (vertices, _) = triangle();

        assert_eq!(
            MeshData::new(vertices, vec![0, 1]),
            Err(MeshError::InvalidTriangleIndexCount(2))
        );
    }

    #[test]
    fn rejects_out_of_range_indices() {
        let (vertices, _) = triangle();

        assert_eq!(
            MeshData::new(vertices, vec![0, 1, 3]),
            Err(MeshError::IndexOutOfBounds {
                index: 3,
                vertex_count: 3,
            })
        );
    }

    #[test]
    fn rejects_non_finite_attributes_and_zero_normals() {
        let (mut vertices, indices) = triangle();
        vertices[1].position[0] = f32::NAN;

        assert_eq!(
            MeshData::new(vertices, indices),
            Err(MeshError::NonFiniteAttribute {
                vertex_index: 1,
                attribute: "position",
            })
        );

        let (mut vertices, indices) = triangle();
        vertices[2].normal = [0.0; 3];

        assert_eq!(
            MeshData::new(vertices, indices),
            Err(MeshError::ZeroLengthNormal(2))
        );
    }
}
