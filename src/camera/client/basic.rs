use anyhow::{Result, anyhow};
use log::{error, info, warn};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use std::time::Duration;

/// Trait for basic client operations
pub trait ClientOperations {
    /// Get the HTTP client
    fn client(&self) -> &Client;

    /// Get the base URL
    fn base_url(&self) -> &str;

    /// Make a simple GET request to the camera
    fn get_page(&self, endpoint: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url(), endpoint);
        info!("Request: {}", url);

        // Send request with exact headers that work
        let response = self
            .client()
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        // Log but don't validate status code
        self.log_response_info(&response, "Page request");

        // If status is not successful, return an error
        if !response.status().is_success() {
            return Err(anyhow!("Request failed with status: {}", response.status()));
        }

        Ok(())
    }

    /// Make a GET request and return the response body
    fn get_binary(&self, endpoint: &str) -> Result<Vec<u8>> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}{}", self.base_url(), endpoint)
        };

        info!("Binary request: {}", url);

        // Send request with proper headers and longer timeout
        let response = self
            .client()
            .get(&url)
            .timeout(Duration::from_secs(30)) // Longer timeout for image data
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .header("accept", "image/jpeg,*/*")
            .send()?;

        self.log_response_info(&response, "Binary request");

        // Check if request was successful with detailed logging
        match response.status() {
            StatusCode::OK => {
                // Get the binary data
                match response.bytes() {
                    Ok(bytes) => {
                        let bytes_vec = bytes.to_vec();
                        info!("Received {} bytes of binary data", bytes_vec.len());

                        // Check if it looks like an image (JPGs start with FFD8)
                        if bytes_vec.len() < 2 || bytes_vec[0] != 0xFF || bytes_vec[1] != 0xD8 {
                            warn!(
                                "WARNING: Downloaded data doesn't appear to be a JPEG image (bytes start with: {:02X} {:02X})",
                                bytes_vec.get(0).unwrap_or(&0),
                                bytes_vec.get(1).unwrap_or(&0)
                            );

                            // Check if it might be error text
                            if bytes_vec.len() > 10 {
                                let text = String::from_utf8_lossy(
                                    &bytes_vec[0..bytes_vec.len().min(100)],
                                );
                                warn!("Response might be an error message: {}", text);

                                // If it clearly indicates an error, return an error
                                if text.contains("ERROR")
                                    || text.contains("error")
                                    || text.contains("Not Found")
                                {
                                    return Err(anyhow!("Camera returned error message: {}", text));
                                }
                            }
                        } else {
                            info!("✅ Confirmed valid JPEG image data (starts with FFD8)");
                        }

                        Ok(bytes_vec)
                    }
                    Err(e) => Err(anyhow!("Failed to get binary data: {}", e)),
                }
            }
            StatusCode::NOT_FOUND => {
                error!("404 Not Found error for URL: {}", url);

                // Try to extract helpful information from the response
                match response.bytes() {
                    Ok(bytes) => {
                        let bytes_vec = bytes.to_vec();
                        if bytes_vec.len() > 0 {
                            let text =
                                String::from_utf8_lossy(&bytes_vec[0..bytes_vec.len().min(100)]);
                            error!("404 response content: {}", text);
                        }
                        Err(anyhow!("404 Not Found: URL doesn't exist on camera"))
                    }
                    Err(_) => Err(anyhow!("404 Not Found: URL doesn't exist on camera")),
                }
            }
            status if status.as_u16() == 520 => {
                error!("520 Unknown Status error for URL: {}", url);
                Err(anyhow!(
                    "520 Unknown Status: Camera returned unexpected status code"
                ))
            }
            other => {
                error!("Request failed with status: {} for URL: {}", other, url);
                Err(anyhow!("Request failed with status code: {}", other))
            }
        }
    }

    /// Log response information for debugging
    fn log_response_info(&self, response: &Response, request_type: &str) {
        let status = response.status();
        if status.is_success() {
            info!("{} response status: {} OK", request_type, status.as_u16());
        } else {
            warn!(
                "{} response status: {} {}",
                request_type,
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            );

            // Log any headers that might be useful for debugging
            for (name, value) in response.headers() {
                info!("Response header: {} = {:?}", name, value);
            }
        }
    }
}
