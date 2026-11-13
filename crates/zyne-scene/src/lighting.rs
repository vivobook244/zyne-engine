//! Renderer-independent light parameters for scene and rendering systems.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Validation errors for light parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightError {
    NonFiniteDirection,
    ZeroDirection,
    NonFiniteColor { component: usize },
    NegativeColor { component: usize },
    NonFiniteIntensity,
    NegativeIntensity,
    NonFiniteRange,
    NonPositiveRange,
    NonFiniteSpotAngle,
    InvalidSpotAngles,
}

impl std::fmt::Display for LightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteDirection => formatter.write_str("light direction must be finite"),
            Self::ZeroDirection => formatter.write_str("light direction must not be zero"),
            Self::NonFiniteColor { component } => {
                write!(
                    formatter,
                    "light color component {component} must be finite"
                )
            }
            Self::NegativeColor { component } => {
                write!(
                    formatter,
                    "light color component {component} must not be negative"
                )
            }
            Self::NonFiniteIntensity => formatter.write_str("light intensity must be finite"),
            Self::NegativeIntensity => formatter.write_str("light intensity must not be negative"),
            Self::NonFiniteRange => formatter.write_str("light range must be finite"),
            Self::NonPositiveRange => formatter.write_str("light range must be greater than zero"),
            Self::NonFiniteSpotAngle => formatter.write_str("spot angles must be finite"),
            Self::InvalidSpotAngles => {
                formatter.write_str("spot angles must satisfy 0 <= inner <= outer <= 90 degrees")
            }
        }
    }
}

impl std::error::Error for LightError {}

/// A directional light whose direction points along the light rays.
///
/// Color is linear RGB. Values above one are allowed for HDR workflows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DirectionalLight {
    direction: [f32; 3],
    color: [f32; 3],
    intensity: f32,
}

impl DirectionalLight {
    /// Creates a directional light with a normalized ray direction.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero or non-finite direction, invalid color, or invalid intensity.
    pub fn new(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Result<Self, LightError> {
        let direction = normalize_direction(direction)?;
        validate_color(color)?;
        validate_intensity(intensity)?;
        Ok(Self {
            direction,
            color,
            intensity,
        })
    }

    #[must_use]
    pub const fn direction(&self) -> [f32; 3] {
        self.direction
    }

    #[must_use]
    pub const fn color(&self) -> [f32; 3] {
        self.color
    }

    #[must_use]
    pub const fn intensity(&self) -> f32 {
        self.intensity
    }
}

impl<'de> Deserialize<'de> for DirectionalLight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Document {
            direction: [f32; 3],
            color: [f32; 3],
            intensity: f32,
        }
        let document = Document::deserialize(deserializer)?;
        Self::new(document.direction, document.color, document.intensity).map_err(D::Error::custom)
    }
}

/// A point light emitted from its owning scene entity's world position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PointLight {
    color: [f32; 3],
    intensity: f32,
    range: f32,
}

impl PointLight {
    /// Creates a point light with a finite positive influence range.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid color, intensity, or range values.
    pub fn new(color: [f32; 3], intensity: f32, range: f32) -> Result<Self, LightError> {
        validate_color(color)?;
        validate_intensity(intensity)?;
        validate_range(range)?;
        Ok(Self {
            color,
            intensity,
            range,
        })
    }

    #[must_use]
    pub const fn color(&self) -> [f32; 3] {
        self.color
    }

    #[must_use]
    pub const fn intensity(&self) -> f32 {
        self.intensity
    }

    #[must_use]
    pub const fn range(&self) -> f32 {
        self.range
    }
}

impl<'de> Deserialize<'de> for PointLight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Document {
            color: [f32; 3],
            intensity: f32,
            range: f32,
        }
        let document = Document::deserialize(deserializer)?;
        Self::new(document.color, document.intensity, document.range).map_err(D::Error::custom)
    }
}

/// A spot light emitted from its owning entity, along a normalized direction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SpotLight {
    direction: [f32; 3],
    color: [f32; 3],
    intensity: f32,
    range: f32,
    inner_angle: f32,
    outer_angle: f32,
}

impl SpotLight {
    /// Creates a spot light. Cone angles are expressed in radians and may not
    /// exceed 90 degrees; the inner cone may equal the outer cone.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid direction, color, intensity, range, or cone angles.
    pub fn new(
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Result<Self, LightError> {
        let direction = normalize_direction(direction)?;
        validate_color(color)?;
        validate_intensity(intensity)?;
        validate_range(range)?;
        if !inner_angle.is_finite() || !outer_angle.is_finite() {
            return Err(LightError::NonFiniteSpotAngle);
        }
        if inner_angle < 0.0
            || outer_angle < inner_angle
            || outer_angle > std::f32::consts::FRAC_PI_2
        {
            return Err(LightError::InvalidSpotAngles);
        }
        Ok(Self {
            direction,
            color,
            intensity,
            range,
            inner_angle,
            outer_angle,
        })
    }

    #[must_use]
    pub const fn direction(&self) -> [f32; 3] {
        self.direction
    }

    #[must_use]
    pub const fn color(&self) -> [f32; 3] {
        self.color
    }

    #[must_use]
    pub const fn intensity(&self) -> f32 {
        self.intensity
    }

    #[must_use]
    pub const fn range(&self) -> f32 {
        self.range
    }

    #[must_use]
    pub const fn inner_angle(&self) -> f32 {
        self.inner_angle
    }

    #[must_use]
    pub const fn outer_angle(&self) -> f32 {
        self.outer_angle
    }
}

impl<'de> Deserialize<'de> for SpotLight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Document {
            direction: [f32; 3],
            color: [f32; 3],
            intensity: f32,
            range: f32,
            inner_angle: f32,
            outer_angle: f32,
        }
        let document = Document::deserialize(deserializer)?;
        Self::new(
            document.direction,
            document.color,
            document.intensity,
            document.range,
            document.inner_angle,
            document.outer_angle,
        )
        .map_err(D::Error::custom)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn normalize_direction(direction: [f32; 3]) -> Result<[f32; 3], LightError> {
    if direction.iter().any(|component| !component.is_finite()) {
        return Err(LightError::NonFiniteDirection);
    }
    let length = direction
        .iter()
        .map(|component| f64::from(*component) * f64::from(*component))
        .sum::<f64>()
        .sqrt();
    if length <= f64::EPSILON {
        return Err(LightError::ZeroDirection);
    }
    Ok(direction.map(|component| (f64::from(component) / length) as f32))
}

fn validate_color(color: [f32; 3]) -> Result<(), LightError> {
    for (component, value) in color.into_iter().enumerate() {
        if !value.is_finite() {
            return Err(LightError::NonFiniteColor { component });
        }
        if value < 0.0 {
            return Err(LightError::NegativeColor { component });
        }
    }
    Ok(())
}

fn validate_intensity(intensity: f32) -> Result<(), LightError> {
    if !intensity.is_finite() {
        return Err(LightError::NonFiniteIntensity);
    }
    if intensity < 0.0 {
        return Err(LightError::NegativeIntensity);
    }
    Ok(())
}

fn validate_range(range: f32) -> Result<(), LightError> {
    if !range.is_finite() {
        return Err(LightError::NonFiniteRange);
    }
    if range <= 0.0 {
        return Err(LightError::NonPositiveRange);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DirectionalLight, LightError, PointLight, SpotLight};

    #[test]
    fn directional_light_normalizes_direction_and_preserves_hdr_color() {
        let light = DirectionalLight::new([0.0, -4.0, 0.0], [1.5, 1.0, 0.5], 2.0).unwrap();
        assert_eq!(light.direction(), [0.0, -1.0, 0.0]);
        assert_eq!(light.color(), [1.5, 1.0, 0.5]);
        assert_eq!(light.intensity(), 2.0);
    }

    #[test]
    fn point_light_accepts_zero_intensity_and_positive_range() {
        let light = PointLight::new([1.0; 3], 0.0, 12.0).unwrap();
        assert_eq!(light.intensity(), 0.0);
        assert_eq!(light.range(), 12.0);
    }

    #[test]
    fn spot_light_normalizes_direction_and_stores_cone_angles() {
        let light = SpotLight::new(
            [0.0, 0.0, -2.0],
            [1.0; 3],
            3.0,
            20.0,
            20.0_f32.to_radians(),
            35.0_f32.to_radians(),
        )
        .unwrap();
        assert_eq!(light.direction(), [0.0, 0.0, -1.0]);
        assert!((light.inner_angle().to_degrees() - 20.0).abs() < 1.0e-5);
        assert!((light.outer_angle().to_degrees() - 35.0).abs() < 1.0e-5);
    }

    #[test]
    fn rejects_invalid_directions() {
        assert_eq!(
            DirectionalLight::new([0.0; 3], [1.0; 3], 1.0),
            Err(LightError::ZeroDirection)
        );
        assert_eq!(
            DirectionalLight::new([f32::NAN, 0.0, 0.0], [1.0; 3], 1.0),
            Err(LightError::NonFiniteDirection)
        );
    }

    #[test]
    fn rejects_invalid_color_and_intensity() {
        assert_eq!(
            PointLight::new([1.0, -0.1, 1.0], 1.0, 5.0),
            Err(LightError::NegativeColor { component: 1 })
        );
        assert_eq!(
            PointLight::new([1.0; 3], f32::INFINITY, 5.0),
            Err(LightError::NonFiniteIntensity)
        );
        assert_eq!(
            PointLight::new([1.0; 3], -1.0, 5.0),
            Err(LightError::NegativeIntensity)
        );
    }

    #[test]
    fn rejects_invalid_point_ranges() {
        assert_eq!(
            PointLight::new([1.0; 3], 1.0, 0.0),
            Err(LightError::NonPositiveRange)
        );
        assert_eq!(
            PointLight::new([1.0; 3], 1.0, f32::INFINITY),
            Err(LightError::NonFiniteRange)
        );
    }

    #[test]
    fn rejects_invalid_spot_cones() {
        assert_eq!(
            SpotLight::new([0.0, 0.0, -1.0], [1.0; 3], 1.0, 5.0, 0.8, 0.4),
            Err(LightError::InvalidSpotAngles)
        );
        assert_eq!(
            SpotLight::new(
                [0.0, 0.0, -1.0],
                [1.0; 3],
                1.0,
                5.0,
                0.0,
                std::f32::consts::PI,
            ),
            Err(LightError::InvalidSpotAngles)
        );
        assert_eq!(
            SpotLight::new([0.0, 0.0, -1.0], [1.0; 3], 1.0, 5.0, f32::NAN, 0.5,),
            Err(LightError::NonFiniteSpotAngle)
        );
    }

    #[test]
    fn ron_deserialization_rejects_invalid_light_values() {
        let source = "(direction:[0.0,0.0,0.0],color:[1.0,1.0,1.0],intensity:1.0)";
        assert!(ron::de::from_str::<DirectionalLight>(source).is_err());
    }
}
