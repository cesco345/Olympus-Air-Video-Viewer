// src/terminal/video_viewer/handlers.rs
use crate::terminal::state::{AppMode, AppState};
use crate::terminal::video_viewer::olympus_udp;
use crate::terminal::video_viewer::state::VideoViewerState;
use anyhow::{Result, anyhow};
use crossterm::event::KeyCode;
use log::{error, info, warn};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create a video viewer for the given stream
pub fn create_video_viewer(
    app_state: &mut AppState,
    stream_url: &str,
    stream_name: &str,
) -> Result<()> {
    info!("Creating Olympus video viewer for stream: {}", stream_name);

    // Check if MPlayer is available
    match Command::new("which").arg("mplayer").output() {
        Ok(output) if output.status.success() => {
            info!("MPlayer is available for Olympus streaming");
        }
        _ => {
            // Check if FFplay is available as fallback
            match Command::new("which").arg("ffplay").output() {
                Ok(output) if output.status.success() => {
                    info!("FFplay is available as fallback player");
                }
                _ => {
                    warn!(
                        "Neither MPlayer nor FFplay found. Please install one of them for streaming"
                    );
                    app_state
                        .set_status("Video player not found. Please install MPlayer or FFplay");
                }
            }
        }
    }

    // Create the viewer state
    let viewer_state = VideoViewerState::new(stream_url, stream_name);
    app_state.video_viewer = Some(viewer_state);
    app_state.set_mode(AppMode::ViewingVideo);
    app_state.set_status(&format!("Viewing video stream: {}", stream_name));

    Ok(())
}

/// Create a live view stream to the Olympus camera
pub fn create_live_view(app_state: &mut AppState) -> Result<()> {
    info!("Creating live view stream to camera");
    app_state.set_status("Initializing camera for live view...");

    // Make sure the camera is connected
    match app_state.camera.connect() {
        Ok(_) => info!("Camera connection successful"),
        Err(e) => {
            error!("Failed to connect to camera: {}", e);
            app_state.set_status(&format!("Failed to connect to camera: {}", e));
            return Err(anyhow!("Failed to connect to camera: {}", e));
        }
    }

    // Default UDP port
    let udp_port = 65001;

    // Initialize camera for live view
    match olympus_udp::initialize_camera(&app_state.camera, udp_port) {
        Ok(_) => {
            info!("Camera initialized for live view on port {}", udp_port);
            app_state.set_status(&format!("Live view started on port {}", udp_port));
        }
        Err(e) => {
            error!("Failed to start live view: {}", e);
            app_state.set_status(&format!("Failed to start live view: {}", e));
            return Err(anyhow!("Failed to start live view: {}", e));
        }
    }

    // Create the video viewer
    match create_video_viewer(app_state, "192.168.0.10", "Camera Live View") {
        Ok(_) => {
            if let Some(viewer_state) = &mut app_state.video_viewer {
                viewer_state.udp_port = udp_port;

                // Start the stream
                if let Err(e) = olympus_udp::start_udp_receiver(viewer_state) {
                    error!("Failed to start UDP receiver: {}", e);
                    app_state.set_status(&format!("Failed to start video stream: {}", e));
                } else {
                    app_state.set_status("Video stream started successfully");
                }
            }
        }
        Err(e) => {
            error!("Failed to create video viewer: {}", e);
            return Err(anyhow!("Failed to create video viewer: {}", e));
        }
    }

    Ok(())
}

/// Handle input for the video viewer
pub fn handle_video_viewer_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => {
            // Quit application
            if let Some(viewer_state) = &mut state.video_viewer {
                let _ = olympus_udp::stop_udp_receiver(viewer_state);
                let _ = olympus_udp::stop_live_view(&state.camera);
            }
            return Ok(true);
        }
        KeyCode::Esc => {
            // Return to main menu
            if let Some(viewer_state) = &mut state.video_viewer {
                let _ = olympus_udp::stop_udp_receiver(viewer_state);
                let _ = olympus_udp::stop_live_view(&state.camera);
            }
            state.set_mode(AppMode::Main);
            state.video_viewer = None;
            state.set_status("Returned to main menu");
        }
        KeyCode::Enter => {
            // Restart stream
            if let Some(viewer_state) = &mut state.video_viewer {
                // Store the UDP port for later use
                let udp_port = viewer_state.udp_port;

                // Stop current stream
                let _ = olympus_udp::stop_udp_receiver(viewer_state);
                let _ = olympus_udp::stop_live_view(&state.camera);

                // Drop the borrow of viewer_state
                drop(viewer_state);

                // Restart with a status update
                state.set_status("Restarting stream...");

                // Add delay for better recovery
                std::thread::sleep(std::time::Duration::from_millis(1000));

                // Re-borrow and initialize
                if let Some(viewer_state) = &mut state.video_viewer {
                    match olympus_udp::initialize_camera(&state.camera, udp_port) {
                        Ok(_) => {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            if let Err(e) = olympus_udp::start_udp_receiver(viewer_state) {
                                state.set_status(&format!("Failed to restart stream: {}", e));
                            } else {
                                state.set_status("Stream restarted successfully");
                            }
                        }
                        Err(e) => state.set_status(&format!("Failed to restart live view: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            // Toggle play/pause
            if let Some(viewer_state) = &mut state.video_viewer {
                if viewer_state.is_playing {
                    let _ = olympus_udp::stop_udp_receiver(viewer_state);
                    let _ = olympus_udp::stop_live_view(&state.camera);

                    // Drop the borrow of viewer_state
                    drop(viewer_state);

                    state.set_status("Playback paused");
                } else {
                    // Store the UDP port for later use
                    let udp_port = if let Some(vs) = &state.video_viewer {
                        vs.udp_port
                    } else {
                        65001 // Default port
                    };

                    match olympus_udp::initialize_camera(&state.camera, udp_port) {
                        Ok(_) => {
                            std::thread::sleep(std::time::Duration::from_millis(500));

                            if let Some(viewer_state) = &mut state.video_viewer {
                                if let Err(e) = olympus_udp::start_udp_receiver(viewer_state) {
                                    state.set_status(&format!("Failed to resume: {}", e));
                                } else {
                                    state.set_status("Playback resumed");
                                }
                            }
                        }
                        Err(e) => state.set_status(&format!("Failed to restart live view: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('r') => {
            // Toggle recording
            if let Some(viewer_state) = &mut state.video_viewer {
                if viewer_state.is_recording {
                    viewer_state.stop_recording();

                    // Drop the borrow of viewer_state
                    drop(viewer_state);

                    state.set_status("Recording stopped");
                } else {
                    // Create recordings directory if it doesn't exist
                    let recordings_dir = Path::new("./recordings");
                    if !recordings_dir.exists() {
                        if let Err(e) = std::fs::create_dir_all(recordings_dir) {
                            state.set_status(&format!(
                                "Failed to create recordings directory: {}",
                                e
                            ));
                            return Ok(false);
                        }
                    }

                    // Generate filename based on current time
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let recording_path =
                        recordings_dir.join(format!("olympus_recording_{}.mjpeg", now));

                    if let Some(viewer_state) = &mut state.video_viewer {
                        viewer_state.start_recording(recording_path);

                        // Drop the borrow of viewer_state
                        drop(viewer_state);

                        state
                            .set_status("Recording started - note: requires manual encoding later");
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            // Run diagnostics
            state.set_status("Running diagnostics...");

            // First store any data we need from viewer_state to use later
            let udp_port = if let Some(vs) = &state.video_viewer {
                vs.udp_port
            } else {
                65001 // Default port
            };

            // Stop the viewer
            if let Some(viewer_state) = &mut state.video_viewer {
                let _ = olympus_udp::stop_udp_receiver(viewer_state);
                let _ = olympus_udp::stop_live_view(&state.camera);
            }

            // At this point, we no longer need the viewer_state reference
            std::thread::sleep(std::time::Duration::from_secs(1));

            // Check camera connection
            match state.camera.connect() {
                Ok(_) => {
                    state.set_status("Camera connection verified");

                    // Test camera initialization
                    match olympus_udp::initialize_camera(&state.camera, udp_port) {
                        Ok(_) => {
                            state.set_status("Camera initialized successfully");
                            std::thread::sleep(std::time::Duration::from_millis(500));

                            // Now we can re-borrow viewer_state for UDP streaming
                            if let Some(viewer_state) = &mut state.video_viewer {
                                match olympus_udp::start_udp_receiver(viewer_state) {
                                    Ok(_) => {
                                        // Don't forget to drop before status update
                                        drop(viewer_state);
                                        state.set_status("Diagnostics complete, stream restarted");
                                    }
                                    Err(e) => {
                                        // Don't need to drop here since we didn't mutate
                                        drop(viewer_state);
                                        state.set_status(&format!(
                                            "Failed to start UDP receiver: {}",
                                            e
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => state.set_status(&format!("Failed to initialize camera: {}", e)),
                    }
                }
                Err(e) => state.set_status(&format!("Camera connection failed: {}", e)),
            }
        }
        _ => {}
    }

    Ok(false)
}
