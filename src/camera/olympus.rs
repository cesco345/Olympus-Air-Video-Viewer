// src/camera/olympus.rs
use anyhow::{Result, anyhow};
use log::{error, info};
use regex::Regex;
use reqwest::blocking::Client;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration; // Add this for file writing

/// Main camera client for Olympus Air
pub struct OlympusCamera {
    base_url: String,
    client: Client,
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

        // Create HTTP client with longer timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(30)) // Increase timeout
            .build()
            .unwrap();

        Self { base_url, client }
    }

    /// Connect to camera with required initialization steps
    pub fn connect(&self) -> Result<()> {
        info!("Connecting to camera...");

        // Simple initialization sequence from the working warm_up_photo.rs
        let steps = [
            "get_connectmode.cgi",
            "switch_cameramode.cgi?mode=rec",
            "get_state.cgi",
            "exec_takemisc.cgi?com=startliveview&port=5555", // Critical step from working example
        ];

        for step in &steps {
            info!("Connection step: {}", step);
            self.get_page(step)?;
            thread::sleep(Duration::from_millis(100));
        }

        info!("Camera connected successfully");
        Ok(())
    }

    /// Take a photo with warm-up approach
    pub fn take_photo(&self) -> Result<()> {
        info!("Taking a photo with warm-up sequence");

        // First make sure camera is connected
        self.connect()?;

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
                error!("Failed to verify new images: {}", e);
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
        let url = format!("{}exec_takemotion.cgi?com=newstarttake", self.base_url);

        // Send the request with exact headers from working example
        let response = self
            .client
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        // Log but don't check status (matching working code behavior)
        info!("Photo command sent with status: {}", response.status());

        Ok(())
    }

    /// Get a list of images on the camera
    pub fn get_image_list(&self) -> Result<Vec<String>> {
        info!("Getting list of images");

        let url = format!("{}get_imglist.cgi?DIR=/DCIM/100OLYMP", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        let text = response.text()?;

        // Use both regex patterns to find all image files (matching working code)
        let re1 = Regex::new(r"P\w\d+\.JPG").unwrap();
        let re2 = Regex::new(r"P.\d+\.JPG").unwrap();

        let mut filenames = Vec::new();

        // Add matches from both patterns
        filenames.extend(re1.find_iter(&text).map(|m| m.as_str().to_string()));
        filenames.extend(re2.find_iter(&text).map(|m| m.as_str().to_string()));

        // Remove duplicates
        filenames.sort();
        filenames.dedup();

        info!("Found {} images", filenames.len());
        Ok(filenames)
    }

    /// Make a simple GET request to the camera
    pub fn get_page(&self, endpoint: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, endpoint);
        info!("Request: {}", url);

        // Send request with exact headers that work
        let response = self
            .client
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        // Log but don't validate status code (matching working code behavior)
        info!("Response status: {}", response.status());
        Ok(())
    }

    /// Download an image from the camera to the local file system
    /// This version uses a more direct approach with multiple retries
    pub fn download_image(&self, image_name: &str, destination: &Path) -> Result<()> {
        info!("Downloading image: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Set of URLs to try (from most likely to least likely)
        let urls = [
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url, image_name
            ),
            format!("{}DCIM/100OLYMP/{}", self.base_url, image_name),
            format!(
                "{}get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url, image_name
            ),
        ];

        // Initialize response for use outside the loop
        let mut final_response = None;
        let mut success = false;

        // Try each URL
        for url in &urls {
            info!("Trying download URL: {}", url);

            // Get image data
            match self
                .client
                .get(url)
                .header("user-agent", "OlympusCameraKit")
                .header("content-length", "4096")
                .send()
            {
                Ok(response) => {
                    info!("Download response status: {}", response.status());

                    if response.status().is_success() {
                        final_response = Some(response);
                        success = true;
                        break;
                    }
                }
                Err(e) => {
                    info!("Download request failed: {}", e);
                    continue;
                }
            }
        }

        // If all URLs failed, try a more direct approach with chunked download
        if !success {
            info!("All standard URLs failed, trying direct download approach");

            // Try direct URL for image
            let direct_url = format!("{}DCIM/100OLYMP/{}", self.base_url, image_name);

            match self
                .client
                .get(&direct_url)
                .header("user-agent", "OlympusCameraKit")
                .header("accept", "image/jpeg,*/*")
                .header("content-length", "4096")
                .send()
            {
                Ok(response) => {
                    info!("Direct download response status: {}", response.status());

                    if response.status().is_success() {
                        final_response = Some(response);
                        success = true;
                    }
                }
                Err(e) => {
                    info!("Direct download request failed: {}", e);
                }
            }
        }

        // If we have a successful response, save the image
        if success && final_response.is_some() {
            let response = final_response.unwrap();

            // Create parent directories if they don't exist
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }

            // Get the bytes and write to file
            match response.bytes() {
                Ok(bytes) => {
                    info!("Received {} bytes of image data", bytes.len());

                    // Check if it looks like an image (JPGs start with FFD8)
                    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
                        info!("WARNING: Downloaded data doesn't appear to be a JPEG image");
                    }

                    // Manual file writing to ensure proper handling
                    let mut file = std::fs::File::create(destination)?;
                    file.write_all(&bytes)?;
                    file.flush()?;

                    info!("Image saved to: {:?}", destination);
                    return Ok(());
                }
                Err(e) => {
                    return Err(anyhow!("Failed to get image bytes: {}", e));
                }
            }
        }

        return Err(anyhow!("Failed to download image after multiple attempts"));
    }

    /// Delete an image from the camera - alternative approach
    pub fn delete_image(&self, image_name: &str) -> Result<()> {
        info!("Deleting image: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Try methods in succession with different approaches

        // APPROACH 1: Switch to playback mode before trying to delete
        info!("APPROACH 1: Switch to playback mode first");
        let play_mode_url = format!("{}switch_cameramode.cgi?mode=play", self.base_url);

        match self
            .client
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
            self.base_url, image_name
        );

        match self
            .client
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
            self.base_url, image_name
        );

        match self
            .client
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
            self.base_url, image_name
        );

        match self
            .client
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
    pub fn get_image_data(&self, image_name: &str) -> Result<Vec<u8>> {
        info!("Getting image data for: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Set of URLs to try (from most likely to least likely)
        let urls = [
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url, image_name
            ),
            format!("{}DCIM/100OLYMP/{}", self.base_url, image_name),
            format!(
                "{}get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url, image_name
            ),
        ];

        // Try each URL
        for url in &urls {
            info!("Trying image data URL: {}", url);

            // Get image data
            match self
                .client
                .get(url)
                .header("user-agent", "OlympusCameraKit")
                .header("content-length", "4096")
                .send()
            {
                Ok(response) => {
                    info!("Image data response status: {}", response.status());

                    if response.status().is_success() {
                        // Get the bytes
                        match response.bytes() {
                            Ok(bytes) => {
                                let bytes_vec = bytes.to_vec();
                                info!("Received {} bytes of image data", bytes_vec.len());

                                // Check if it looks like an image (JPGs start with FFD8)
                                if bytes_vec.len() < 2
                                    || bytes_vec[0] != 0xFF
                                    || bytes_vec[1] != 0xD8
                                {
                                    info!(
                                        "WARNING: Downloaded data doesn't appear to be a JPEG image"
                                    );
                                }

                                return Ok(bytes_vec);
                            }
                            Err(e) => {
                                info!("Failed to get image bytes: {}", e);
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("Image data request failed: {}", e);
                    continue;
                }
            }
        }

        // If all URLs failed, try a more direct approach
        info!("All standard URLs failed, trying direct approach");
        let direct_url = format!("{}DCIM/100OLYMP/{}", self.base_url, image_name);

        match self
            .client
            .get(&direct_url)
            .header("user-agent", "OlympusCameraKit")
            .header("accept", "image/jpeg,*/*")
            .header("content-length", "4096")
            .send()
        {
            Ok(response) => {
                info!("Direct image data response status: {}", response.status());

                if response.status().is_success() {
                    match response.bytes() {
                        Ok(bytes) => {
                            let bytes_vec = bytes.to_vec();
                            info!("Received {} bytes of image data (direct)", bytes_vec.len());
                            return Ok(bytes_vec);
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to get image bytes: {}", e));
                        }
                    }
                }
            }
            Err(e) => {
                info!("Direct image data request failed: {}", e);
            }
        }

        return Err(anyhow!(
            "Failed to download image data after multiple attempts"
        ));
    }
}
