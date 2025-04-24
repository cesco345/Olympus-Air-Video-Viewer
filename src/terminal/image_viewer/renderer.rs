// src/terminal/image_viewer/renderer.rs
use crate::terminal::image_viewer::state::{DisplayMethod, ImageViewerState};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::ResetColor,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use log::{error, info, warn};
use std::process::Command;
use std::{
    io::{Read, Write, stdout},
    thread,
    time::Duration,
};
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the image viewer interface
pub fn render<B: Backend>(viewer_state: &ImageViewerState, frame: &mut Frame<B>, area: Rect) {
    // Split area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Image area
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Render title
    let title = Paragraph::new(vec![
        Spans::from(vec![Span::styled(
            format!("Image Viewer - {}", viewer_state.image_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Spans::from(vec![Span::styled(
            format!("Zoom: {:.1}x", viewer_state.zoom_factor),
            Style::default().fg(Color::Green),
        )]),
    ])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, chunks[0]);

    // Render image placeholder
    let image_area = Paragraph::new(vec![
        Spans::from(vec![Span::styled(
            "To view the image, press Enter. The image will be displayed using viuer.",
            Style::default().fg(Color::Yellow),
        )]),
        Spans::from(vec![Span::raw(
            "The terminal UI will be temporarily suspended while viewing the image.",
        )]),
        Spans::from(vec![Span::raw(
            "Press any key to return to the application after viewing.",
        )]),
    ])
    .block(
        Block::default()
            .title("Image Preview")
            .borders(Borders::ALL),
    )
    .wrap(Wrap { trim: true });

    frame.render_widget(image_area, chunks[1]);

    // Render controls
    let controls = Paragraph::new(vec![Spans::from(vec![
        Span::styled("Controls: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("+/- - Zoom in/out   "),
        Span::raw("0 - Reset zoom   "),
        Span::raw("d - Cycle display modes   "),
        Span::raw("Esc - Return to image list   "),
        Span::raw("q - Quit"),
    ])])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(controls, chunks[2]);
}

/// Clean the terminal completely - more robust version
fn clean_terminal() -> Result<()> {
    // Reset any potential escape sequences
    print!("\x1b[0m");
    stdout().flush()?;

    // Clear the screen completely
    execute!(stdout(), ResetColor, Clear(ClearType::All))?;

    // Flush to ensure all operations are completed
    stdout().flush()?;

    // Small delay to ensure terminal has processed everything
    thread::sleep(Duration::from_millis(50));

    Ok(())
}

/// Restore the terminal to a usable state - more robust version
fn restore_terminal() -> Result<()> {
    // Clear any remaining escape sequences
    print!("\x1b[0m");
    stdout().flush()?;

    execute!(
        stdout(),
        ResetColor,
        Clear(ClearType::All),
        EnterAlternateScreen,
        Hide
    )?;

    enable_raw_mode()?;

    // Flush to ensure all operations are completed
    stdout().flush()?;

    // Small delay to ensure terminal has processed everything
    thread::sleep(Duration::from_millis(50));

    Ok(())
}

/// Detect terminal capabilities for image display
fn detect_terminal_capabilities() -> (bool, bool, bool) {
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

    let supports_kitty = term_program.contains("kitty") || std::env::var("KITTY_WINDOW_ID").is_ok();
    let supports_iterm =
        term_program.contains("iTerm") || std::env::var("ITERM_SESSION_ID").is_ok();
    let supports_sixel = term.contains("sixel");

    info!(
        "Terminal capabilities: TERM={}, TERM_PROGRAM={}, supports_kitty={}, supports_iterm={}, supports_sixel={}",
        term, term_program, supports_kitty, supports_iterm, supports_sixel
    );

    (supports_kitty, supports_iterm, supports_sixel)
}

/// Display image using iTerm2 protocol directly (without base64)
#[cfg(target_os = "macos")]
fn try_iterm_display(image_path: &std::path::Path) -> Result<bool> {
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

#[cfg(not(target_os = "macos"))]
fn try_iterm_display(_image_path: &std::path::Path) -> Result<bool> {
    Ok(false)
}

/// Use sixel if available
#[cfg(unix)]
fn try_sixel_display(image_path: &std::path::Path) -> Result<bool> {
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

#[cfg(not(unix))]
fn try_sixel_display(_image_path: &std::path::Path) -> Result<bool> {
    Ok(false)
}

/// Display image using a more basic approach when sophisticated methods fail
fn display_basic_image(image_path: &std::path::Path) -> Result<bool> {
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

/// Display the actual image using viuer with better terminal handling
pub fn display_image(viewer_state: &ImageViewerState) -> Result<()> {
    info!("Displaying image: {:?}", viewer_state.image_path);

    // Properly clean up terminal state before displaying image
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    execute!(stdout(), Show, ResetColor)?;

    // Clear the terminal to prevent artifacts
    clean_terminal()?;

    // Add extra newlines for clarity
    println!("\n\nViewing image: {}", viewer_state.image_name);
    println!("Press any key to return to the application...\n");
    stdout().flush()?;

    // Small delay to ensure terminal has processed everything
    thread::sleep(Duration::from_millis(100));

    // Calculate optimal dimensions based on terminal size
    let (term_width, term_height) = termsize::get()
        .map(|size| (size.cols as u32, size.rows as u32))
        .unwrap_or((80, 24));

    let (width, height) = viewer_state.calculate_dimensions(term_width, term_height);

    // Try different display methods based on viewer state preferences
    let mut display_success = false;

    match viewer_state.display_method {
        DisplayMethod::ITerm => {
            display_success = try_iterm_display(&viewer_state.image_path)?;
        }
        DisplayMethod::Sixel => {
            display_success = try_sixel_display(&viewer_state.image_path)?;
        }
        DisplayMethod::Basic => {
            display_success = display_basic_image(&viewer_state.image_path)?;
        }
        _ => {
            // Auto or Kitty - try all methods in sequence
            let (supports_kitty, supports_iterm, supports_sixel) = detect_terminal_capabilities();

            // Try iTerm2 protocol first on macOS
            if supports_iterm && !display_success {
                display_success = try_iterm_display(&viewer_state.image_path)?;
            }

            // Try SIXEL if available and previous methods failed
            if supports_sixel && !display_success {
                display_success = try_sixel_display(&viewer_state.image_path)?;
            }

            // Try viuer with best configuration if previous methods failed
            if !display_success {
                // Create a viuer config that works for the detected terminal
                let conf = viuer::Config {
                    width: Some(width),
                    height: Some(height),
                    truecolor: true,
                    absolute_offset: false,
                    x: 0,
                    y: 0,
                    restore_cursor: true,
                    use_kitty: supports_kitty,
                    use_iterm: supports_iterm,
                    transparent: false,
                };

                match viuer::print_from_file(&viewer_state.image_path, &conf) {
                    Ok(_) => {
                        // Success with first attempt
                        display_success = true;
                        println!("\nImage displayed successfully");
                    }
                    Err(e) => {
                        // First attempt failed, try with fallback settings
                        warn!("Standard display method failed: {}", e);
                        println!("Trying alternative display method...");

                        // Try with alternative settings
                        let fallback_conf = viuer::Config {
                            width: Some(width.min(80)),   // Limit width for better compatibility
                            height: Some(height.min(40)), // Limit height for better compatibility
                            truecolor: false, // Use basic colors for better compatibility
                            absolute_offset: true,
                            x: 0,
                            y: 0,
                            restore_cursor: true,
                            use_kitty: false,
                            use_iterm: false,
                            transparent: false,
                        };

                        match viuer::print_from_file(&viewer_state.image_path, &fallback_conf) {
                            Ok(_) => {
                                display_success = true;
                                println!("\nAlternative display method succeeded");
                            }
                            Err(e) => {
                                error!("Alternative display method failed: {}", e);
                                println!("Alternative display method failed: {}", e);
                            }
                        }
                    }
                }
            }

            // Try basic display as last resort
            if !display_success {
                display_success = display_basic_image(&viewer_state.image_path)?;
            }
        }
    }

    if !display_success {
        println!("\nFailed to display image with all available methods.");
    }

    println!("\nPress any key to return to the application...");
    stdout().flush()?;

    // Wait for user input using a more robust approach
    let mut buffer = [0; 1];
    std::io::stdin().read_exact(&mut buffer)?;

    // Ensure terminal is completely clean before restoring
    clean_terminal()?;

    // Restore terminal state with the more robust method
    restore_terminal()?;

    Ok(())
}
