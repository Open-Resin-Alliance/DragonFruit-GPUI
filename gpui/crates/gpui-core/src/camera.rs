//! A Z-up orbit camera, matching the app's scene convention.

use glam::{Mat4, Vec3};

pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    pub fov_y: f32,
    pub orthographic: bool,
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
        self.yaw -= dx * 0.008;
        // OrbitControls leaves the polar range at its default 0..π, so a
        // straight top-down view is reachable; only the look-at singularity
        // (forward parallel to up) is excluded.
        const LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;
        self.pitch = (self.pitch + dy * 0.008).clamp(-LIMIT, LIMIT);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let forward = (self.target - self.eye()).normalize();
        let right = forward.cross(Vec3::Z).normalize();
        let up = right.cross(forward).normalize();
        let scale = self.distance * 0.0016;
        self.target += right * (-dx * scale) + up * (dy * scale);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta * 0.1)).clamp(0.01, 1.0e6);
    }
}
