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
}
