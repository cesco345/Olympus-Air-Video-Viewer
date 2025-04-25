// src/terminal/image_viewer/display/viuer.rs
use anyhow::Result;
use log::{error, info, warn};
use std::path::Path;

/// Terminal capabilities information
pub use super::kitty::TerminalCapabilities;

/// Try to display an image using viuer library
pub fn try_display(
    image_path: &Path,
    width: u32,
    height: u32,
    capabilities: &TerminalCapabilities,
) -> Result<bool> {
    info!("Attempting to display image using viuer");

    // Create a viuer config that works for the detected terminal
    let conf = viuer::Config {
        width: Some(width),
        height: Some(height),
        truecolor: true,
        absolute_offset: false,
        x: 0,
        y: 0,
        restore_cursor: true,
        use_kitty: capabilities.supports_kitty,
        use_iterm: capabilities.supports_iterm,
        transparent: false,
    };

    // Try first display attempt
    match viuer::print_from_file(image_path, &conf) {
        Ok(_) => {
            // Success with first attempt
            println!("\nImage displayed successfully");
            Ok(true)
        }
        Err(e) => {
            // First attempt failed, try with fallback settings
            warn!("Standard display method failed: {}", e);
            println!("Trying alternative display method...");

            try_fallback_display(image_path, width, height)
        }
    }
}

/// Try fallback display options when the primary method fails
fn try_fallback_display(image_path: &Path, width: u32, height: u32) -> Result<bool> {
    // Try with alternative settings
    let fallback_conf = viuer::Config {
        width: Some(width.min(80)),   // Limit width for better compatibility
        height: Some(height.min(40)), // Limit height for better compatibility
        truecolor: false,             // Use basic colors for better compatibility
        absolute_offset: true,
        x: 0,
        y: 0,
        restore_cursor: true,
        use_kitty: false,
        use_iterm: false,
        transparent: false,
    };

    match viuer::print_from_file(image_path, &fallback_conf) {
        Ok(_) => {
            println!("\nAlternative display method succeeded");
            Ok(true)
        }
        Err(e) => {
            error!("Alternative display method failed: {}", e);
            println!("Alternative display method failed: {}", e);
            Ok(false)
        }
    }
}
