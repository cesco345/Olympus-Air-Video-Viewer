use anyhow::{Result, anyhow};
use log::info;
use std::thread;
use std::time::Duration;

use crate::camera::client::basic::ClientOperations;

/// Image deletion functionality
pub trait ImageDeleter: ClientOperations {
    /// Delete an image from the camera - alternative approach
    fn delete_image(&self, image_name: &str) -> Result<()> {
        info!("Deleting image: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Try methods in succession with different approaches

        // APPROACH 1: Switch to playback mode before trying to delete
        info!("APPROACH 1: Switch to playback mode first");
        let play_mode_url = format!("{}switch_cameramode.cgi?mode=play", self.base_url());

        match self
            .client()
            .get(&play_mode_url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()
        {
            Ok(response) => {
                info!("Switch to play mode response: {}", response.status());
                thread::sleep(Duration::from_secs(1)); // Give camera time to change modes
            }
            Err(e) => {
                info!("Failed to switch to play mode: {}", e);
            }
        }

        // APPROACH 2: Try standard delete URL
        info!("APPROACH 2: Standard delete URL");
        let delete_url = format!(
            "{}exec_erase.cgi?DIR=/DCIM/100OLYMP&FILE={}",
            self.base_url(),
            image_name
        );

        match self
            .client()
            .get(&delete_url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()
        {
            Ok(response) => {
                info!("Delete response status: {}", response.status());
                if response.status().is_success() {
                    match response.text() {
                        Ok(text) => {
                            if !text.contains("WIFI_INTERNAL_ERROR") {
                                info!("Delete successful with APPROACH 2");
                                return Ok(());
                            } else {
                                info!("WIFI_INTERNAL_ERROR detected in APPROACH 2");
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                info!("Delete request failed in APPROACH 2: {}", e);
            }
        }

        // APPROACH 3: Try alternative delete URL format
        info!("APPROACH 3: Alternative delete URL format");
        let alt_delete_url = format!(
            "{}exec_erase.cgi?com=exec&DIR=/DCIM/100OLYMP&FILE={}",
            self.base_url(),
            image_name
        );

        match self
            .client()
            .get(&alt_delete_url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()
        {
            Ok(response) => {
                info!(
                    "Delete response status for APPROACH 3: {}",
                    response.status()
                );
                if response.status().is_success() {
                    match response.text() {
                        Ok(text) => {
                            if !text.contains("WIFI_INTERNAL_ERROR") {
                                info!("Delete successful with APPROACH 3");
                                return Ok(());
                            } else {
                                info!("WIFI_INTERNAL_ERROR detected in APPROACH 3");
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                info!("Delete request failed in APPROACH 3: {}", e);
            }
        }

        // APPROACH 4: Try direct file path approach
        info!("APPROACH 4: Try direct file path approach");
        let direct_url = format!(
            "{}exec_erase.cgi?DIR=/DCIM/100OLYMP/{}",
            self.base_url(),
            image_name
        );

        match self
            .client()
            .get(&direct_url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()
        {
            Ok(response) => {
                info!(
                    "Delete response status for APPROACH 4: {}",
                    response.status()
                );
                if response.status().is_success() {
                    match response.text() {
                        Ok(text) => {
                            if !text.contains("WIFI_INTERNAL_ERROR") {
                                info!("Delete successful with APPROACH 4");
                                return Ok(());
                            } else {
                                info!("WIFI_INTERNAL_ERROR detected in APPROACH 4");
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                info!("Delete request failed in APPROACH 4: {}", e);
            }
        }

        // If all the above approaches failed, return error with guidance
        return Err(anyhow!(
            "Camera does not support deletion via WiFi. Please try:\n1. Using a different mode on the camera\n2. Using the camera's built-in delete function\n3. Formatting the card in the camera"
        ));
    }
}
