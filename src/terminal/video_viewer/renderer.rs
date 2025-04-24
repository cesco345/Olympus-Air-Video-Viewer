// src/terminal/video_viewer/renderer.rs
use crate::terminal::video_viewer::state::VideoViewerState;
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the video viewer interface
pub fn render<B: Backend>(viewer_state: &VideoViewerState, frame: &mut Frame<B>, area: Rect) {
    // Split area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Video area
            Constraint::Length(3), // Controls
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    // Render title
    let title = Paragraph::new(vec![Spans::from(vec![Span::styled(
        format!("Olympus Video Viewer - {}", viewer_state.stream_name),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, chunks[0]);

    // Render video info area
    let stream_status = if viewer_state.is_playing {
        "Playing"
    } else {
        "Paused"
    };

    let recording_status = if viewer_state.is_recording {
        "Recording"
    } else {
        "Not Recording"
    };

    // Get statistics
    let (packets, frames, frame_size) = viewer_state.get_statistics();
    let time_since_last_frame = viewer_state.get_time_since_last_frame();
    let frame_rate = if time_since_last_frame.as_secs() > 0 {
        0.0
    } else {
        1.0 / time_since_last_frame.as_millis() as f64 * 1000.0
    };

    // Format stats with colors based on health
    let health_status = if time_since_last_frame.as_secs() < 1 {
        Span::styled("Good", Style::default().fg(Color::Green))
    } else if time_since_last_frame.as_secs() < 5 {
        Span::styled("Degraded", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("Poor/Stalled", Style::default().fg(Color::Red))
    };

    let health_text = Spans::from(vec![Span::raw("Stream Health: "), health_status]);

    // Create full video info content
    let video_content = vec![
        Spans::from(vec![Span::styled(
            "Olympus UDP stream is displayed in a separate player window.",
            Style::default().fg(Color::Yellow),
        )]),
        Spans::from(vec![Span::raw(
            "Use the controls below to manage the stream.",
        )]),
        Spans::from(vec![Span::raw(format!(
            "Stream URL: {}",
            viewer_state.generate_stream_url()
        ))]),
        Spans::from(vec![Span::raw(format!(
            "Status: {} | {} | UDP Port: {}",
            stream_status, recording_status, viewer_state.udp_port
        ))]),
        health_text,
        Spans::from(vec![Span::raw(format!(
            "Statistics: {} packets, {} frames, {:.1} FPS",
            packets, frames, frame_rate
        ))]),
        Spans::from(vec![Span::raw(format!(
            "Last frame: {} KB, received {:.1}s ago",
            frame_size / 1024,
            time_since_last_frame.as_secs_f64()
        ))]),
        Spans::from(vec![Span::raw(format!(
            "Player PID: {}",
            viewer_state
                .external_viewer_pid
                .map_or("None".to_string(), |pid| pid.to_string())
        ))]),
    ];

    let video_area = Paragraph::new(video_content)
        .block(
            Block::default()
                .title("Olympus Live View")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(video_area, chunks[1]);

    // Render controls
    let controls = Paragraph::new(vec![Spans::from(vec![
        Span::styled("Controls: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Enter - Restart stream   "),
        Span::raw("Space - Play/Pause   "),
        Span::raw("d - Diagnostics   "),
        Span::raw("r - Toggle recording   "), // Added recording toggle
        Span::raw("Esc - Return to menu   "),
        Span::raw("q - Quit"),
    ])])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(controls, chunks[2]);

    // Render status bar - show diagnostic info
    let status_text = if time_since_last_frame.as_secs() > 5 {
        "Stream may be stalled. Press Enter to restart stream or d to run diagnostics."
    } else if frames == 0 {
        "Waiting for video data. Check camera connection if this persists."
    } else {
        "Stream active. Press q to quit, Esc to return to menu."
    };

    let status_style = if time_since_last_frame.as_secs() > 5 {
        Style::default().fg(Color::Red)
    } else if frames == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let status_bar = Paragraph::new(Spans::from(Span::styled(status_text, status_style)))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status_bar, chunks[3]);
}
