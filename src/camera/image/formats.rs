/// URL format utilities for accessing images
pub struct UrlFormatGenerator;

impl UrlFormatGenerator {
    /// Generate various URL formats to try for accessing images
    pub fn generate_url_formats(base_url: &str, image_name: &str) -> Vec<String> {
        vec![
            // Format 1: Standard thumbnail format
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                base_url, image_name
            ),
            // Format 2: Without leading slash in DIR
            format!(
                "{}get_thumbnail.cgi?DIR=DCIM/100OLYMP&FILE={}&size=1024",
                base_url, image_name
            ),
            // Format 3: Without DIR parameter
            format!(
                "{}get_thumbnail.cgi?FILE={}&size=1024",
                base_url, image_name
            ),
            // Format 4: Direct path
            format!("{}DCIM/100OLYMP/{}", base_url, image_name),
            // Format 5: Using get_img.cgi instead
            format!(
                "{}get_img.cgi?DIR=/DCIM/100OLYMP&FILE={}",
                base_url, image_name
            ),
            // Format 6: Using get_img.cgi without leading slash
            format!(
                "{}get_img.cgi?DIR=DCIM/100OLYMP&FILE={}",
                base_url, image_name
            ),
            // Format 7: Using get_resized_img.cgi
            format!(
                "{}get_resized_img.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                base_url, image_name
            ),
            // Format 8: Alternative path structure
            format!("{}get_img.cgi?PATH=/DCIM/100OLYMP/{}", base_url, image_name),
            // Format 9: With uppercase filename
            format!(
                "{}get_thumbnail.cgi?DIR=/DCIM/100OLYMP&FILE={}&size=1024",
                base_url,
                image_name.to_uppercase()
            ),
            // Format 10: With lowercase path
            format!(
                "{}get_thumbnail.cgi?DIR=/dcim/100olymp&FILE={}&size=1024",
                base_url, image_name
            ),
        ]
    }
}
