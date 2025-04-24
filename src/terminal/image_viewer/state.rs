// src/terminal/image_viewer/state.rs
use std::path::PathBuf;

/// Available display methods for images
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMethod {
    /// Automatically select the best available method
    Auto,
    /// Use kitty graphics protocol
    Kitty,
    /// Use iTerm2 graphics protocol
    ITerm,
    /// Use SIXEL graphics
    Sixel,
    /// Use basic terminal rendering (ASCII art)
    Basic,
}

impl Default for DisplayMethod {
    fn default() -> Self {
        DisplayMethod::Auto
    }
}

/// State for the image viewer mode
pub struct ImageViewerState {
    /// Path to the image file
    pub image_path: PathBuf,

    /// Name of the image
    pub image_name: String,

    /// Current zoom factor
    pub zoom_factor: f32,

    /// Whether to preserve aspect ratio
    pub preserve_aspect: bool,

    /// Preferred display method
    pub display_method: DisplayMethod,
}

impl ImageViewerState {
    /// Create a new image viewer state
    pub fn new(image_path: PathBuf, image_name: &str) -> Self {
        Self {
            image_path,
            image_name: image_name.to_string(),
            zoom_factor: 1.0,
            preserve_aspect: true,
            display_method: DisplayMethod::default(),
        }
    }

    /// Create a new image viewer state with specific display method
    pub fn with_display_method(
        image_path: PathBuf,
        image_name: &str,
        method: DisplayMethod,
    ) -> Self {
        Self {
            image_path,
            image_name: image_name.to_string(),
            zoom_factor: 1.0,
            preserve_aspect: true,
            display_method: method,
        }
    }

    /// Zoom in
    pub fn zoom_in(&mut self) {
        self.zoom_factor += 0.1;
        if self.zoom_factor > 3.0 {
            self.zoom_factor = 3.0;
        }
    }

    /// Zoom out
    pub fn zoom_out(&mut self) {
        self.zoom_factor -= 0.1;
        if self.zoom_factor < 0.1 {
            self.zoom_factor = 0.1;
        }
    }

    /// Reset zoom
    pub fn reset_zoom(&mut self) {
        self.zoom_factor = 1.0;
    }

    /// Toggle aspect ratio preservation
    pub fn toggle_aspect_ratio(&mut self) {
        self.preserve_aspect = !self.preserve_aspect;
    }

    /// Cycle through display methods
    pub fn cycle_display_method(&mut self) {
        self.display_method = match self.display_method {
            DisplayMethod::Auto => DisplayMethod::Kitty,
            DisplayMethod::Kitty => DisplayMethod::ITerm,
            DisplayMethod::ITerm => DisplayMethod::Sixel,
            DisplayMethod::Sixel => DisplayMethod::Basic,
            DisplayMethod::Basic => DisplayMethod::Auto,
        };
    }

    /// Get display method name as string
    pub fn display_method_name(&self) -> &'static str {
        match self.display_method {
            DisplayMethod::Auto => "Auto",
            DisplayMethod::Kitty => "Kitty",
            DisplayMethod::ITerm => "iTerm2",
            DisplayMethod::Sixel => "SIXEL",
            DisplayMethod::Basic => "Basic",
        }
    }

    /// Calculate dimensions for display based on zoom factor
    pub fn calculate_dimensions(&self, term_width: u32, term_height: u32) -> (u32, u32) {
        // Calculate available display area (accounting for margins)
        let available_width = term_width.saturating_sub(4);
        let available_height = term_height.saturating_sub(6);

        // Apply zoom factor
        let width = (available_width as f32 * self.zoom_factor) as u32;
        let height = (available_height as f32 * self.zoom_factor) as u32;

        // Ensure minimum size
        let width = width.max(10);
        let height = height.max(5);

        (width, height)
    }
}
