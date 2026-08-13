use glam::Vec3;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub normal: Vec3,
    pub color: Option<(u8, u8, u8)>, // Face/material color if present
}

impl Triangle {
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        Self::new_with_color(v0, v1, v2, None)
    }

    pub fn new_with_color(v0: Vec3, v1: Vec3, v2: Vec3, color: Option<(u8, u8, u8)>) -> Self {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();
        Triangle { v0, v1, v2, normal, color }
    }

    pub fn recalculate_normal(&mut self) {
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        self.normal = edge1.cross(edge2).normalize_or_zero();
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub name: String,
    pub base_color: (u8, u8, u8),
}

#[derive(Clone, Copy, Debug)]
pub struct ModelTheme {
    pub primary: (u8, u8, u8),
    pub secondary: (u8, u8, u8),
}

pub fn get_theme_for_file_name(filename: &str) -> ModelTheme {
    let name = filename.to_lowercase();
    if name.contains("ferrari") || name.contains("car") {
        ModelTheme {
            primary: (225, 45, 45),     // Sport Ferrari Red
            secondary: (50, 55, 65),    // Carbon Steel / Black Trim
        }
    } else if name.contains("arch") || name.contains("pillar") || name.contains("stone") {
        ModelTheme {
            primary: (215, 200, 180),   // Warm Sandstone / Marble
            secondary: (150, 140, 125), // Ancient Stone Shadow
        }
    } else if name.contains("sofa") || name.contains("couch") || name.contains("chair") || name.contains("room") {
        ModelTheme {
            primary: (180, 105, 60),    // Terracotta Leather
            secondary: (75, 50, 40),     // Dark Espresso Wood
        }
    } else if name.contains("skull") || name.contains("head") || name.contains("girl") || name.contains("human") || name.contains("wolf") {
        ModelTheme {
            primary: (225, 215, 195),   // Ivory Bone / Smooth Skin
            secondary: (140, 128, 112), // Bone Shadow
        }
    } else if name.contains("sword") || name.contains("blade") {
        ModelTheme {
            primary: (200, 215, 230),   // Polished Steel Blade
            secondary: (225, 175, 45),  // Gold Hilt & Guard
        }
    } else if name.contains("laptop") || name.contains("phone") {
        ModelTheme {
            primary: (175, 185, 200),   // Space Gray Body
            secondary: (0, 210, 255),   // Cyan Screen
        }
    } else {
        // Fallback: 4 elegant 2-color duotone themes based on filename hash
        let hash = name.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        match hash % 4 {
            0 => ModelTheme { primary: (0, 210, 255), secondary: (30, 80, 160) },   // Cyan / Deep Blue
            1 => ModelTheme { primary: (255, 180, 40), secondary: (160, 90, 20) },   // Amber / Bronze
            2 => ModelTheme { primary: (50, 220, 120), secondary: (20, 100, 50) },   // Emerald / Dark Moss
            _ => ModelTheme { primary: (180, 100, 255), secondary: (90, 40, 150) },  // Amethyst / Dark Violet
        }
    }
}

/// Detects semantic sub-object colors by sub-mesh name (e.g. scarf, hair, eyes, shirt, boots, blade, wheels)
fn get_color_for_subobject_name(model_name: &str) -> Option<(u8, u8, u8)> {
    let name = model_name.to_lowercase();
    if name.contains("scarf") {
        return Some((225, 45, 45)); // Crimson Red Scarf
    }
    if name.contains("hair") || name.contains("ceja") {
        return Some((140, 80, 45)); // Warm Chestnut Hair / Brows
    }
    if name.contains("eye") {
        return Some((0, 210, 255)); // Sapphire Blue Eyes
    }
    if name.contains("top") || name.contains("shirt") || name.contains("cloth") {
        return Some((240, 240, 240)); // Soft White / Cream Shirt
    }
    if name.contains("bot") || name.contains("pant") || name.contains("jean") || name.contains("shoe") || name.contains("boot") {
        return Some((45, 55, 75)); // Dark Denim / Navy Boots
    }
    if name.contains("head") || name.contains("body") || name.contains("face") || name.contains("skin") || name.contains("boca") {
        return Some((235, 215, 195)); // Natural Peach Skin Tone
    }
    if name.contains("blade") {
        return Some((200, 215, 230)); // Polished Steel Blade
    }
    if name.contains("hilt") || name.contains("guard") || name.contains("handle") {
        return Some((225, 175, 45)); // Gold Guard / Hilt
    }
    if name.contains("wheel") || name.contains("tire") {
        return Some((35, 35, 40)); // Rubber Black Tire
    }
    if name.contains("window") || name.contains("glass") || name.contains("screen") {
        return Some((0, 210, 255)); // Glass Cyan
    }
    None
}

impl Mesh {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, triangles: Vec<Triangle>) -> Self {
        Self::new_with_color(name, triangles, (0, 210, 255))
    }

    pub fn new_with_color(name: impl Into<String>, triangles: Vec<Triangle>, color: (u8, u8, u8)) -> Self {
        let mut mesh = Mesh {
            triangles,
            name: name.into(),
            base_color: color,
        };
        mesh.normalize_and_center();
        mesh
    }

    /// Center the model around the origin and scale it into [-1, 1] range.
    pub fn normalize_and_center(&mut self) {
        if self.triangles.is_empty() {
            return;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for tri in &self.triangles {
            for v in &[tri.v0, tri.v1, tri.v2] {
                min = min.min(*v);
                max = max.max(*v);
            }
        }

        let center = (min + max) * 0.5;
        let extent = (max - min).max_element();
        let scale = if extent > 1e-6 { 2.0 / extent } else { 1.0 };

        for tri in &mut self.triangles {
            tri.v0 = (tri.v0 - center) * scale;
            tri.v1 = (tri.v1 - center) * scale;
            tri.v2 = (tri.v2 - center) * scale;
            tri.recalculate_normal();
        }
    }

    /// Load mesh from Wavefront .OBJ file (with .mtl materials, semantic sub-object recognition & duotone themes)
    pub fn from_obj(path: &Path) -> Result<Self, String> {
        let load_options = tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ignore_points: true,
            ignore_lines: true,
        };

        let (models, materials) = tobj::load_obj(path, &load_options)
            .map_err(|e| format!("Failed to load OBJ file '{:?}': {}", path, e))?;

        if models.is_empty() {
            return Err("OBJ file contains no models".to_string());
        }

        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Model");

        let theme = get_theme_for_file_name(filename);
        let parsed_materials = materials.ok();
        let is_multi_group = models.len() > 1;
        let mut triangles = Vec::new();

        for (model_idx, model) in models.into_iter().enumerate() {
            let mesh = &model.mesh;
            let positions = &mesh.positions;

            // Extract material diffuse color if specified in .mtl file
            let mat_color = mesh.material_id.and_then(|mat_id| {
                if let Some(ref mats) = parsed_materials {
                    if mat_id < mats.len() {
                        if let Some(diffuse) = mats[mat_id].diffuse {
                            let r = (diffuse[0] * 255.0).clamp(0.0, 255.0) as u8;
                            let g = (diffuse[1] * 255.0).clamp(0.0, 255.0) as u8;
                            let b = (diffuse[2] * 255.0).clamp(0.0, 255.0) as u8;
                            return Some((r, g, b));
                        }
                    }
                }
                None
            });

            // Semantic sub-object name recognition (e.g. scarf_Cube -> Crimson Red)
            let sub_color = get_color_for_subobject_name(&model.name);

            // Group color priority: .mtl material > sub-object name > theme duotone
            let group_color = mat_color.or(sub_color).or_else(|| {
                if is_multi_group {
                    if model_idx % 4 == 0 {
                        Some(theme.secondary)
                    } else {
                        Some(theme.primary)
                    }
                } else {
                    None
                }
            });

            for i in (0..mesh.indices.len()).step_by(3) {
                if i + 2 >= mesh.indices.len() {
                    break;
                }
                let idx0 = mesh.indices[i] as usize * 3;
                let idx1 = mesh.indices[i + 1] as usize * 3;
                let idx2 = mesh.indices[i + 2] as usize * 3;

                if idx0 + 2 < positions.len() && idx1 + 2 < positions.len() && idx2 + 2 < positions.len() {
                    let v0 = Vec3::new(positions[idx0], positions[idx0 + 1], positions[idx0 + 2]);
                    let v1 = Vec3::new(positions[idx1], positions[idx1 + 1], positions[idx1 + 2]);
                    let v2 = Vec3::new(positions[idx2], positions[idx2 + 1], positions[idx2 + 2]);

                    // Smooth duotone blend for single mesh without material
                    let tri_color = group_color.or_else(|| {
                        let edge1 = v1 - v0;
                        let edge2 = v2 - v0;
                        let n = edge1.cross(edge2).normalize_or_zero();
                        let factor = (n.y * 0.5 + 0.5).clamp(0.0, 1.0);
                        let r = (theme.primary.0 as f32 * factor + theme.secondary.0 as f32 * (1.0 - factor)) as u8;
                        let g = (theme.primary.1 as f32 * factor + theme.secondary.1 as f32 * (1.0 - factor)) as u8;
                        let b = (theme.primary.2 as f32 * factor + theme.secondary.2 as f32 * (1.0 - factor)) as u8;
                        Some((r, g, b))
                    });

                    triangles.push(Triangle::new_with_color(v0, v1, v2, tri_color));
                }
            }
        }

        Ok(Mesh::new_with_color(filename, triangles, theme.primary))
    }

    /// Load mesh from STL file (binary or ASCII)
    pub fn from_stl(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open STL file '{:?}': {}", path, e))?;
        let mut reader = BufReader::new(file);

        let stl = stl_io::read_stl(&mut reader)
            .map_err(|e| format!("Failed to parse STL file '{:?}': {}", path, e))?;

        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Model");
        let theme = get_theme_for_file_name(filename);

        let vertices: Vec<Vec3> = stl.vertices
            .iter()
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect();

        let mut triangles = Vec::new();
        for face in stl.faces {
            let idx0 = face.vertices[0];
            let idx1 = face.vertices[1];
            let idx2 = face.vertices[2];

            if idx0 < vertices.len() && idx1 < vertices.len() && idx2 < vertices.len() {
                let v0 = vertices[idx0];
                let v1 = vertices[idx1];
                let v2 = vertices[idx2];
                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                let n = edge1.cross(edge2).normalize_or_zero();
                let factor = (n.y * 0.5 + 0.5).clamp(0.0, 1.0);
                let r = (theme.primary.0 as f32 * factor + theme.secondary.0 as f32 * (1.0 - factor)) as u8;
                let g = (theme.primary.1 as f32 * factor + theme.secondary.1 as f32 * (1.0 - factor)) as u8;
                let b = (theme.primary.2 as f32 * factor + theme.secondary.2 as f32 * (1.0 - factor)) as u8;

                triangles.push(Triangle::new_with_color(v0, v1, v2, Some((r, g, b))));
            }
        }

        Ok(Mesh::new_with_color(filename, triangles, theme.primary))
    }

    /// Load mesh automatically detecting format from file extension
    pub fn from_file(path_str: &str) -> Result<Self, String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("File does not exist: {}", path_str));
        }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "obj" => Self::from_obj(path),
            "stl" => Self::from_stl(path),
            _ => Err(format!("Unsupported file format '.{}'. Supported formats: .obj, .stl", ext)),
        }
    }
}
