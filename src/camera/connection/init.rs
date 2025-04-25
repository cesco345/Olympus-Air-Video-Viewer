use anyhow::{Result, anyhow};
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::camera::client::basic::ClientOperations;

/// Helper for camera connection management
pub trait ConnectionManager: ClientOperations {
    /// Get connection state
    fn connected(&self) -> &Arc<AtomicBool>;

    /// Connect to camera with required initialization steps
    fn connect(&self) -> Result<()> {
        // If already connected, don't reconnect
        if self.connected().load(Ordering::Relaxed) {
            info!("Camera already connected");
            return Ok(());
        }

        info!("Connecting to camera at {}", self.base_url());

        // More robust connection sequence with timeouts between steps
        let steps = [
            "get_connectmode.cgi",
            "switch_cameramode.cgi?mode=rec",
            "get_state.cgi",
            "exec_takemisc.cgi?com=startliveview&port=5555",
        ];

        for (i, step) in steps.iter().enumerate() {
            info!("Connection step {}/{}: {}", i + 1, steps.len(), step);

            // Try each step with multiple attempts
            let mut success = false;
            for attempt in 1..=3 {
                info!("Attempt {} for step '{}'", attempt, step);

                match self.get_page(step) {
                    Ok(_) => {
                        info!("✅ Step successful: {}", step);
                        success = true;
                        // Add increasing delay between successful steps
                        let delay = Duration::from_millis(500 * (i as u64 + 1));
                        info!("Waiting {:?} before next step", delay);
                        thread::sleep(delay);
                        break;
                    }
                    Err(e) => {
                        info!(
                            "❌ Connection step '{}' failed (attempt {}/3): {}",
                            step, attempt, e
                        );

                        // Add backoff delay between attempts
                        if attempt < 3 {
                            let delay = Duration::from_millis(500 * attempt as u64);
                            info!("Retrying in {:?}...", delay);
                            thread::sleep(delay);
                        }
                    }
                }
            }

            if !success {
                error!(
                    "Failed to complete connection step '{}' after 3 attempts",
                    step
                );
                return Err(anyhow!(
                    "Failed to connect: step '{}' failed after multiple attempts",
                    step
                ));
            }
        }

        // Add final delay after all steps complete
        thread::sleep(Duration::from_secs(1));

        // Verify connection with a state check
        info!("Verifying camera connection with state check");
        match self.get_page("get_state.cgi") {
            Ok(_) => {
                info!("✅ Connection verification successful");
                // Mark as connected
                self.connected().store(true, Ordering::Relaxed);
                info!("Camera connected successfully");
                Ok(())
            }
            Err(e) => {
                error!("❌ Connection verification failed: {}", e);
                Err(anyhow!("Failed to verify camera connection: {}", e))
            }
        }
    }
}
