// src/terminal/image_viewer/display/image.rs
use anyhow::Result;
use log::{error, info, warn};
use std::io::Write;
use std::path::Path;

use super::{basic, iterm, kitty, sixel, viuer};
use crate::terminal::image_viewer::state::{DisplayMethod, ImageViewerState};

/// Display the actual image using the best available method
pub fn display_image(viewer_state: &ImageViewerState) -> Result<()> {
    info!("Displaying image: {:?}", viewer_state.image_path);

    // Prepare terminal for image display
    // Clean up terminal state before displaying image
    use crossterm::{
        cursor::Show,
        execute,
        style::ResetColor,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    };

    // Leave the alternate screen
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), Show, ResetColor)?;

    // Reset any potential escape sequences
    print!("\x1b[0m");
    std::io::stdout().flush()?;

    // Clear the screen completely
    execute!(
        std::io::stdout(),
        ResetColor,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    // Flush to ensure all operations are completed
    std::io::stdout().flush()?;

    // Small delay to ensure terminal has processed everything
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Add extra newlines for clarity
    println!(
        "\n\nViewing image: {} (Resolution: {})",
        viewer_state.image_name,
        viewer_state.get_resolution_name()
    );
    println!("Press any key to return to the application...\n");
    std::io::stdout().flush()?;

    // Calculate optimal dimensions based on terminal size
    let term_dims = termsize::get()
        .map(|size| (size.cols as u32, size.rows as u32))
        .unwrap_or((80, 24));

    let (width, height) = viewer_state.calculate_dimensions(term_dims.0, term_dims.1);

    // Try different display methods based on viewer state preferences
    let mut display_success = false;

    if let Some(high_res_data) = &viewer_state.high_res_data {
        // If we have high-res data, write it to a temporary file
        info!("Using higher resolution image data for display");

        // Create a temporary file for high-res image data
        use tempfile::NamedTempFile;
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(high_res_data)?;
        temp_file.flush()?;

        // Convert to PathBuf and prevent deletion when file goes out of scope
        let (file, temp_path) = temp_file.keep()?;
        // Drop the file handle to allow other processes to access it
        drop(file);

        match try_display_image(viewer_state, &temp_path, width, height) {
            Ok(success) => display_success = success,
            Err(e) => error!("Failed to display high-res image: {}", e),
        }

        // Clean up the temporary file
        if let Err(e) = std::fs::remove_file(&temp_path) {
            warn!("Failed to remove temporary file: {}", e);
        }
    } else {
        // Use the original image path
        match try_display_image(viewer_state, &viewer_state.image_path, width, height) {
            Ok(success) => display_success = success,
            Err(e) => error!("Failed to display image: {}", e),
        }
    }

    if !display_success {
        println!("\nFailed to display image with all available methods.");
    }

    println!("\nPress any key to return to the application...");
    std::io::stdout().flush()?;

    // Wait for user input
    let mut buffer = [0; 1];
    let mut stdin = std::io::stdin();
    std::io::Read::read_exact(&mut stdin, &mut buffer)?;

    // Restore terminal
    // Reset any potential escape sequences
    print!("\x1b[0m");
    std::io::stdout().flush()?;

    // Restore terminal to alternate screen
    use crossterm::{
        cursor::Hide,
        terminal::{EnterAlternateScreen, enable_raw_mode},
    };

    execute!(
        std::io::stdout(),
        ResetColor,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        EnterAlternateScreen,
        Hide
    )?;

    enable_raw_mode()?;

    // Flush to ensure all operations are completed
    std::io::stdout().flush()?;

    // Small delay to ensure terminal has processed everything
    std::thread::sleep(std::time::Duration::from_millis(50));

    Ok(())
}

/// Try to display image using the best available method
pub fn try_display_image(
    viewer_state: &ImageViewerState,
    image_path: &Path,
    width: u32,
    height: u32,
) -> Result<bool> {
    let mut display_success = false;

    // Get terminal capabilities
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

    let supports_kitty = term_program.contains("kitty") || std::env::var("KITTY_WINDOW_ID").is_ok();
    let supports_iterm =
        term_program.contains("iTerm") || std::env::var("ITERM_SESSION_ID").is_ok();
    let supports_sixel = term.contains("sixel");

    let capabilities = kitty::TerminalCapabilities {
        supports_kitty,
        supports_iterm,
        supports_sixel,
    };

    match viewer_state.display_method {
        DisplayMethod::ITerm => {
            display_success = iterm::try_display(image_path)?;
        }
        DisplayMethod::Sixel => {
            display_success = sixel::try_display(image_path)?;
        }
        DisplayMethod::Kitty => {
            display_success = kitty::try_display(image_path, width, height, &capabilities)?;
        }
        DisplayMethod::Basic => {
            display_success = basic::try_display(image_path)?;
        }
        DisplayMethod::Auto => {
            // Try methods in sequence from most to least sophisticated

            // Try iTerm2 protocol first if supported
            if capabilities.supports_iterm && !display_success {
                display_success = iterm::try_display(image_path)?;
            }

            // Try Kitty protocol if available and previous methods failed
            if capabilities.supports_kitty && !display_success {
                display_success = kitty::try_display(image_path, width, height, &capabilities)?;
            }

            // Try SIXEL if available and previous methods failed
            if capabilities.supports_sixel && !display_success {
                display_success = sixel::try_display(image_path)?;
            }

            // Try viuer as fallback
            if !display_success {
                display_success = viuer::try_display(image_path, width, height, &capabilities)?;
            }

            // Try basic display as last resort
            if !display_success {
                display_success = basic::try_display(image_path)?;
            }
        }
    }

    Ok(display_success)
}
