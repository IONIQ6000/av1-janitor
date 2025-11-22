use av1d_daemon::sidecars::{create_skip_marker, write_why_file, has_skip_marker};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_create_skip_marker() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mkv");
    
    // Create a dummy video file
    fs::write(&video_path, "dummy content").unwrap();
    
    // Initially, no skip marker should exist
    assert!(!has_skip_marker(&video_path));
    
    // Create skip marker
    create_skip_marker(&video_path).unwrap();
    
    // Now skip marker should exist
    assert!(has_skip_marker(&video_path));
    
    // Verify the skip marker file exists
    let skip_marker_path = video_path.with_extension("mkv.av1skip");
    assert!(skip_marker_path.exists());
}

#[test]
fn test_write_why_file() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    
    // Create a dummy video file
    fs::write(&video_path, "dummy content").unwrap();
    
    let reason = "File is too small (below min_bytes threshold)";
    
    // Write why file
    write_why_file(&video_path, reason).unwrap();
    
    // Verify the why file exists and contains the correct content
    let why_file_path = video_path.with_extension("mp4.why.txt");
    assert!(why_file_path.exists());
    
    let content = fs::read_to_string(&why_file_path).unwrap();
    assert_eq!(content, reason);
}

#[test]
fn test_has_skip_marker_returns_false_when_no_marker() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.avi");
    
    // Create a dummy video file
    fs::write(&video_path, "dummy content").unwrap();
    
    // Should return false when no marker exists
    assert!(!has_skip_marker(&video_path));
}

#[test]
fn test_skip_marker_with_various_extensions() {
    let temp_dir = TempDir::new().unwrap();
    
    let extensions = vec!["mkv", "mp4", "avi", "mov", "m4v", "ts", "m2ts"];
    
    for ext in extensions {
        let video_path = temp_dir.path().join(format!("test_video.{}", ext));
        fs::write(&video_path, "dummy content").unwrap();
        
        // Create skip marker
        create_skip_marker(&video_path).unwrap();
        
        // Verify it exists
        assert!(has_skip_marker(&video_path));
        
        // Verify the file has the correct extension
        let skip_marker_path = video_path.with_extension(format!("{}.av1skip", ext));
        assert!(skip_marker_path.exists());
    }
}

#[test]
fn test_skip_marker_and_why_file_together() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mkv");
    
    // Create a dummy video file
    fs::write(&video_path, "dummy content").unwrap();
    
    let reason = "Already encoded in AV1";
    
    // Create both skip marker and why file
    create_skip_marker(&video_path).unwrap();
    write_why_file(&video_path, reason).unwrap();
    
    // Verify both exist
    assert!(has_skip_marker(&video_path));
    
    let skip_marker_path = video_path.with_extension("mkv.av1skip");
    let why_file_path = video_path.with_extension("mkv.why.txt");
    
    assert!(skip_marker_path.exists());
    assert!(why_file_path.exists());
    
    // Verify why file content
    let content = fs::read_to_string(&why_file_path).unwrap();
    assert_eq!(content, reason);
}

#[test]
fn test_skip_marker_in_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("movies").join("action");
    fs::create_dir_all(&subdir).unwrap();
    
    let video_path = subdir.join("movie.mkv");
    fs::write(&video_path, "dummy content").unwrap();
    
    // Create skip marker
    create_skip_marker(&video_path).unwrap();
    
    // Verify it exists
    assert!(has_skip_marker(&video_path));
    
    let skip_marker_path = video_path.with_extension("mkv.av1skip");
    assert!(skip_marker_path.exists());
}

#[test]
fn test_overwrite_why_file() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mkv");
    
    // Create a dummy video file
    fs::write(&video_path, "dummy content").unwrap();
    
    // Write initial reason
    let reason1 = "First reason";
    write_why_file(&video_path, reason1).unwrap();
    
    let why_file_path = video_path.with_extension("mkv.why.txt");
    let content1 = fs::read_to_string(&why_file_path).unwrap();
    assert_eq!(content1, reason1);
    
    // Overwrite with new reason
    let reason2 = "Second reason - updated";
    write_why_file(&video_path, reason2).unwrap();
    
    let content2 = fs::read_to_string(&why_file_path).unwrap();
    assert_eq!(content2, reason2);
}
