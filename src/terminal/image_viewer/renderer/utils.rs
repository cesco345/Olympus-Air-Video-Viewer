// src/terminal/image_viewer/renderer/utils.rs
use anyhow::Result;
use log::warn;
use std::{io::Write, path::PathBuf};
use tempfile::NamedTempFile;

/// Create a temporary file for high-res image data
pub fn write_temp_image_file(image_data: &[u8]) -> Result<PathBuf> {
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(image_data)?;
    temp_file.flush()?;

    // Convert to PathBuf and prevent deletion when file goes out of scope
    let (file, path) = temp_file.keep()?;
    // We need to drop the file handle to allow other processes to access it
    drop(file);

    Ok(path)
}

/// Clean up a temporary file
pub fn cleanup_temp_file(temp_path: &PathBuf) {
    if let Err(e) = std::fs::remove_file(temp_path) {
        warn!("Failed to remove temporary file: {}", e);
    }
}

/// Get terminal dimensions
pub fn get_terminal_dimensions() -> (u32, u32) {
    termsize::get()
        .map(|size| (size.cols as u32, size.rows as u32))
        .unwrap_or((80, 24))
}
