// src/terminal/renderer.rs
use crate::terminal::state::{AppMode, AppState};
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

/// Render the application interface
pub fn render_app<B: Backend>(state: &AppState, frame: &mut Frame<B>) {
    let size = frame.size();

    // Split the layout into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Main content
                Constraint::Length(3), // Status
            ]
            .as_ref(),
        )
        .split(size);

    // Render different content based on mode
    render_title(state, frame, chunks[0]);
    render_content(state, frame, chunks[1]);
    render_status(state, frame, chunks[2]);
}

/// Render the title bar
fn render_title<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Create title text
    let title_text = match state.mode {
        AppMode::Main => "Olympus Camera Control - Main Menu",
        AppMode::ImageList => "Olympus Camera Control - Image List",
        AppMode::Downloading => "Olympus Camera Control - Download Image",
        AppMode::Deleting => "Olympus Camera Control - Delete Image",
        AppMode::ViewingImage => "Olympus Camera Control - Image Viewer",
        AppMode::ViewingVideo => "Olympus Camera Control - Video Viewer",
    };

    // Create the title paragraph
    let title = Paragraph::new(Spans::from(vec![Span::styled(
        title_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, area);
}

/// Render the main content
fn render_content<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    match state.mode {
        AppMode::Main => render_main_menu(state, frame, area),
        AppMode::ImageList => render_image_list(state, frame, area),
        AppMode::Downloading => render_download_screen(state, frame, area),
        AppMode::Deleting => render_delete_screen(state, frame, area),
        // Don't render anything in viewing mode - this is handled by image_viewer
        AppMode::ViewingImage => {}
        AppMode::ViewingVideo => {}
    }
}

/// Render the main menu
fn render_main_menu<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Create menu items
    let menu_items = vec![
        ListItem::new(Spans::from(Span::raw("Take Photo"))),
        ListItem::new(Spans::from(Span::raw("View Images"))),
        ListItem::new(Spans::from(Span::raw("Live View"))),
        ListItem::new(Spans::from(Span::raw("Refresh Image List"))),
        ListItem::new(Spans::from(Span::raw("Quit"))),
    ];

    // Create the menu list
    let menu = List::new(menu_items)
        .block(Block::default().title("Main Menu").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Create a ListState from the selected index
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    // Render the menu with the current selection
    frame.render_stateful_widget(menu, area, &mut list_state);
}

/// Render the image list
fn render_image_list<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Get pagination info
    let start_idx = state.page_start_index();
    let end_idx = state.page_end_index();
    let total_pages = state.total_pages();

    // Create list items for current page
    let items: Vec<ListItem> = state
        .images
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .enumerate()
        .map(|(i, image_name)| {
            let content = Spans::from(vec![Span::raw(format!("{}", image_name))]);
            ListItem::new(content)
        })
        .collect();

    // Create image list with pagination info
    let list_title = format!(
        "Images ({} total) - Page {}/{}",
        state.images.len(),
        state.current_page_index + 1,
        total_pages
    );

    let images_list = List::new(items)
        .block(Block::default().title(list_title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Calculate the adjusted index for rendering
    let adjusted_index = state.selected_index.saturating_sub(start_idx);

    // Create a ListState for the adjusted index
    let mut list_state = ListState::default();
    // Only select if there are items in the list
    if end_idx > start_idx {
        list_state.select(Some(adjusted_index));
    }

    // Create help text
    let help_text = vec![
        Spans::from(Span::raw("Enter - View selected image")),
        Spans::from(Span::raw("d - Download selected image")),
        Spans::from(Span::raw("Delete - Delete selected image")),
        Spans::from(Span::raw("r - Refresh image list")),
        Spans::from(Span::raw("Esc - Return to main menu")),
    ];

    // Split area for list and help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(5)].as_ref())
        .split(area);

    // Render the image list
    frame.render_stateful_widget(images_list, chunks[0], &mut list_state);

    // Render help
    let help =
        Paragraph::new(help_text).block(Block::default().title("Controls").borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}

/// Render the download confirmation screen
fn render_download_screen<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Get the selected image
    let image = match state.selected_image() {
        Some(img) => img,
        None => "No image selected",
    };

    // Create confirmation text
    let confirmation_text = vec![
        Spans::from(Span::styled(
            "Download Confirmation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Spans::from(Span::raw("")),
        Spans::from(Span::raw(format!("Download the image: {}", image))),
        Spans::from(Span::raw(
            "The image will be saved to the 'downloads' directory.",
        )),
        Spans::from(Span::raw("")),
        Spans::from(Span::styled(
            "Press Enter to confirm or Esc to cancel",
            Style::default().fg(Color::Yellow),
        )),
    ];

    // Create confirmation dialog
    let confirmation = Paragraph::new(confirmation_text)
        .block(Block::default().title("Download").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    frame.render_widget(confirmation, area);
}

/// Render the delete confirmation screen
fn render_delete_screen<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Get the selected image
    let image = match state.selected_image() {
        Some(img) => img,
        None => "No image selected",
    };

    // Create warning text
    let warning_text = vec![
        Spans::from(Span::styled(
            "Delete Confirmation",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Spans::from(Span::raw("")),
        Spans::from(Span::raw(format!(
            "Are you sure you want to delete: {}",
            image
        ))),
        Spans::from(Span::styled(
            "This action cannot be undone!",
            Style::default().fg(Color::Red),
        )),
        Spans::from(Span::raw("")),
        Spans::from(Span::styled(
            "Press Enter to confirm or Esc to cancel",
            Style::default().fg(Color::Yellow),
        )),
        Spans::from(Span::raw("")),
        Spans::from(Span::raw(
            "Note: Some Olympus cameras do not support deleting images via WiFi.",
        )),
    ];

    // Create delete dialog
    let warning = Paragraph::new(warning_text)
        .block(Block::default().title("Delete").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    frame.render_widget(warning, area);
}

/// Render status bar
fn render_status<B: Backend>(state: &AppState, frame: &mut Frame<B>, area: Rect) {
    // Create status bar
    let status = Paragraph::new(Spans::from(vec![Span::styled(
        &state.status,
        Style::default().add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status, area);
}
