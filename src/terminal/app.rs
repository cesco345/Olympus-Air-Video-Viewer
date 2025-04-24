// src/terminal/app.rs
use crate::terminal::{handlers, image_viewer, state::AppState};
use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use log::info;
use std::io;
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
};

/// The main application struct
pub struct App {
    state: Option<AppState>,
    camera_url: String,
    connection_error: Option<String>,
}

impl App {
    /// Create a new App instance
    pub fn new(camera_url: &str) -> Result<Self> {
        info!("Initializing application");

        // Print initial connection message
        println!("{}", "Connecting to Olympus camera...".cyan().bold());

        // Initialize the application state
        let state_result = AppState::new(camera_url);
        let has_error = state_result.is_err();

        let state = match state_result {
            Ok(state) => {
                println!(
                    "{}",
                    format!("Found {} images on camera", state.images.len()).cyan()
                );
                Some(state)
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Error connecting to camera: {}", e).red().bold()
                );
                println!(
                    "{}",
                    "Starting in offline mode. Press any key to continue.".yellow()
                );
                None
            }
        };

        println!("{}", "Starting terminal interface...".cyan().italic());

        Ok(Self {
            state,
            camera_url: camera_url.to_string(),
            connection_error: if has_error {
                Some("Failed to connect to camera".to_string())
            } else {
                None
            },
        })
    }

    /// Attempt to reconnect to the camera
    fn attempt_reconnect(&mut self) -> Result<bool> {
        info!("Attempting to reconnect to camera");

        match AppState::new(&self.camera_url) {
            Ok(state) => {
                self.state = Some(state);
                self.connection_error = None;
                info!("Successfully reconnected to camera");
                Ok(true)
            }
            Err(e) => {
                self.connection_error = Some(format!("Failed to connect: {}", e));
                info!("Reconnection failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Run the application
    pub fn run(mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Clear the terminal
        terminal.clear()?;

        info!("Starting application loop");

        // Run the application loop
        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        // Check for errors
        if let Err(err) = result {
            println!("{}", format!("Error: {}", err).red().bold());
            return Err(err);
        }

        // Show exit message
        println!(
            "{}",
            "Olympus Camera Control terminated successfully."
                .green()
                .bold()
        );
        println!("{}", "Thank you for using the application!".cyan());

        info!("Application shutdown");

        Ok(())
    }

    fn run_app<B: tui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Set up a buffer to prevent excessive screen redraws
        let mut last_screen_refresh = std::time::Instant::now();
        let refresh_rate = std::time::Duration::from_millis(50); // 50ms refresh rate (20 FPS)

        loop {
            // Only redraw if enough time has passed
            let now = std::time::Instant::now();
            if now.duration_since(last_screen_refresh) >= refresh_rate {
                terminal.draw(|f| {
                    let size = f.size(); // Get the area for rendering

                    if let Some(state) = &self.state {
                        // If we have a state, render the appropriate UI based on mode
                        if state.mode == crate::terminal::state::AppMode::ViewingImage {
                            // In image viewer mode, use the image viewer renderer
                            if let Some(viewer_state) = &state.image_viewer {
                                // Pass the viewer_state, frame, and area to the render function
                                image_viewer::renderer::render(viewer_state, f, size);
                            }
                        } else {
                            // For all other modes, use the main renderer
                            crate::terminal::renderer::render_app(state, f);
                        }
                    } else {
                        // If we don't have a state, render the offline mode UI
                        let size = f.size();

                        // Create a layout
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(2)
                            .constraints(
                                [
                                    Constraint::Length(3), // Title
                                    Constraint::Min(5),    // Message
                                    Constraint::Length(3), // Controls
                                ]
                                .as_ref(),
                            )
                            .split(size);

                        // Title
                        let title = Paragraph::new(vec![Spans::from(vec![Span::styled(
                            "Olympus Camera Control - OFFLINE MODE",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )])])
                        .block(Block::default().borders(Borders::ALL));

                        f.render_widget(title, chunks[0]);

                        // Error message
                        let error_text = vec![
                            Spans::from(vec![Span::styled(
                                "Camera Connection Error",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            )]),
                            Spans::from(vec![Span::raw("")]),
                            Spans::from(vec![Span::raw(
                                self.connection_error.as_deref().unwrap_or("Unknown error"),
                            )]),
                            Spans::from(vec![Span::raw("")]),
                            Spans::from(vec![Span::raw("Please check:")]),
                            Spans::from(vec![Span::raw("1. Camera is powered on")]),
                            Spans::from(vec![Span::raw("2. WiFi connection is active")]),
                            Spans::from(vec![Span::raw("3. Camera IP address is correct")]),
                            Spans::from(vec![Span::raw("")]),
                            Spans::from(vec![Span::styled(
                                "Press 'r' to attempt reconnection or 'q' to quit",
                                Style::default().fg(Color::Yellow),
                            )]),
                        ];

                        let error_msg = Paragraph::new(error_text).block(
                            Block::default()
                                .title("Connection Status")
                                .borders(Borders::ALL),
                        );

                        f.render_widget(error_msg, chunks[1]);

                        // Controls
                        let controls = Paragraph::new(vec![Spans::from(vec![
                            Span::styled(
                                "Controls: ",
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                            Span::raw("r - Attempt reconnection   "),
                            Span::raw("q - Quit application"),
                        ])])
                        .block(Block::default().borders(Borders::ALL));

                        f.render_widget(controls, chunks[2]);
                    }
                })?;

                last_screen_refresh = now;
            }

            // Handle events with a timeout to prevent UI blocking
            if crossterm::event::poll(std::time::Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(state) = &mut self.state {
                        // Normal mode - pass events to the handler
                        if handlers::handle_input(state, key.code)? {
                            return Ok(());
                        }
                    } else {
                        // Offline mode - limited options
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('r') => {
                                // Try to reconnect
                                let _ = self.attempt_reconnect();
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Small sleep to prevent CPU hogging
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
