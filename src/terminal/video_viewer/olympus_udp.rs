// src/terminal/video_viewer/olympus_udp.rs
use crate::terminal::video_viewer::state::VideoViewerState;
use anyhow::{Result, anyhow};
use log::{debug, error, info, warn};
use std::process::{Command, Stdio};
use std::{
    fs,
    io::Write,
    net::UdpSocket,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

/// Initialize the camera for Olympus live view streaming
pub fn initialize_camera(
    camera: &crate::camera::olympus::OlympusCamera,
    udp_port: u16,
) -> Result<()> {
    info!(
        "Initializing Olympus camera for live view streaming on port {}",
        udp_port
    );

    // Full initialization sequence for Olympus camera
    let init_steps = [
        "get_connectmode.cgi",
        "switch_cameramode.cgi?mode=rec",
        "get_state.cgi",
        "exec_takemisc.cgi?com=stopliveview", // Stop any existing stream first
    ];

    // Run initialization steps
    for step in &init_steps {
        match camera.get_page(step) {
            Ok(_) => info!("Camera initialization step successful: {}", step),
            Err(e) => {
                error!("Camera initialization step failed: {} - {}", step, e);
                return Err(anyhow!("Failed to initialize camera: {}", e));
            }
        }
        // Add delay between commands
        thread::sleep(Duration::from_millis(300));
    }

    // Start the live view stream with the specified port
    let start_command = format!("exec_takemisc.cgi?com=startliveview&port={}", udp_port);

    match camera.get_page(&start_command) {
        Ok(_) => {
            info!("Live view started successfully on port {}", udp_port);
            // Wait for camera to initialize streaming
            thread::sleep(Duration::from_secs(1));
            Ok(())
        }
        Err(e) => {
            error!("Failed to start live view: {}", e);
            Err(anyhow!("Failed to start live view: {}", e))
        }
    }
}

/// Stop the live view on the camera
pub fn stop_live_view(camera: &crate::camera::olympus::OlympusCamera) -> Result<()> {
    info!("Stopping live view on Olympus camera");

    match camera.get_page("exec_takemisc.cgi?com=stopliveview") {
        Ok(_) => {
            info!("Live view stopped successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to stop live view: {}", e);
            Err(anyhow!("Failed to stop live view: {}", e))
        }
    }
}

/// Start the UDP receiver for Olympus streaming
pub fn start_udp_receiver(viewer_state: &mut VideoViewerState) -> Result<()> {
    info!(
        "Starting Olympus UDP receiver on port {}",
        viewer_state.udp_port
    );

    // Bind to UDP port
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", viewer_state.udp_port)) {
        Ok(s) => {
            info!("Successfully bound to UDP port {}", viewer_state.udp_port);
            s
        }
        Err(e) => {
            error!(
                "Failed to bind to UDP port {}: {}",
                viewer_state.udp_port, e
            );

            // Try a different port
            viewer_state.udp_port = 65002;
            info!("Trying alternate port: {}", viewer_state.udp_port);

            match UdpSocket::bind(format!("0.0.0.0:{}", viewer_state.udp_port)) {
                Ok(s) => {
                    info!(
                        "Successfully bound to alternate UDP port {}",
                        viewer_state.udp_port
                    );
                    s
                }
                Err(e) => {
                    error!(
                        "Failed to bind to alternate UDP port {}: {}",
                        viewer_state.udp_port, e
                    );
                    return Err(anyhow!("Failed to bind to UDP ports: {}", e));
                }
            }
        }
    };

    // Set timeouts for non-blocking operation
    socket.set_read_timeout(Some(Duration::from_millis(500)))?;

    // Initialize shared socket and thread control flag
    let socket_arc = Arc::new(Mutex::new(socket));
    *viewer_state.udp_running.lock().unwrap() = true;

    // Setup for MPlayer
    setup_pipe_for_player()?;

    // Try starting MPlayer first, fallback to FFplay if it fails
    let mplayer_result = start_mplayer_process(viewer_state);
    if let Err(e) = mplayer_result {
        warn!(
            "Failed to start MPlayer: {}. Trying FFplay as fallback...",
            e
        );
        if let Err(e) = start_ffplay_process(viewer_state) {
            return Err(anyhow!("Failed to start video players: {}", e));
        }
    }

    // Initialize statistics with proper mutex handling
    if let Ok(mut counter) = viewer_state.packets_received.lock() {
        *counter = 0;
    }
    if let Ok(mut frames) = viewer_state.jpeg_frames.lock() {
        *frames = 0;
    }
    if let Ok(mut time) = viewer_state.last_frame_time.lock() {
        *time = Instant::now();
    }
    if let Ok(mut size) = viewer_state.last_frame_size.lock() {
        *size = 0;
    }

    // Pass viewer state stats counters as Arc<Mutex> to allow updating from thread
    let packets_received = Arc::clone(&viewer_state.packets_received);
    let jpeg_frames = Arc::clone(&viewer_state.jpeg_frames);
    let last_frame_time = Arc::clone(&viewer_state.last_frame_time);
    let last_frame_size = Arc::clone(&viewer_state.last_frame_size);

    // Start UDP processing thread
    let running_flag = Arc::clone(&viewer_state.udp_running);
    let socket_clone = Arc::clone(&socket_arc);

    let thread_handle = thread::spawn(move || {
        process_udp_stream(
            socket_clone,
            running_flag,
            packets_received,
            jpeg_frames,
            last_frame_time,
            last_frame_size,
        );
    });

    viewer_state.udp_thread_handle = Some(thread_handle);
    viewer_state.is_playing = true;

    Ok(())
}

/// Setup named pipe for MPlayer
fn setup_pipe_for_player() -> Result<()> {
    let pipe_path = Path::new("olympus_stream.pipe");

    // Log the current directory to ensure we know where to look for the pipe
    info!(
        "Current directory: {:?}",
        std::env::current_dir().unwrap_or_default()
    );

    if pipe_path.exists() {
        info!("Removing existing pipe");
        match fs::remove_file(pipe_path) {
            Ok(_) => info!("Successfully removed existing pipe"),
            Err(e) => warn!("Failed to remove existing pipe: {}", e),
        }
    }

    #[cfg(unix)]
    {
        info!("Creating named pipe with mkfifo");
        let output = Command::new("mkfifo")
            .arg("-m")
            .arg("0666") // More permissive mode for the pipe
            .arg("olympus_stream.pipe")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("mkfifo error: {}", stderr);
            return Err(anyhow!("Failed to create pipe: {}", stderr));
        }

        info!("Successfully created named pipe");
    }

    #[cfg(windows)]
    {
        info!("Creating file for Windows");
        match std::fs::File::create(pipe_path) {
            Ok(file) => {
                let _ = file.set_len(0);
                info!("Successfully created file for streaming on Windows");
            }
            Err(e) => {
                warn!("Failed to create file: {}", e);
                return Err(anyhow!("Failed to create file: {}", e));
            }
        }
    }

    // Verify pipe exists after creation
    if pipe_path.exists() {
        info!(
            "Pipe exists at {:?}",
            pipe_path.canonicalize().unwrap_or_default()
        );
    } else {
        warn!("Pipe still doesn't exist after creation attempt");
    }

    Ok(())
}

/// Launch MPlayer to display stream
fn start_mplayer_process(viewer_state: &mut VideoViewerState) -> Result<()> {
    info!("Attempting to start MPlayer...");

    // First check if MPlayer is installed
    let mplayer_check = Command::new("which").arg("mplayer").output();

    match mplayer_check {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout);
            info!("MPlayer found at: {}", path.trim());
        }
        _ => {
            error!("MPlayer not found in path!");
            return Err(anyhow!("MPlayer not found. Please install MPlayer first."));
        }
    }

    // Create a log file for MPlayer output
    let log_path = Path::new("mplayer_log.txt");
    let log_file = std::fs::File::create(log_path)?;

    // MPlayer arguments with more debugging
    let mplayer_args = [
        "-demuxer",
        "lavf",
        "-lavfdopts",
        "format=mjpeg",
        "-really-quiet", // Don't flood console
        "-loop",
        "0",
        "-v", // Verbose output
        "olympus_stream.pipe",
    ];

    info!("MPlayer command: mplayer {}", mplayer_args.join(" "));

    let child = Command::new("mplayer")
        .args(&mplayer_args)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()?;

    let pid = child.id();
    viewer_state.external_viewer_pid = Some(pid);
    info!("Started MPlayer with PID: {}", pid);

    Ok(())
}

/// Launch FFplay as fallback player
fn start_ffplay_process(viewer_state: &mut VideoViewerState) -> Result<()> {
    info!("Attempting to start FFplay...");

    // First check if FFplay is installed
    let ffplay_check = Command::new("which").arg("ffplay").output();

    match ffplay_check {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout);
            info!("FFplay found at: {}", path.trim());
        }
        _ => {
            warn!("FFplay not found in path!");
            return Err(anyhow!("FFplay not found"));
        }
    }

    // Create log file for FFplay
    let log_path = Path::new("ffplay_log.txt");
    let log_file = std::fs::File::create(log_path)?;

    // FFplay arguments for MJPEG stream
    let ffplay_args = [
        "-f",
        "mjpeg",
        "-i",
        "olympus_stream.pipe",
        "-loglevel",
        "warning",
        "-x",
        "800",
        "-y",
        "600",
    ];

    info!("FFplay command: ffplay {}", ffplay_args.join(" "));

    let child = Command::new("ffplay")
        .args(&ffplay_args)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()?;

    let pid = child.id();
    viewer_state.external_viewer_pid = Some(pid);
    info!("Started FFplay with PID: {}", pid);

    Ok(())
}

/// Process stream data in a thread
fn process_udp_stream(
    socket_clone: Arc<Mutex<UdpSocket>>,
    running_flag: Arc<Mutex<bool>>,
    packets_received: Arc<Mutex<u32>>,
    jpeg_frames: Arc<Mutex<u32>>,
    last_frame_time: Arc<Mutex<Instant>>,
    last_frame_size: Arc<Mutex<usize>>,
) {
    info!("UDP receiver thread started");

    // Get current process ID for debugging
    info!("UDP thread process: {}", std::process::id());

    // Open named pipe for writing
    let pipe_result = std::fs::OpenOptions::new()
        .write(true)
        .open("olympus_stream.pipe");

    let mut pipe = match pipe_result {
        Ok(file) => {
            info!("Successfully opened pipe for writing");
            Some(file)
        }
        Err(e) => {
            error!("Failed to open pipe: {}", e);
            None
        }
    };

    // Main receive loop - RTP protocol handling for Olympus camera
    let mut buffer = [0u8; 65535]; // Max UDP packet size
    let mut local_packets_received = 0;
    let mut local_jpeg_frames = 0;

    // RTP frame assembly variables
    let mut first_frame_received = false;
    let mut current_frame_id = 0;
    let mut current_packet_id = 0;
    let mut jpeg_data = Vec::with_capacity(262144); // larger capacity for better performance

    // Frame rate control - increased to 30 FPS for smoother video
    let mut last_write_time = Instant::now();
    let frame_interval = Duration::from_millis(33); // ~30 FPS

    // Last activity tracking for reconnection
    let mut last_activity = Instant::now();
    let mut last_heartbeat = Instant::now();

    // Pipe maintenance - periodically recreate pipe to avoid degradation
    let mut last_pipe_reset = Instant::now();
    let pipe_reset_interval = Duration::from_secs(30); // Reset pipe every 30 seconds

    // Frame skip counter to handle high frame rates
    let mut frame_counter = 0;
    let frame_skip_rate = 1; // Process every frame (0 = skip none, 1 = process all, 2 = every other)

    while *running_flag.lock().unwrap() {
        // Receive and process data
        if let Ok(socket) = socket_clone.lock() {
            match socket.recv_from(&mut buffer) {
                Ok((size, _addr)) => {
                    local_packets_received += 1;
                    if let Ok(mut counter) = packets_received.lock() {
                        *counter = local_packets_received;
                    }
                    last_activity = Instant::now();

                    // Log every 100th packet for debugging
                    if local_packets_received % 100 == 0 {
                        info!(
                            "Received {} packets, {} JPEG frames",
                            local_packets_received, local_jpeg_frames
                        );
                    }

                    if size >= 12 {
                        // Decode RTP header (based on Python implementation)
                        let v = (buffer[0] & 0xC0) >> 6;
                        let p = (buffer[0] & 0x20) >> 5;
                        let x = (buffer[0] & 0x10) >> 4;
                        let cc = buffer[0] & 0x0F;

                        let m = (buffer[1] & 0x80) >> 7;
                        let pt = buffer[1] & 0x7F;

                        let rtp_seq = ((buffer[2] as u16) << 8) | (buffer[3] as u16);
                        let frame_seq = ((buffer[4] as u32) << 24)
                            | ((buffer[5] as u32) << 16)
                            | ((buffer[6] as u32) << 8)
                            | (buffer[7] as u32);

                        // First packet of frame
                        if v == 2 && p == 0 && x == 1 && m == 0 && pt == 96 && !first_frame_received
                        {
                            debug!("First packet of frame received, frame ID: {}", frame_seq);

                            current_packet_id = rtp_seq;
                            current_frame_id = frame_seq;
                            first_frame_received = true;

                            // Get extension header length (in 32-bit words)
                            let ext_header_len = if size >= 16 {
                                ((buffer[14] as u16) << 8) | (buffer[15] as u16)
                            } else {
                                0
                            };

                            // Skip RTP header (12 bytes) + extension header (4 bytes + extension length)
                            let header_size = 12 + 4 + (ext_header_len as usize) * 4;
                            if size > header_size {
                                jpeg_data.clear();
                                jpeg_data.extend_from_slice(&buffer[header_size..size]);
                            }
                        }
                        // Middle packets of frame
                        else if v == 2
                            && p == 0
                            && x == 0
                            && cc == 0
                            && m == 0
                            && pt == 96
                            && first_frame_received
                            && current_packet_id + 1 == rtp_seq
                            && current_frame_id == frame_seq
                        {
                            current_packet_id = rtp_seq;
                            jpeg_data.extend_from_slice(&buffer[12..size]);
                        }
                        // Last packet of frame
                        else if v == 2
                            && p == 0
                            && x == 0
                            && cc == 0
                            && m == 1
                            && pt == 96
                            && first_frame_received
                            && current_packet_id + 1 == rtp_seq
                            && current_frame_id == frame_seq
                        {
                            jpeg_data.extend_from_slice(&buffer[12..size]);

                            // Check if we have valid JPEG data (starts with FF D8)
                            if jpeg_data.len() >= 2 && jpeg_data[0] == 0xFF && jpeg_data[1] == 0xD8
                            {
                                // Apply adaptive frame skipping when under high load
                                if last_write_time.elapsed() < Duration::from_millis(20) {
                                    // If we're processing frames too quickly, skip some frames
                                    // to avoid overwhelming the player
                                    if frame_counter % 2 != 0 {
                                        // Skip every other frame when under pressure
                                        debug!("Skipping frame under high load");
                                        first_frame_received = false;
                                        jpeg_data.clear();
                                        continue;
                                    }
                                }

                                // Apply frame skipping if needed
                                frame_counter += 1;
                                if frame_counter % frame_skip_rate == 0 {
                                    local_jpeg_frames += 1;

                                    // Update shared statistics
                                    if let Ok(mut frames) = jpeg_frames.lock() {
                                        *frames = local_jpeg_frames;
                                    }
                                    if let Ok(mut time) = last_frame_time.lock() {
                                        *time = Instant::now();
                                    }
                                    if let Ok(mut size) = last_frame_size.lock() {
                                        *size = jpeg_data.len();
                                    }

                                    debug!(
                                        "Complete JPEG frame assembled: {} bytes",
                                        jpeg_data.len()
                                    );

                                    // Apply frame rate control to avoid flooding player
                                    let elapsed = last_write_time.elapsed();
                                    if elapsed < frame_interval {
                                        thread::sleep(frame_interval - elapsed);
                                    }

                                    // Check if we need to reset the pipe
                                    if last_pipe_reset.elapsed() > pipe_reset_interval {
                                        info!(
                                            "Performing periodic pipe reset to maintain performance"
                                        );
                                        drop(pipe);

                                        // Sleep to let player release the pipe
                                        thread::sleep(Duration::from_millis(100));

                                        // Reopen pipe
                                        pipe = std::fs::OpenOptions::new()
                                            .write(true)
                                            .open("olympus_stream.pipe")
                                            .ok();

                                        if pipe.is_some() {
                                            info!("Successfully reopened pipe");
                                        } else {
                                            error!("Failed to reopen pipe during maintenance");
                                        }

                                        last_pipe_reset = Instant::now();
                                    }

                                    // Write to pipe with error handling for broken pipe
                                    if let Some(pipe_file) = pipe.as_mut() {
                                        match pipe_file.write_all(&jpeg_data) {
                                            Ok(_) => {
                                                // Successfully wrote the data, now flush
                                                if let Err(e) = pipe_file.flush() {
                                                    warn!("Failed to flush pipe: {}", e);
                                                }
                                                last_write_time = Instant::now();
                                            }
                                            Err(e) => {
                                                error!("Failed to write to pipe: {}", e);

                                                // Check if the pipe is broken and try to recover
                                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                                    warn!("Pipe broken, attempting to reopen...");
                                                    // Drop the broken pipe
                                                    drop(pipe_file);
                                                    pipe = None;

                                                    // Reopen pipe after a short delay
                                                    thread::sleep(Duration::from_millis(100));
                                                    pipe = std::fs::OpenOptions::new()
                                                        .write(true)
                                                        .open("olympus_stream.pipe")
                                                        .ok();

                                                    if pipe.is_some() {
                                                        info!("Successfully reopened pipe");
                                                        last_pipe_reset = Instant::now();
                                                    } else {
                                                        error!("Failed to reopen pipe");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                warn!("Invalid JPEG data (missing FF D8 header)");
                            }

                            // Reset state and free memory
                            first_frame_received = false;
                            jpeg_data.clear();

                            // Keep capacity reasonable
                            if jpeg_data.capacity() > 524288 {
                                // 512 KB
                                jpeg_data = Vec::with_capacity(262144); // Resize to 256 KB
                            }
                        } else {
                            // Reset on unexpected packet
                            if first_frame_received {
                                debug!("Unexpected packet, resetting frame assembly");
                                first_frame_received = false;
                                jpeg_data.clear();
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        error!("UDP receive error: {}", e);
                    }
                }
            }
        }

        // Check for inactivity
        if last_activity.elapsed() > Duration::from_secs(10) {
            warn!("No packets received for 10 seconds, stream may be stalled");
            last_activity = Instant::now(); // Reset to avoid spam
        }

        // Send periodic log heartbeats
        if last_heartbeat.elapsed() > Duration::from_secs(5) {
            let frame_size = if let Ok(size) = last_frame_size.lock() {
                *size
            } else {
                0
            };

            // Calculate approximate FPS over last 5 seconds
            let time_window = last_heartbeat.elapsed().as_secs_f32();
            let frames_per_second = if time_window > 0.0 {
                local_jpeg_frames as f32 / time_window
            } else {
                0.0
            };

            info!(
                "Stream status: {} packets, {} frames ({:.1} FPS), last frame: {}KB",
                local_packets_received,
                local_jpeg_frames,
                frames_per_second,
                frame_size / 1024
            );
            last_heartbeat = Instant::now();
            local_jpeg_frames = 0; // Reset for next FPS calculation
        }

        thread::sleep(Duration::from_millis(5)); // Shorter sleep for more responsive processing
    }

    info!(
        "UDP receiver thread terminated. Processed {} packets, {} JPEG frames",
        local_packets_received, local_jpeg_frames
    );
}

/// Stop the UDP receiver
pub fn stop_udp_receiver(viewer_state: &mut VideoViewerState) -> Result<()> {
    info!("Stopping Olympus UDP receiver");

    // First stop thread to prevent further pipe writes
    if let Ok(mut running) = viewer_state.udp_running.lock() {
        *running = false;
    }

    // Give the thread time to clean up and stop writing to the pipe
    thread::sleep(Duration::from_millis(200));

    if let Some(handle) = viewer_state.udp_thread_handle.take() {
        match handle.join() {
            Ok(_) => info!("UDP thread joined successfully"),
            Err(e) => warn!("Error joining UDP thread: {:?}", e),
        }
    }

    // Send SIGTERM to player process first (gentler than SIGKILL)
    if let Some(pid) = viewer_state.external_viewer_pid {
        #[cfg(unix)]
        {
            info!("Gracefully stopping player process with PID: {}", pid);

            // First try graceful shutdown
            let _ = Command::new("kill")
                .arg("-15") // SIGTERM
                .arg(&pid.to_string())
                .output();

            // Wait for process to exit gracefully
            thread::sleep(Duration::from_millis(300));

            // Check if process is still running
            let check_process = Command::new("ps").arg("-p").arg(&pid.to_string()).output();

            // If still running, force kill
            if let Ok(output) = check_process {
                if output.status.success() {
                    info!("Process still running, sending SIGKILL");
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(&pid.to_string())
                        .output();
                }
            }

            // Additional cleanup for all possible instances
            let _ = Command::new("killall").arg("-15").arg("mplayer").output();
            thread::sleep(Duration::from_millis(200));
            let _ = Command::new("killall").arg("-9").arg("mplayer").output();
        }

        #[cfg(windows)]
        {
            // On Windows, first try graceful termination
            let _ = Command::new("taskkill")
                .arg("/PID")
                .arg(&pid.to_string())
                .output();

            thread::sleep(Duration::from_millis(300));

            // Then force if needed
            let _ = Command::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(&pid.to_string())
                .output();
        }

        viewer_state.external_viewer_pid = None;
    }

    // Now clean up pipe after player is stopped
    let pipe_path = Path::new("olympus_stream.pipe");
    if pipe_path.exists() {
        info!("Removing pipe file");
        match fs::remove_file(pipe_path) {
            Ok(_) => info!("Pipe file removed successfully"),
            Err(e) => warn!("Failed to remove pipe file: {}", e),
        }
    }

    viewer_state.is_playing = false;

    // Give the system a moment to finish cleanup
    thread::sleep(Duration::from_millis(100));

    Ok(())
}
