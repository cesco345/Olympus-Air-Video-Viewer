// Export all submodules
pub mod client;
pub mod connection;
pub mod image;
pub mod olympus;
pub mod photo;

// Re-export the main camera type for convenience
pub use olympus::OlympusCamera;
