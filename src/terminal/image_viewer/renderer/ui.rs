// src/terminal/image_viewer/renderer/ui.rs
use crate::terminal::image_viewer::state::ImageViewerState;
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

    render_title(viewer_state, frame, chunks[0]);
    render_image_area(viewer_state, frame, chunks[1]);
    render_controls(frame, chunks[2]);
}

/// Render the title section with resolution information
fn render_title<B: Backend>(viewer_state: &ImageViewerState, frame: &mut Frame<B>, area: Rect) {
    // Render title with resolution information
    let resolution_status = if viewer_state.is_high_res_loading {
        format!(
            "(Resolution: {} - Loading higher...)",
            viewer_state.get_resolution_name()
        )
    } else if viewer_state.can_increase_resolution() {
        format!(
            "(Resolution: {} - Press 'r' for higher)",
            viewer_state.get_resolution_name()
        )
    } else {
        format!("(Resolution: {})", viewer_state.get_resolution_name())
    };

    let title = Paragraph::new(vec![
        Spans::from(vec![Span::styled(
            format!("Image Viewer - {}", viewer_state.image_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Spans::from(vec![
            Span::styled(
                format!("Zoom: {:.1}x ", viewer_state.zoom_factor),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                resolution_status,
                Style::default().fg(if viewer_state.is_high_res_loading {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, area);
}

/// Render the image content area
fn render_image_area<B: Backend>(
    viewer_state: &ImageViewerState,
    frame: &mut Frame<B>,
    area: Rect,
) {
    // Render image placeholder
    let image_info = if viewer_state.high_res_data.is_some() {
        "Higher resolution version loaded. Press Enter to view it."
    } else {
        "To view the image, press Enter. The image will be displayed using viuer."
    };

    let image_area = Paragraph::new(vec![
        Spans::from(vec![Span::styled(
            image_info,
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

    frame.render_widget(image_area, area);
}

/// Render the controls section
fn render_controls<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    // Render controls with added resolution control
    let controls = Paragraph::new(vec![Spans::from(vec![
        Span::styled("Controls: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("+/- - Zoom in/out   "),
        Span::raw("0 - Reset zoom   "),
        Span::raw("d - Cycle display modes   "),
        Span::raw("r - Higher resolution   "),
        Span::raw("a - Toggle aspect ratio   "),
        Span::raw("Esc - Return to image list   "),
        Span::raw("q - Quit"),
    ])])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(controls, area);
}
