use av1d_daemon::scan::{is_video_file, scan_libraries};
use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// **Feature: av1-reencoder, Property 4: Recursive file discovery**
/// *For any* directory structure, scanning should discover all files with allowed video extensions in all subdirectories
/// **Validates: Requirements 4.1, 4.2, 4.3**
#[test]
fn property_recursive_file_discovery() {
    proptest!(|(
        video_files in prop::collection::vec(video_file_name(), 1..20),
        non_video_files in prop::collection::vec(non_video_file_name(), 0..10),
        subdirs in prop::collection::vec(subdir_name(), 0..5)
    )| {
        // Create temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Track expected video files
        let mut expected_paths = Vec::new();

        // Create video files in root
        for (i, name) in video_files.iter().enumerate() {
            let path = root.join(name);
            fs::write(&path, format!("video content {}", i)).unwrap();
            expected_paths.push(path);
        }

        // Create non-video files in root (should be ignored)
        for (i, name) in non_video_files.iter().enumerate() {
            let path = root.join(name);
            fs::write(&path, format!("non-video content {}", i)).unwrap();
        }

        // Create subdirectories with video files
        for (dir_idx, subdir) in subdirs.iter().enumerate() {
            let subdir_path = root.join(subdir);
            fs::create_dir_all(&subdir_path).unwrap();

            // Add some video files to subdirectory
            for (file_idx, name) in video_files.iter().take(3).enumerate() {
                let filename = format!("sub_{}_{}", dir_idx, name);
                let path = subdir_path.join(&filename);
                fs::write(&path, format!("subdir video {}", file_idx)).unwrap();
                expected_paths.push(path);
            }
        }

        // Scan the directory
        let results = scan_libraries(&[root.to_path_buf()]).unwrap();

        // Verify all expected video files were found
        prop_assert_eq!(results.len(), expected_paths.len(),
            "Should find exactly {} video files, found {}",
            expected_paths.len(), results.len());

        // Verify each expected file is in results
        for expected_path in &expected_paths {
            let found = results.iter().any(|c| c.path == *expected_path);
            prop_assert!(found, "Expected file not found: {}", expected_path.display());
        }

        // Verify all results have valid metadata
        for candidate in &results {
            prop_assert!(candidate.size_bytes > 0, "File should have non-zero size");
            prop_assert!(candidate.path.exists(), "File path should exist");
        }

        // Verify no non-video files were included
        for non_video in &non_video_files {
            let non_video_path = root.join(non_video);
            let found = results.iter().any(|c| c.path == non_video_path);
            prop_assert!(!found, "Non-video file should not be included: {}", non_video);
        }
    });
}

/// Generate valid video file names with allowed extensions
fn video_file_name() -> impl Strategy<Value = String> {
    let extensions = vec![".mkv", ".mp4", ".avi", ".mov", ".m4v", ".ts", ".m2ts"];
    ("[a-zA-Z0-9_-]{3,20}", prop::sample::select(extensions))
        .prop_map(|(name, ext)| format!("{}{}", name, ext))
}

/// Generate non-video file names
fn non_video_file_name() -> impl Strategy<Value = String> {
    let extensions = vec![".txt", ".jpg", ".png", ".nfo", ".srt", ".sub"];
    ("[a-zA-Z0-9_-]{3,20}", prop::sample::select(extensions))
        .prop_map(|(name, ext)| format!("{}{}", name, ext))
}

/// Generate subdirectory names
fn subdir_name() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{3,15}".prop_map(|s| s)
}

/// Test is_video_file with various extensions
#[test]
fn property_video_file_extension_detection() {
    proptest!(|(
        filename in "[a-zA-Z0-9_-]{3,20}",
        ext in prop::sample::select(vec![
            ".mkv", ".mp4", ".avi", ".mov", ".m4v", ".ts", ".m2ts",
            ".MKV", ".MP4", ".AVI", // Test case insensitivity
        ])
    )| {
        let path = PathBuf::from(format!("{}{}", filename, ext));
        prop_assert!(is_video_file(&path),
            "File with extension {} should be recognized as video", ext);
    });
}

#[test]
fn property_non_video_file_rejection() {
    proptest!(|(
        filename in "[a-zA-Z0-9_-]{3,20}",
        ext in prop::sample::select(vec![
            ".txt", ".jpg", ".png", ".nfo", ".srt", ".sub", ".xml", ".json"
        ])
    )| {
        let path = PathBuf::from(format!("{}{}", filename, ext));
        prop_assert!(!is_video_file(&path),
            "File with extension {} should not be recognized as video", ext);
    });
}

/// Test handling of inaccessible directories
#[test]
fn test_inaccessible_directory_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a valid video file
    let video_path = root.join("test.mkv");
    fs::write(&video_path, "test content").unwrap();

    // Create a non-existent directory path
    let non_existent = root.join("does_not_exist");

    // Scan should handle non-existent directory gracefully
    let results = scan_libraries(&[root.to_path_buf(), non_existent]).unwrap();

    // Should still find the valid video file
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, video_path);
}

/// Test empty directory handling
#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let results = scan_libraries(&[root.to_path_buf()]).unwrap();
    assert_eq!(results.len(), 0, "Empty directory should return no files");
}

/// Test nested directory structure
#[test]
fn test_deeply_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create deeply nested structure: root/a/b/c/d/video.mkv
    let nested_path = root.join("a").join("b").join("c").join("d");
    fs::create_dir_all(&nested_path).unwrap();

    let video_path = nested_path.join("video.mkv");
    fs::write(&video_path, "nested video").unwrap();

    let results = scan_libraries(&[root.to_path_buf()]).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, video_path);
}

/// **Feature: av1-reencoder, Property 5: Skip marker enforcement**
/// *For any* file with a `.av1skip` sidecar, the system should skip processing and not create a job
/// **Validates: Requirements 5.1, 5.2, 5.3**
#[test]
fn property_skip_marker_enforcement() {
    proptest!(|(
        video_files_with_markers in prop::collection::vec(video_file_name(), 1..10),
        video_files_without_markers in prop::collection::vec(video_file_name(), 1..10)
    )| {
        use av1d_daemon::sidecars::create_skip_marker;
        
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create video files with skip markers
        for name in &video_files_with_markers {
            let path = root.join(name);
            fs::write(&path, "video with marker").unwrap();
            create_skip_marker(&path).unwrap();
        }
        
        // Create video files without skip markers
        for name in &video_files_without_markers {
            let path = root.join(name);
            fs::write(&path, "video without marker").unwrap();
        }
        
        // Scan the directory
        let results = scan_libraries(&[root.to_path_buf()]).unwrap();
        
        // Should only find files without markers
        prop_assert_eq!(results.len(), video_files_without_markers.len(),
            "Should only find {} files without markers, found {}",
            video_files_without_markers.len(), results.len());
        
        // Verify no files with markers are in results
        for name in &video_files_with_markers {
            let path = root.join(name);
            let found = results.iter().any(|c| c.path == path);
            prop_assert!(!found, "File with skip marker should not be in results: {}", name);
        }
        
        // Verify all files without markers are in results
        for name in &video_files_without_markers {
            let path = root.join(name);
            let found = results.iter().any(|c| c.path == path);
            prop_assert!(found, "File without skip marker should be in results: {}", name);
        }
    });
}

/// Test skip marker creation and detection
#[test]
fn test_skip_marker_creation() {
    use av1d_daemon::sidecars::{create_skip_marker, has_skip_marker};
    
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, "test video").unwrap();
    
    // Initially no skip marker
    assert!(!has_skip_marker(&video_path));
    
    // Create skip marker
    create_skip_marker(&video_path).unwrap();
    
    // Now should have skip marker
    assert!(has_skip_marker(&video_path));
    
    // Verify the marker file exists
    let marker_path = video_path.with_extension("mkv.av1skip");
    assert!(marker_path.exists());
}

/// Test skip marker with different video extensions
#[test]
fn test_skip_marker_various_extensions() {
    use av1d_daemon::sidecars::{create_skip_marker, has_skip_marker};
    
    let temp_dir = TempDir::new().unwrap();
    let extensions = vec!["mkv", "mp4", "avi", "mov", "m4v", "ts", "m2ts"];
    
    for ext in extensions {
        let video_path = temp_dir.path().join(format!("test.{}", ext));
        fs::write(&video_path, "test video").unwrap();
        
        create_skip_marker(&video_path).unwrap();
        assert!(has_skip_marker(&video_path), 
            "Skip marker should work for .{} files", ext);
    }
}

/// Test that scan respects skip markers in subdirectories
#[test]
fn test_skip_marker_in_subdirectories() {
    use av1d_daemon::sidecars::create_skip_marker;
    
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create subdirectory
    let subdir = root.join("subdir");
    fs::create_dir(&subdir).unwrap();
    
    // Create video with skip marker in subdirectory
    let video_with_marker = subdir.join("skip_me.mkv");
    fs::write(&video_with_marker, "skip this").unwrap();
    create_skip_marker(&video_with_marker).unwrap();
    
    // Create video without skip marker in subdirectory
    let video_without_marker = subdir.join("process_me.mkv");
    fs::write(&video_without_marker, "process this").unwrap();
    
    // Scan
    let results = scan_libraries(&[root.to_path_buf()]).unwrap();
    
    // Should only find the file without marker
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, video_without_marker);
}
