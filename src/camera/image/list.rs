use anyhow::Result;
use log::info;
use regex::Regex;

use crate::camera::client::basic::ClientOperations;

/// Image listing functionality
pub trait ImageLister: ClientOperations {
    /// Get a list of images on the camera
    fn get_image_list(&self) -> Result<Vec<String>> {
        info!("Getting list of images");

        let url = format!("{}get_imglist.cgi?DIR=/DCIM/100OLYMP", self.base_url());

        let response = self
            .client()
            .get(&url)
            .header("user-agent", "OlympusCameraKit")
            .header("content-length", "4096")
            .send()?;

        self.log_response_info(&response, "Image list");

        let text = response.text()?;

        // Use both regex patterns to find all image files
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
}
