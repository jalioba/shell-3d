mod camera;
mod mesh;
mod primitives;
mod renderer;

use std::io::stdout;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use camera::Camera;
use mesh::Mesh;
use primitives::{create_cube, create_pyramid, create_sphere, create_torus};
use renderer::{RenderMode, Renderer};

/// Terminal 3D Model Viewer
#[derive(Parser, Debug)]
#[command(author, version, about = "3D Model Viewer in the Terminal (ASCII Renderer with Z-Buffer)", long_about = None)]
struct Args {
    /// Path to 3D model file (.obj or .stl)
    #[arg(short, long)]
    file: Option<String>,

    /// Preset primitive model if no file provided: 'cube', 'pyramid', 'sphere', 'torus'
    #[arg(short, long, default_value = "cube")]
    primitive: String,
}

fn set_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));
}

fn load_mesh(args: &Args) -> Mesh {
    if let Some(ref path_str) = args.file {
        println!("Loading 3D model: {}...", path_str);
        match Mesh::from_file(path_str) {
            Ok(mesh) => return mesh,
            Err(err) => {
                eprintln!("Error loading model: {}. Falling back to default primitive.", err);
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }

    match args.primitive.to_lowercase().as_str() {
        "pyramid" => create_pyramid(),
        "sphere" => create_sphere(16, 24),
        "torus" | "donut" => create_torus(0.7, 0.3, 20, 16),
        _ => create_cube(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    set_panic_hook();

    let mut current_primitive_idx = match args.primitive.to_lowercase().as_str() {
        "pyramid" => 1,
        "sphere" => 2,
        "torus" | "donut" => 3,
        _ => 0,
    };

    let mut mesh = load_mesh(&args);

    // Initialize Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let (mut term_w, mut term_h) = terminal::size()?;
    // Reserve 1 line at the bottom for status bar
    let render_h = (term_h.saturating_sub(1) as usize).max(5);
    let render_w = (term_w as usize).max(5);

    let mut renderer = Renderer::new(render_w, render_h);
    let mut camera = Camera::new();

    let mut last_time = Instant::now();
    let mut fps = 0.0;
    let mut frame_count = 0;
    let mut fps_timer = Instant::now();

    loop {
        let now = Instant::now();
        let delta_time = now.duration_since(last_time).as_secs_f32();
        last_time = now;

        frame_count += 1;
        if fps_timer.elapsed() >= Duration::from_secs(1) {
            fps = frame_count as f32 / fps_timer.elapsed().as_secs_f32();
            frame_count = 0;
            fps_timer = Instant::now();
        }

        // Handle Keyboard Events
        let rotation_speed = 2.0 * delta_time;
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key_event) = event::read()? {
                // Exit condition
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }

                match key_event.code {
                    // Arrow Keys Rotation
                    KeyCode::Left => camera.rotate(0.0, -rotation_speed * 1.5, 0.0),
                    KeyCode::Right => camera.rotate(0.0, rotation_speed * 1.5, 0.0),
                    KeyCode::Up => camera.rotate(-rotation_speed * 1.5, 0.0, 0.0),
                    KeyCode::Down => camera.rotate(rotation_speed * 1.5, 0.0, 0.0),

                    // Z-axis Rotation (Q / E)
                    KeyCode::Char('q') | KeyCode::Char('Q') => camera.rotate(0.0, 0.0, -rotation_speed),
                    KeyCode::Char('e') | KeyCode::Char('E') => camera.rotate(0.0, 0.0, rotation_speed),

                    // Zoom Controls (+ / - or W / S)
                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('w') | KeyCode::Char('W') => {
                        camera.zoom(0.15);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('s') | KeyCode::Char('S') => {
                        camera.zoom(-0.15);
                    }

                    // Mode Toggle (M)
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        renderer.toggle_render_mode();
                    }

                    // Reset Camera (R)
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        camera.reset();
                    }

                    // Cycle Primitives (P)
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        current_primitive_idx = (current_primitive_idx + 1) % 4;
                        mesh = match current_primitive_idx {
                            0 => create_cube(),
                            1 => create_pyramid(),
                            2 => create_sphere(16, 24),
                            _ => create_torus(0.7, 0.3, 20, 16),
                        };
                    }

                    _ => {}
                }
            } else if let Event::Resize(w, h) = event::read()? {
                term_w = w;
                term_h = h;
                let new_h = (term_h.saturating_sub(1) as usize).max(5);
                let new_w = (term_w as usize).max(5);
                renderer.resize(new_w, new_h);
            }
        }

        // Render Frame
        renderer.render_mesh(&mesh, &camera);

        // Status overlay line
        let mode_str = match renderer.render_mode {
            RenderMode::ShadedASCII => "ASCII Solid",
            RenderMode::Wireframe => "Wireframe",
        };

        let status = format!(
            " Model: {} | Triangles: {} | FPS: {:.0} | Mode: {} [M] | Nav: Arrow Keys, Q/E, +/- | P: Cycle | ESC: Quit",
            mesh.name,
            mesh.triangles.len(),
            fps,
            mode_str
        );

        renderer.present(&mut stdout, &status)?;
    }

    // Teardown Terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    println!("Exited Shell-Blender successfully.");
    Ok(())
}
