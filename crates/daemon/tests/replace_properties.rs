use av1d_daemon::replace::atomic_replace;
use proptest::prelude::*;

use tempfile::TempDir;
use std::fs;

/// **Feature: av1-reencoder, Property 24: Atomic file replacement**
/// *For any* successful encoding, the replacement process should rename original to .orig, 
/// rename output to original name, and handle keep_original flag correctly
/// **Validates: Requirements 22.1, 22.2, 22.3, 22.4**
#[test]
fn property_atomic_file_replacement() {
    proptest!(|(
        original_content in prop::collection::vec(any::<u8>(), 100..10000),
        new_content in prop::collection::vec(any::<u8>(), 100..10000),
        keep_original in any::<bool>(),
    )| {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("video.mkv");
        let new_path = temp_dir.path().join("video.mkv.new");
        
        // Write original and new files
        fs::write(&original_path, &original_content).unwrap();
        fs::write(&new_path, &new_content).unwrap();
        
        // Perform atomic replacement
        let result = atomic_replace(&original_path, &new_path, keep_original);
        
        prop_assert!(result.is_ok(), "Atomic replacement should succeed: {:?}", result);
        
        // Property 1: Original path should now contain the new content
        let final_content = fs::read(&original_path).unwrap();
        prop_assert_eq!(
            final_content, new_content,
            "Original path should contain new content after replacement"
        );
        
        // Property 2: New file should no longer exist
        prop_assert!(
            !new_path.exists(),
            "New file should not exist after replacement"
        );
        
        // Property 3: Backup file behavior depends on keep_original flag
        let read_dir = fs::read_dir(temp_dir.path()).unwrap();
        let mut backup_files = Vec::new();
        for entry in read_dir {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.contains(".orig."))
                .unwrap_or(false)
            {
                backup_files.push(path);
            }
        }
        
        if keep_original {
            // Should have exactly one backup file
            prop_assert_eq!(
                backup_files.len(), 1,
                "Should have exactly one backup file when keep_original=true"
            );
            
            // Backup should contain original content
            let backup_content = fs::read(&backup_files[0]).unwrap();
            prop_assert_eq!(
                backup_content, original_content,
                "Backup file should contain original content"
            );
        } else {
            // Should have no backup files
            prop_assert_eq!(
                backup_files.len(), 0,
                "Should have no backup files when keep_original=false"
            );
        }
    });
}

/// Test that replacement fails gracefully when new file doesn't exist
#[test]
fn test_new_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    let new_path = temp_dir.path().join("video.mkv.new");
    
    // Create only the original file
    fs::write(&original_path, b"original content").unwrap();
    
    // Try to replace with non-existent new file
    let result = atomic_replace(&original_path, &new_path, false);
    
    assert!(result.is_err(), "Should fail when new file doesn't exist");
    
    // Original should still exist and be unchanged
    assert!(original_path.exists());
    let content = fs::read(&original_path).unwrap();
    assert_eq!(content, b"original content");
}

/// Test that replacement fails gracefully when original file doesn't exist
#[test]
fn test_original_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    let new_path = temp_dir.path().join("video.mkv.new");
    
    // Create only the new file
    fs::write(&new_path, b"new content").unwrap();
    
    // Try to replace non-existent original
    let result = atomic_replace(&original_path, &new_path, false);
    
    assert!(result.is_err(), "Should fail when original file doesn't exist");
    
    // New file should still exist
    assert!(new_path.exists());
}

/// Test successful replacement with keep_original=true
#[test]
fn test_keep_original_true() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    let new_path = temp_dir.path().join("video.mkv.new");
    
    let original_content = b"original content";
    let new_content = b"new content";
    
    fs::write(&original_path, original_content).unwrap();
    fs::write(&new_path, new_content).unwrap();
    
    let result = atomic_replace(&original_path, &new_path, true);
    
    assert!(result.is_ok(), "Replacement should succeed");
    
    // Check original path has new content
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content, new_content);
    
    // Check backup exists with original content
    let read_dir = fs::read_dir(temp_dir.path()).unwrap();
    let mut backup_files = Vec::new();
    for entry in read_dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.contains(".orig."))
            .unwrap_or(false)
        {
            backup_files.push(path);
        }
    }
    
    assert_eq!(backup_files.len(), 1, "Should have one backup file");
    
    let backup_content = fs::read(&backup_files[0]).unwrap();
    assert_eq!(backup_content, original_content);
}

/// Test successful replacement with keep_original=false
#[test]
fn test_keep_original_false() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    let new_path = temp_dir.path().join("video.mkv.new");
    
    let original_content = b"original content";
    let new_content = b"new content";
    
    fs::write(&original_path, original_content).unwrap();
    fs::write(&new_path, new_content).unwrap();
    
    let result = atomic_replace(&original_path, &new_path, false);
    
    assert!(result.is_ok(), "Replacement should succeed");
    
    // Check original path has new content
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content, new_content);
    
    // Check no backup exists
    let read_dir = fs::read_dir(temp_dir.path()).unwrap();
    let mut backup_files = Vec::new();
    for entry in read_dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.contains(".orig."))
            .unwrap_or(false)
        {
            backup_files.push(path);
        }
    }
    
    assert_eq!(backup_files.len(), 0, "Should have no backup files");
}

/// Test that multiple replacements create unique backup names
#[test]
fn test_multiple_replacements() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    
    // First replacement
    let new_path1 = temp_dir.path().join("video.mkv.new1");
    fs::write(&original_path, b"content 1").unwrap();
    fs::write(&new_path1, b"content 2").unwrap();
    
    atomic_replace(&original_path, &new_path1, true).unwrap();
    
    // Small delay to ensure different timestamp
    std::thread::sleep(std::time::Duration::from_millis(1100));
    
    // Second replacement
    let new_path2 = temp_dir.path().join("video.mkv.new2");
    fs::write(&new_path2, b"content 3").unwrap();
    
    atomic_replace(&original_path, &new_path2, true).unwrap();
    
    // Should have two backup files with different names
    let read_dir = fs::read_dir(temp_dir.path()).unwrap();
    let mut backup_files = Vec::new();
    for entry in read_dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.contains(".orig."))
            .unwrap_or(false)
        {
            backup_files.push(path);
        }
    }
    
    assert_eq!(backup_files.len(), 2, "Should have two backup files");
    
    // Final content should be from second replacement
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content, b"content 3");
}

/// Test replacement with large files
#[test]
fn test_large_file_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video.mkv");
    let new_path = temp_dir.path().join("video.mkv.new");
    
    // Create 10MB files
    let original_content = vec![0u8; 10 * 1024 * 1024];
    let new_content = vec![1u8; 10 * 1024 * 1024];
    
    fs::write(&original_path, &original_content).unwrap();
    fs::write(&new_path, &new_content).unwrap();
    
    let result = atomic_replace(&original_path, &new_path, false);
    
    assert!(result.is_ok(), "Large file replacement should succeed");
    
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content.len(), new_content.len());
    assert_eq!(final_content, new_content);
}

/// Test replacement with files in nested directories
#[test]
fn test_nested_directory_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("media").join("movies");
    fs::create_dir_all(&nested_dir).unwrap();
    
    let original_path = nested_dir.join("video.mkv");
    let new_path = nested_dir.join("video.mkv.new");
    
    fs::write(&original_path, b"original").unwrap();
    fs::write(&new_path, b"new").unwrap();
    
    let result = atomic_replace(&original_path, &new_path, true);
    
    assert!(result.is_ok(), "Nested directory replacement should succeed");
    
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content, b"new");
    
    // Check backup is in the same directory
    let read_dir = fs::read_dir(&nested_dir).unwrap();
    let mut backup_files = Vec::new();
    for entry in read_dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.contains(".orig."))
            .unwrap_or(false)
        {
            backup_files.push(path);
        }
    }
    
    assert_eq!(backup_files.len(), 1);
    assert_eq!(backup_files[0].parent().unwrap(), nested_dir);
}

/// Test replacement with special characters in filename
#[test]
fn test_special_characters_in_filename() {
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("video [1080p] (2024).mkv");
    let new_path = temp_dir.path().join("video [1080p] (2024).mkv.new");
    
    fs::write(&original_path, b"original").unwrap();
    fs::write(&new_path, b"new").unwrap();
    
    let result = atomic_replace(&original_path, &new_path, true);
    
    assert!(result.is_ok(), "Replacement with special chars should succeed");
    
    let final_content = fs::read(&original_path).unwrap();
    assert_eq!(final_content, b"new");
}
