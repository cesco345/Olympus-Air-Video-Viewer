use anyhow::Result;
use log::info;
use reqwest::blocking::Client;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::camera::client::basic::ClientOperations;
use crate::camera::client::error::ErrorHandler;
use crate::camera::connection::init::ConnectionManager;
use crate::camera::image::delete::ImageDeleter;
use crate::camera::image::download::ImageDownloader;
use crate::camera::image::list::ImageLister;
use crate::camera::photo::capture::PhotoCapture;

/// Main camera client for Olympus Air
pub struct OlympusCamera {
    pub base_url: String,
    pub client: Client,
    pub connected: Arc<AtomicBool>,
}

impl OlympusCamera {
    /// Create a new camera client
    pub fn new(base_url: &str) -> Self {
        // Ensure URL ends with trailing slash
        let base_url = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{}/", base_url)
        };

        info!("Creating camera client with base URL: {}", base_url);

        // Create HTTP client with longer timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(30)) // Increase timeout
            .build()
            .unwrap_or_else(|e| {
                info!(
                    "Failed to create custom client: {}. Using default client.",
                    e
                );
                Client::new()
            });

        Self {
            base_url,
            client,
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Clone the camera for thread safety
    pub fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            connected: Arc::clone(&self.connected),
        }
    }
}

// Implement core client operations
impl ClientOperations for OlympusCamera {
    fn client(&self) -> &Client {
        &self.client
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

// Implement error handling
impl ErrorHandler for OlympusCamera {}

// Implement connection management
impl ConnectionManager for OlympusCamera {
    fn connected(&self) -> &Arc<AtomicBool> {
        &self.connected
    }
}

// Implement image listing
impl ImageLister for OlympusCamera {}

// Implement image downloading
impl ImageDownloader for OlympusCamera {}

// Implement image deletion
impl ImageDeleter for OlympusCamera {}

// Implement photo capture
impl PhotoCapture for OlympusCamera {
    // We need to implement this method for PhotoCapture
    fn get_image_list(&self) -> Result<Vec<String>> {
        // Since we already implemented ImageLister, we can just call that implementation
        ImageLister::get_image_list(self)
    }
}
