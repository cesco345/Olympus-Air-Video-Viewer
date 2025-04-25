// src/terminal/image_viewer/renderer/detection.rs
use log::info;

/// Terminal capabilities information
pub struct TerminalCapabilities {
    /// Supports Kitty graphics protocol
    pub supports_kitty: bool,
    /// Supports iTerm2 graphics protocol
    pub supports_iterm: bool,
    /// Supports SIXEL graphics
    pub supports_sixel: bool,
}

/// Detect terminal capabilities for image display
pub fn detect_terminal_capabilities() -> TerminalCapabilities {
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

    TerminalCapabilities {
        supports_kitty,
        supports_iterm,
        supports_sixel,
    }
}
