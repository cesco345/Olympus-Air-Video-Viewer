// src/terminal/video_viewer/state.rs
use log::info;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Available streaming modes for video
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamingMode {
    /// Olympus UDP custom protocol (only supported mode)
    OlympusUDP,
}

/// State for the video viewer mode
pub struct VideoViewerState {
    /// Stream URL (camera IP)
    pub stream_url: String,

    /// Name of the stream/video
    pub stream_name: String,

    /// Whether video is currently playing
    pub is_playing: bool,

    /// Path to save the stream (if recording)
    pub recording_path: Option<PathBuf>,

    /// Whether stream is being recorded
    pub is_recording: bool,

    /// UDP Local port for receiving stream
    pub udp_port: u16,

    /// Process ID of external viewer (if applicable)
    pub external_viewer_pid: Option<u32>,

    /// Thread handle for UDP receiver
    pub udp_thread_handle: Option<std::thread::JoinHandle<()>>,

    /// Thread handle for stats updater
    pub stats_thread_handle: Option<std::thread::JoinHandle<()>>,

    /// Flag to control UDP thread
    pub udp_running: Arc<Mutex<bool>>,

    /// Number of packets received
    pub packets_received: Arc<Mutex<u32>>,

    /// Number of JPEG frames processed
    pub jpeg_frames: Arc<Mutex<u32>>,

    /// Time of last frame received
    pub last_frame_time: Arc<Mutex<Instant>>,

    /// Size of last frame (bytes)
    pub last_frame_size: Arc<Mutex<usize>>,
}

impl VideoViewerState {
    /// Create a new video viewer state
    pub fn new(stream_url: &str, stream_name: &str) -> Self {
        Self {
            stream_url: stream_url.to_string(),
            stream_name: stream_name.to_string(),
            is_playing: false,
            recording_path: None,
            is_recording: false,
            udp_port: 65001, // Default UDP port for Olympus
            external_viewer_pid: None,
            udp_thread_handle: None,
            stats_thread_handle: None,
            udp_running: Arc::new(Mutex::new(false)),
            packets_received: Arc::new(Mutex::new(0)),
            jpeg_frames: Arc::new(Mutex::new(0)),
            last_frame_time: Arc::new(Mutex::new(Instant::now())),
            last_frame_size: Arc::new(Mutex::new(0)),
        }
    }

    /// Generate URL for display purposes
    pub fn generate_stream_url(&self) -> String {
        let url = format!(
            "olympus-udp://{}:{}",
            self.stream_url.split(':').next().unwrap_or("192.168.0.10"),
            self.udp_port
        );
        info!("Generated URL for streaming: {}", url);
        url
    }

    /// Get time since last frame
    pub fn get_time_since_last_frame(&self) -> Duration {
        if let Ok(last_time) = self.last_frame_time.lock() {
            last_time.elapsed()
        } else {
            Duration::from_secs(0)
        }
    }

    /// Get packet and frame statistics
    pub fn get_statistics(&self) -> (u32, u32, usize) {
        let packets = self.packets_received.lock().map(|p| *p).unwrap_or(0);
        let frames = self.jpeg_frames.lock().map(|f| *f).unwrap_or(0);
        let last_size = self.last_frame_size.lock().map(|s| *s).unwrap_or(0);

        (packets, frames, last_size)
    }

    /// Start recording
    pub fn start_recording(&mut self, path: PathBuf) {
        self.recording_path = Some(path);
        self.is_recording = true;
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.is_recording = false;
    }
}
