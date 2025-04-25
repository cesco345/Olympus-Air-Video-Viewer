// src/terminal/image_viewer/handlers.rs
use crate::camera::client::basic::ClientOperations;
use crate::terminal::image_viewer::display::image;
use crate::terminal::image_viewer::state::ImageViewerState;
use crate::terminal::state::{AppMode, AppState};
use anyhow::Result;
use crossterm::event::KeyCode;
use log::{error, info};
use std::io::Write;
use tempfile::NamedTempFile;

/// Create an image viewer for the given image data
pub fn create_image_viewer(
    app_state: &mut AppState,
    image_data: Vec<u8>,
    image_name: &str,
) -> Result<()> {
    info!("Creating image viewer for image: {}", image_name);

    // Create a temporary file to store the image data
    let mut temp_file = NamedTempFile::new()?;

    // Write the image data to the file
    temp_file.write_all(&image_data)?;
    temp_file.flush()?;

    // Get the path to the temp file
    let image_path = temp_file.path().to_path_buf();

    // Create the image viewer state
    let viewer_state = ImageViewerState::new(image_path, image_name);

    // Store the image viewer state in the app state
    app_state.image_viewer = Some(viewer_state);

    // Store the temp file so it doesn't get deleted when it goes out of scope
    app_state.temp_file = Some(temp_file);

    // Set the mode to viewing image
    app_state.set_mode(AppMode::ViewingImage);

    // Set status
    app_state.set_status(&format!("Viewing image: {}", image_name));

    info!("Image viewer created successfully");

    Ok(())
}

/// Create an image viewer with original URL for higher resolution
pub fn create_image_viewer_with_url(
    app_state: &mut AppState,
    image_data: Vec<u8>,
    image_name: &str,
    original_url: Option<String>,
) -> Result<()> {
    info!(
        "Creating image viewer for image: {} with original URL",
        image_name
    );

    // Create a temporary file to store the image data
    let mut temp_file = NamedTempFile::new()?;

    // Write the image data to the file
    temp_file.write_all(&image_data)?;
    temp_file.flush()?;

    // Get the path to the temp file
    let image_path = temp_file.path().to_path_buf();

    // Create the image viewer state with original URL for higher resolution
    let viewer_state = ImageViewerState::with_original_url(image_path, image_name, original_url);

    // Get resolution info before moving
    let resolution_name = viewer_state.get_resolution_name().to_string();

    // Store the image viewer state in the app state
    app_state.image_viewer = Some(viewer_state);

    // Store the temp file so it doesn't get deleted when it goes out of scope
    app_state.temp_file = Some(temp_file);

    // Set the mode to viewing image
    app_state.set_mode(AppMode::ViewingImage);

    // Set status with resolution info
    app_state.set_status(&format!(
        "Viewing image: {} (Resolution: {})",
        image_name, resolution_name
    ));

    info!("Image viewer created successfully with URL for higher resolution");

    Ok(())
}

/// Handle input for the image viewer
pub fn handle_image_viewer_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true), // Signal to quit
        KeyCode::Esc => {
            info!("Returning to image list");

            // Return to image list
            state.set_mode(AppMode::ImageList);

            // Clear image viewer state and temp file
            state.image_viewer = None;
            state.temp_file = None;

            // Set status
            state.set_status("Returned to image list");
        }
        KeyCode::Enter => {
            // Display the full image using viuer if image viewer state exists
            if let Some(viewer_state) = &state.image_viewer {
                let mode = viewer_state.display_method_name();
                let resolution = viewer_state.get_resolution_name();

                info!(
                    "Displaying image using {} mode at {} resolution",
                    mode, resolution
                );

                // Temporarily suspend TUI and show image
                match image::display_image(viewer_state) {
                    Ok(_) => {
                        state.set_status("Image displayed successfully");
                    }
                    Err(e) => {
                        state.set_status(&format!("Failed to display image: {}", e));
                        error!("Failed to display image: {}", e);
                    }
                }
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.zoom_in();
                let zoom = viewer_state.zoom_factor;
                state.set_status(&format!("Zoom: {:.1}x", zoom));
                info!("Zoomed in to {:.1}x", zoom);
            }
        }
        KeyCode::Char('-') => {
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.zoom_out();
                let zoom = viewer_state.zoom_factor;
                state.set_status(&format!("Zoom: {:.1}x", zoom));
                info!("Zoomed out to {:.1}x", zoom);
            }
        }
        KeyCode::Char('0') => {
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.reset_zoom();
                state.set_status("Zoom reset to 1.0x");
                info!("Zoom reset to 1.0x");
            }
        }
        KeyCode::Char('a') => {
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.toggle_aspect_ratio();
                let preserve = viewer_state.preserve_aspect;

                let status = if preserve {
                    "Aspect ratio preservation enabled"
                } else {
                    "Aspect ratio preservation disabled"
                };
                state.set_status(status);
                info!("{}", status);
            }
        }
        KeyCode::Char('d') => {
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.cycle_display_method();
                let method = viewer_state.display_method_name();
                state.set_status(&format!("Display method: {}", method));
                info!("Changed display method to: {}", method);
            }
        }
        KeyCode::Char('r') => {
            // Fix for borrowing issues: First check if we can improve resolution
            // and collect the necessary information
            let can_process_resolution = if let Some(viewer) = &state.image_viewer {
                let can_increase = viewer.can_increase_resolution() && !viewer.is_high_res_loading;
                let current_res_name = viewer.get_resolution_name().to_string();
                let original_url = viewer.original_url.clone();

                if can_increase {
                    // Set loading status while we still have the immutable borrow
                    let status_msg = format!(
                        "Loading higher resolution image... (Current: {})",
                        current_res_name
                    );

                    // Store resolution processing info
                    Some((status_msg, original_url))
                } else if viewer.is_high_res_loading {
                    state.set_status("Higher resolution image is already loading...");
                    None
                } else if !viewer.can_increase_resolution() {
                    state.set_status(&format!(
                        "Already at {} resolution (maximum available)",
                        current_res_name
                    ));
                    None
                } else {
                    None
                }
            } else {
                None
            };

            // Now process resolution upgrade if needed
            if let Some((status_msg, url_opt)) = can_process_resolution {
                // Update status
                state.set_status(&status_msg);

                // Mark as loading
                if let Some(viewer) = &mut state.image_viewer {
                    viewer.is_high_res_loading = true;
                }

                // Process the URL if we have one
                if let Some(url) = url_opt {
                    // Get a clone of camera
                    let camera_clone = state.camera.clone();

                    // Attempt to get a higher resolution
                    let higher_res_result = if url.contains("&size=") {
                        // Just increase the size parameter
                        let higher_res_url = url.replace("&size=1024", "&size=2048");
                        camera_clone.get_binary(&higher_res_url)
                    } else {
                        // No size parameter to change, use original URL
                        camera_clone.get_binary(&url)
                    };

                    // Update viewer state with result
                    match higher_res_result {
                        Ok(image_data) => {
                            // Store data and update resolution level
                            if let Some(viewer) = &mut state.image_viewer {
                                viewer.high_res_data = Some(image_data);
                                viewer.increase_resolution();
                                let new_res = viewer.get_resolution_name();
                                viewer.is_high_res_loading = false;
                                state.set_status(&format!(
                                    "Image resolution increased to {}",
                                    new_res
                                ));
                            }
                        }
                        Err(e) => {
                            // Reset loading flag and update status
                            if let Some(viewer) = &mut state.image_viewer {
                                viewer.is_high_res_loading = false;
                            }
                            state.set_status(&format!("Failed to load higher resolution: {}", e));
                            error!("Failed to load higher resolution: {}", e);
                        }
                    }
                } else {
                    // No URL available
                    if let Some(viewer) = &mut state.image_viewer {
                        viewer.is_high_res_loading = false;
                    }
                    state.set_status("No URL available for higher resolution");
                }
            }
        }
        _ => {}
    }

    Ok(false)
}
