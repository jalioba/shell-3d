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
}

impl Triangle {
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();
        Triangle { v0, v1, v2, normal }
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
}

impl Mesh {
    pub fn new(name: impl Into<String>, triangles: Vec<Triangle>) -> Self {
        let mut mesh = Mesh {
            triangles,
            name: name.into(),
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

    /// Load mesh from Wavefront .OBJ file
    pub fn from_obj(path: &Path) -> Result<Self, String> {
        let load_options = tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ignore_points: true,
            ignore_lines: true,
        };

        let (models, _) = tobj::load_obj(path, &load_options)
            .map_err(|e| format!("Failed to load OBJ file '{:?}': {}", path, e))?;

        if models.is_empty() {
            return Err("OBJ file contains no models".to_string());
        }

        let mut triangles = Vec::new();

        for model in models {
            let mesh = &model.mesh;
            let positions = &mesh.positions;

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

                    triangles.push(Triangle::new(v0, v1, v2));
                }
            }
        }

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Model")
            .to_string();

        Ok(Mesh::new(name, triangles))
    }

    /// Load mesh from STL file (binary or ASCII)
    pub fn from_stl(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open STL file '{:?}': {}", path, e))?;
        let mut reader = BufReader::new(file);

        let stl = stl_io::read_stl(&mut reader)
            .map_err(|e| format!("Failed to parse STL file '{:?}': {}", path, e))?;

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
                triangles.push(Triangle::new(v0, v1, v2));
            }
        }

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Model")
            .to_string();

        Ok(Mesh::new(name, triangles))
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
