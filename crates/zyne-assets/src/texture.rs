//! Validated renderer-independent CPU texture payloads.

/// The interpretation of RGB channels when a texture is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureColorSpace {
    /// RGB values are encoded with the sRGB transfer function.
    Srgb,
    /// RGB values are already linear.
    Linear,
}

/// Errors produced when constructing CPU texture data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDataError {
    /// Texture width must be greater than zero.
    ZeroWidth,
    /// Texture height must be greater than zero.
    ZeroHeight,
    /// The dimensions overflow the addressable pixel-byte length.
    DimensionsOverflow,
    /// The supplied byte count does not match tightly packed RGBA8 pixels.
    PixelDataLengthMismatch {
        expected: usize,
        actual: usize,
    },
}

impl std::fmt::Display for TextureDataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("texture width must be greater than zero"),
            Self::ZeroHeight => formatter.write_str("texture height must be greater than zero"),
            Self::DimensionsOverflow => {
                formatter.write_str("texture dimensions overflow the pixel byte length")
            }
            Self::PixelDataLengthMismatch { expected, actual } => write!(
                formatter,
                "texture contains {actual} pixel bytes, expected {expected} for tightly packed RGBA8 data"
            ),
        }
    }
}

impl std::error::Error for TextureDataError {}

/// Owned, validated, tightly packed RGBA8 texture data.
///
/// This type is independent of graphics APIs. It records whether RGB channels
/// are sRGB-encoded or linear; alpha is always stored as a linear component.
/// GPU-specific limits and upload alignment are validated by the graphics layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureData {
    width: u32,
    height: u32,
    color_space: TextureColorSpace,
    pixels: Vec<u8>,
}

impl TextureData {
    /// Creates RGBA8 texture data after validating dimensions and byte length.
    ///
    /// Pixel order is row-major with four consecutive bytes per pixel in
    /// red, green, blue, alpha order.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions, an unrepresentable byte length,
    /// or a pixel buffer whose length does not exactly match the dimensions.
    pub fn new_rgba8(
        width: u32,
        height: u32,
        color_space: TextureColorSpace,
        pixels: Vec<u8>,
    ) -> Result<Self, TextureDataError> {
        if width == 0 {
            return Err(TextureDataError::ZeroWidth);
        }
        if height == 0 {
            return Err(TextureDataError::ZeroHeight);
        }

        let expected = Self::expected_rgba8_byte_len(width, height)?;
        if pixels.len() != expected {
            return Err(TextureDataError::PixelDataLengthMismatch {
                expected,
                actual: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            color_space,
            pixels,
        })
    }

    /// Returns the width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the RGB channel color-space interpretation.
    #[must_use]
    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }

    /// Returns the owned, tightly packed RGBA8 pixel bytes.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the expected tightly packed RGBA8 byte count.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions or arithmetic overflow.
    pub fn expected_rgba8_byte_len(
        width: u32,
        height: u32,
    ) -> Result<usize, TextureDataError> {
        if width == 0 {
            return Err(TextureDataError::ZeroWidth);
        }
        if height == 0 {
            return Err(TextureDataError::ZeroHeight);
        }

        usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixel_count| pixel_count.checked_mul(4))
            .ok_or(TextureDataError::DimensionsOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::{TextureColorSpace, TextureData, TextureDataError};

    #[test]
    fn creates_valid_rgba8_texture_data() {
        let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255];
        let texture =
            TextureData::new_rgba8(2, 1, TextureColorSpace::Srgb, pixels.clone()).unwrap();

        assert_eq!(texture.width(), 2);
        assert_eq!(texture.height(), 1);
        assert_eq!(texture.color_space(), TextureColorSpace::Srgb);
        assert_eq!(texture.pixels(), pixels);
    }

    #[test]
    fn accepts_linear_texture_data() {
        let texture =
            TextureData::new_rgba8(1, 1, TextureColorSpace::Linear, vec![32, 64, 128, 255])
                .unwrap();

        assert_eq!(texture.color_space(), TextureColorSpace::Linear);
    }

    #[test]
    fn rejects_zero_dimensions() {
        assert_eq!(
            TextureData::new_rgba8(0, 1, TextureColorSpace::Srgb, Vec::new()),
            Err(TextureDataError::ZeroWidth)
        );
        assert_eq!(
            TextureData::new_rgba8(1, 0, TextureColorSpace::Srgb, Vec::new()),
            Err(TextureDataError::ZeroHeight)
        );
    }

    #[test]
    fn rejects_pixel_buffers_with_incorrect_lengths() {
        assert_eq!(
            TextureData::new_rgba8(1, 1, TextureColorSpace::Srgb, vec![0, 1, 2]),
            Err(TextureDataError::PixelDataLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            TextureData::new_rgba8(1, 1, TextureColorSpace::Srgb, vec![0; 8]),
            Err(TextureDataError::PixelDataLengthMismatch {
                expected: 4,
                actual: 8,
            })
        );
    }

    #[test]
    fn detects_dimension_arithmetic_overflow() {
        assert_eq!(
            TextureData::expected_rgba8_byte_len(u32::MAX, u32::MAX),
            Err(TextureDataError::DimensionsOverflow)
        );
    }
}
