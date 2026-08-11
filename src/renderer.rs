use glam::{Vec3, Vec4};
use std::io::{self, Write};
use crossterm::{
    cursor,
    queue,
    style::{self, Color, ResetColor, SetForegroundColor},
    terminal,
};

use crate::camera::Camera;
use crate::mesh::Mesh;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderMode {
    ShadedASCII,
    ShadedBlock,
    Wireframe,
}

pub struct Renderer {
    width: usize,
    height: usize,
    char_buffer: Vec<char>,
    z_buffer: Vec<f32>,
    pub render_mode: RenderMode,
    pub show_hud: bool,
}

// Grayscale ASCII ramp from darkest to brightest
const ASCII_RAMP: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

// Unicode Block character ramp from darkest to brightest (░ -> ▒ -> ▓ -> █)
const BLOCK_RAMP: &[char] = &[' ', '░', '▒', '▓', '█'];

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            char_buffer: vec![' '; size],
            z_buffer: vec![f32::INFINITY; size],
            render_mode: RenderMode::ShadedASCII,
            show_hud: true,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        let size = width * height;
        self.char_buffer = vec![' '; size];
        self.z_buffer = vec![f32::INFINITY; size];
    }

    pub fn clear(&mut self) {
        self.char_buffer.fill(' ');
        self.z_buffer.fill(f32::INFINITY);
    }

    pub fn toggle_render_mode(&mut self) {
        self.render_mode = match self.render_mode {
            RenderMode::ShadedASCII => RenderMode::ShadedBlock,
            RenderMode::ShadedBlock => RenderMode::Wireframe,
            RenderMode::Wireframe => RenderMode::ShadedASCII,
        };
    }

    pub fn toggle_hud(&mut self) {
        self.show_hud = !self.show_hud;
    }

    /// Main render method for a mesh given camera state
    pub fn render_mesh(&mut self, mesh: &Mesh, camera: &Camera) {
        self.clear();

        if self.width == 0 || self.height == 0 {
            return;
        }

        let (mvp, model) = camera.get_mvp_matrix(self.width as u16, self.height as u16);
        // Camera-facing directional light source
        let light_dir = Vec3::new(0.2, 0.4, 1.0).normalize();

        for triangle in &mesh.triangles {
            // Transform normal to world space
            let world_normal = (model.transform_vector3(triangle.normal)).normalize_or_zero();

            // Transform vertices to clip space
            let v0_clip = mvp * Vec4::from((triangle.v0, 1.0));
            let v1_clip = mvp * Vec4::from((triangle.v1, 1.0));
            let v2_clip = mvp * Vec4::from((triangle.v2, 1.0));

            // Project to screen space
            let p0 = Camera::project_to_screen(v0_clip, self.width, self.height);
            let p1 = Camera::project_to_screen(v1_clip, self.width, self.height);
            let p2 = Camera::project_to_screen(v2_clip, self.width, self.height);

            if let (Some(p0), Some(p1), Some(p2)) = (p0, p1, p2) {
                // Backface culling check in 2D screen space (only for solid shaded modes)
                let area = (p1.x - p0.x) * (p2.y - p0.y) - (p1.y - p0.y) * (p2.x - p0.x);
                if area <= 0.0 && self.render_mode != RenderMode::Wireframe {
                    continue; // Skip back-facing triangles in solid modes
                }

                // Two-sided lighting calculation to ensure all faces light up properly
                let dot_light = world_normal.dot(light_dir).abs();
                let intensity = (dot_light * 0.85 + 0.15).clamp(0.0, 1.0);

                match self.render_mode {
                    RenderMode::ShadedASCII => {
                        self.draw_triangle_shaded(p0, p1, p2, intensity, ASCII_RAMP);
                    }
                    RenderMode::ShadedBlock => {
                        self.draw_triangle_shaded(p0, p1, p2, intensity, BLOCK_RAMP);
                    }
                    RenderMode::Wireframe => {
                        self.draw_line_3d(p0, p1, '*');
                        self.draw_line_3d(p1, p2, '*');
                        self.draw_line_3d(p2, p0, '*');
                    }
                }
            }
        }
    }

    /// Draw shaded triangle using Barycentric Coordinates and character ramp
    fn draw_triangle_shaded(&mut self, p0: Vec3, p1: Vec3, p2: Vec3, intensity: f32, ramp: &[char]) {
        let min_x = (p0.x.min(p1.x).min(p2.x).floor() as i32).clamp(0, self.width as i32 - 1) as usize;
        let max_x = (p0.x.max(p1.x).max(p2.x).ceil() as i32).clamp(0, self.width as i32 - 1) as usize;
        let min_y = (p0.y.min(p1.y).min(p2.y).floor() as i32).clamp(0, self.height as i32 - 1) as usize;
        let max_y = (p0.y.max(p1.y).max(p2.y).ceil() as i32).clamp(0, self.height as i32 - 1) as usize;

        let denom = (p1.y - p2.y) * (p0.x - p2.x) + (p2.x - p1.x) * (p0.y - p2.y);
        if denom.abs() < 1e-6 {
            return;
        }

        // Map intensity evenly across all ramp indices, ensuring top index (e.g. '█' or '@') is reached cleanly
        let max_idx = (ramp.len() - 1) as f32;
        let ramp_idx = ((intensity * (max_idx + 0.8)).floor() as usize).clamp(0, ramp.len() - 1);
        let ch = ramp[ramp_idx];

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                let w0 = ((p1.y - p2.y) * (px - p2.x) + (p2.x - p1.x) * (py - p2.y)) / denom;
                let w1 = ((p2.y - p0.y) * (px - p2.x) + (p0.x - p2.x) * (py - p2.y)) / denom;
                let w2 = 1.0 - w0 - w1;

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let depth = w0 * p0.z + w1 * p1.z + w2 * p2.z;
                    let idx = x + y * self.width;

                    if depth < self.z_buffer[idx] {
                        self.z_buffer[idx] = depth;
                        self.char_buffer[idx] = ch;
                    }
                }
            }
        }
    }

    /// Bresenham's 3D Line rasterizer for wireframe mode
    fn draw_line_3d(&mut self, p0: Vec3, p1: Vec3, ch: char) {
        let mut x0 = p0.x as i32;
        let mut y0 = p0.y as i32;
        let x1 = p1.x as i32;
        let y1 = p1.y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let total_dist = p0.distance(p1).max(1e-5);

        loop {
            if x0 >= 0 && x0 < self.width as i32 && y0 >= 0 && y0 < self.height as i32 {
                let ux = x0 as usize;
                let uy = y0 as usize;
                let curr_pos = Vec3::new(x0 as f32, y0 as f32, 0.0);
                let t = (p0.distance(curr_pos) / total_dist).clamp(0.0, 1.0);
                let depth = p0.z * (1.0 - t) + p1.z * t;
                let idx = ux + uy * self.width;

                if depth < self.z_buffer[idx] {
                    self.z_buffer[idx] = depth;
                    self.char_buffer[idx] = ch;
                }
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Flush the char buffer to terminal stdout cleanly
    pub fn present(&self, stdout: &mut io::Stdout, status_line: &str) -> io::Result<()> {
        queue!(stdout, cursor::MoveTo(0, 0))?;

        let mut output = String::with_capacity((self.width + 1) * self.height + 200);

        for y in 0..self.height {
            let start = y * self.width;
            let end = start + self.width;
            let row = &self.char_buffer[start..end];
            output.extend(row);
            output.push('\n');
        }

        // Output rendered frame
        write!(stdout, "{}", output)?;

        // Output overlay status bar if HUD is enabled
        if self.show_hud {
            queue!(
                stdout,
                cursor::MoveTo(0, self.height as u16),
                terminal::Clear(terminal::ClearType::CurrentLine),
                SetForegroundColor(Color::Cyan),
                style::Print(status_line),
                ResetColor
            )?;
        } else {
            queue!(
                stdout,
                cursor::MoveTo(0, self.height as u16),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
        }

        stdout.flush()
    }
}
