use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Create a .av1skip sidecar file for a video file
pub fn create_skip_marker(video_path: &Path) -> Result<()> {
    let skip_marker_path = get_skip_marker_path(video_path);
    
    // Create an empty file
    fs::write(&skip_marker_path, "")
        .with_context(|| format!("Failed to create skip marker at {}", skip_marker_path.display()))?;
    
    Ok(())
}

/// Write a .why.txt sidecar file explaining why a video was skipped
pub fn write_why_file(video_path: &Path, reason: &str) -> Result<()> {
    let why_file_path = get_why_file_path(video_path);
    
    fs::write(&why_file_path, reason)
        .with_context(|| format!("Failed to write why file at {}", why_file_path.display()))?;
    
    Ok(())
}

/// Check if a video file has a .av1skip marker
pub fn has_skip_marker(video_path: &Path) -> bool {
    let skip_marker_path = get_skip_marker_path(video_path);
    skip_marker_path.exists()
}

/// Get the path for the .av1skip marker file
fn get_skip_marker_path(video_path: &Path) -> std::path::PathBuf {
    video_path.with_extension(
        format!("{}.av1skip", 
            video_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        )
    )
}

/// Get the path for the .why.txt file
fn get_why_file_path(video_path: &Path) -> std::path::PathBuf {
    video_path.with_extension(
        format!("{}.why.txt", 
            video_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        )
    )
}
