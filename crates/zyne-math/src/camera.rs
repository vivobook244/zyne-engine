//! Validated 3D camera projection and view math.
//!
//! Matrices are column-major and use column-vector algebra. The camera looks
//! along its local +Z engine-forward axis; the view matrix maps visible points
//! into conventional right-handed view space where forward is -Z.

/// Errors produced when camera projection or view parameters are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraError {
    NonFiniteParameter,
    InvalidFieldOfView,
    InvalidAspectRatio,
    InvalidClipPlanes,
    InvalidCameraDistance,
    DegenerateViewDirection,
    DegenerateUpDirection,
}

impl std::fmt::Display for CameraError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NonFiniteParameter => "camera parameters must be finite",
            Self::InvalidFieldOfView => "camera field of view must be between zero and pi radians",
            Self::InvalidAspectRatio => "camera aspect ratio must be positive",
            Self::InvalidClipPlanes => "camera clip planes must be positive and ordered",
            Self::InvalidCameraDistance => "camera distance must be positive",
            Self::DegenerateViewDirection => "camera position and target must differ",
            Self::DegenerateUpDirection => "camera up direction must not be parallel to view direction",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CameraError {}

/// A right-handed perspective camera using wgpu's [0, 1] clip-depth range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3d {
    position: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
    vertical_fov_radians: f32,
    aspect_ratio: f32,
    near: f32,
    far: f32,
}

impl Camera3d {
    /// Creates a perspective camera looking from `position` toward `target`.
    ///
    /// The vertical field of view is in radians. All distances use engine
    /// world units; the near plane must be positive and the far plane greater.
    ///
    /// # Errors
    ///
    /// Returns `CameraError` when any parameter is non-finite, the field of
    /// view or aspect ratio is invalid, the clip planes are invalid, or the
    /// view direction/up basis is degenerate.
    pub fn new(
        position: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Result<Self, CameraError> {
        if !position
            .iter()
            .chain(target.iter())
            .chain(up.iter())
            .chain([vertical_fov_radians, aspect_ratio, near, far].iter())
            .all(|value| value.is_finite())
        {
            return Err(CameraError::NonFiniteParameter);
        }
        if vertical_fov_radians <= 0.0 || vertical_fov_radians >= std::f32::consts::PI {
            return Err(CameraError::InvalidFieldOfView);
        }
        if aspect_ratio <= 0.0 {
            return Err(CameraError::InvalidAspectRatio);
        }
        if near <= 0.0 || far <= near {
            return Err(CameraError::InvalidClipPlanes);
        }
        if length_squared(sub(target, position)) <= f32::EPSILON {
            return Err(CameraError::DegenerateViewDirection);
        }
        if length_squared(cross(normalize(sub(target, position)), up)) <= f32::EPSILON {
            return Err(CameraError::DegenerateUpDirection);
        }
        Ok(Self {
            position,
            target,
            up,
            vertical_fov_radians,
            aspect_ratio,
            near,
            far,
        })
    }

    /// Changes the viewport aspect ratio after validating it.
    ///
    /// # Errors
    ///
    /// Returns `CameraError::NonFiniteParameter` for a non-finite value or
    /// `CameraError::InvalidAspectRatio` when the value is not positive.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) -> Result<(), CameraError> {
        if !aspect_ratio.is_finite() {
            return Err(CameraError::NonFiniteParameter);
        }
        if aspect_ratio <= 0.0 {
            return Err(CameraError::InvalidAspectRatio);
        }
        self.aspect_ratio = aspect_ratio;
        Ok(())
    }

    /// Returns the camera's view matrix in column-major layout.
    #[must_use]
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let forward = normalize(sub(self.target, self.position));
        let right = normalize(cross(forward, self.up));
        let up = cross(right, forward);
        [
            [right[0], up[0], -forward[0], 0.0],
            [right[1], up[1], -forward[1], 0.0],
            [right[2], up[2], -forward[2], 0.0],
            [
                -dot(right, self.position),
                -dot(up, self.position),
                dot(forward, self.position),
                1.0,
            ],
        ]
    }

    /// Returns a right-handed perspective matrix with a [0, 1] depth range.
    #[must_use]
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        let focal = 1.0 / (self.vertical_fov_radians * 0.5).tan();
        [
            [focal / self.aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, focal, 0.0, 0.0],
            [0.0, 0.0, self.far / (self.near - self.far), -1.0],
            [
                0.0,
                0.0,
                self.near * (self.far / (self.near - self.far)),
                0.0,
            ],
        ]
    }

    /// Returns projection * view, suitable for multiplying world-space points.
    #[must_use]
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        multiply(self.projection_matrix(), self.view_matrix())
    }

    /// Returns the camera world-space position.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    /// Returns the camera world-space target.
    #[must_use]
    pub const fn target(&self) -> [f32; 3] {
        self.target
    }

    /// Returns the current viewport aspect ratio.
    #[must_use]
    pub const fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }
}

fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            for inner in 0..4 {
                result[column][row] += a[inner][row] * b[column][inner];
            }
        }
    }
    result
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length_squared(v: [f32; 3]) -> f32 {
    dot(v, v)
}
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let inverse_length = length_squared(v).sqrt().recip();
    [
        v[0] * inverse_length,
        v[1] * inverse_length,
        v[2] * inverse_length,
    ]
}

#[cfg(test)]
mod tests {
    use super::{Camera3d, CameraError};

    fn camera() -> Camera3d {
        Camera3d::new(
            [4.0, 2.8, 4.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            55.0_f32.to_radians(),
            16.0 / 9.0,
            0.1,
            100.0,
        )
        .expect("test camera parameters should be valid")
    }

    #[test]
    fn rejects_invalid_projection_parameters() {
        for invalid_fov in [0.0, -0.1, std::f32::consts::PI, f32::NAN, f32::INFINITY] {
            let expected_error = if invalid_fov.is_finite() {
                CameraError::InvalidFieldOfView
            } else {
                CameraError::NonFiniteParameter
            };
            assert_eq!(
                Camera3d::new(
                    [0.0; 3],
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0, 0.0],
                    invalid_fov,
                    1.0,
                    0.1,
                    10.0
                ),
                Err(expected_error)
            );
        }
        assert_eq!(
            Camera3d::new(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
                1.0,
                0.0,
                0.1,
                10.0
            ),
            Err(CameraError::InvalidAspectRatio)
        );
        assert_eq!(
            Camera3d::new(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                10.0,
                1.0
            ),
            Err(CameraError::InvalidClipPlanes)
        );
    }

    #[test]
    fn rejects_degenerate_camera_basis() {
        assert_eq!(
            Camera3d::new([0.0; 3], [0.0; 3], [0.0, 1.0, 0.0], 1.0, 1.0, 0.1, 10.0),
            Err(CameraError::DegenerateViewDirection)
        );
        assert_eq!(
            Camera3d::new(
                [0.0; 3],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                10.0
            ),
            Err(CameraError::DegenerateUpDirection)
        );
    }

    #[test]
    fn projection_maps_near_and_far_planes_to_zero_and_one() {
        let matrix = camera().projection_matrix();
        let near = 0.1_f32;
        let far = 100.0_f32;
        let project_depth = |z: f32| (matrix[2][2] * z + matrix[3][2]) / -z;
        assert!(project_depth(-near).abs() < 1.0e-5);
        assert!((project_depth(-far) - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn view_projection_places_target_in_clip_volume() {
        let matrix = camera().view_projection_matrix();
        let clip_z = matrix[3][2];
        let clip_w = matrix[3][3];
        assert!(clip_w > 0.0);
        assert!(clip_z >= 0.0 && clip_z <= clip_w);
    }

    #[test]
    fn view_matrix_maps_camera_position_to_origin() {
        let camera = camera();
        let view = camera.view_matrix();
        let position = camera.position();
        let transformed = transform_point(view, position);

        for component in transformed {
            assert!(component.abs() < 1.0e-5);
        }
    }

    #[test]
    fn view_matrix_maps_target_to_negative_view_z() {
        let camera = camera();
        let view = camera.view_matrix();
        let target = camera.target();
        let transformed = transform_point(view, target);
        let distance = ((camera.position()[0] - target[0]).powi(2)
            + (camera.position()[1] - target[1]).powi(2)
            + (camera.position()[2] - target[2]).powi(2))
        .sqrt();

        assert!(transformed[0].abs() < 1.0e-5);
        assert!(transformed[1].abs() < 1.0e-5);
        assert!((transformed[2] + distance).abs() < 1.0e-4);
    }

    #[test]
    fn view_projection_maps_near_and_far_points_into_wgpu_depth_range() {
        let camera = Camera3d::new(
            [0.0; 3],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            60.0_f32.to_radians(),
            1.0,
            0.5,
            20.0,
        )
        .expect("camera parameters should be valid");
        let matrix = camera.view_projection_matrix();
        let near = transform_point(matrix, [0.0, 0.0, -0.5]);
        let far = transform_point(matrix, [0.0, 0.0, -20.0]);

        assert!((near[2] - 0.0).abs() < 1.0e-5);
        assert!((far[2] - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn projection_scales_horizontal_axis_by_aspect_ratio() {
        let wide = Camera3d::new(
            [0.0; 3],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            60.0_f32.to_radians(),
            2.0,
            0.1,
            100.0,
        )
        .expect("wide camera should be valid");
        let square = Camera3d::new(
            [0.0; 3],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            60.0_f32.to_radians(),
            1.0,
            0.1,
            100.0,
        )
        .expect("square camera should be valid");

        assert!(
            (wide.projection_matrix()[0][0] * 2.0 - square.projection_matrix()[0][0]).abs()
                < 1.0e-6
        );
    }

    fn transform_point(matrix: [[f32; 4]; 4], point: [f32; 3]) -> [f32; 3] {
        let vector = [point[0], point[1], point[2], 1.0];
        let mut clip = [0.0; 4];

        for row in 0..4 {
            for column in 0..4 {
                clip[row] += matrix[column][row] * vector[column];
            }
        }

        [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]]
    }

    #[test]
    fn aspect_ratio_updates_only_after_valid_value() {
        let mut camera = camera();
        assert_eq!(camera.set_aspect_ratio(2.0), Ok(()));
        assert_eq!(camera.aspect_ratio(), 2.0);
        assert_eq!(
            camera.set_aspect_ratio(0.0),
            Err(CameraError::InvalidAspectRatio)
        );
        assert_eq!(camera.aspect_ratio(), 2.0);
    }
}
