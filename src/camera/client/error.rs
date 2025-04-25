use anyhow::{Result, anyhow};
use log::{error, info};
use reqwest::blocking::Response;

/// Helper for extracting error information
pub trait ErrorHandler {
    /// Attempt to get detailed error information from response
    fn extract_error_info(&self, response: Response) -> Result<Vec<u8>> {
        let status = response.status();

        // Try to read response content
        match response.bytes() {
            Ok(bytes) => {
                let bytes_vec = bytes.to_vec();
                error!("Error response size: {} bytes", bytes_vec.len());

                // Log beginning of response for debugging
                if !bytes_vec.is_empty() {
                    if bytes_vec[0] == 0xFF && bytes_vec[1] == 0xD8 {
                        // Seems to be a JPEG despite error status
                        info!("Response appears to be a JPEG image despite error status");
                        return Ok(bytes_vec);
                    } else if bytes_vec.len() > 20 {
                        // Try to show beginning of response as text
                        match String::from_utf8_lossy(&bytes_vec[0..20.min(bytes_vec.len())])
                            .to_string()
                        {
                            s if !s.is_empty() => {
                                info!("Response begins with: {}", s);
                            }
                            _ => {}
                        }
                    }
                }

                return Err(anyhow!(
                    "Request failed with status: {} (response size: {} bytes)",
                    status,
                    bytes_vec.len()
                ));
            }
            Err(e) => {
                return Err(anyhow!(
                    "Request failed with status: {} and error reading bytes: {}",
                    status,
                    e
                ));
            }
        }
    }
}
