// src/terminal/handlers.rs
use crate::terminal::state::{AppMode, AppState};
use anyhow::Result;
use crossterm::event::KeyCode;
use std::{path::Path, thread, time::Duration};

/// Handle input based on the current application mode
pub fn handle_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    // Handle error dialog if it's showing
    if state.show_error_dialog {
        match key {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                state.dismiss_error_dialog();
                return Ok(false);
            }
            _ => return Ok(false),
        }
    }

    // Normal input handling
    match state.mode {
        AppMode::Main => handle_main_input(state, key),
        AppMode::ImageList => handle_image_list_input(state, key),
        AppMode::Downloading => handle_download_input(state, key),
        AppMode::Deleting => handle_delete_input(state, key),
        AppMode::ViewingImage => {
            crate::terminal::image_viewer::handlers::handle_image_viewer_input(state, key)
        }
    }
}

/// Handle input in the main menu
fn handle_main_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true), // Signal to quit
        KeyCode::Up => state.selection_up(),
        KeyCode::Down => state.selection_down(),
        KeyCode::Enter => {
            match state.selected_index {
                0 => {
                    state.set_status("Taking photo with warm-up...");
                    take_photo_with_warmup(state)?;
                }
                1 => {
                    // Just show the list of images - DON'T take a photo
                    state.set_status("Loading image list...");
                    state.refresh_images()?;
                    state.set_mode(AppMode::ImageList);
                }
                2 => {
                    state.set_status("Refreshing image count...");
                    state.refresh_images()?;
                }
                3 => {
                    return Ok(true); // Signal to quit
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}

/// Handle input in the image list
fn handle_image_list_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true), // Signal to quit
        KeyCode::Up => state.selection_up(),
        KeyCode::Down => state.selection_down(),
        KeyCode::PageUp => state.prev_page(),
        KeyCode::PageDown => state.next_page(),
        KeyCode::Home => state.first_image(),
        KeyCode::End => state.last_image(),
        KeyCode::Char('d') => {
            if state.selected_image().is_some() {
                log::info!(
                    "Moving to download screen for image at index: {}",
                    state.selected_index
                );
                state.set_mode(AppMode::Downloading);
            } else {
                state.set_status("No image selected for download");
            }
        }
        KeyCode::Delete => {
            if state.selected_image().is_some() {
                log::info!(
                    "Moving to delete screen for image at index: {}",
                    state.selected_index
                );
                state.set_mode(AppMode::Deleting);
            } else {
                state.set_status("No image selected for deletion");
            }
        }
        KeyCode::Enter => {
            // New: View the selected image
            if state.selected_image().is_some() {
                log::info!("Viewing image at index: {}", state.selected_index);
                match state.view_selected_image() {
                    Ok(_) => {
                        log::info!("Image viewer opened successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to open image viewer: {}", e);
                        state.set_status(&format!("Failed to view image: {}", e));
                    }
                }
            } else {
                state.set_status("No image selected to view");
            }
        }
        KeyCode::Char('r') => {
            state.refresh_images()?;
            state.set_status(&format!(
                "Image list refreshed - {} images found",
                state.images.len()
            ));
        }
        KeyCode::Esc => {
            state.set_mode(AppMode::Main);
        }
        _ => {}
    }
    Ok(false)
}

/// Handle input in the download screen
fn handle_download_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true), // Signal to quit
        KeyCode::Enter => {
            // IMPORTANT: Get the currently selected image by index
            // Store the index for debugging
            let selected_idx = state.selected_index;

            // Get the image name by direct array access to ensure correct selection
            let image_to_download = if !state.images.is_empty() && selected_idx < state.images.len()
            {
                let image = &state.images[selected_idx];
                log::info!(
                    "Selected for download by direct access: index={}, image={}",
                    selected_idx,
                    image
                );
                image.trim().to_string() // Ensure no whitespace
            } else {
                state.set_status("Error: No image selected");
                state.set_mode(AppMode::ImageList);
                return Ok(false);
            };

            // Log which image we're trying to download
            log::info!(
                "Downloading image at index: {}, filename: {}",
                selected_idx,
                image_to_download
            );
            state.set_status(&format!("Downloading image: {}...", image_to_download));

            // Try to download the image
            match download_image(state, &image_to_download) {
                Ok(_) => {
                    state.set_status(&format!("Successfully downloaded: {}", image_to_download));
                    log::info!("Download success: {}", image_to_download);
                }
                Err(e) => {
                    state.set_status(&format!("Download failed: {}", e));
                    log::error!("Download error: {}", e);
                }
            }

            // Return to image list
            state.set_mode(AppMode::ImageList);
        }
        KeyCode::Esc => {
            state.set_mode(AppMode::ImageList);
        }
        _ => {}
    }
    Ok(false)
}

/// Handle input in the delete screen
fn handle_delete_input(state: &mut AppState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Char('q') => return Ok(true), // Signal to quit
        KeyCode::Enter => {
            // IMPORTANT: Get the currently selected image by index
            // Store the index for debugging
            let selected_idx = state.selected_index;

            // Get the image name by direct array access to ensure correct selection
            let image_to_delete = if !state.images.is_empty() && selected_idx < state.images.len() {
                let image = &state.images[selected_idx];
                log::info!(
                    "Selected for deletion by direct access: index={}, image={}",
                    selected_idx,
                    image
                );
                image.trim().to_string() // Ensure no whitespace
            } else {
                state.set_status("Error: No image selected");
                state.set_mode(AppMode::ImageList);
                return Ok(false);
            };

            // Log which image we're trying to delete
            log::info!(
                "Deleting image at index: {}, filename: {}",
                selected_idx,
                image_to_delete
            );
            state.set_status(&format!("Attempting to delete: {}...", image_to_delete));

            // Try to delete the image with enhanced error handling
            match delete_image(state, &image_to_delete) {
                Ok(_) => {
                    // Successful deletion
                    state.set_status(&format!("Successfully deleted: {}", image_to_delete));
                    log::info!("Deletion successful for: {}", image_to_delete);

                    // Refresh immediately to confirm the image is gone
                    let _ = state.refresh_images();
                }
                Err(e) => {
                    // Enhanced error reporting
                    let error_msg = format!("{}", e);
                    log::error!("Deletion error: {}", error_msg);

                    if error_msg.contains("WiFi") {
                        // WiFi-specific error with guidance
                        state.set_status(
                            "Camera doesn't support WiFi deletion. Try using camera's menu.",
                        );
                    } else {
                        state.set_status(&format!("Deletion failed: {}", e));
                    }

                    // Show longer explanation in a dialog
                    show_delete_error_dialog(state);

                    // Refresh anyway to ensure our list is current
                    let _ = state.refresh_images();
                }
            }

            // Return to image list
            state.set_mode(AppMode::ImageList);
        }
        KeyCode::Esc => {
            state.set_mode(AppMode::ImageList);
        }
        _ => {}
    }
    Ok(false)
}

/// Show a detailed error dialog for delete operations
fn show_delete_error_dialog(state: &mut AppState) {
    state.set_error_message(
        "Olympus Camera Delete Limitation",
        "Most Olympus cameras do not support deleting images over WiFi. This is a limitation of the camera's firmware.\n\nAlternatives:\n1. Use the camera's menu to delete images\n2. Connect the SD card to your computer\n3. Format the SD card (will delete ALL images)"
    );
    state.set_show_error_dialog(true);
}

// Camera operation functions

/// Take a photo with warm-up
fn take_photo_with_warmup(state: &mut AppState) -> Result<()> {
    // Your olympus.rs implementation already includes warm-up functionality
    // in the take_photo() method, so we can just call that
    state.camera.take_photo()?;
    state.refresh_images()?;
    state.set_status("Photo captured successfully");
    Ok(())
}

/// Download an image
fn download_image(state: &mut AppState, image: &str) -> Result<()> {
    // Log which image is being downloaded
    log::info!("Downloading image: {}", image);

    // Create a downloads directory if it doesn't exist
    let download_dir = Path::new("downloads");
    if !download_dir.exists() {
        std::fs::create_dir_all(download_dir)?;
    }

    // Set status to indicate which image is being downloaded
    state.set_status(&format!("Downloading: {} to downloads directory...", image));

    // Create the destination path
    let destination = download_dir.join(image);

    // Download the image
    match state.camera.download_image(image, &destination) {
        Ok(_) => {
            log::info!("Successfully downloaded: {}", image);
            state.set_status(&format!("Downloaded: {} to downloads/{}", image, image));
        }
        Err(e) => {
            log::error!("Download error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Delete an image
fn delete_image(state: &mut AppState, image: &str) -> Result<()> {
    // Log which image is being deleted
    log::info!("Attempting to delete image: {}", image);

    // Set status to indicate which image is being deleted
    state.set_status(&format!("Deleting: {}...", image));

    // Try to delete the image
    match state.camera.delete_image(image) {
        Ok(_) => {
            log::info!("Delete operation completed for: {}", image);
            state.set_status(&format!("Deletion attempt for {} completed", image));
        }
        Err(e) => {
            log::error!("Delete error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
