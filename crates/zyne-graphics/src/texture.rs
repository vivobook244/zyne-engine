//! GPU-resident sampled textures owned by a graphics device.

/// Interpretation of RGB bytes when a sampled texture is uploaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureColorSpace {
    /// RGB bytes use the sRGB transfer function.
    Srgb,
    /// RGB bytes already contain linear values.
    Linear,
}

/// A sampled 2D texture uploaded to one graphics device.
pub struct GpuTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl GpuTexture {
    pub(crate) const fn new(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        bind_group: wgpu::BindGroup,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            _texture: texture,
            _view: view,
            _sampler: sampler,
            bind_group,
            width,
            height,
            format,
        }
    }

    /// Returns the uploaded texture width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the uploaded texture height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the GPU storage format used for this texture.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    const fn texture(&self) -> &wgpu::Texture {
        &self._texture
    }
}

/// Validates tightly packed RGBA8 upload dimensions and returns the row stride.
pub(crate) fn validate_rgba8_upload(
    width: u32,
    height: u32,
    maximum: u32,
    pixels: &[u8],
) -> Result<u32, crate::GraphicsError> {
    if width == 0 || height == 0 || width > maximum || height > maximum {
        return Err(crate::GraphicsError::InvalidTextureDimensions {
            width,
            height,
            maximum,
        });
    }

    let expected = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|count| count.checked_mul(4))
        .ok_or(crate::GraphicsError::InvalidTextureDimensions {
            width,
            height,
            maximum,
        })?;
    if pixels.len() != expected {
        return Err(crate::GraphicsError::InvalidTextureDataLength {
            expected,
            actual: pixels.len(),
        });
    }

    width
        .checked_mul(4)
        .ok_or(crate::GraphicsError::InvalidTextureDimensions {
            width,
            height,
            maximum,
        })
}

/// Creates the texture bind-group layout shared by the checker fallback and uploaded textures.
pub(crate) fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

/// Uploads validated RGBA8 pixels and creates renderer-compatible sampling resources.
pub(crate) fn create_gpu_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    color_space: TextureColorSpace,
    pixels: &[u8],
) -> Result<GpuTexture, crate::GraphicsError> {
    let format = match color_space {
        TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
    };
    let bytes_per_row = validate_rgba8_upload(
        width,
        height,
        device.limits().max_texture_dimension_2d,
        pixels,
    )?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ZYNE Uploaded Diffuse Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ZYNE Diffuse Texture Sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ZYNE Uploaded Texture Bind Group"),
        layout,
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
    Ok(GpuTexture::new(
        texture, view, sampler, bind_group, width, height, format,
    ))
}

#[cfg(test)]
mod tests {
    use crate::GraphicsError;

    use super::{TextureColorSpace, validate_rgba8_upload};

    #[test]
    fn accepts_exact_rgba8_payload_and_returns_tight_row_stride() {
        assert_eq!(validate_rgba8_upload(3, 2, 4096, &[0; 24]), Ok(12));
    }

    #[test]
    fn rejects_zero_dimensions() {
        assert_eq!(
            validate_rgba8_upload(0, 2, 4096, &[]),
            Err(GraphicsError::InvalidTextureDimensions {
                width: 0,
                height: 2,
                maximum: 4096,
            })
        );
    }

    #[test]
    fn rejects_dimensions_above_device_limit() {
        assert_eq!(
            validate_rgba8_upload(4097, 1, 4096, &[]),
            Err(GraphicsError::InvalidTextureDimensions {
                width: 4097,
                height: 1,
                maximum: 4096,
            })
        );
    }

    #[test]
    fn rejects_short_and_long_pixel_payloads() {
        assert_eq!(
            validate_rgba8_upload(1, 1, 4096, &[0; 3]),
            Err(GraphicsError::InvalidTextureDataLength {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            validate_rgba8_upload(1, 1, 4096, &[0; 5]),
            Err(GraphicsError::InvalidTextureDataLength {
                expected: 4,
                actual: 5,
            })
        );
    }

    #[test]
    fn color_spaces_remain_distinct_upload_contracts() {
        assert_ne!(TextureColorSpace::Srgb, TextureColorSpace::Linear);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn headless_gpu_upload_preserves_srgb_and_linear_rgba8_pixels() {
        use super::{create_gpu_texture, create_texture_bind_group_layout};
        use std::sync::mpsc;
        use wgpu::{DeviceDescriptor, Instance, MapMode, PollType, RequestAdapterOptions};

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
                    label: Some("ZYNE Headless Texture Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                })
                .await
                .expect("headless test device should initialize");
            let layout = create_texture_bind_group_layout(&device);
            let width = 3_u32;
            let height = 2_u32;
            let pixels: [u8; 24] = [
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 10, 20, 30, 255, 40, 50, 60, 255,
                70, 80, 90, 255,
            ];
            for (space, expected_format) in [
                (
                    super::TextureColorSpace::Srgb,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                ),
                (
                    super::TextureColorSpace::Linear,
                    wgpu::TextureFormat::Rgba8Unorm,
                ),
            ] {
                let uploaded =
                    create_gpu_texture(&device, &queue, &layout, width, height, space, &pixels)
                        .expect("valid texture upload should succeed");
                assert_eq!(uploaded.width(), width);
                assert_eq!(uploaded.height(), height);
                assert_eq!(uploaded.format(), expected_format);

                let padded_bytes_per_row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ZYNE Texture Pixel Readback"),
                    size: u64::from(padded_bytes_per_row) * u64::from(height),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ZYNE Texture Readback Encoder"),
                });
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: uploaded.texture(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(padded_bytes_per_row),
                            rows_per_image: Some(height),
                        },
                    },
                    wgpu::Extent3d {
                        width,
                        height,
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
                    .expect("GPU readback should complete");
                receiver
                    .recv()
                    .expect("GPU map callback should run")
                    .expect("GPU readback buffer should map");
                {
                    let mapped = slice.get_mapped_range();
                    let mut actual = Vec::with_capacity(pixels.len());
                    for row in 0..height as usize {
                        let start = row * padded_bytes_per_row as usize;
                        actual.extend_from_slice(&mapped[start..start + width as usize * 4]);
                    }
                    assert_eq!(actual, pixels, "GPU readback must preserve raw RGBA8 bytes");
                }
                readback.unmap();
            }
        });
    }
}
