mod animation;
mod camera;
mod mesh;
mod primitives;
mod renderer;

use std::io::stdout;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use animation::{AnimationRecording, FrameData};
use camera::Camera;
use mesh::Mesh;
use primitives::{create_cube, create_pyramid, create_sphere, create_torus};
use renderer::{ColorMode, RenderMode, Renderer};

/// Terminal 3D Model Viewer (Shell-3D)
#[derive(Parser, Debug)]
#[command(author, version, about = "3D Model Viewer in the Terminal (ASCII Renderer with Z-Buffer)", long_about = None)]
struct Args {
    /// Path to 3D model file (.obj or .stl)
    #[arg(short, long)]
    file: Option<String>,

    /// Preset primitive model if no file provided: 'cube', 'pyramid', 'sphere', 'torus'
    #[arg(short, long, default_value = "cube")]
    primitive: String,

    /// Path to animation JSON file to replay in a smooth loop
    #[arg(long)]
    play: Option<String>,

    /// Output file path for saving recorded camera animation
    #[arg(long, default_value = "recording.json")]
    record_out: String,
}

#[cfg(target_os = "windows")]
fn enable_windows_utf8() {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
    }
    unsafe {
        SetConsoleOutputCP(65001);
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_windows_utf8() {}

fn set_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));
}

fn load_mesh_by_file_or_primitive(file: Option<&String>, primitive: &str, quiet: bool) -> Mesh {
    if let Some(path_str) = file {
        if !quiet {
            println!("Loading 3D model: {}...", path_str);
        }
        match Mesh::from_file(path_str) {
            Ok(mesh) => return mesh,
            Err(err) => {
                if !quiet {
                    eprintln!("Error loading model: {}. Falling back to default primitive.", err);
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }

    match primitive.to_lowercase().as_str() {
        "pyramid" => create_pyramid(),
        "sphere" => create_sphere(16, 24),
        "torus" | "donut" => create_torus(0.7, 0.3, 20, 16),
        _ => create_cube(),
    }
}

fn run_replay_mode(animation_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let recording = AnimationRecording::load_from_file(animation_file)?;

    if recording.frames.is_empty() {
        return Err("Animation file contains no recorded frames.".into());
    }

    // Load mesh silently for immediate launch
    let mesh = load_mesh_by_file_or_primitive(recording.model_file.as_ref(), &recording.primitive_name, true);

    // Immediately enter alternate screen with zero console output delay
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let (term_w, term_h) = terminal::size()?;
    let mut renderer = Renderer::new(term_w as usize, term_h as usize);
    renderer.show_hud = false; // Pure full-screen playback without HUD

    let mut camera = Camera::new();
    let total_duration_ms = recording.frames.last().unwrap().time_ms.max(1);
    let playback_start = Instant::now();

    loop {
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind != KeyEventKind::Release {
                    if key_event.code == KeyCode::Esc
                        || (key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        break;
                    }
                }
            } else if let Event::Resize(w, h) = event::read()? {
                renderer.resize(w as usize, h as usize);
            }
        }

        // Calculate loop timestamp
        let elapsed_ms = (playback_start.elapsed().as_millis() as u64) % total_duration_ms;

        // Smooth LERP (Linear Interpolation) between keyframe pairs to eliminate shaking/jitter
        let frames = &recording.frames;
        let (f0, f1, t) = if frames.len() == 1 {
            (&frames[0], &frames[0], 0.0f32)
        } else {
            let mut f0_idx = 0;
            for (idx, f) in frames.iter().enumerate() {
                if f.time_ms <= elapsed_ms {
                    f0_idx = idx;
                } else {
                    break;
                }
            }
            let f1_idx = (f0_idx + 1).min(frames.len() - 1);
            let f0 = &frames[f0_idx];
            let f1 = &frames[f1_idx];
            let duration = (f1.time_ms.saturating_sub(f0.time_ms)) as f32;
            let t = if duration > 0.0 {
                ((elapsed_ms.saturating_sub(f0.time_ms)) as f32 / duration).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (f0, f1, t)
        };

        // Sub-frame smooth camera position interpolation
        camera.rotation_x = f0.rotation_x + t * (f1.rotation_x - f0.rotation_x);
        camera.rotation_y = f0.rotation_y + t * (f1.rotation_y - f0.rotation_y);
        camera.rotation_z = f0.rotation_z + t * (f1.rotation_z - f0.rotation_z);
        camera.distance = f0.distance + t * (f1.distance - f0.distance);

        renderer.color_mode = ColorMode::from_u8(f0.color_mode_u8);
        renderer.render_mode = if t < 0.5 {
            match f0.render_mode {
                0 => RenderMode::ShadedASCII,
                1 => RenderMode::ShadedBlock,
                _ => RenderMode::Wireframe,
            }
        } else {
            match f1.render_mode {
                0 => RenderMode::ShadedASCII,
                1 => RenderMode::ShadedBlock,
                _ => RenderMode::Wireframe,
            }
        };

        renderer.render_mesh(&mesh, &camera);
        renderer.present(&mut stdout, "")?;
    }

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;
    Ok(())
}

enum RecordState {
    Idle,
    Countdown(Instant),
    Recording(Instant, AnimationRecording),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_windows_utf8();
    let args = Args::parse();
    set_panic_hook();

    // Instant replay launch if --play flag is passed
    if let Some(ref play_file) = args.play {
        return run_replay_mode(play_file);
    }

    let mut current_primitive_idx = match args.primitive.to_lowercase().as_str() {
        "pyramid" => 1,
        "sphere" => 2,
        "torus" | "donut" => 3,
        _ => 0,
    };

    let mut mesh = load_mesh_by_file_or_primitive(args.file.as_ref(), &args.primitive, false);

    // Initialize Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let (mut term_w, mut term_h) = terminal::size()?;
    let render_h = (term_h.saturating_sub(1) as usize).max(5);
    let render_w = (term_w as usize).max(5);

    let mut renderer = Renderer::new(render_w, render_h);
    let mut camera = Camera::new();

    let mut last_time = Instant::now();
    let mut fps = 0.0;
    let mut frame_count = 0;
    let mut fps_timer = Instant::now();

    // Debounce timers
    let mut last_mode_switch = Instant::now() - Duration::from_secs(1);
    let mut last_primitive_switch = Instant::now() - Duration::from_secs(1);
    let mut last_hud_switch = Instant::now() - Duration::from_secs(1);
    let mut last_color_switch = Instant::now() - Duration::from_secs(1);
    let mut last_record_switch = Instant::now() - Duration::from_secs(1);
    const DEBOUNCE_COOLDOWN: Duration = Duration::from_millis(300);

    // Recording State Machine
    let mut record_state = RecordState::Idle;
    let mut notification_msg: Option<(String, Instant)> = None;

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
                if key_event.kind != KeyEventKind::Release {
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
                            if now.duration_since(last_mode_switch) >= DEBOUNCE_COOLDOWN {
                                renderer.toggle_render_mode();
                                last_mode_switch = now;
                            }
                        }

                        // HUD Toggle (H)
                        KeyCode::Char('h') | KeyCode::Char('H') => {
                            if now.duration_since(last_hud_switch) >= DEBOUNCE_COOLDOWN {
                                renderer.toggle_hud();
                                last_hud_switch = now;
                            }
                        }

                        // Color Toggle / Cycle (C)
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if now.duration_since(last_color_switch) >= DEBOUNCE_COOLDOWN {
                                renderer.toggle_color();
                                last_color_switch = now;
                            }
                        }

                        // Record Toggle (K): Starts 3-second countdown or stops & saves recording
                        KeyCode::Char('k') | KeyCode::Char('K') => {
                            if now.duration_since(last_record_switch) >= DEBOUNCE_COOLDOWN {
                                match record_state {
                                    RecordState::Idle => {
                                        // Start 3-second countdown
                                        record_state = RecordState::Countdown(Instant::now());
                                    }
                                    RecordState::Countdown(_) => {
                                        // Cancel countdown
                                        record_state = RecordState::Idle;
                                        notification_msg = Some(("Recording cancelled.".to_string(), Instant::now()));
                                    }
                                    RecordState::Recording(_, ref active_rec) => {
                                        // Stop & Save Recording
                                        match active_rec.save_to_file(&args.record_out) {
                                            Ok(_) => {
                                                notification_msg = Some((
                                                    format!("Saved {} frames to '{}'!", active_rec.frames.len(), args.record_out),
                                                    Instant::now(),
                                                ));
                                            }
                                            Err(e) => {
                                                notification_msg = Some((format!("Save Error: {}", e), Instant::now()));
                                            }
                                        }
                                        record_state = RecordState::Idle;
                                    }
                                }
                                last_record_switch = now;
                            }
                        }

                        // Discard / Cancel Recording (X)
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            if matches!(record_state, RecordState::Countdown(_) | RecordState::Recording(_, _)) {
                                record_state = RecordState::Idle;
                                notification_msg = Some(("Recording DISCARDED.".to_string(), Instant::now()));
                            }
                        }

                        // Reset Camera (R)
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            camera.reset();
                        }

                        // Cycle Primitives (P)
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            if now.duration_since(last_primitive_switch) >= DEBOUNCE_COOLDOWN {
                                current_primitive_idx = (current_primitive_idx + 1) % 4;
                                mesh = match current_primitive_idx {
                                    0 => create_cube(),
                                    1 => create_pyramid(),
                                    2 => create_sphere(16, 24),
                                    _ => create_torus(0.7, 0.3, 20, 16),
                                };
                                last_primitive_switch = now;
                            }
                        }

                        _ => {}
                    }
                }
            } else if let Event::Resize(w, h) = event::read()? {
                term_w = w;
                term_h = h;
                let new_h = (term_h.saturating_sub(1) as usize).max(5);
                let new_w = (term_w as usize).max(5);
                renderer.resize(new_w, new_h);
            }
        }

        // Process Recording State Machine & Keyframes
        match record_state {
            RecordState::Countdown(start_time) => {
                let elapsed_sec = start_time.elapsed().as_secs_f32();
                if elapsed_sec >= 3.0 {
                    // Transition to active recording after 3s countdown
                    record_state = RecordState::Recording(
                        Instant::now(),
                        AnimationRecording::new(args.file.clone(), args.primitive.clone()),
                    );
                    notification_msg = Some(("🔴 RECORDING STARTED!".to_string(), Instant::now()));
                }
            }
            RecordState::Recording(start_time, ref mut active_rec) => {
                let time_ms = start_time.elapsed().as_millis() as u64;
                let mode_u8 = match renderer.render_mode {
                    RenderMode::ShadedASCII => 0,
                    RenderMode::ShadedBlock => 1,
                    RenderMode::Wireframe => 2,
                };
                active_rec.frames.push(FrameData {
                    rotation_x: camera.rotation_x,
                    rotation_y: camera.rotation_y,
                    rotation_z: camera.rotation_z,
                    distance: camera.distance,
                    render_mode: mode_u8,
                    color_mode_u8: renderer.color_mode.to_u8(),
                    time_ms,
                });
            }
            RecordState::Idle => {}
        }

        // Render Frame
        renderer.render_mesh(&mesh, &camera);

        // Status overlay line
        let mode_str = match renderer.render_mode {
            RenderMode::ShadedASCII => "ASCII Solid",
            RenderMode::ShadedBlock => "Unicode Block (█░▒)",
            RenderMode::Wireframe => "Wireframe",
        };

        let color_str = renderer.color_mode.display_name();

        let status = match record_state {
            RecordState::Countdown(start_time) => {
                let remaining_secs = (3.0 - start_time.elapsed().as_secs_f32()).ceil() as u32;
                format!(
                    " ⏱️ GET READY TO RECORD... STARTING IN {} SECONDS | K: Cancel",
                    remaining_secs.max(1)
                )
            }
            RecordState::Recording(_, ref active_rec) => {
                format!(
                    " 🔴 REC [Frames: {}] | K: Stop & Save ('{}') | X: Cancel",
                    active_rec.frames.len(),
                    args.record_out
                )
            }
            RecordState::Idle => {
                if let Some((ref msg, time)) = notification_msg {
                    if time.elapsed() < Duration::from_secs(3) {
                        format!(" NOTICE: {} | Mode: {} [M] | Color: {} [C] | ESC: Quit", msg, mode_str, color_str)
                    } else {
                        notification_msg = None;
                        format!(
                            " Model: {} | Triangles: {} | FPS: {:.0} | Mode: {} [M] | Color: {} [C] | K: Record | ESC: Quit",
                            mesh.name, mesh.triangles.len(), fps, mode_str, color_str
                        )
                    }
                } else {
                    format!(
                        " Model: {} | Triangles: {} | FPS: {:.0} | Mode: {} [M] | Color: {} [C] | K: Record | ESC: Quit",
                        mesh.name,
                        mesh.triangles.len(),
                        fps,
                        mode_str,
                        color_str
                    )
                }
            }
        };

        renderer.present(&mut stdout, &status)?;
    }

    // Teardown Terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    println!("Exited Shell-3D successfully.");
    Ok(())
}
