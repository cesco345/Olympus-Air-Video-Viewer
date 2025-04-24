// src/main.rs
mod camera;
mod terminal;
mod utils;

use anyhow::Result;
use colored::*;
use std::env;
use std::process;

fn main() {
    // Check for debug mode argument
    let debug_mode = env::args().any(|arg| arg == "--debug");

    // Initialize logging only if in debug mode
    if debug_mode {
        utils::logging::init();
        println!(
            "{}",
            "Running in debug mode - logs will be displayed".yellow()
        );
    } else {
        // Initialize logging but set level to warn for reduced output
        utils::logging::init_quiet();
    }

    // Print welcome message
    println!(
        "{}",
        "╔════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║  OLYMPUS AIR CAMERA AND VIDEO CONTROL  ║"
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════╝".bright_cyan()
    );

    // Run the application with proper error handling
    if let Err(e) = run() {
        eprintln!("{} {}", "ERROR:".red().bold(), e);
        eprintln!("{}", "Application terminated with errors.".red());
        process::exit(1);
    }
}

fn run() -> Result<()> {
    // Define camera URL
    let camera_url = "http://192.168.0.10";

    // Create and run application, handling any errors
    let app = terminal::app::App::new(camera_url)?;
    app.run()?;

    Ok(())
}
