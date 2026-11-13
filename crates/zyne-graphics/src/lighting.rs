//! Renderer-facing, renderer-independent light snapshots.

/// Maximum number of directional lights uploaded per frame.
pub const MAX_DIRECTIONAL_LIGHTS: usize = 4;
/// Maximum number of point lights uploaded per frame.
pub const MAX_POINT_LIGHTS: usize = 16;
/// Maximum number of spot lights uploaded per frame.
pub const MAX_SPOT_LIGHTS: usize = 8;

/// A directional light snapshot in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLightData {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

/// A point light snapshot in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLightData {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
}

/// A spot light snapshot in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotLightData {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
}

/// A compact snapshot of all lights used to shade one frame.
///
/// The renderer uploads at most the first supported number of each light type.
/// Callers should provide a deterministic order when more lights are present.
#[derive(Debug, Clone, PartialEq)]
pub struct LightingData {
    pub ambient_color: [f32; 3],
    pub directional_lights: Vec<DirectionalLightData>,
    pub point_lights: Vec<PointLightData>,
    pub spot_lights: Vec<SpotLightData>,
}

impl Default for LightingData {
    fn default() -> Self {
        Self {
            ambient_color: [0.16, 0.16, 0.18],
            directional_lights: vec![DirectionalLightData {
                direction: [-0.4, -1.0, -0.3],
                color: [1.0; 3],
                intensity: 1.0,
            }],
            point_lights: Vec::new(),
            spot_lights: Vec::new(),
        }
    }
}

impl LightingData {
    /// Creates a scene lighting snapshot with ambient illumination and no direct lights.
    #[must_use]
    pub fn with_ambient(ambient_color: [f32; 3]) -> Self {
        Self {
            ambient_color,
            directional_lights: Vec::new(),
            point_lights: Vec::new(),
            spot_lights: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LightingData, MAX_DIRECTIONAL_LIGHTS, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS};

    #[test]
    fn default_lighting_has_a_soft_ambient_and_directional_key_light() {
        let lighting = LightingData::default();
        assert_eq!(lighting.ambient_color, [0.16, 0.16, 0.18]);
        assert_eq!(lighting.directional_lights.len(), 1);
        assert!(lighting.point_lights.is_empty());
        assert!(lighting.spot_lights.is_empty());
    }

    #[test]
    fn scene_lighting_starts_without_direct_lights() {
        let lighting = LightingData::with_ambient([0.1, 0.2, 0.3]);
        assert_eq!(lighting.ambient_color, [0.1, 0.2, 0.3]);
        assert!(lighting.directional_lights.is_empty());
        assert!(lighting.point_lights.is_empty());
        assert!(lighting.spot_lights.is_empty());
    }

    #[test]
    fn gpu_light_limits_are_bounded() {
        assert_eq!(MAX_DIRECTIONAL_LIGHTS, 4);
        assert_eq!(MAX_POINT_LIGHTS, 16);
        assert_eq!(MAX_SPOT_LIGHTS, 8);
    }
}
