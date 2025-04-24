// src/terminal/image_viewer/handlers.rs
use crate::terminal::image_viewer::renderer;
use crate::terminal::image_viewer::state::{DisplayMethod, ImageViewerState};
use crate::terminal::state::{AppMode, AppState};
use anyhow::Result;
use crossterm::event::KeyCode;
use log::info;
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
                info!(
                    "Displaying image using {} mode",
                    viewer_state.display_method_name()
                );

                // Temporarily suspend TUI and show image
                match renderer::display_image(viewer_state) {
                    Ok(_) => {
                        state.set_status("Image displayed successfully");
                    }
                    Err(e) => {
                        state.set_status(&format!("Failed to display image: {}", e));
                        log::error!("Failed to display image: {}", e);
                    }
                }
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            // Zoom in if image viewer state exists
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.zoom_in();
                // Store zoom factor before updating status
                let zoom = viewer_state.zoom_factor;
                state.set_status(&format!("Zoom: {:.1}x", zoom));
                info!("Zoomed in to {:.1}x", zoom);
            }
        }
        KeyCode::Char('-') => {
            // Zoom out if image viewer state exists
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.zoom_out();
                // Store zoom factor before updating status
                let zoom = viewer_state.zoom_factor;
                state.set_status(&format!("Zoom: {:.1}x", zoom));
                info!("Zoomed out to {:.1}x", zoom);
            }
        }
        KeyCode::Char('0') => {
            // Reset zoom if image viewer state exists
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.reset_zoom();
                state.set_status("Zoom reset to 1.0x");
                info!("Zoom reset to 1.0x");
            }
        }
        KeyCode::Char('a') => {
            // Toggle aspect ratio preservation
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.toggle_aspect_ratio();
                let status = if viewer_state.preserve_aspect {
                    "Aspect ratio preservation enabled"
                } else {
                    "Aspect ratio preservation disabled"
                };
                state.set_status(status);
                info!("{}", status);
            }
        }
        KeyCode::Char('d') => {
            // Cycle through display methods
            if let Some(viewer_state) = &mut state.image_viewer {
                viewer_state.cycle_display_method();
                let method = viewer_state.display_method_name();
                state.set_status(&format!("Display method: {}", method));
                info!("Changed display method to: {}", method);
            }
        }
        _ => {}
    }

    Ok(false)
}
