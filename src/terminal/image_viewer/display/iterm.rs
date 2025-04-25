// src/terminal/image_viewer/display/iterm.rs
use anyhow::Result;
use log::info;
use std::path::Path;
use std::process::Command;

/// Display image using iTerm2 protocol
#[cfg(target_os = "macos")]
pub fn try_display(image_path: &Path) -> Result<bool> {
    if std::env::var("TERM_PROGRAM").unwrap_or_default() != "iTerm.app" {
        return Ok(false);
    }

    info!("Attempting iTerm2 native display via imgcat");

    // Use imgcat if available (comes with iTerm2)
    let imgcat_result = Command::new("imgcat").arg(image_path).status();

    if let Ok(status) = imgcat_result {
        if status.success() {
            info!("Successfully displayed image using imgcat");
            return Ok(true);
        }
    }

    // If imgcat isn't available, try using the qlmanage preview
    let preview_result = Command::new("qlmanage")
        .args(&["-p", image_path.to_str().unwrap_or("")])
        .status();

    if let Ok(status) = preview_result {
        if status.success() {
            info!("Displayed image using Quick Look");
            return Ok(true);
        }
    }

    Ok(false)
}

/// Fallback for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn try_display(_image_path: &Path) -> Result<bool> {
    Ok(false)
}
