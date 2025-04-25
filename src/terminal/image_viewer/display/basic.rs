// src/terminal/image_viewer/display/basic.rs
use anyhow::Result;
use log::{info, warn};
use std::path::Path;
use std::process::Command;

/// Display image using a more basic approach when sophisticated methods fail
pub fn try_display(image_path: &Path) -> Result<bool> {
    // Try to use a more basic display method
    println!("Attempting basic image rendering methods...");

    #[cfg(unix)]
    {
        // Try multiple different tools that might be available
        let tools = [
            ("catimg", vec![image_path.to_str().unwrap_or("")]),
            ("timg", vec![image_path.to_str().unwrap_or("")]),
            (
                "img2txt",
                vec!["-W", "80", image_path.to_str().unwrap_or("")],
            ),
            ("imgcat", vec![image_path.to_str().unwrap_or("")]),
        ];

        for (tool, args) in tools.iter() {
            info!("Trying {} for image display", tool);

            let result = Command::new(tool).args(args).status();

            if let Ok(status) = result {
                if status.success() {
                    info!("Successfully displayed image using {}", tool);
                    return Ok(true);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, try to use qlmanage as a last resort to preview
        let preview_result = Command::new("qlmanage")
            .args(&["-p", image_path.to_str().unwrap_or("")])
            .status();

        if let Ok(status) = preview_result {
            if status.success() {
                info!("Opened image with Quick Look");
                return Ok(true);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, try to open with the default viewer as last resort
        let open_result = Command::new("cmd")
            .args(&["/C", "start", "", image_path.to_str().unwrap_or("")])
            .status();

        if let Ok(status) = open_result {
            if status.success() {
                info!("Opened image in default viewer");
                return Ok(true);
            }
        }
    }

    // If all else fails, just inform the user
    warn!("Could not display image with any available method");
    println!("Could not display image. Your terminal may not support image display.");
    println!("The image is located at: {}", image_path.display());

    Ok(false)
}
