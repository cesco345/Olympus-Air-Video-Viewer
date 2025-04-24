// src/terminal/state.rs
use crate::camera::olympus::OlympusCamera;
use crate::terminal::image_viewer::state::ImageViewerState;
use anyhow::{Result, anyhow};
use tempfile::NamedTempFile;

/// Different application states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Main,
    ImageList,
    Downloading,
    Deleting,
    ViewingImage, // New mode for image viewing
}

/// Application state
pub struct AppState {
    /// Camera connection
    pub camera: OlympusCamera,

    /// Current application mode
    pub mode: AppMode,

    /// Index of the currently selected item
    pub selected_index: usize,

    /// List of images on the camera
    pub images: Vec<String>,

    /// Status message
    pub status: String,

    /// Images per page (for pagination)
    pub items_per_page: usize,

    /// Current page in image list
    pub current_page_index: usize,

    /// Whether to show error dialog
    pub show_error_dialog: bool,

    /// Error dialog title
    pub error_title: String,

    /// Error dialog message
    pub error_message: String,

    /// Image viewer state (when in viewing mode)
    pub image_viewer: Option<ImageViewerState>,

    /// Temporary file for image viewing (needed to prevent early deletion)
    pub temp_file: Option<NamedTempFile>,
}

impl AppState {
    /// Create a new application state
    pub fn new(camera_url: &str) -> Result<Self> {
        // Create the camera
        let camera = OlympusCamera::new(camera_url);

        // Connect to the camera
        camera.connect()?;

        // Get the image list
        let images = camera.get_image_list()?;

        Ok(Self {
            camera,
            mode: AppMode::Main,
            selected_index: 0,
            images,
            status: "Ready".to_string(),
            items_per_page: 15, // Show 15 items per page
            current_page_index: 0,
            show_error_dialog: false,
            error_title: String::new(),
            error_message: String::new(),
            image_viewer: None,
            temp_file: None,
        })
    }

    /// Set error dialog message
    pub fn set_error_message(&mut self, title: &str, message: &str) {
        self.error_title = title.to_string();
        self.error_message = message.to_string();
    }

    /// Set whether to show the error dialog
    pub fn set_show_error_dialog(&mut self, show: bool) {
        self.show_error_dialog = show;
    }

    /// Dismiss the error dialog
    pub fn dismiss_error_dialog(&mut self) {
        self.show_error_dialog = false;
    }

    /// Update the status message
    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    /// View the currently selected image
    pub fn view_selected_image(&mut self) -> Result<()> {
        // Get the selected image by index
        let selected_idx = self.selected_index;

        if !self.images.is_empty() && selected_idx < self.images.len() {
            // Clone the image name to avoid borrowing issues
            let image_name = self.images[selected_idx].clone();

            // Now we can use mutable methods with self
            self.set_status(&format!("Loading image: {}...", image_name));

            // Download the image data
            let image_data = self.camera.get_image_data(&image_name)?;

            // Create image viewer
            crate::terminal::image_viewer::handlers::create_image_viewer(
                self,
                image_data,
                &image_name,
            )?;

            self.set_status(&format!("Viewing image: {}", image_name));

            Ok(())
        } else {
            self.set_status("No image selected");
            Err(anyhow!("No image selected"))
        }
    }

    /// Refresh the image list with better error handling
    pub fn refresh_images(&mut self) -> Result<()> {
        self.set_status("Refreshing image count...");

        match self.camera.get_image_list() {
            Ok(images) => {
                self.images = images;
                self.set_status(&format!("Found {} images", self.images.len()));

                // Reset to first page when refreshing
                self.current_page_index = 0;

                // Update selected index if it's now out of bounds
                if !self.images.is_empty() && self.selected_index >= self.images.len() {
                    self.selected_index = self.images.len() - 1;
                }
            }
            Err(e) => {
                // Handle the error but don't crash
                self.set_status(&format!("Error refreshing images: {}", e));

                // Don't clear existing images list, but let the user know there was an error
                return Err(e);
            }
        }

        Ok(())
    }

    /// Set the application mode
    pub fn set_mode(&mut self, mode: AppMode) {
        // When switching to Download, Delete, or View mode, preserve the selection index
        let preserve_selection = mode == AppMode::Downloading
            || mode == AppMode::Deleting
            || mode == AppMode::ViewingImage;

        self.mode = mode;

        // Only reset selection if we're not going to operation screens
        if !preserve_selection {
            self.selected_index = 0;
        }

        // Always log the mode change for debugging
        log::info!(
            "Mode changed to {:?}, selected_index={}",
            mode,
            self.selected_index
        );
    }

    /// Get the maximum index for the current mode
    pub fn get_max_index(&self) -> usize {
        match self.mode {
            AppMode::Main => 3, // Updated for new menu items
            AppMode::ImageList => self.images.len().saturating_sub(1),
            AppMode::Downloading | AppMode::Deleting | AppMode::ViewingImage => 0,
        }
    }

    /// Move the selection up
    pub fn selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            log::debug!("Selection moved up to index: {}", self.selected_index);

            // Update page if selection moves outside current page
            if self.mode == AppMode::ImageList {
                let start_idx = self.page_start_index();
                if self.selected_index < start_idx {
                    self.current_page_index = self.current_page_index.saturating_sub(1);
                    log::debug!("Page moved up to: {}", self.current_page_index);
                }
            }
        }
    }

    /// Move the selection down
    pub fn selection_down(&mut self) {
        let max = self.get_max_index();
        if self.selected_index < max {
            self.selected_index += 1;
            log::debug!("Selection moved down to index: {}", self.selected_index);

            // Update page if selection moves outside current page
            if self.mode == AppMode::ImageList {
                let end_idx = self.page_end_index();
                if self.selected_index >= end_idx {
                    if self.current_page_index < self.total_pages().saturating_sub(1) {
                        self.current_page_index += 1;
                        log::debug!("Page moved down to: {}", self.current_page_index);
                    }
                }
            }
        }
    }

    /// Move to the next page
    pub fn next_page(&mut self) {
        if self.current_page_index < self.total_pages().saturating_sub(1) {
            self.current_page_index += 1;
            log::debug!("Page moved to: {}", self.current_page_index);

            // Update selected index to first item on new page
            self.selected_index = self.page_start_index();
            log::debug!("Selection set to start of page: {}", self.selected_index);
        }
    }

    /// Move to the previous page
    pub fn prev_page(&mut self) {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
            log::debug!("Page moved to: {}", self.current_page_index);

            // Update selected index to first item on new page
            self.selected_index = self.page_start_index();
            log::debug!("Selection set to start of page: {}", self.selected_index);
        }
    }

    /// Jump to first image
    pub fn first_image(&mut self) {
        self.selected_index = 0;
        self.current_page_index = 0;
        log::debug!("Selection set to first image: index=0, page=0");
    }

    /// Jump to last image
    pub fn last_image(&mut self) {
        if !self.images.is_empty() {
            self.selected_index = self.images.len() - 1;
            self.current_page_index = self.total_pages().saturating_sub(1);
            log::debug!(
                "Selection set to last image: index={}, page={}",
                self.selected_index,
                self.current_page_index
            );
        }
    }

    /// Get the currently selected image, if any
    pub fn selected_image(&self) -> Option<&str> {
        // Make sure index is valid
        if self.images.is_empty() || self.selected_index >= self.images.len() {
            log::warn!(
                "Invalid selection index: {}, images count: {}",
                self.selected_index,
                self.images.len()
            );
            None
        } else {
            // Important: Get image directly from the array using the index
            let selected = &self.images[self.selected_index];
            log::debug!(
                "Getting selected image: index={}, image={}",
                self.selected_index,
                selected
            );
            Some(selected)
        }
    }

    /// Get the starting index for the current page
    pub fn page_start_index(&self) -> usize {
        let start = self.current_page_index * self.items_per_page;
        log::debug!("Page start index: {}", start);
        start
    }

    /// Get the ending index (exclusive) for the current page
    pub fn page_end_index(&self) -> usize {
        let start = self.page_start_index();
        let end = start + self.items_per_page;
        let actual_end = end.min(self.images.len());
        log::debug!(
            "Page end index: {} (min of {} and {})",
            actual_end,
            end,
            self.images.len()
        );
        actual_end
    }

    /// Get the total number of pages
    pub fn total_pages(&self) -> usize {
        if self.images.is_empty() {
            1
        } else {
            (self.images.len() + self.items_per_page - 1) / self.items_per_page
        }
    }
}
