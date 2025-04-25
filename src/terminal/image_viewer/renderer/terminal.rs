// src/terminal/image_viewer/renderer/terminal.rs
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
use log::info;
use std::{
    io::{Write, stdout},
    thread,
    time::Duration,
};

/// Clean the terminal completely - more robust version
pub fn clean_terminal() -> Result<()> {
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
pub fn restore_terminal() -> Result<()> {
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

/// Prepare terminal for image display
pub fn prepare_for_image_display() -> Result<()> {
    // Properly clean up terminal state before displaying image
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    execute!(stdout(), Show, ResetColor)?;

    // Clean terminal to prevent artifacts
    clean_terminal()?;

    // Small delay to ensure terminal has processed everything
    thread::sleep(Duration::from_millis(100));

    Ok(())
}

/// Wait for a key press
pub fn wait_for_keypress() -> Result<()> {
    // Wait for user input using a more robust approach
    let mut buffer = [0; 1];
    let mut stdin = std::io::stdin();
    std::io::Read::read_exact(&mut stdin, &mut buffer)?;

    Ok(())
}
