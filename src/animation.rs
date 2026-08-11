use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrameData {
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub distance: f32,
    pub render_mode: u8, // 0: ASCII, 1: Block, 2: Wireframe
    pub time_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimationRecording {
    pub model_file: Option<String>,
    pub primitive_name: String,
    pub frames: Vec<FrameData>,
}

impl AnimationRecording {
    pub fn new(model_file: Option<String>, primitive_name: String) -> Self {
        Self {
            model_file,
            primitive_name,
            frames: Vec::new(),
        }
    }

    pub fn save_to_file(&self, path_str: &str) -> Result<(), String> {
        let file = File::create(path_str)
            .map_err(|e| format!("Failed to create animation file '{}': {}", path_str, e))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| format!("Failed to serialize animation JSON: {}", e))?;
        Ok(())
    }

    pub fn load_from_file(path_str: &str) -> Result<Self, String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("Animation file '{}' does not exist", path_str));
        }
        let file = File::open(path)
            .map_err(|e| format!("Failed to open animation file '{}': {}", path_str, e))?;
        let reader = BufReader::new(file);
        let recording: Self = serde_json::from_reader(reader)
            .map_err(|e| format!("Failed to parse animation JSON from '{}': {}", path_str, e))?;
        Ok(recording)
    }
}
