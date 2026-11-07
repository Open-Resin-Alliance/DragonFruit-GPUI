//! A Z-up orbit camera, matching the app's scene convention.

use glam::{Mat4, Vec3};

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    pub fov_y: f32,
    pub orthographic: bool,
    /// OrbitControls' damping: input writes the goal, `step` chases it.
    goal_yaw: f32,
    goal_pitch: f32,
    goal_distance: f32,
    goal_target: Vec3,
}

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
        }
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

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.goal_yaw -= dx * 0.008;
        // OrbitControls leaves the polar range at its default 0..π, so a
        // straight top-down view is reachable; only the look-at singularity
        // (forward parallel to up) is excluded.
        const LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;
        self.goal_pitch = (self.goal_pitch + dy * 0.008).clamp(-LIMIT, LIMIT);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let forward = (self.goal_target - self.eye()).normalize();
        let right = forward.cross(Vec3::Z).normalize();
        let up = right.cross(forward).normalize();
        let scale = self.goal_distance * 0.0016;
        self.goal_target += right * (-dx * scale) + up * (dy * scale);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.goal_distance = (self.goal_distance * (1.0 - delta * 0.1)).clamp(0.01, 1.0e6);
    }

    /// `CameraHomeResetController`: the home pose is the orbit target plus
    /// `(-220, -220, +260)`.
    pub fn home(&mut self, target: Vec3) {
        let offset = Vec3::new(-220.0, -220.0, 260.0);
        self.goal_target = target;
        self.goal_distance = offset.length();
        self.goal_pitch = (offset.z / offset.length()).asin();
        self.goal_yaw = offset.y.atan2(offset.x);
    }

    /// `CameraIntroController`'s prepare framing: fit the sphere at 1.05x the
    /// vertical FOV distance, from the source's `(-sin20°, -cos20°, 1)` view.
    pub fn focus(&mut self, center: Vec3, radius: f32) {
        let radius = radius.max(0.001);
        let v_fov = self.fov_y;
        let distance = radius / (v_fov * 0.5).tan() * 1.05;
        let yaw = 20f32.to_radians();
        let dir = Vec3::new(-yaw.sin(), -yaw.cos(), 1.0).normalize();
        self.goal_target = center;
        self.goal_distance = distance;
        self.goal_pitch = dir.z.asin();
        self.goal_yaw = dir.y.atan2(dir.x);
    }

    /// One frame of OrbitControls-style damping (dampingFactor 0.12). Returns
    /// whether the camera is still moving, so the caller can request a frame.
    pub fn step(&mut self) -> bool {
        const DAMPING: f32 = 0.12;
        let mut moving = false;
        let mut approach = |value: &mut f32, goal: f32| {
            let delta = goal - *value;
            if delta.abs() > 1e-4 {
                *value += delta * DAMPING;
                moving = true;
            } else {
                *value = goal;
            }
        };
        approach(&mut self.yaw, self.goal_yaw);
        approach(&mut self.pitch, self.goal_pitch);
        approach(&mut self.distance, self.goal_distance);
        let target_delta = self.goal_target - self.target;
        if target_delta.length_squared() > 1e-6 {
            self.target += target_delta * DAMPING;
            moving = true;
        } else {
            self.target = self.goal_target;
        }
        moving
    }
}
