//! Renderer-independent Blinn–Phong material data and validation.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use zyne_core::ResourceId;

/// Errors produced when material parameters are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialError {
    /// A color component is not finite.
    NonFiniteColor {
        property: &'static str,
        component: usize,
    },
    /// A color component is negative.
    NegativeColor {
        property: &'static str,
        component: usize,
    },
    /// Shininess is not finite.
    NonFiniteShininess,
    /// Shininess must be greater than zero.
    NonPositiveShininess,
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteColor {
                property,
                component,
            } => {
                write!(
                    formatter,
                    "{property} color component {component} must be finite"
                )
            }
            Self::NegativeColor {
                property,
                component,
            } => {
                write!(
                    formatter,
                    "{property} color component {component} must not be negative"
                )
            }
            Self::NonFiniteShininess => formatter.write_str("material shininess must be finite"),
            Self::NonPositiveShininess => {
                formatter.write_str("material shininess must be greater than zero")
            }
        }
    }
}

impl std::error::Error for MaterialError {}

/// CPU-side Blinn–Phong material parameters.
///
/// Colors are linear RGB values. Components may exceed one to support
/// high-dynamic-range material values, but must be finite and non-negative.
/// The optional diffuse texture is a persistent resource identity, never a
/// renderer-specific GPU handle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlinnPhongMaterial {
    ambient: [f32; 3],
    diffuse: [f32; 3],
    specular: [f32; 3],
    shininess: f32,
    diffuse_texture: Option<ResourceId>,
}

impl BlinnPhongMaterial {
    /// Creates a validated Blinn–Phong material without a diffuse texture.
    ///
    /// # Errors
    ///
    /// Returns an error if a color component is non-finite or negative, or if
    /// shininess is non-finite or not greater than zero.
    pub fn new(
        ambient: [f32; 3],
        diffuse: [f32; 3],
        specular: [f32; 3],
        shininess: f32,
    ) -> Result<Self, MaterialError> {
        validate_color("ambient", ambient)?;
        validate_color("diffuse", diffuse)?;
        validate_color("specular", specular)?;
        validate_shininess(shininess)?;

        Ok(Self {
            ambient,
            diffuse,
            specular,
            shininess,
            diffuse_texture: None,
        })
    }

    /// Returns the ambient reflectance color in linear RGB.
    #[must_use]
    pub const fn ambient(&self) -> [f32; 3] {
        self.ambient
    }

    /// Returns the diffuse reflectance color in linear RGB.
    #[must_use]
    pub const fn diffuse(&self) -> [f32; 3] {
        self.diffuse
    }

    /// Returns the specular reflectance color in linear RGB.
    #[must_use]
    pub const fn specular(&self) -> [f32; 3] {
        self.specular
    }

    /// Returns the specular highlight exponent.
    #[must_use]
    pub const fn shininess(&self) -> f32 {
        self.shininess
    }

    /// Returns the persistent diffuse texture resource identity, if assigned.
    #[must_use]
    pub const fn diffuse_texture(&self) -> Option<ResourceId> {
        self.diffuse_texture
    }

    /// Assigns or clears the diffuse texture resource identity.
    pub fn set_diffuse_texture(&mut self, texture: Option<ResourceId>) {
        self.diffuse_texture = texture;
    }
}

impl Default for BlinnPhongMaterial {
    fn default() -> Self {
        Self {
            ambient: [0.1; 3],
            diffuse: [0.8; 3],
            specular: [0.2; 3],
            shininess: 32.0,
            diffuse_texture: None,
        }
    }
}

impl<'de> Deserialize<'de> for BlinnPhongMaterial {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MaterialDocument {
            ambient: [f32; 3],
            diffuse: [f32; 3],
            specular: [f32; 3],
            shininess: f32,
            diffuse_texture: Option<ResourceId>,
        }

        let document = MaterialDocument::deserialize(deserializer)?;
        let mut material = Self::new(
            document.ambient,
            document.diffuse,
            document.specular,
            document.shininess,
        )
        .map_err(D::Error::custom)?;
        material.diffuse_texture = document.diffuse_texture;
        Ok(material)
    }
}

fn validate_color(property: &'static str, color: [f32; 3]) -> Result<(), MaterialError> {
    for (component, value) in color.into_iter().enumerate() {
        if !value.is_finite() {
            return Err(MaterialError::NonFiniteColor {
                property,
                component,
            });
        }
        if value < 0.0 {
            return Err(MaterialError::NegativeColor {
                property,
                component,
            });
        }
    }
    Ok(())
}

fn validate_shininess(shininess: f32) -> Result<(), MaterialError> {
    if !shininess.is_finite() {
        return Err(MaterialError::NonFiniteShininess);
    }
    if shininess <= 0.0 {
        return Err(MaterialError::NonPositiveShininess);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BlinnPhongMaterial, MaterialError};
    use zyne_core::ResourceId;

    #[test]
    fn default_material_has_valid_blinn_phong_parameters() {
        let material = BlinnPhongMaterial::default();

        assert_eq!(material.ambient(), [0.1; 3]);
        assert_eq!(material.diffuse(), [0.8; 3]);
        assert_eq!(material.specular(), [0.2; 3]);
        assert_eq!(material.shininess(), 32.0);
        assert_eq!(material.diffuse_texture(), None);
    }

    #[test]
    fn creates_material_with_high_dynamic_range_colors() {
        let material = BlinnPhongMaterial::new([0.1; 3], [1.5, 0.8, 0.4], [2.0; 3], 64.0).unwrap();

        assert_eq!(material.diffuse(), [1.5, 0.8, 0.4]);
        assert_eq!(material.specular(), [2.0; 3]);
    }

    #[test]
    fn rejects_non_finite_and_negative_color_components() {
        assert_eq!(
            BlinnPhongMaterial::new([f32::NAN, 0.0, 0.0], [1.0; 3], [1.0; 3], 32.0),
            Err(MaterialError::NonFiniteColor {
                property: "ambient",
                component: 0,
            })
        );
        assert_eq!(
            BlinnPhongMaterial::new([0.0; 3], [1.0, -0.1, 1.0], [1.0; 3], 32.0),
            Err(MaterialError::NegativeColor {
                property: "diffuse",
                component: 1,
            })
        );
    }

    #[test]
    fn rejects_invalid_shininess() {
        assert_eq!(
            BlinnPhongMaterial::new([0.0; 3], [1.0; 3], [1.0; 3], f32::INFINITY),
            Err(MaterialError::NonFiniteShininess)
        );
        assert_eq!(
            BlinnPhongMaterial::new([0.0; 3], [1.0; 3], [1.0; 3], 0.0),
            Err(MaterialError::NonPositiveShininess)
        );
    }

    #[test]
    fn stores_persistent_diffuse_texture_identity() {
        let mut material = BlinnPhongMaterial::default();
        let texture = ResourceId::new();

        material.set_diffuse_texture(Some(texture));
        assert_eq!(material.diffuse_texture(), Some(texture));

        material.set_diffuse_texture(None);
        assert_eq!(material.diffuse_texture(), None);
    }

    #[test]
    fn ron_round_trip_preserves_material_parameters_and_texture_reference() {
        let mut material =
            BlinnPhongMaterial::new([0.05, 0.06, 0.07], [0.8, 0.7, 0.6], [0.3, 0.4, 0.5], 48.0)
                .unwrap();
        let texture = ResourceId::new();
        material.set_diffuse_texture(Some(texture));

        let encoded = ron::ser::to_string(&material).unwrap();
        let decoded: BlinnPhongMaterial = ron::de::from_str(&encoded).unwrap();

        assert_eq!(decoded, material);
    }

    #[test]
    fn ron_deserialization_rejects_invalid_material_parameters() {
        let source = "(ambient:[0.0,0.0,0.0],diffuse:[1.0,-1.0,1.0],specular:[1.0,1.0,1.0],shininess:32.0,diffuse_texture:None)";
        assert!(ron::de::from_str::<BlinnPhongMaterial>(source).is_err());
    }
}
