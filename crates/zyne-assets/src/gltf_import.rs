//! glTF 2.0 and GLB 2.0 import into renderer-neutral CPU data.

use std::fmt;
use std::path::Path;

use gltf::image::Format as ImageFormat;
use gltf::mesh::Mode;

/// A successfully imported glTF asset containing CPU-side mesh and image data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportedAsset {
    /// Meshes in source-document order.
    pub meshes: Vec<ImportedMesh>,
    /// Images in source-document order.
    pub images: Vec<ImportedImage>,
}

/// A source mesh and its primitive list.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedMesh {
    /// Source-document mesh index.
    pub index: usize,
    /// Optional source name.
    pub name: Option<String>,
    /// Mesh primitives in source order.
    pub primitives: Vec<ImportedPrimitive>,
}

/// A triangle primitive with validated vertex and index data.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPrimitive {
    /// Source primitive index within its mesh.
    pub index: usize,
    /// Source material index, if the primitive references a material.
    pub material_index: Option<usize>,
    /// Required local-space positions.
    pub positions: Vec<[f32; 3]>,
    /// Optional vertex normals.
    pub normals: Option<Vec<[f32; 3]>>,
    /// Optional first texture-coordinate set, converted to f32.
    pub tex_coords_0: Option<Vec<[f32; 2]>>,
    /// Triangle indices. Non-indexed primitives receive sequential indices.
    pub indices: Vec<u32>,
}

/// Decoded image data in its source pixel format.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedImage {
    /// Source-document image index.
    pub index: usize,
    /// Optional source name.
    pub name: Option<String>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Decoded channel layout and component width.
    pub format: ImageFormat,
    /// Owned, tightly packed pixel bytes.
    pub pixels: Vec<u8>,
}

/// Errors returned when a source asset cannot be imported safely.
#[derive(Debug)]
pub enum ImportError {
    /// The glTF parser, validator, buffer loader, or image decoder failed.
    Gltf(Box<gltf::Error>),
    /// A required extension is not implemented by this importer.
    UnsupportedRequiredExtension(String),
    /// A primitive uses a topology other than triangles.
    UnsupportedPrimitiveMode {
        mesh_index: usize,
        primitive_index: usize,
        mode: Mode,
    },
    /// A triangle primitive does not provide positions.
    MissingPositions {
        mesh_index: usize,
        primitive_index: usize,
    },
    /// A vertex position contains NaN or infinity.
    NonFiniteAttribute {
        attribute: &'static str,
        mesh_index: usize,
        primitive_index: usize,
        element_index: usize,
    },
    /// An optional vertex attribute has a different element count.
    AttributeCountMismatch {
        attribute: &'static str,
        mesh_index: usize,
        primitive_index: usize,
        expected: usize,
        actual: usize,
    },
    /// A vertex index is outside the primitive's position array.
    IndexOutOfBounds {
        mesh_index: usize,
        primitive_index: usize,
        index: u32,
        vertex_count: usize,
    },
    /// Triangle index count is not divisible by three.
    InvalidTriangleIndexCount {
        mesh_index: usize,
        primitive_index: usize,
        index_count: usize,
    },
    /// A source mesh contains too many vertices for 32-bit indices.
    TooManyVertices {
        mesh_index: usize,
        primitive_index: usize,
    },
    /// Decoded image dimensions or pixel byte count are invalid.
    InvalidImageData { image_index: usize },
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gltf(error) => write!(formatter, "glTF import failed: {error}"),
            Self::UnsupportedRequiredExtension(extension) => write!(
                formatter,
                "required glTF extension is unsupported: {extension}"
            ),
            Self::UnsupportedPrimitiveMode {
                mesh_index,
                primitive_index,
                mode,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} uses unsupported topology {mode:?}"
            ),
            Self::MissingPositions {
                mesh_index,
                primitive_index,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} has no POSITION attribute"
            ),
            Self::NonFiniteAttribute {
                attribute,
                mesh_index,
                primitive_index,
                element_index,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} {attribute} element {element_index} contains a non-finite value"
            ),
            Self::AttributeCountMismatch {
                attribute,
                mesh_index,
                primitive_index,
                expected,
                actual,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} {attribute} count is {actual}, expected {expected}"
            ),
            Self::IndexOutOfBounds {
                mesh_index,
                primitive_index,
                index,
                vertex_count,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} index {index} exceeds vertex count {vertex_count}"
            ),
            Self::InvalidTriangleIndexCount {
                mesh_index,
                primitive_index,
                index_count,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} has {index_count} indices; triangle lists require a multiple of three"
            ),
            Self::TooManyVertices {
                mesh_index,
                primitive_index,
            } => write!(
                formatter,
                "mesh {mesh_index} primitive {primitive_index} exceeds the u32 vertex index limit"
            ),
            Self::InvalidImageData { image_index } => write!(
                formatter,
                "image {image_index} has invalid dimensions or pixel data"
            ),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Gltf(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<gltf::Error> for ImportError {
    fn from(error: gltf::Error) -> Self {
        Self::Gltf(Box::new(error))
    }
}

/// Imports a glTF 2.0 JSON or GLB 2.0 file from disk.
///
/// External buffers and images are resolved relative to the source file's
/// directory. The result contains owned CPU data and no GPU resources.
///
/// # Errors
///
/// Returns an error for malformed files, missing required data, unsupported
/// required extensions or primitive topologies, invalid vertex attributes,
/// invalid indices, and invalid decoded image data.
pub fn import_gltf(path: impl AsRef<Path>) -> Result<ImportedAsset, ImportError> {
    let (document, buffers, images) = gltf::import(path)?;
    if let Some(extension) = document.extensions_required().next() {
        return Err(ImportError::UnsupportedRequiredExtension(
            extension.to_owned(),
        ));
    }

    let mut imported = ImportedAsset::default();

    for mesh in document.meshes() {
        let mesh_index = mesh.index();
        let mut imported_mesh = ImportedMesh {
            index: mesh_index,
            name: mesh.name().map(str::to_owned),
            primitives: Vec::new(),
        };

        for (primitive_index, primitive) in mesh.primitives().enumerate() {
            let mode = primitive.mode();
            if mode != Mode::Triangles {
                return Err(ImportError::UnsupportedPrimitiveMode {
                    mesh_index,
                    primitive_index,
                    mode,
                });
            }

            let reader = primitive
                .reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
            let positions = reader
                .read_positions()
                .ok_or(ImportError::MissingPositions {
                    mesh_index,
                    primitive_index,
                })?
                .collect::<Vec<_>>();
            validate_finite_vectors3(&positions, "POSITION", mesh_index, primitive_index)?;

            let normals = reader
                .read_normals()
                .map(|values| values.collect::<Vec<_>>());
            validate_attribute_count(
                "NORMAL",
                normals.as_ref().map(Vec::len),
                positions.len(),
                mesh_index,
                primitive_index,
            )?;
            if let Some(values) = &normals {
                validate_finite_vectors3(values, "NORMAL", mesh_index, primitive_index)?;
            }

            let tex_coords_0 = reader
                .read_tex_coords(0)
                .map(|coordinates| coordinates.into_f32().collect::<Vec<_>>());
            validate_attribute_count(
                "TEXCOORD_0",
                tex_coords_0.as_ref().map(Vec::len),
                positions.len(),
                mesh_index,
                primitive_index,
            )?;
            if let Some(values) = &tex_coords_0 {
                for (element_index, coordinate) in values.iter().enumerate() {
                    if coordinate.iter().any(|component| !component.is_finite()) {
                        return Err(ImportError::NonFiniteAttribute {
                            attribute: "TEXCOORD_0",
                            mesh_index,
                            primitive_index,
                            element_index,
                        });
                    }
                }
            }

            let indices = if let Some(source_indices) = reader.read_indices() {
                source_indices.into_u32().collect::<Vec<_>>()
            } else {
                let vertex_count =
                    u32::try_from(positions.len()).map_err(|_| ImportError::TooManyVertices {
                        mesh_index,
                        primitive_index,
                    })?;
                (0..vertex_count).collect()
            };

            if indices.len() % 3 != 0 {
                return Err(ImportError::InvalidTriangleIndexCount {
                    mesh_index,
                    primitive_index,
                    index_count: indices.len(),
                });
            }
            for &index in &indices {
                if !usize::try_from(index).is_ok_and(|value| value < positions.len()) {
                    return Err(ImportError::IndexOutOfBounds {
                        mesh_index,
                        primitive_index,
                        index,
                        vertex_count: positions.len(),
                    });
                }
            }

            imported_mesh.primitives.push(ImportedPrimitive {
                index: primitive_index,
                material_index: primitive.material().index(),
                positions,
                normals,
                tex_coords_0,
                indices,
            });
        }
        imported.meshes.push(imported_mesh);
    }

    for (source_image, image) in document.images().zip(images) {
        if image.width == 0
            || image.height == 0
            || image_byte_len(image.width, image.height, image.format) != Some(image.pixels.len())
        {
            return Err(ImportError::InvalidImageData {
                image_index: source_image.index(),
            });
        }
        imported.images.push(ImportedImage {
            index: source_image.index(),
            name: source_image.name().map(str::to_owned),
            width: image.width,
            height: image.height,
            format: image.format,
            pixels: image.pixels,
        });
    }

    Ok(imported)
}

fn validate_attribute_count(
    attribute: &'static str,
    actual: Option<usize>,
    expected: usize,
    mesh_index: usize,
    primitive_index: usize,
) -> Result<(), ImportError> {
    if let Some(actual) = actual {
        if actual != expected {
            return Err(ImportError::AttributeCountMismatch {
                attribute,
                mesh_index,
                primitive_index,
                expected,
                actual,
            });
        }
    }
    Ok(())
}

fn validate_finite_vectors3(
    values: &[[f32; 3]],
    attribute: &'static str,
    mesh_index: usize,
    primitive_index: usize,
) -> Result<(), ImportError> {
    for (element_index, value) in values.iter().enumerate() {
        if value.iter().any(|component| !component.is_finite()) {
            return Err(ImportError::NonFiniteAttribute {
                attribute,
                mesh_index,
                primitive_index,
                element_index,
            });
        }
    }
    Ok(())
}

fn image_byte_len(width: u32, height: u32, format: ImageFormat) -> Option<usize> {
    let bytes_per_pixel = match format {
        ImageFormat::R8 => 1,
        ImageFormat::R8G8 => 2,
        ImageFormat::R8G8B8 => 3,
        ImageFormat::R8G8B8A8 => 4,
        ImageFormat::R16 => 2,
        ImageFormat::R16G16 => 4,
        ImageFormat::R16G16B16 => 6,
        ImageFormat::R16G16B16A16 => 8,
        ImageFormat::R32G32B32FLOAT => 12,
        ImageFormat::R32G32B32A32FLOAT => 16,
    };
    usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(bytes_per_pixel)
}

#[cfg(test)]
mod tests {
    use super::{ImportError, import_gltf};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const POSITIONS: [u8; 36] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 128, 63, 0, 0, 0, 0,
    ];
    const PIXEL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("zyne-assets-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("temporary test directory should be created");
            Self(path)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn imports_mesh_and_external_image_relative_to_source_file() {
        let directory = TestDirectory::new();
        fs::write(directory.path("triangle.bin"), POSITIONS)
            .expect("external mesh buffer should be written");
        fs::write(directory.path("pixel.png"), decode_base64(PIXEL_PNG))
            .expect("external image should be written");
        let document = r#"{
            "asset": { "version": "2.0" },
            "buffers": [{ "uri": "triangle.bin", "byteLength": 36 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [{
                "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                "min": [0, 0, 0], "max": [1, 1, 0]
            }],
            "meshes": [{ "name": "Triangle", "primitives": [{ "attributes": { "POSITION": 0 } }] }],
            "images": [{ "name": "Pixel", "uri": "pixel.png" }]
        }"#;
        fs::write(directory.path("scene.gltf"), document).expect("glTF document should be written");

        let imported = import_gltf(directory.path("scene.gltf"))
            .expect("valid glTF and relative resources should import");
        assert_eq!(imported.meshes.len(), 1);
        assert_eq!(imported.meshes[0].name.as_deref(), Some("Triangle"));
        assert_eq!(imported.meshes[0].primitives.len(), 1);
        let primitive = &imported.meshes[0].primitives[0];
        assert_eq!(primitive.index, 0);
        assert_eq!(primitive.positions.len(), 3);
        assert_eq!(primitive.indices, [0, 1, 2]);
        assert!(primitive.normals.is_none());
        assert!(primitive.tex_coords_0.is_none());
        assert_eq!(imported.images.len(), 1);
        assert_eq!(imported.images[0].name.as_deref(), Some("Pixel"));
        assert_eq!(
            (imported.images[0].width, imported.images[0].height),
            (1, 1)
        );
        assert_eq!(imported.images[0].pixels.len(), 4);
    }

    #[test]
    fn imports_binary_gltf_container() {
        let directory = TestDirectory::new();
        let json = r#"{
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": 36 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [{
                "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                "min": [0, 0, 0], "max": [1, 1, 0]
            }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
        }"#;
        let mut json_chunk = json.as_bytes().to_vec();
        while json_chunk.len() % 4 != 0 {
            json_chunk.push(b' ');
        }
        let mut binary_chunk = POSITIONS.to_vec();
        while binary_chunk.len() % 4 != 0 {
            binary_chunk.push(0);
        }
        let total_length = 12 + 8 + json_chunk.len() + 8 + binary_chunk.len();
        let mut glb = Vec::with_capacity(total_length);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2_u32.to_le_bytes());
        glb.extend_from_slice(
            &u32::try_from(total_length)
                .expect("GLB fixture length should fit u32")
                .to_le_bytes(),
        );
        glb.extend_from_slice(
            &u32::try_from(json_chunk.len())
                .expect("JSON chunk length should fit u32")
                .to_le_bytes(),
        );
        glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
        glb.extend_from_slice(&json_chunk);
        glb.extend_from_slice(
            &u32::try_from(binary_chunk.len())
                .expect("BIN chunk length should fit u32")
                .to_le_bytes(),
        );
        glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
        glb.extend_from_slice(&binary_chunk);
        fs::write(directory.path("triangle.glb"), glb).expect("GLB fixture should be written");

        let imported =
            import_gltf(directory.path("triangle.glb")).expect("valid GLB should import");
        assert_eq!(imported.meshes.len(), 1);
        assert_eq!(imported.meshes[0].primitives[0].positions.len(), 3);
    }

    #[test]
    fn rejects_malformed_source_files() {
        let directory = TestDirectory::new();
        fs::write(directory.path("broken.gltf"), b"{ not json")
            .expect("invalid fixture should be written");
        assert!(matches!(
            import_gltf(directory.path("broken.gltf")),
            Err(ImportError::Gltf(_))
        ));
    }

    fn decode_base64(encoded: &str) -> Vec<u8> {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = Vec::new();
        let mut accumulator = 0_u32;
        let mut bits = 0_u8;
        for byte in encoded.bytes().filter(|byte| *byte != b'=') {
            let value = u32::try_from(
                TABLE
                    .iter()
                    .position(|candidate| *candidate == byte)
                    .expect("fixture base64 should be valid"),
            )
            .expect("base64 digit should fit u32");
            accumulator = (accumulator << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push(
                    u8::try_from((accumulator >> bits) & 0xff).expect("decoded byte should fit u8"),
                );
            }
        }
        output
    }
}
