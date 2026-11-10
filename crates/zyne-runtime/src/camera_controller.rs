//! Input-driven camera controllers exposed by the runtime.

use zyne_core::{InputState, MouseButton};
use zyne_math::{Camera3d, CameraError};

const DEFAULT_YAW: f32 = std::f32::consts::FRAC_PI_4;
const DEFAULT_PITCH: f32 = std::f32::consts::FRAC_PI_6;
const DEFAULT_FOV: f32 = 55.0_f32.to_radians();
const MIN_DISTANCE: f32 = 0.25;
const MAX_DISTANCE: f32 = 1_000.0;
const MIN_PITCH: f32 = -std::f32::consts::FRAC_PI_2 + 0.01;
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
const ORBIT_SENSITIVITY: f32 = 0.005;
const ZOOM_SENSITIVITY: f32 = 0.12;
const PAN_SENSITIVITY: f32 = 0.0015;

/// Mouse-driven orbit camera for inspecting a 3D scene.
///
/// Hold the right mouse button to orbit, the middle button to pan, and use
/// the scroll wheel to zoom. The controller is independent of the renderer;
/// call `camera()` and pass the result to a rendering pipeline.
#[derive(Debug, Clone)]
pub struct OrbitCameraController {
    target: [f32; 3],
    distance: f32,
    yaw: f32,
    pitch: f32,
    aspect_ratio: f32,
    last_cursor_position: Option<(f64, f64)>,
}

impl OrbitCameraController {
    /// Creates an orbit controller focused on target.
    ///
    /// The initial orbit uses a 45-degree yaw and 30-degree pitch. The camera
    /// uses a 55-degree vertical field of view and 0.1–1000 world-unit clip
    /// planes.
    ///
    /// # Errors
    ///
    /// Returns a camera validation error when the target, distance, or aspect
    /// ratio is invalid.
    pub fn new(target: [f32; 3], distance: f32, aspect_ratio: f32) -> Result<Self, CameraError> {
        if !distance.is_finite() {
            return Err(CameraError::NonFiniteParameter);
        }
        if distance <= 0.0 {
            return Err(CameraError::InvalidCameraDistance);
        }

        let controller = Self {
            target,
            distance: distance.clamp(MIN_DISTANCE, MAX_DISTANCE),
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            aspect_ratio,
            last_cursor_position: None,
        };
        controller.camera()?;
        Ok(controller)
    }

    /// Updates orbit, pan, and zoom from the current frame's input state.
    ///
    /// # Errors
    ///
    /// Returns a camera validation error if accumulated input produces
    /// non-finite camera parameters.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn update(&mut self, input: &InputState) -> Result<Camera3d, CameraError> {
        let cursor = input.cursor_position();
        if let Some((x, y)) = cursor.filter(|(x, y)| x.is_finite() && y.is_finite()) {
            if let Some((delta_x, delta_y)) = self.drag_delta(input, MouseButton::Right, (x, y)) {
                self.yaw -= delta_x * ORBIT_SENSITIVITY;
                self.pitch = (self.pitch + delta_y * ORBIT_SENSITIVITY).clamp(MIN_PITCH, MAX_PITCH);
            } else if let Some((delta_x, delta_y)) =
                self.drag_delta(input, MouseButton::Middle, (x, y))
            {
                self.pan(delta_x, delta_y);
            }
            self.last_cursor_position = Some((x, y));
        } else {
            self.last_cursor_position = None;
        }

        let (_, scroll_y) = input.scroll_delta();
        if scroll_y.is_finite() && scroll_y != 0.0 {
            self.distance = (self.distance * (-(scroll_y as f32) * ZOOM_SENSITIVITY).exp())
                .clamp(MIN_DISTANCE, MAX_DISTANCE);
        }

        self.camera()
    }

    /// Builds the current validated perspective camera.
    ///
    /// # Errors
    ///
    /// Returns a camera validation error if the controller contains invalid
    /// parameters.
    pub fn camera(&self) -> Result<Camera3d, CameraError> {
        self.camera_with_aspect_ratio(self.aspect_ratio)
    }

    /// Builds the current camera using the supplied viewport aspect ratio.
    ///
    /// This does not mutate the controller, allowing the runtime to supply the
    /// latest surface dimensions on every render.
    ///
    /// # Errors
    ///
    /// Returns a camera validation error when the aspect ratio or controller
    /// parameters are invalid.
    pub fn camera_with_aspect_ratio(
        &self,
        aspect_ratio: f32,
    ) -> Result<Camera3d, CameraError> {
        let cos_pitch = self.pitch.cos();
        let offset = [
            self.distance * cos_pitch * self.yaw.sin(),
            self.distance * self.pitch.sin(),
            self.distance * cos_pitch * self.yaw.cos(),
        ];
        let position = [
            self.target[0] + offset[0],
            self.target[1] + offset[1],
            self.target[2] + offset[2],
        ];

        Camera3d::new(
            position,
            self.target,
            [0.0, 1.0, 0.0],
            DEFAULT_FOV,
            aspect_ratio,
            0.1,
            MAX_DISTANCE * 2.0,
        )
    }

    /// Changes the viewport aspect ratio after validating it.
    ///
    /// # Errors
    ///
    /// Returns a camera validation error when the aspect ratio is invalid.
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

    /// Returns the current orbit target in world space.
    #[must_use]
    pub const fn target(&self) -> [f32; 3] {
        self.target
    }

    /// Returns the current distance from the camera to its target.
    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn drag_delta(
        &self,
        input: &InputState,
        button: MouseButton,
        cursor: (f64, f64),
    ) -> Option<(f32, f32)> {
        if input.is_mouse_pressed(button) && !input.just_mouse_pressed(button) {
            self.last_cursor_position
                .map(|(last_x, last_y)| ((cursor.0 - last_x) as f32, (cursor.1 - last_y) as f32))
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let right = [cos_yaw, 0.0, -sin_yaw];
        let up = [-sin_yaw * sin_pitch, cos_pitch, -cos_yaw * sin_pitch];
        let scale = self.distance * PAN_SENSITIVITY;
        for axis in 0..3 {
            self.target[axis] += (-delta_x * right[axis] + delta_y * up[axis]) * scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OrbitCameraController;
    use zyne_core::{ButtonState, InputEvent, InputState, MouseButton, ScrollDelta};

    #[test]
    fn creates_camera_with_requested_target_and_distance() {
        let controller = OrbitCameraController::new([1.0, 2.0, 3.0], 8.0, 16.0 / 9.0)
            .expect("camera controller should be valid");
        let camera = controller.camera().expect("camera should be valid");

        assert_eq!(camera.target(), [1.0, 2.0, 3.0]);
        let position = camera.position();
        let distance = ((position[0] - 1.0).powi(2)
            + (position[1] - 2.0).powi(2)
            + (position[2] - 3.0).powi(2))
        .sqrt();
        assert!((distance - 8.0).abs() < 1.0e-5);
    }

    #[test]
    fn right_drag_orbits_around_the_target() {
        let mut controller =
            OrbitCameraController::new([0.0; 3], 5.0, 1.0).expect("controller should be valid");
        let initial_position = controller
            .camera()
            .expect("initial camera should be valid")
            .position();
        let mut input = InputState::new();
        input.process_event(InputEvent::CursorMoved { x: 100.0, y: 100.0 });
        input.process_event(InputEvent::MouseButton {
            button: MouseButton::Right,
            state: ButtonState::Pressed,
        });
        controller
            .update(&input)
            .expect("initial drag should be valid");

        input.begin_frame();
        input.process_event(InputEvent::CursorMoved { x: 140.0, y: 80.0 });
        let camera = controller.update(&input).expect("orbit should be valid");

        assert_eq!(camera.target(), [0.0; 3]);
        assert_ne!(camera.position(), initial_position);
    }

    #[test]
    fn scroll_zoom_respects_minimum_and_maximum_distance() {
        let mut controller =
            OrbitCameraController::new([0.0; 3], 5.0, 1.0).expect("controller should be valid");
        let mut input = InputState::new();
        input.process_event(InputEvent::MouseWheel(ScrollDelta::Lines {
            x: 0.0,
            y: 100.0,
        }));
        controller.update(&input).expect("zoom should be valid");
        assert!((controller.distance() - 0.25).abs() < f32::EPSILON);

        input.begin_frame();
        input.process_event(InputEvent::MouseWheel(ScrollDelta::Lines {
            x: 0.0,
            y: -100.0,
        }));
        controller.update(&input).expect("zoom out should be valid");
        assert!((controller.distance() - 1_000.0).abs() < 1.0e-3);
    }

    #[test]
    fn middle_drag_pans_the_orbit_target() {
        let mut controller =
            OrbitCameraController::new([0.0; 3], 5.0, 1.0).expect("controller should be valid");
        let mut input = InputState::new();
        input.process_event(InputEvent::CursorMoved { x: 0.0, y: 0.0 });
        input.process_event(InputEvent::MouseButton {
            button: MouseButton::Middle,
            state: ButtonState::Pressed,
        });
        controller
            .update(&input)
            .expect("initial pan should be valid");

        input.begin_frame();
        input.process_event(InputEvent::CursorMoved { x: 20.0, y: 0.0 });
        controller.update(&input).expect("pan should be valid");

        assert_ne!(controller.target(), [0.0; 3]);
    }
}
