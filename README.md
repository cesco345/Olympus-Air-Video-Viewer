# Olympus Air Camera Terminal Viewer

A terminal-based application for controlling Olympus cameras over WiFi. This application allows you to take photos, download images, view images directly from the camera, live view through the Olympus Air camera, and manage your camera remotely using a clean terminal interface.

![Olympus Camera Controller](docs/screenshot.png)

## Features

- Connect to Olympus cameras over WiFi
- Take photos with warm-up sequence
- Browse images stored on the camera
- View images directly on the camera without downloading
- Live view from Olympus Air camera using `mplayer`
- Record live video directly from camera stream
- Download images to your computer
- Delete images (on supported models)
- Offline mode with reconnection capability
- Responsive terminal UI with pagination

## New Image Viewing Functionality

The latest addition to the application allows users to preview images directly from the camera's SD card over WiFi before downloading them. This feature provides several advantages:

### Benefits of Wireless Image Viewing

- **Immediate Feedback**: View and evaluate images without removing the SD card or connecting cables
- **Field Review**: Review images while still on location to ensure you've captured what you need
- **Efficient Workflow**: Quickly browse through images and only download those worth keeping
- **Bandwidth Efficiency**: Preview using optimized data transfer instead of downloading full-resolution files
- **Battery Preservation**: Less demanding on both camera and device batteries compared to transferring full images

### Technical Details

- **Resolution**: While previewed images are displayed at a lower resolution than the originals (limited by terminal capabilities), they provide sufficient detail for evaluation purposes
- **Multiple Display Methods**: Support for various terminal image display methods (iTerm2, Kitty, SIXEL, etc.)
- **Zoom Functionality**: Adjust the zoom level to see more or less detail
- **Cross-Platform Support**: Works on macOS, Linux, and Windows with appropriate fallbacks

## Live View and Recording

The application supports **live view streaming** from Olympus Air cameras, along with the ability to **record video** from the stream.

### Benefits of Live Viewing and Recording

- **Real-Time Composition**: Perfectly frame your shots before capturing
- **Remote Monitoring**: View the camera feed from a distance using `mplayer`
- **Video Capture**: Record directly from the camera for documentation or creative use
- **Low-Latency Feedback**: Instant video stream helps with on-the-fly adjustments

### Technical Details

- **Stream Format**: MJPEG stream used for live viewing
- **Viewer Integration**: Utilizes `mplayer` for displaying the stream in real time
- **Recording**: Optionally pipe the stream to a file for recording purposes
- **Fallbacks**: If live view is not supported, defaults to image preview mode

## Prerequisites

- Rust (1.65 or newer)
- Cargo (included with Rust)
- Working WiFi connection to an Olympus camera
- `mplayer` installed for live view and recording
- macOS, Linux, or Windows

## Project Structure

```
src/
├── camera/
│   ├── mod.rs           # Camera module export
│   └── olympus.rs       # Olympus camera implementation
├── main.rs              # Program entry point
├── terminal/
│   ├── app.rs           # Main application
│   ├── handlers.rs      # Input handlers
│   ├── image_viewer/
│   │   ├── handlers.rs  # Image viewer input handlers
│   │   ├── mod.rs       # Image viewer module export
│   │   ├── renderer.rs  # Image viewer UI rendering
│   │   └── state.rs     # Image viewer state
│   ├── mod.rs           # Terminal module export
│   ├── renderer.rs      # UI rendering
│   ├── state.rs         # Application state
│   └── video_viewer/
│       ├── handlers.rs  # Video viewer input handlers
│       ├── mod.rs       # Video viewer module export
│       ├── olympus_udp.rs # Olympus UDP communication
│       ├── renderer.rs  # Video viewer UI rendering
│       └── state.rs     # Video viewer state
└── utils/
    ├── logging.rs       # Logging utilities
    └── mod.rs           # Utils module export
```

## Dependencies

- `anyhow` - Error handling
- `log` & `env_logger` - Logging
- `reqwest` - HTTP client for camera communication
- `tui` - Terminal user interface
- `crossterm` - Terminal manipulation
- `colored` - Colored terminal output
- `regex` - Regular expressions for parsing camera responses
- `viuer` - Terminal image display
- `termsize` - Terminal size detection
- `base64` - Encoding/decoding for image transfer
- `tempfile` - Temporary file handling for image preview

## Installation

1. Clone the repository:

```bash
git clone https://github.com/username/olympus-air-video.git
cd olympus-air-video
```

2. Build the project:

```bash
cargo build
```

## Usage

First, make sure your Olympus camera is in WiFi mode and your computer is connected to it.

### Run the app:

```bash
cargo run
```

### View images from the camera:

Use the terminal interface to browse thumbnails and preview images wirelessly.

### Use live view:

Make sure the camera supports live view over MJPEG and run:

```bash
mplayer http://<camera-ip>:<port>/liveview.mjpg
```

Replace `<camera-ip>` and `<port>` with your camera's IP and live view port.

### Record the live stream:

```bash
mplayer -dumpstream -dumpfile output.avi http://<camera-ip>:<port>/liveview.mjpg
```

This command saves the live stream to `output.avi`.
