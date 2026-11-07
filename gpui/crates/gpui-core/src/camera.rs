//! A Z-up orbit camera, matching the app's scene convention.

use glam::{Mat4, Vec3};

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    pub fov_y: f32,
    pub orthographic: bool,
    /// Exponential-smoothing goals. Input writes the goal; `step` eases the
    /// camera toward it at a time-based rate, so the easing is visible without
    /// the input ever feeling disconnected from the pointer.
    goal_yaw: f32,
    goal_pitch: f32,
    goal_distance: f32,
    goal_target: Vec3,
    /// Home / focus tween: (start pose, end pose, elapsed seconds, duration).
    tween: Option<Tween>,
    /// True while a pointer drag is in progress: the camera then tracks the
    /// pointer almost immediately, and only the release keeps the soft tail.
    dragging: bool,
}

struct Tween {
    start_yaw: f32,
    start_pitch: f32,
    start_distance: f32,
    start_target: Vec3,
    end_yaw: f32,
    end_pitch: f32,
    end_distance: f32,
    end_target: Vec3,
    elapsed: f32,
    duration: f32,
}

// `cameraFeelPreset = 'precise'`: rotateSpeed 0.72, panSpeed 0.82,
// zoomSpeed 1.25, dampingFactor 0.15.
const ROTATE_SPEED: f32 = 0.72 * SENSITIVITY;
const PAN_SPEED: f32 = 0.82 * SENSITIVITY;
const ZOOM_SPEED: f32 = 1.25 * ZOOM_SENSITIVITY;
// The preset's `dampingFactor 0.15` is not a constant here: OrbitControls applies
// it per frame, so the port expresses it as the settle rate below, chosen to sum
// to the same motion at 60 Hz.
/// Gesture gain on top of the 'precise' preset.
const SENSITIVITY: f32 = 0.7;
const ZOOM_SENSITIVITY: f32 = 0.75;

impl Camera {
    pub fn fit(center: Vec3, radius: f32) -> Self {
        let fov_y = 45f32.to_radians();
        let distance = radius / (fov_y * 0.5).tan() * 1.15;
        Self {
            yaw: -std::f32::consts::FRAC_PI_4,
            pitch: 0.5,
            distance,
            target: center,
            fov_y,
            orthographic: true,
            goal_yaw: -std::f32::consts::FRAC_PI_4,
            goal_pitch: 0.5,
            goal_distance: distance,
            goal_target: center,
            tween: None,
            dragging: false,
        }
    }

    /// The empty scene's parked pose: `useStlLoadCameraIntro`'s default camera,
    /// the build-volume centre plus `(-220, -220, +260)`.
    pub fn parked(target: Vec3) -> Self {
        let mut camera = Self::fit(target, 60.0);
        let offset = Vec3::new(-220.0, -220.0, 260.0);
        camera.target = target;
        camera.distance = offset.length();
        camera.pitch = (offset.z / offset.length()).asin();
        camera.yaw = offset.y.atan2(offset.x);
        // The goals must match, or the first `step` eases back to `fit`'s pose.
        camera.goal_target = camera.target;
        camera.goal_distance = camera.distance;
        camera.goal_pitch = camera.pitch;
        camera.goal_yaw = camera.yaw;
        camera
    }

    /// Aim the camera without a tween: the smoothing goals are what `step`
    /// eases toward, so writing `yaw`/`pitch` alone is undone on the next frame
    /// (the view cube snapped back to its old angle for exactly that reason).
    pub fn orient_to(&mut self, yaw: f32, pitch: f32) {
        self.goal_yaw = yaw;
        self.goal_pitch = pitch;
        self.tween = None;
    }

    pub fn eye(&self) -> Vec3 {
        let cp = self.pitch.cos();
        self.target
            + Vec3::new(cp * self.yaw.cos(), cp * self.yaw.sin(), self.pitch.sin())
                * self.distance
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let aspect = aspect.max(0.0001);
        if self.orthographic {
            // `orthoDolly.ts`: the frustum is derived from the dolly radius and
            // the depth range is symmetric about the camera (`ORTHO_NEAR =
            // -50000`, `ORTHO_FAR = 50000`). A near plane in front of the
            // camera clips the model once the dolly radius shrinks, because the
            // radius *is* the zoom.
            let half_h = self.distance * (self.fov_y * 0.5).tan();
            let half_w = half_h * aspect;
            const DEPTH: f32 = 50000.0;
            Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, -DEPTH, DEPTH)
        } else {
            let near = (self.distance * 0.001).max(0.0001);
            let far = self.distance * 100.0;
            Mat4::perspective_rh(self.fov_y, aspect, near, far)
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Z);
        self.projection(aspect) * view
    }

    /// A world-space ray through a normalised device coordinate.
    pub fn ray(&self, aspect: f32, ndc_x: f32, ndc_y: f32) -> (Vec3, Vec3) {
        let inverse = self.view_proj(aspect).inverse();
        let near = inverse * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far = inverse * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
        let near = near.truncate() / near.w;
        let far = far.truncate() / far.w;
        (near, (far - near).normalize())
    }

    /// OrbitControls: `2π * rotateSpeed / clientHeight` radians per pixel.
    pub fn set_dragging(&mut self, dragging: bool) {
        self.dragging = dragging;
    }

    pub fn orbit(&mut self, dx: f32, dy: f32, viewport_height: f32) {
        let factor = std::f32::consts::TAU * ROTATE_SPEED / viewport_height.max(1.0);
        self.goal_yaw -= dx * factor;
        // OrbitControls leaves the polar range at its default 0..π, so a
        // straight top-down view is reachable; only the look-at singularity
        // (forward parallel to up) is excluded.
        const LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;
        self.goal_pitch = (self.goal_pitch + dy * factor).clamp(-LIMIT, LIMIT);
    }

    /// OrbitControls' screen-space pan: `panSpeed * (2 * distance *
    /// tan(fov/2)) / viewportHeight` world units per pixel.
    pub fn pan(&mut self, dx: f32, dy: f32, viewport_height: f32) {
        let forward = (self.goal_target - self.eye()).normalize();
        let right = forward.cross(Vec3::Z).normalize();
        let up = right.cross(forward).normalize();
        let scale = PAN_SPEED * 2.0 * self.goal_distance * (self.fov_y * 0.5).tan()
            / viewport_height.max(1.0);
        self.goal_target += right * (-dx * scale) + up * (dy * scale);
    }

    pub fn zoom(&mut self, delta: f32) {
        // OrbitControls' wheel scale: `0.95^zoomSpeed` per wheel event, one
        // step regardless of the event's magnitude.
        self.goal_distance = (self.goal_distance * 0.95f32.powf(delta.signum() * ZOOM_SPEED))
            .clamp(0.01, 1.0e6);
    }

    /// `CameraHomeResetController`: the home pose is the orbit target plus
    /// `(-220, -220, +260)`.
    pub fn home(&mut self, target: Vec3) {
        let offset = Vec3::new(-220.0, -220.0, 260.0);
        self.begin_tween(
            target,
            offset.length(),
            (offset.z / offset.length()).asin(),
            offset.y.atan2(offset.x),
            0.65,
        );
    }

    fn begin_tween(&mut self, target: Vec3, distance: f32, pitch: f32, yaw: f32, duration: f32) {
        // The goals follow the tween's end so the tween does not fight easing.
        self.goal_yaw = yaw;
        self.goal_pitch = pitch;
        self.goal_distance = distance;
        self.goal_target = target;
        self.tween = Some(Tween {
            start_yaw: self.yaw,
            start_pitch: self.pitch,
            start_distance: self.distance,
            start_target: self.target,
            end_yaw: yaw,
            end_pitch: pitch,
            end_distance: distance,
            end_target: target,
            elapsed: 0.0,
            duration,
        });
    }

    /// `CameraIntroController`'s prepare framing: fit the sphere at 1.05x the
    /// vertical FOV distance, from the source's `(-sin20°, -cos20°, 1)` view.
    pub fn focus(&mut self, center: Vec3, radius: f32) {
        let radius = radius.max(0.001);
        // A little more margin than the source's 1.05 so the model does not
        // touch the frame, and a shallower elevation than the 45° fallback.
        let distance = radius / (self.fov_y * 0.5).tan() * 1.15;
        let yaw = 20f32.to_radians();
        let dir = Vec3::new(-yaw.sin(), -yaw.cos(), 0.65).normalize();
        // `CameraIntroController`'s prepare bias: look slightly below the
        // centre so the model sits higher in the frame.
        let bias = (radius * 0.12).clamp(1.5, 10.0);
        self.begin_tween(
            center - Vec3::Z * bias,
            distance,
            dir.z.asin(),
            dir.y.atan2(dir.x),
            1.0,
        );
    }

    /// One frame of OrbitControls-style damping (dampingFactor 0.12). Returns
    /// whether the camera is still moving, so the caller can request a frame.
    pub fn step(&mut self, dt: f32) -> bool {
        if let Some(tween) = self.tween.as_mut() {
            tween.elapsed += dt;
            let t = (tween.elapsed / tween.duration).clamp(0.0, 1.0);
            // `smootherstep`.
            let eased = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
            self.yaw = tween.start_yaw + (tween.end_yaw - tween.start_yaw) * eased;
            self.pitch = tween.start_pitch + (tween.end_pitch - tween.start_pitch) * eased;
            self.distance =
                tween.start_distance + (tween.end_distance - tween.start_distance) * eased;
            self.target = tween.start_target + (tween.end_target - tween.start_target) * eased;
            if t >= 1.0 {
                self.tween = None;
            }
            return true;
        }

        // OrbitControls applies the whole pending delta, then decays it.
        // Framerate-independent exponential smoothing. While the pointer drags
        // the rate is high (the view tracks the hand); after release it drops
        // to the soft settling rate, which is the visible tail.
        let rate = if self.dragging { 45.0 } else { 14.0 };
        let alpha = 1.0 - (-rate * dt).exp();
        let approach = |value: &mut f32, goal: f32, eps: f32| {
            let delta = goal - *value;
            if delta.abs() > eps {
                *value += delta * alpha;
                true
            } else {
                *value = goal;
                false
            }
        };
        let mut moving = false;
        moving |= approach(&mut self.yaw, self.goal_yaw, 1e-5);
        moving |= approach(&mut self.pitch, self.goal_pitch, 1e-5);
        moving |= approach(&mut self.distance, self.goal_distance, 1e-3);
        let target_delta = self.goal_target - self.target;
        if target_delta.length_squared() > 1e-6 {
            self.target += target_delta * alpha;
            moving = true;
        }
        moving
    }
}
