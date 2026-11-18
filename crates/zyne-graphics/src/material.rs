//! Renderer-facing, GPU-independent material parameters.

/// Blinn–Phong material parameters consumed by the graphics pipeline.
///
/// Values use linear RGB. This type is intentionally independent from asset
/// registry identities and GPU handles so render data can be prepared by the
/// runtime without coupling graphics to the asset system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialData {
    ambient: [f32; 3],
    diffuse: [f32; 3],
    specular: [f32; 3],
    shininess: f32,
}

impl MaterialData {
    /// Creates renderer-facing material parameters.
    #[must_use]
    pub const fn new(
        ambient: [f32; 3],
        diffuse: [f32; 3],
        specular: [f32; 3],
        shininess: f32,
    ) -> Self {
        Self {
            ambient,
            diffuse,
            specular,
            shininess,
        }
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
}

impl Default for MaterialData {
    fn default() -> Self {
        Self::new([0.1; 3], [0.8; 3], [0.2; 3], 32.0)
    }
}

#[cfg(test)]
mod tests {
    use super::MaterialData;

    #[test]
    fn default_material_matches_asset_material_defaults() {
        let material = MaterialData::default();

        assert_eq!(material.ambient(), [0.1; 3]);
        assert_eq!(material.diffuse(), [0.8; 3]);
        assert_eq!(material.specular(), [0.2; 3]);
        assert_eq!(material.shininess(), 32.0);
    }

    #[test]
    fn stores_renderer_facing_material_parameters() {
        let material = MaterialData::new(
            [0.05, 0.06, 0.07],
            [0.8, 0.7, 0.6],
            [0.3, 0.4, 0.5],
            48.0,
        );

        assert_eq!(material.ambient(), [0.05, 0.06, 0.07]);
        assert_eq!(material.diffuse(), [0.8, 0.7, 0.6]);
        assert_eq!(material.specular(), [0.3, 0.4, 0.5]);
        assert_eq!(material.shininess(), 48.0);
    }
}
