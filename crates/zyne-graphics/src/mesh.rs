use bytemuck::{Pod, Zeroable};
use std::{error::Error, fmt};

/// A mesh uploaded to a specific graphics device.
///
/// GPU meshes are created by `GraphicsContext::create_mesh` and reused across frames.
pub struct GpuMesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    pub(crate) const fn new(
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        index_count: u32,
    ) -> Self {
        Self {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }

    /// Returns the number of indices submitted by the indexed draw call.
    #[must_use]
    pub const fn index_count(&self) -> u32 {
        self.index_count
    }

    pub(crate) const fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(crate) const fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
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
