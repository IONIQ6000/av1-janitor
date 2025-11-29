use crate::scan::CandidateFile;
use anyhow::{Context, Result};
use std::fs;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Check if a file is stable (not being written to)
///
/// This function records the initial file size, waits for the configured duration,
/// then checks the file size again. If the sizes match, the file is considered stable.
///
/// # Arguments
/// * `file` - The candidate file to check
/// * `duration` - How long to wait before checking again (typically 10 seconds)
///
/// # Returns
/// * `Ok(true)` if the file size hasn't changed (stable)
/// * `Ok(false)` if the file size has changed (unstable)
/// * `Err` if there was an error accessing the file
pub async fn check_stability(file: &CandidateFile, duration: Duration) -> Result<bool> {
    // Record initial file size
    let initial_size = file.size_bytes;
    debug!(
        "Checking stability for {}: initial size = {} bytes",
        file.path.display(),
        initial_size
    );

    // Wait for configured duration
    sleep(duration).await;

    // Check file size again
    let metadata = fs::metadata(&file.path)
        .with_context(|| format!("Failed to get metadata for {}", file.path.display()))?;

    let current_size = metadata.len();
    debug!(
        "Stability check for {}: current size = {} bytes",
        file.path.display(),
        current_size
    );

    // Return true if sizes match (stable), false otherwise (unstable)
    let is_stable = initial_size == current_size;

    if is_stable {
        debug!("File is stable: {}", file.path.display());
    } else {
        debug!(
            "File is unstable: {} (size changed from {} to {} bytes)",
            file.path.display(),
            initial_size,
            current_size
        );
    }

    Ok(is_stable)
}
