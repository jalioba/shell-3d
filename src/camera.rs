use glam::{Mat4, Vec3, Vec4};

pub struct Camera {
    pub rotation_x: f32, // Pitch
    pub rotation_y: f32, // Yaw
    pub rotation_z: f32, // Roll
    pub distance: f32,   // Camera distance
    pub fov_deg: f32,    // Field of view in degrees
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            rotation_x: 0.3,
            rotation_y: 0.5,
            rotation_z: 0.0,
            distance: 2.5,
            fov_deg: 60.0,
        }
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn rotate(&mut self, dx: f32, dy: f32, dz: f32) {
        self.rotation_x += dx;
        self.rotation_y += dy;
        self.rotation_z += dz;
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta).clamp(0.8, 10.0);
    }

    /// Computes combined Model-View-Projection (MVP) matrix.
    /// Incorporates character aspect ratio correction (terminal chars are ~1:2 AR).
    pub fn get_mvp_matrix(&self, screen_width: u16, screen_height: u16) -> (Mat4, Mat4) {
        // Model Matrix: Rotation around X, Y, Z
        let model = Mat4::from_rotation_x(self.rotation_x)
            * Mat4::from_rotation_y(self.rotation_y)
            * Mat4::from_rotation_z(self.rotation_z);

        // View Matrix: Camera pulled back along Z
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 0.0, self.distance), // Eye position
            Vec3::ZERO,                          // Target
            Vec3::Y,                             // Up vector
        );

        // Aspect ratio with character height correction factor (~2.0)
        let char_aspect_correction = 2.0;
        let aspect = (screen_width as f32) / (screen_height as f32 * char_aspect_correction);

        // Perspective Projection
        let proj = Mat4::perspective_rh(
            self.fov_deg.to_radians(),
            aspect.max(0.1),
            0.1,
            100.0,
        );

        let mvp = proj * view * model;
        (mvp, model)
    }

    /// Maps normalized clip coordinate (-1..1) to screen pixel space (0..width, 0..height)
    pub fn project_to_screen(v_clip: Vec4, width: usize, height: usize) -> Option<Vec3> {
        // Perspective divide
        if v_clip.w <= 0.0 {
            return None;
        }

        let ndc = v_clip.truncate() / v_clip.w;

        // Clip bounds check
        if ndc.x < -1.5 || ndc.x > 1.5 || ndc.y < -1.5 || ndc.y > 1.5 || ndc.z < -1.0 || ndc.z > 1.0 {
            return None;
        }

        let screen_x = (ndc.x + 1.0) * 0.5 * (width as f32);
        // Invert Y for screen coordinates (0 is top)
        let screen_y = (1.0 - ndc.y) * 0.5 * (height as f32);
        let depth = ndc.z;

        Some(Vec3::new(screen_x, screen_y, depth))
    }
}
