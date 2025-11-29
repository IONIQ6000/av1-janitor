use av1d_daemon::scan::CandidateFile;
use av1d_daemon::stable::check_stability;
use proptest::prelude::*;
use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// **Feature: av1-reencoder, Property 6: Stable file detection**
/// *For any* file whose size changes between two measurements, the system should mark it as unstable and skip it
/// **Validates: Requirements 6.3, 6.4**
#[test]
fn property_stable_file_detection() {
    proptest!(|(
        initial_size in 1000u64..10_000_000u64,
        should_modify in any::<bool>(),
        size_delta in 100i64..10000i64,
        wait_ms in 100u64..500u64 // Use shorter wait for tests
    )| {
        let rt = Runtime::new().unwrap();
        let result = rt.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test_video.mkv");

            // Create initial file with initial_size
            let initial_content = vec![0u8; initial_size as usize];
            fs::write(&file_path, &initial_content).unwrap();

            // Get metadata for CandidateFile
            let metadata = fs::metadata(&file_path).unwrap();
            let candidate = CandidateFile {
                path: file_path.clone(),
                size_bytes: metadata.len(),
                modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
            };

            // Spawn a task to modify the file if should_modify is true
            if should_modify {
                let file_path_clone = file_path.clone();
                tokio::spawn(async move {
                    // Wait a bit before modifying
                    tokio::time::sleep(Duration::from_millis(wait_ms / 2)).await;

                    // Modify file size
                    let new_size = (initial_size as i64 + size_delta).max(0) as u64;
                    let new_content = vec![0u8; new_size as usize];
                    let _ = fs::write(&file_path_clone, &new_content);
                });
            }

            // Check stability with short duration for testing
            let is_stable = check_stability(&candidate, Duration::from_millis(wait_ms)).await.unwrap();

            // Get final file size to check if modification actually happened
            let final_metadata = fs::metadata(&file_path).unwrap();
            let final_size = final_metadata.len();

            (is_stable, final_size)
        });

        let (is_stable, final_size) = result;
        let size_changed = final_size != initial_size;

        // Verify the result: stability should match whether size changed
        prop_assert_eq!(is_stable, !size_changed,
            "Stability detection mismatch: is_stable={}, size_changed={} (initial={}, final={})",
            is_stable, size_changed, initial_size, final_size);
    });
}

/// Test that a file with no changes is detected as stable
#[test]
fn test_stable_file_no_changes() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("stable.mkv");

        // Create file
        fs::write(&file_path, "stable content").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        // Check stability with short duration
        let is_stable = check_stability(&candidate, Duration::from_millis(100))
            .await
            .unwrap();

        assert!(is_stable, "File with no changes should be stable");
    });
}

/// Test that a file being written to is detected as unstable
#[test]
fn test_unstable_file_being_written() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("unstable.mkv");

        // Create initial file
        fs::write(&file_path, "initial content").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        // Spawn task to modify file during stability check
        let file_path_clone = file_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            fs::write(&file_path_clone, "modified content with more data").unwrap();
        });

        // Check stability
        let is_stable = check_stability(&candidate, Duration::from_millis(150))
            .await
            .unwrap();

        assert!(!is_stable, "File being written to should be unstable");
    });
}

/// Test that a file being appended to is detected as unstable
#[test]
fn test_unstable_file_growing() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("growing.mkv");

        // Create initial file
        let initial_content = vec![0u8; 1000];
        fs::write(&file_path, &initial_content).unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        // Spawn task to grow file during stability check
        let file_path_clone = file_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let new_content = vec![0u8; 2000]; // Double the size
            fs::write(&file_path_clone, &new_content).unwrap();
        });

        // Check stability
        let is_stable = check_stability(&candidate, Duration::from_millis(150))
            .await
            .unwrap();

        assert!(!is_stable, "Growing file should be unstable");
    });
}

/// Test that a file being truncated is detected as unstable
#[test]
fn test_unstable_file_shrinking() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("shrinking.mkv");

        // Create initial file
        let initial_content = vec![0u8; 2000];
        fs::write(&file_path, &initial_content).unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        // Spawn task to shrink file during stability check
        let file_path_clone = file_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let new_content = vec![0u8; 1000]; // Half the size
            fs::write(&file_path_clone, &new_content).unwrap();
        });

        // Check stability
        let is_stable = check_stability(&candidate, Duration::from_millis(150))
            .await
            .unwrap();

        assert!(!is_stable, "Shrinking file should be unstable");
    });
}

/// Test error handling when file is deleted during stability check
#[test]
fn test_file_deleted_during_check() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("deleted.mkv");

        // Create initial file
        fs::write(&file_path, "content").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        // Spawn task to delete file during stability check
        let file_path_clone = file_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = fs::remove_file(&file_path_clone);
        });

        // Check stability - should return error
        let result = check_stability(&candidate, Duration::from_millis(150)).await;

        assert!(result.is_err(), "Should return error when file is deleted");
    });
}

/// Test with various file sizes
#[test]
fn test_stability_various_sizes() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();

        let sizes = vec![0, 1, 100, 1024, 1024 * 1024, 100 * 1024 * 1024];

        for size in sizes {
            let file_path = temp_dir.path().join(format!("size_{}.mkv", size));
            let content = vec![0u8; size];
            fs::write(&file_path, &content).unwrap();

            let metadata = fs::metadata(&file_path).unwrap();
            let candidate = CandidateFile {
                path: file_path.clone(),
                size_bytes: metadata.len(),
                modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
            };

            let is_stable = check_stability(&candidate, Duration::from_millis(100))
                .await
                .unwrap();

            assert!(
                is_stable,
                "Stable file of size {} should be detected as stable",
                size
            );
        }
    });
}

/// Test with zero-byte file
#[test]
fn test_zero_byte_file_stability() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.mkv");

        // Create empty file
        fs::write(&file_path, "").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();
        let candidate = CandidateFile {
            path: file_path.clone(),
            size_bytes: metadata.len(),
            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        };

        let is_stable = check_stability(&candidate, Duration::from_millis(100))
            .await
            .unwrap();

        assert!(is_stable, "Empty file should be stable if not modified");
    });
}
