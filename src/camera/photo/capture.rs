use anyhow::Result;
use log::info;
use std::thread;
use std::time::Duration;

use crate::camera::client::basic::ClientOperations;

/// Photo capture functionality
pub trait PhotoCapture: ClientOperations {
    /// Take a photo with warm-up approach
    fn take_photo(&self) -> Result<()> {
        info!("Taking a photo with warm-up sequence");

        // Get existing images before starting
        let existing_images = match self.get_image_list() {
            Ok(images) => images,
            Err(_) => Vec::new(),
        };

        // Take a warm-up photo first
        info!("Taking warm-up photo to initialize camera state");
        self.take_raw_photo()?;

        // Wait for camera to process warm-up
        info!("Waiting 3 seconds after warm-up photo");
        thread::sleep(Duration::from_secs(3));

        // Now take the actual photo
        info!("Taking actual photo");
        self.take_raw_photo()?;

        // Wait for camera to process
        thread::sleep(Duration::from_secs(3));

        // Verify if new images were captured
        match self.get_image_list() {
            Ok(current_images) => {
                let new_images: Vec<_> = current_images
                    .into_iter()
                    .filter(|img| !existing_images.contains(img))
                    .collect();

                let expected_count = 2; // Warm-up photo + actual photo
                if !new_images.is_empty() {
                    info!(
                        "Photo capture successful - captured {} new images (including warm-up shot)",
                        new_images.len()
                    );

                    if new_images.len() != expected_count {
                        info!(
                            "Expected {} photos but found {}",
                            expected_count,
                            new_images.len()
                        );
                    }
                } else {
                    info!("No new images were detected after photo sequence");
                }
            }
            Err(e) => {
                info!("Failed to verify new images: {}", e);
            }
        }

        info!("Photo sequence complete");
        Ok(())
    }

    /// Internal method to take a raw photo
    fn take_raw_photo(&self) -> Result<()> {
        info!("Sending direct photo command to camera");

        // Make sure we're in rec mode
        self.get_page("switch_cameramode.cgi?mode=rec")?;

        // Get state
        self.get_page("get_state.cgi")?;

        // Send the photo command - exact URL that works
        let url = format!("{}exec_takemotion.cgi?com=newstarttake", self.base_url());

        // Send the request with exact headers from working example
        let response = self
            .client()
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        // Log but don't check status
        info!("Photo command sent with status: {}", response.status());

        Ok(())
    }

    /// Get a list of images on the camera - needed for take_photo
    fn get_image_list(&self) -> Result<Vec<String>>;
}
