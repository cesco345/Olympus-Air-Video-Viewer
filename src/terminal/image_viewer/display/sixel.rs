// src/terminal/image_viewer/display/sixel.rs
use anyhow::Result;
use log::info;
use std::path::Path;
use std::process::Command;

/// Use sixel if available
#[cfg(unix)]
pub fn try_display(image_path: &Path) -> Result<bool> {
    // Check if terminal supports SIXEL
    let term = std::env::var("TERM").unwrap_or_default();
    if !term.contains("sixel") {
        return Ok(false);
    }

    info!("Attempting SIXEL display");

    // Try img2sixel if available
    let img2sixel_result = Command::new("img2sixel")
        .arg("-w")
        .arg("80%")
        .arg(image_path)
        .status();

    if let Ok(status) = img2sixel_result {
        if status.success() {
            info!("Successfully displayed image using SIXEL");
            return Ok(true);
        }
    }

    Ok(false)
}

/// Fallback for non-unix platforms
#[cfg(not(unix))]
pub fn try_display(_image_path: &Path) -> Result<bool> {
    Ok(false)
}
