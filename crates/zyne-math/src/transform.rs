//! Validated local 3D transforms using ZYNE's coordinate conventions.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Errors produced when a 3D transform contains invalid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformError {
    /// A translation, rotation, or scale component is not finite.
    NonFiniteComponent,
    /// A quaternion has zero length and cannot represent a rotation.
    DegenerateRotation,
}

impl std::fmt::Display for TransformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteComponent => formatter.write_str("transform components must be finite"),
            Self::DegenerateRotation => {
                formatter.write_str("transform rotation quaternion must have non-zero length")
            }
        }
    }
}

impl std::error::Error for TransformError {}

/// A validated local-space 3D transform.
///
/// Quaternion components use `x, y, z, w` order. Values are normalized when
/// constructed or deserialized. Matrices are column-major and compose as
/// `T * R * S`, so scale is applied before rotation and translation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Transform3d {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

impl Transform3d {
    /// Creates a transform after validating and normalizing its rotation.
    ///
    /// # Errors
    ///
    /// Returns an error if any component is non-finite or the quaternion has
    /// zero length.
    pub fn new(
        translation: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
    ) -> Result<Self, TransformError> {
        if !translation
            .iter()
            .chain(rotation.iter())
            .chain(scale.iter())
            .all(|component| component.is_finite())
        {
            return Err(TransformError::NonFiniteComponent);
        }

        let length_squared = rotation
            .iter()
            .map(|component| component * component)
            .sum::<f32>();
        if length_squared <= f32::EPSILON {
            return Err(TransformError::DegenerateRotation);
        }
        let inverse_length = length_squared.sqrt().recip();
        let rotation = rotation.map(|component| component * inverse_length);

        Ok(Self {
            translation,
            rotation,
            scale,
        })
    }

    /// Returns the identity transform.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        }
    }

    /// Returns the local translation.
    #[must_use]
    pub const fn translation(&self) -> [f32; 3] {
        self.translation
    }

    /// Returns the normalized quaternion in `x, y, z, w` order.
    #[must_use]
    pub const fn rotation(&self) -> [f32; 4] {
        self.rotation
    }

    /// Returns the local scale.
    #[must_use]
    pub const fn scale(&self) -> [f32; 3] {
        self.scale
    }

    /// Returns the local-to-parent matrix in column-major layout.
    ///
    /// The matrix applies scale, then rotation, then translation
    /// (`T * R * S`) to column vectors.
    #[must_use]
    pub fn to_matrix(self) -> [[f32; 4]; 4] {
        let [x, y, z, w] = self.rotation;
        let [sx, sy, sz] = self.scale;
        let [tx, ty, tz] = self.translation;

        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let xw = x * w;
        let yw = y * w;
        let zw = z * w;

        [
            [
                (1.0 - 2.0 * (yy + zz)) * sx,
                (2.0 * (xy + zw)) * sx,
                (2.0 * (xz - yw)) * sx,
                0.0,
            ],
            [
                (2.0 * (xy - zw)) * sy,
                (1.0 - 2.0 * (xx + zz)) * sy,
                (2.0 * (yz + xw)) * sy,
                0.0,
            ],
            [
                (2.0 * (xz + yw)) * sz,
                (2.0 * (yz - xw)) * sz,
                (1.0 - 2.0 * (xx + yy)) * sz,
                0.0,
            ],
            [tx, ty, tz, 1.0],
        ]
    }
}

impl Default for Transform3d {
    fn default() -> Self {
        Self::identity()
    }
}

impl<'de> Deserialize<'de> for Transform3d {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TransformDocument {
            translation: [f32; 3],
            rotation: [f32; 4],
            scale: [f32; 3],
        }

        let document = TransformDocument::deserialize(deserializer)?;
        Self::new(document.translation, document.rotation, document.scale)
            .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{Transform3d, TransformError};

    #[test]
    fn identity_transform_has_identity_matrix() {
        let matrix = Transform3d::identity().to_matrix();
        assert_eq!(
            matrix,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
    }

    #[test]
    fn transform_matrix_applies_scale_then_translation() {
        let transform = Transform3d::new(
            [3.0, 4.0, 5.0],
            [0.0, 0.0, 0.0, 1.0],
            [2.0, 3.0, 4.0],
        )
        .expect("transform values should be valid");
        let matrix = transform.to_matrix();

        assert_eq!(matrix[0][0], 2.0);
        assert_eq!(matrix[1][1], 3.0);
        assert_eq!(matrix[2][2], 4.0);
        assert_eq!(matrix[3], [3.0, 4.0, 5.0, 1.0]);
    }

    #[test]
    fn positive_x_rotation_maps_positive_y_to_positive_z() {
        let half_angle = std::f32::consts::FRAC_PI_4;
        let transform = Transform3d::new(
            [0.0; 3],
            [half_angle.sin(), 0.0, 0.0, half_angle.cos()],
            [1.0; 3],
        )
        .expect("rotation should be valid");
        let matrix = transform.to_matrix();

        // Column 1 is the transformed +Y basis vector.
        assert!(matrix[1][1].abs() < 1.0e-5);
        assert!((matrix[1][2] - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn normalizes_rotation_quaternion() {
        let transform = Transform3d::new(
            [0.0; 3],
            [0.0, 0.0, 0.0, 2.0],
            [1.0; 3],
        )
        .expect("non-zero quaternion should be valid");

        assert_eq!(transform.rotation(), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rejects_non_finite_components_and_zero_rotation() {
        assert_eq!(
            Transform3d::new([f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]),
            Err(TransformError::NonFiniteComponent)
        );
        assert_eq!(
            Transform3d::new([0.0; 3], [0.0; 4], [1.0; 3]),
            Err(TransformError::DegenerateRotation)
        );
    }
}
