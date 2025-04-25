// src/terminal/state.rs
use crate::camera::client::basic::ClientOperations;
use crate::camera::connection::init::ConnectionManager;
use crate::camera::image::download::ImageDownloader;
use crate::camera::image::list::ImageLister;
use crate::camera::olympus::OlympusCamera;
use crate::terminal::image_viewer::state::ImageViewerState;
use crate::terminal::video_viewer::state::VideoViewerState;
use anyhow::{Result, anyhow};
use log::{error, info, warn};
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Different application states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Main,
    ImageList,
    Downloading,
    Deleting,
    ViewingImage,
    ViewingVideo,
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

    /// Video viewer state (when in video viewing mode)
    pub video_viewer: Option<VideoViewerState>,

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
            video_viewer: None,
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

    /// Function to retry a request with backoff
    fn retry_with_backoff<F, T, E>(&self, mut operation: F, max_retries: usize) -> Result<T>
    where
        F: FnMut() -> std::result::Result<T, E>,
        E: std::fmt::Display,
    {
        let mut retries = 0;
        let mut last_error = None;

        while retries < max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(format!("{}", e));
                    retries += 1;
                    let delay = Duration::from_millis(500 * 2u64.pow(retries as u32));
                    info!(
                        "Request failed, retrying in {:?}... (attempt {}/{})",
                        delay, retries, max_retries
                    );
                    thread::sleep(delay);
                }
            }
        }

        Err(anyhow!(
            "Operation failed after {} retries. Last error: {}",
            max_retries,
            last_error.unwrap_or_default()
        ))
    }

    /// Verify camera connection and reconnect if needed
    fn ensure_camera_connected(&mut self) -> Result<()> {
        if !self
            .camera
            .connected
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            info!("Camera connection inactive, attempting to reconnect");
            match self.camera.connect() {
                Ok(_) => {
                    info!("Successfully reconnected to camera");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to reconnect to camera: {}", e);
                    Err(anyhow!("Camera connection lost: {}", e))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Explore camera API for debugging
    pub fn explore_camera_api(&self) -> Result<()> {
        info!("🔍 Beginning camera API exploration");

        // Basic endpoints that most cameras support
        let basic_endpoints = [
            "",
            "get_state.cgi",
            "get_imglist.cgi?DIR=/DCIM/100OLYMP",
            "get_capability.cgi",
            "get_connectmode.cgi",
            "exec_takemisc.cgi?com=getdevicestatus",
        ];

        // Additional endpoints to try for image access
        let image_endpoints = [
            "DCIM",
            "DCIM/100OLYMP",
            "DCIM/",
            "/DCIM/100OLYMP",
            "get_imglist.cgi",
            "get_imglist.cgi?DIR=/DCIM",
        ];

        // Try all basic endpoints first
        info!("Testing basic camera endpoints...");
        for endpoint in basic_endpoints.iter() {
            info!("Trying endpoint: {}", endpoint);
            match self.camera.get_page(endpoint) {
                Ok(_) => info!("✅ Endpoint {} succeeded", endpoint),
                Err(e) => info!("❌ Endpoint {} failed: {}", endpoint, e),
            }
            // Add delay between requests
            thread::sleep(Duration::from_millis(500));
        }

        // Try image-specific endpoints
        info!("Testing image-related endpoints...");
        for endpoint in image_endpoints.iter() {
            info!("Trying endpoint: {}", endpoint);
            match self.camera.get_page(endpoint) {
                Ok(_) => info!("✅ Endpoint {} succeeded", endpoint),
                Err(e) => info!("❌ Endpoint {} failed: {}", endpoint, e),
            }
            // Add delay between requests
            thread::sleep(Duration::from_millis(500));
        }

        // If we have images, try to get the first one with different URL formats
        if !self.images.is_empty() {
            let test_image = &self.images[0];
            info!("Testing image access for: {}", test_image);

            // Test different image access URLs
            let image_urls = [
                format!(
                    "get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                    test_image
                ),
                format!(
                    "get_thumbnail.cgi?DIR=DCIM/100OLYMP&FILE={}&size=1024",
                    test_image
                ),
                format!("get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}", test_image),
                format!("DCIM/100OLYMP/{}", test_image),
            ];

            for (i, url) in image_urls.iter().enumerate() {
                info!("Trying image URL format #{}: {}", i + 1, url);
                match self.camera.get_binary(url) {
                    Ok(data) => {
                        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
                            info!(
                                "✅ Successfully loaded image with format #{} ({} bytes - valid JPEG)",
                                i + 1,
                                data.len()
                            );
                        } else {
                            info!(
                                "⚠️ Got response with format #{} but not valid JPEG ({} bytes)",
                                i + 1,
                                data.len()
                            );
                        }
                    }
                    Err(e) => info!("❌ Failed with URL format #{}: {}", i + 1, e),
                }
                // Add delay between requests
                thread::sleep(Duration::from_millis(500));
            }
        }

        info!("🔍 Camera API exploration complete");
        Ok(())
    }

    /// View the currently selected image with enhanced debugging
    pub fn view_selected_image(&mut self) -> Result<()> {
        // Check if we have images and a valid selection
        if self.images.is_empty() || self.selected_index >= self.images.len() {
            return Err(anyhow!("No image selected or invalid selection"));
        }

        // Clone the image name to avoid borrow issues
        let image_name = self.images[self.selected_index].clone();
        info!("Attempting to load image: {}", image_name);

        // Ensure camera is connected
        self.ensure_camera_connected()?;

        // Update status with the cloned name
        self.set_status(&format!(
            "Loading image: {} (Trying multiple formats...)",
            image_name
        ));

        // Try different URL formats
        let url_formats = self.generate_url_formats(&image_name);

        // Log all formats we'll try
        for (i, url) in url_formats.iter().enumerate() {
            info!("URL format #{}: {}", i + 1, url);
        }

        // Try each URL format with retries
        for (i, url) in url_formats.iter().enumerate() {
            info!("🔍 Trying URL format #{}: {}", i + 1, url);
            self.set_status(&format!(
                "Loading image: {} (Trying format #{}/{})",
                image_name,
                i + 1,
                url_formats.len()
            ));

            // Use retry logic
            let result = self.retry_with_backoff(|| self.camera.get_binary(url), 2);

            match result {
                Ok(image_data) => {
                    info!(
                        "✅ Successfully loaded image with format #{}: {}",
                        i + 1,
                        url
                    );
                    info!("Image data size: {} bytes", image_data.len());

                    // Verify image data is valid
                    if image_data.len() < 100 || !self.check_image_valid(&image_data) {
                        warn!(
                            "⚠️ Received data doesn't look like a valid image (size: {} bytes)",
                            image_data.len()
                        );
                        continue;
                    }

                    // Create image viewer with original URL for high-res loading
                    info!("Creating image viewer with URL: {}", url);
                    crate::terminal::image_viewer::handlers::create_image_viewer_with_url(
                        self,
                        image_data,
                        &image_name,
                        Some(url.clone()),
                    )?;

                    self.set_status(&format!(
                        "Image loaded successfully using format #{}",
                        i + 1
                    ));
                    return Ok(());
                }
                Err(e) => {
                    warn!("❌ Failed with URL format #{}: {}", i + 1, e);
                    // Continue to next format
                }
            }
        }

        // If all formats failed, try to download the full image directly
        info!("All thumbnail formats failed, trying direct image download");
        self.set_status(&format!(
            "All thumbnail formats failed, trying direct image download"
        ));

        match self.try_load_direct_image(&image_name) {
            Ok(_) => {
                info!("✅ Successfully loaded image directly");
                self.set_status(&format!("Image loaded successfully (direct method)"));
                return Ok(());
            }
            Err(e) => {
                error!("❌ Failed to load image directly: {}", e);
                // Fall through to error
            }
        }

        // If all approaches failed, show error and suggest exploration
        self.set_status(&format!("Failed to load image: All URL formats failed"));
        self.set_error_message(
            "Image Loading Failed", 
            &format!("Failed to load image {} after trying multiple formats.\n\nTry exploring the camera API or refreshing the image list.", image_name)
        );
        self.set_show_error_dialog(true);

        // Suggest API exploration
        info!("Suggesting API exploration. Try calling explore_camera_api() for more info.");

        Err(anyhow!("Failed to load image: All URL formats failed"))
    }

    /// Generate various URL formats to try
    fn generate_url_formats(&self, image_name: &str) -> Vec<String> {
        vec![
            // Format 1: Standard thumbnail format
            format!(
                "get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                image_name
            ),
            // Format 2: Without leading slash in DIR
            format!(
                "get_thumbnail.cgi?DIR=DCIM/100OLYMP&FILE={}&size=1024",
                image_name
            ),
            // Format 3: Without DIR parameter
            format!("get_thumbnail.cgi?FILE={}&size=1024", image_name),
            // Format 4: Direct path
            format!("DCIM/100OLYMP/{}", image_name),
            // Format 5: Using get_img.cgi instead
            format!("get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}", image_name),
            // Format 6: Using get_img.cgi without leading slash
            format!("get_img.cgi?DIR=DCIM/100OLYMP&FILE={}", image_name),
            // Format 7: Using get_resized_img.cgi
            format!(
                "get_resized_img.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                image_name
            ),
            // Format 8: Alternative path structure
            format!("get_img.cgi?PATH=/DCIM/100OLYMP/{}", image_name),
            // Format 9: With uppercase filename
            format!(
                "get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                image_name.to_uppercase()
            ),
            // Format 10: With lowercase path
            format!(
                "get_thumbnail.cgi?DIR=/dcim/100olymp&FILE={}&size=1024",
                image_name
            ),
        ]
    }

    /// Try to load image directly
    fn try_load_direct_image(&mut self, image_name: &str) -> Result<()> {
        // Get the full image list first to confirm existence
        info!("Refreshing image list to confirm image existence");
        let images = self.camera.get_image_list()?;

        // Check if the image exists in the camera's list
        if !images.contains(&image_name.to_string()) {
            warn!("Image {} not found in camera's list", image_name);
            return Err(anyhow!("Image {} not found in camera's list", image_name));
        }

        // Try direct access with multiple formats
        let direct_formats = [
            format!("DCIM/100OLYMP/{}", image_name),
            format!("/DCIM/100OLYMP/{}", image_name),
            format!("get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}", image_name),
        ];

        for (i, url) in direct_formats.iter().enumerate() {
            info!("Trying direct format #{}: {}", i + 1, url);
            match self.camera.get_binary(url) {
                Ok(image_data) => {
                    info!("Successfully loaded image directly (format #{})", i + 1);

                    // Verify image data
                    if image_data.len() < 100 || !self.check_image_valid(&image_data) {
                        warn!(
                            "Received data doesn't look like a valid image (size: {} bytes)",
                            image_data.len()
                        );
                        continue;
                    }

                    // Create image viewer without high-res URL since this is already the direct image
                    crate::terminal::image_viewer::handlers::create_image_viewer(
                        self, image_data, image_name,
                    )?;
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed with direct format #{}: {}", i + 1, e);
                    // Continue to next format
                }
            }
        }

        Err(anyhow!("Failed to load image directly with all formats"))
    }

    /// Check if data appears to be a valid image
    fn check_image_valid(&self, data: &[u8]) -> bool {
        // Simple check for JPEG header
        if data.len() >= 2 {
            // JPEG files start with FF D8
            return data[0] == 0xFF && data[1] == 0xD8;
        }
        false
    }

    /// Refresh the image list with better error handling
    pub fn refresh_images(&mut self) -> Result<()> {
        self.set_status("Refreshing image count...");

        // Ensure camera connection
        self.ensure_camera_connected()?;

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
        info!(
            "Mode changed to {:?}, selected_index={}",
            mode, self.selected_index
        );
    }

    /// Get the maximum index for the current mode
    pub fn get_max_index(&self) -> usize {
        match self.mode {
            AppMode::Main => 3, // Updated for new menu items
            AppMode::ImageList => self.images.len().saturating_sub(1),
            AppMode::Downloading
            | AppMode::Deleting
            | AppMode::ViewingImage
            | AppMode::ViewingVideo => 0,
        }
    }

    /// Move the selection up
    pub fn selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            info!("Selection moved up to index: {}", self.selected_index);

            // Update page if selection moves outside current page
            if self.mode == AppMode::ImageList {
                let start_idx = self.page_start_index();
                if self.selected_index < start_idx {
                    self.current_page_index = self.current_page_index.saturating_sub(1);
                    info!("Page moved up to: {}", self.current_page_index);
                }
            }
        }
    }

    /// Move the selection down
    pub fn selection_down(&mut self) {
        let max = self.get_max_index();
        if self.selected_index < max {
            self.selected_index += 1;
            info!("Selection moved down to index: {}", self.selected_index);

            // Update page if selection moves outside current page
            if self.mode == AppMode::ImageList {
                let end_idx = self.page_end_index();
                if self.selected_index >= end_idx {
                    if self.current_page_index < self.total_pages().saturating_sub(1) {
                        self.current_page_index += 1;
                        info!("Page moved down to: {}", self.current_page_index);
                    }
                }
            }
        }
    }

    /// Move to the next page
    pub fn next_page(&mut self) {
        if self.current_page_index < self.total_pages().saturating_sub(1) {
            self.current_page_index += 1;
            info!("Page moved to: {}", self.current_page_index);

            // Update selected index to first item on new page
            self.selected_index = self.page_start_index();
            info!("Selection set to start of page: {}", self.selected_index);
        }
    }

    /// Move to the previous page
    pub fn prev_page(&mut self) {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
            info!("Page moved to: {}", self.current_page_index);

            // Update selected index to first item on new page
            self.selected_index = self.page_start_index();
            info!("Selection set to start of page: {}", self.selected_index);
        }
    }

    /// Jump to first image
    pub fn first_image(&mut self) {
        self.selected_index = 0;
        self.current_page_index = 0;
        info!("Selection set to first image: index=0, page=0");
    }

    /// Jump to last image
    pub fn last_image(&mut self) {
        if !self.images.is_empty() {
            self.selected_index = self.images.len() - 1;
            self.current_page_index = self.total_pages().saturating_sub(1);
            info!(
                "Selection set to last image: index={}, page={}",
                self.selected_index, self.current_page_index
            );
        }
    }

    /// Get the currently selected image, if any
    pub fn selected_image(&self) -> Option<&str> {
        // Make sure index is valid
        if self.images.is_empty() || self.selected_index >= self.images.len() {
            warn!(
                "Invalid selection index: {}, images count: {}",
                self.selected_index,
                self.images.len()
            );
            None
        } else {
            // Important: Get image directly from the array using the index
            let selected = &self.images[self.selected_index];
            info!(
                "Getting selected image: index={}, image={}",
                self.selected_index, selected
            );
            Some(selected)
        }
    }

    /// Get the starting index for the current page
    pub fn page_start_index(&self) -> usize {
        let start = self.current_page_index * self.items_per_page;
        info!("Page start index: {}", start);
        start
    }

    /// Get the ending index (exclusive) for the current page
    pub fn page_end_index(&self) -> usize {
        let start = self.page_start_index();
        let end = start + self.items_per_page;
        let actual_end = end.min(self.images.len());
        info!(
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
