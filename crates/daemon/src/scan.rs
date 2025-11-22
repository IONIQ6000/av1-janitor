use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, warn};
use walkdir::WalkDir;

use crate::sidecars::has_skip_marker;

/// Allowed video file extensions
const VIDEO_EXTENSIONS: &[&str] = &[".mkv", ".mp4", ".avi", ".mov", ".m4v", ".ts", ".m2ts"];

#[derive(Debug, Clone)]
pub struct CandidateFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_time: SystemTime,
}

/// Recursively scan library directories for video files
pub fn scan_libraries(roots: &[PathBuf]) -> Result<Vec<CandidateFile>> {
    let mut candidates = Vec::new();

    for root in roots {
        debug!("Scanning library root: {}", root.display());
        
        // Check if root exists and is accessible
        if !root.exists() {
            warn!("Library root does not exist: {}", root.display());
            continue;
        }

        if !root.is_dir() {
            warn!("Library root is not a directory: {}", root.display());
            continue;
        }

        // Walk directory tree
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories (starting with .), but not the root itself
                if e.file_type().is_dir() && e.path() != root {
                    !e.file_name()
                        .to_str()
                        .map(|s| s.starts_with('.'))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
        {
            match entry {
                Ok(entry) => {
                    // Only process files, not directories
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let path = entry.path();

                    // Check if it's a video file
                    if !is_video_file(path) {
                        continue;
                    }

                    // Check for skip marker
                    if has_skip_marker(path) {
                        debug!("Skipping file with .av1skip marker: {}", path.display());
                        continue;
                    }

                    // Get file metadata
                    match fs::metadata(path) {
                        Ok(metadata) => {
                            let candidate = CandidateFile {
                                path: path.to_path_buf(),
                                size_bytes: metadata.len(),
                                modified_time: metadata
                                    .modified()
                                    .unwrap_or_else(|_| SystemTime::now()),
                            };
                            candidates.push(candidate);
                        }
                        Err(e) => {
                            warn!("Failed to get metadata for {}: {}", path.display(), e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // Log warning but continue scanning
                    warn!("Error accessing directory entry: {}", e);
                    continue;
                }
            }
        }
    }

    debug!("Found {} candidate video files", candidates.len());
    Ok(candidates)
}

/// Check if a file has a video extension
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = format!(".{}", ext.to_lowercase());
            VIDEO_EXTENSIONS.contains(&ext_lower.as_str())
        })
        .unwrap_or(false)
}
