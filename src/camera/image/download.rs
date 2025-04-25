use anyhow::{Result, anyhow};
use log::info;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::camera::client::basic::ClientOperations;

/// Image downloading functionality
pub trait ImageDownloader: ClientOperations {
    /// Download an image from the camera to the local file system
    fn download_image(&self, image_name: &str, destination: &Path) -> Result<()> {
        info!("Downloading image: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Set of URLs to try (from most likely to least likely)
        let urls = [
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url(),
                image_name
            ),
            format!("{}DCIM/100OLYMP/{}", self.base_url(), image_name),
            format!(
                "{}get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url(),
                image_name
            ),
        ];

        // Try each URL
        for (i, url) in urls.iter().enumerate() {
            info!("Trying download URL #{}: {}", i + 1, url);

            // Get image data
            match self
                .client()
                .get(url)
                .header("user-agent", "OlympusCameraKit")
                .header("content-length", "4096")
                .header("accept", "image/jpeg,*/*")
                .send()
            {
                Ok(response) => {
                    info!("Download response status: {}", response.status());

                    if response.status().is_success() {
                        // Get the bytes and write to file
                        match response.bytes() {
                            Ok(bytes) => {
                                info!("Received {} bytes of image data", bytes.len());
                                let bytes_vec = bytes.to_vec();

                                // Check if it looks like an image (JPGs start with FFD8)
                                if bytes_vec.len() < 2
                                    || bytes_vec[0] != 0xFF
                                    || bytes_vec[1] != 0xD8
                                {
                                    info!(
                                        "WARNING: Downloaded data doesn't appear to be a JPEG image"
                                    );
                                    continue; // Try next URL
                                }

                                // Create parent directories if they don't exist
                                if let Some(parent) = destination.parent() {
                                    fs::create_dir_all(parent)?;
                                }

                                // Manual file writing to ensure proper handling
                                let mut file = std::fs::File::create(destination)?;
                                file.write_all(&bytes_vec)?;
                                file.flush()?;

                                info!("Image saved to: {:?}", destination);
                                return Ok(());
                            }
                            Err(e) => {
                                info!("Failed to get image bytes: {}", e);
                                continue; // Try next URL
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("Download request failed with URL #{}: {}", i + 1, e);
                    continue; // Try next URL
                }
            }
        }

        return Err(anyhow!("Failed to download image after trying all URLs"));
    }

    /// Get image data with enhanced error handling
    fn get_image_data(&self, image_name: &str) -> Result<Vec<u8>> {
        info!("Getting image data for: {}", image_name);

        // Make sure we're getting exactly the requested image file
        let image_name = image_name.trim(); // Remove any trailing/leading whitespace

        // Enhanced set of URLs to try (from most likely to least likely)
        let urls = [
            // Format 1: Get thumbnail with absolute DIR path (most common format)
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                self.base_url(),
                image_name
            ),
            // Format 2: Get thumbnail with relative DIR path
            format!(
                "{}get_thumbnail.cgi?DIR=DCIM/100OLYMP&FILE={}&size=1024",
                self.base_url(),
                image_name
            ),
            // Format 3: Get thumbnail with DIR path without leading '/'
            format!(
                "{}get_thumbnail.cgi?DIR=DCIM/100OLYMP&FILE={}&size=1024",
                self.base_url(),
                image_name
            ),
            // Format 4: Direct path - sometimes this works better
            format!("{}DCIM/100OLYMP/{}", self.base_url(), image_name),
            // Format 5: Alternative direct path with leading /
            format!("{}/DCIM/100OLYMP/{}", self.base_url(), image_name),
            // Format 6: Using get_img.cgi for full image instead
            format!(
                "{}get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url(),
                image_name
            ),
            // Format 7: Get resized image
            format!(
                "{}get_resized_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                self.base_url(),
                image_name
            ),
        ];

        // Try each URL with better logging
        for (i, url) in urls.iter().enumerate() {
            info!("📷 Trying image data URL #{}: {}", i + 1, url);

            // Get image data with improved error handling
            match self
                .client()
                .get(url)
                .header("user-agent", "OlympusCameraKit")
                .header("content-length", "4096")
                .header("accept", "image/jpeg,*/*")
                .send()
            {
                Ok(response) => {
                    let status = response.status();
                    info!("📷 Image data response status: {}", status);

                    if status.is_success() {
                        // Get the bytes
                        match response.bytes() {
                            Ok(bytes) => {
                                let bytes_vec = bytes.to_vec();
                                info!("📷 Received {} bytes of image data", bytes_vec.len());

                                // Check if it looks like an image (JPGs start with FFD8)
                                if bytes_vec.len() >= 2
                                    && bytes_vec[0] == 0xFF
                                    && bytes_vec[1] == 0xD8
                                {
                                    info!("✅ Valid JPEG image data detected!");
                                    return Ok(bytes_vec);
                                } else {
                                    info!(
                                        "⚠️ WARNING: Downloaded data doesn't appear to be a JPEG image - first bytes: {:02X} {:02X}",
                                        bytes_vec.get(0).unwrap_or(&0),
                                        bytes_vec.get(1).unwrap_or(&0)
                                    );

                                    // If the data is clearly not an image, try the next URL
                                    if bytes_vec.len() > 10 {
                                        let text = String::from_utf8_lossy(
                                            &bytes_vec[0..bytes_vec.len().min(100)],
                                        );
                                        if text.contains("ERROR")
                                            || text.contains("error")
                                            || text.contains("not found")
                                        {
                                            info!(
                                                "⚠️ Received error message instead of image: {}",
                                                text
                                            );
                                            continue;
                                        }
                                    }

                                    // If we couldn't identify an error, return the data anyway
                                    // Some cameras might use a different format
                                    return Ok(bytes_vec);
                                }
                            }
                            Err(e) => {
                                info!("❌ Failed to get image bytes from URL #{}: {}", i + 1, e);
                                continue;
                            }
                        }
                    } else if status.as_u16() == 404 {
                        info!("❌ 404 Not Found for URL #{}: {}", i + 1, url);
                        // Continue to the next URL format
                    } else if status.as_u16() == 520 {
                        info!("❌ 520 Unknown Status for URL #{}: {}", i + 1, url);
                        // Continue to the next URL format
                    } else {
                        info!("❌ HTTP Error {} for URL #{}: {}", status, i + 1, url);
                        // Continue to the next URL format
                    }
                }
                Err(e) => {
                    info!("❌ Request failed for URL #{}: {}", i + 1, e);
                    continue;
                }
            }
        }

        // If all URLs failed, return a more descriptive error
        return Err(anyhow!(
            "Failed to download image data after trying 7 different URL formats. The camera may be disconnected, or the image may not exist."
        ));
    }

    /// Get image with higher resolution options
    fn get_image_with_resolution(&self, image_path: &str, resolution: &str) -> Result<Vec<u8>> {
        info!(
            "Requesting image at resolution {}: {}",
            resolution, image_path
        );

        // Build URL based on requested resolution
        let url = match resolution.to_lowercase().as_str() {
            "thumbnail" | "low" => {
                format!("get_thumbnail.cgi?DIR={}&size=1024", image_path)
            }
            "medium" => {
                format!("get_resized_img.cgi?DIR={}&size=2048", image_path)
            }
            "high" | "full" => {
                format!("get_img.cgi?DIR={}", image_path)
            }
            _ => {
                // Default to thumbnail for unknown resolution
                format!("get_thumbnail.cgi?DIR={}&size=1024", image_path)
            }
        };

        // Get the binary data
        self.get_binary(&url)
    }
}
