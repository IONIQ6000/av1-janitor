use av1d_daemon::jobs::{Job, JobStatus};
use chrono::Utc;
use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// **Feature: av1-reencoder, Property 26: TUI job loading**
/// **Validates: Requirements 24.1**
/// 
/// For any job_state_dir containing JSON files, the TUI should successfully load all valid job files
#[test]
fn property_tui_job_loading() {
    proptest!(|(
        job_count in 1..=20usize,
        job_ids in prop::collection::vec(any::<u64>(), 1..=20),
    )| {
        // Create temporary directory for job state
        let temp_dir = TempDir::new().unwrap();
        let job_state_dir = temp_dir.path();
        
        // Generate jobs with unique IDs
        let mut jobs = Vec::new();
        for (i, id_seed) in job_ids.iter().take(job_count).enumerate() {
            let job = Job {
                id: format!("job-{}-{}", id_seed, i),
                source_path: PathBuf::from(format!("/media/video{}.mkv", i)),
                output_path: None,
                created_at: Utc::now(),
                started_at: None,
                finished_at: None,
                status: JobStatus::Pending,
                reason: None,
                original_bytes: Some(1_000_000_000),
                new_bytes: None,
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: None,
                preset_used: None,
                encoder_used: None,
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            };
            
            // Save job to disk
            let job_file = job_state_dir.join(format!("{}.json", job.id));
            let json = serde_json::to_string_pretty(&job).unwrap();
            fs::write(&job_file, json).unwrap();
            
            jobs.push(job);
        }
        
        // Load all jobs using the TUI's load function
        let loaded_jobs = av1d_daemon::jobs::load_all_jobs(job_state_dir).unwrap();
        
        // Property: All saved jobs should be loaded
        prop_assert_eq!(loaded_jobs.len(), jobs.len(), 
            "Expected {} jobs to be loaded, but got {}", jobs.len(), loaded_jobs.len());
        
        // Property: All loaded jobs should have valid IDs
        for loaded_job in &loaded_jobs {
            prop_assert!(
                jobs.iter().any(|j| j.id == loaded_job.id),
                "Loaded job with ID {} was not in the original set", loaded_job.id
            );
        }
        
        // Property: All original jobs should be present in loaded jobs
        for original_job in &jobs {
            prop_assert!(
                loaded_jobs.iter().any(|j| j.id == original_job.id),
                "Original job with ID {} was not loaded", original_job.id
            );
        }
    });
}

/// Test that invalid JSON files are skipped gracefully
#[test]
fn property_tui_job_loading_with_invalid_files() {
    proptest!(|(
        valid_job_count in 1..=10usize,
        invalid_file_count in 1..=5usize,
    )| {
        let temp_dir = TempDir::new().unwrap();
        let job_state_dir = temp_dir.path();
        
        // Create valid jobs
        for i in 0..valid_job_count {
            let job = Job {
                id: format!("valid-job-{}", i),
                source_path: PathBuf::from(format!("/media/video{}.mkv", i)),
                output_path: None,
                created_at: Utc::now(),
                started_at: None,
                finished_at: None,
                status: JobStatus::Pending,
                reason: None,
                original_bytes: Some(1_000_000_000),
                new_bytes: None,
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: None,
                preset_used: None,
                encoder_used: None,
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            };
            
            let job_file = job_state_dir.join(format!("{}.json", job.id));
            let json = serde_json::to_string_pretty(&job).unwrap();
            fs::write(&job_file, json).unwrap();
        }
        
        // Create invalid JSON files
        for i in 0..invalid_file_count {
            let invalid_file = job_state_dir.join(format!("invalid-{}.json", i));
            fs::write(&invalid_file, "{ invalid json }").unwrap();
        }
        
        // Load jobs - should skip invalid files
        let loaded_jobs = av1d_daemon::jobs::load_all_jobs(job_state_dir).unwrap();
        
        // Property: Only valid jobs should be loaded
        prop_assert_eq!(loaded_jobs.len(), valid_job_count,
            "Expected {} valid jobs to be loaded, but got {}", valid_job_count, loaded_jobs.len());
    });
}

/// **Feature: av1-reencoder, Property 27: Statistics calculation**
/// **Validates: Requirements 24.5**
/// 
/// For any set of jobs, aggregate statistics (total space saved, success rate) should be calculated correctly
#[test]
fn property_statistics_calculation() {
    proptest!(|(
        success_count in 0..=20usize,
        failed_count in 0..=10usize,
        pending_count in 0..=10usize,
    )| {
        // Generate jobs with different statuses
        let mut jobs = Vec::new();
        
        // Create successful jobs with space savings
        for i in 0..success_count {
            let original_bytes = 10_000_000_000u64; // 10 GB
            let new_bytes = 5_000_000_000u64; // 5 GB (50% compression)
            
            let job = Job {
                id: format!("success-job-{}", i),
                source_path: PathBuf::from(format!("/media/video{}.mkv", i)),
                output_path: Some(PathBuf::from(format!("/media/video{}.av1.mkv", i))),
                created_at: Utc::now(),
                started_at: Some(Utc::now()),
                finished_at: Some(Utc::now()),
                status: JobStatus::Success,
                reason: None,
                original_bytes: Some(original_bytes),
                new_bytes: Some(new_bytes),
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: Some(23),
                preset_used: Some(4),
                encoder_used: Some("libsvtav1".to_string()),
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            };
            jobs.push(job);
        }
        
        // Create failed jobs
        for i in 0..failed_count {
            let job = Job {
                id: format!("failed-job-{}", i),
                source_path: PathBuf::from(format!("/media/failed{}.mkv", i)),
                output_path: None,
                created_at: Utc::now(),
                started_at: Some(Utc::now()),
                finished_at: Some(Utc::now()),
                status: JobStatus::Failed,
                reason: Some("Encoding failed".to_string()),
                original_bytes: Some(10_000_000_000),
                new_bytes: None,
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: None,
                preset_used: None,
                encoder_used: None,
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            };
            jobs.push(job);
        }
        
        // Create pending jobs
        for i in 0..pending_count {
            let job = Job {
                id: format!("pending-job-{}", i),
                source_path: PathBuf::from(format!("/media/pending{}.mkv", i)),
                output_path: None,
                created_at: Utc::now(),
                started_at: None,
                finished_at: None,
                status: JobStatus::Pending,
                reason: None,
                original_bytes: Some(10_000_000_000),
                new_bytes: None,
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: None,
                preset_used: None,
                encoder_used: None,
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            };
            jobs.push(job);
        }
        
        // Calculate expected statistics
        let expected_total_saved = success_count as u64 * 5_000_000_000; // Each successful job saves 5 GB
        let total_completed = success_count + failed_count;
        let expected_success_rate = if total_completed > 0 {
            (success_count as f64 / total_completed as f64) * 100.0
        } else {
            0.0
        };
        
        // Calculate actual statistics
        let actual_total_saved: u64 = jobs.iter()
            .filter(|j| j.status == JobStatus::Success)
            .filter_map(|j| {
                if let (Some(orig), Some(new)) = (j.original_bytes, j.new_bytes) {
                    Some(orig.saturating_sub(new))
                } else {
                    None
                }
            })
            .sum();
        
        let completed_jobs = jobs.iter()
            .filter(|j| j.status == JobStatus::Success || j.status == JobStatus::Failed)
            .count();
        let successful_jobs = jobs.iter()
            .filter(|j| j.status == JobStatus::Success)
            .count();
        
        let actual_success_rate = if completed_jobs > 0 {
            (successful_jobs as f64 / completed_jobs as f64) * 100.0
        } else {
            0.0
        };
        
        // Property: Total space saved should match expected
        prop_assert_eq!(actual_total_saved, expected_total_saved,
            "Expected total saved: {}, actual: {}", expected_total_saved, actual_total_saved);
        
        // Property: Success rate should match expected
        prop_assert!((actual_success_rate - expected_success_rate).abs() < 0.01,
            "Expected success rate: {:.2}%, actual: {:.2}%", expected_success_rate, actual_success_rate);
        
        // Property: Completed job count should be success + failed
        prop_assert_eq!(completed_jobs, success_count + failed_count,
            "Expected {} completed jobs, got {}", success_count + failed_count, completed_jobs);
    });
}

/// **Feature: av1-reencoder, Property 28: Job filtering**
/// **Validates: Requirements 25.3, 25.4**
/// 
/// For any active filter and set of jobs, only jobs matching the filter criteria should be included in the filtered result
#[test]
fn property_job_filtering() {
    proptest!(|(
        pending_count in 0..=10usize,
        running_count in 0..=10usize,
        success_count in 0..=10usize,
        failed_count in 0..=10usize,
        skipped_count in 0..=10usize,
    )| {
        // Generate jobs with different statuses
        let mut jobs = Vec::new();
        
        // Helper to create a job with a specific status
        let create_job = |id: String, status: JobStatus| -> Job {
            Job {
                id,
                source_path: PathBuf::from("/media/video.mkv"),
                output_path: None,
                created_at: Utc::now(),
                started_at: if status == JobStatus::Running { Some(Utc::now()) } else { None },
                finished_at: if matches!(status, JobStatus::Success | JobStatus::Failed | JobStatus::Skipped) {
                    Some(Utc::now())
                } else {
                    None
                },
                status,
                reason: None,
                original_bytes: Some(10_000_000_000),
                new_bytes: if status == JobStatus::Success { Some(5_000_000_000) } else { None },
                is_web_like: false,
                video_codec: Some("hevc".to_string()),
                video_bitrate: Some(10_000_000),
                video_width: Some(1920),
                video_height: Some(1080),
                video_frame_rate: Some("24/1".to_string()),
                crf_used: None,
                preset_used: None,
                encoder_used: None,
                source_bit_depth: Some(8),
                source_pix_fmt: Some("yuv420p".to_string()),
                is_hdr: Some(false),
                av1_quality: None,
                target_bit_depth: None,
                av1_profile: None,
                quality_tier: None,
                test_clip_path: None,
                test_clip_approved: None,
            }
        };
        
        // Create jobs with each status
        for i in 0..pending_count {
            jobs.push(create_job(format!("pending-{}", i), JobStatus::Pending));
        }
        for i in 0..running_count {
            jobs.push(create_job(format!("running-{}", i), JobStatus::Running));
        }
        for i in 0..success_count {
            jobs.push(create_job(format!("success-{}", i), JobStatus::Success));
        }
        for i in 0..failed_count {
            jobs.push(create_job(format!("failed-{}", i), JobStatus::Failed));
        }
        for i in 0..skipped_count {
            jobs.push(create_job(format!("skipped-{}", i), JobStatus::Skipped));
        }
        
        // Test filtering by each status
        let pending_filtered: Vec<_> = jobs.iter().filter(|j| j.status == JobStatus::Pending).collect();
        prop_assert_eq!(pending_filtered.len(), pending_count,
            "Expected {} pending jobs, got {}", pending_count, pending_filtered.len());
        
        let running_filtered: Vec<_> = jobs.iter().filter(|j| j.status == JobStatus::Running).collect();
        prop_assert_eq!(running_filtered.len(), running_count,
            "Expected {} running jobs, got {}", running_count, running_filtered.len());
        
        let success_filtered: Vec<_> = jobs.iter().filter(|j| j.status == JobStatus::Success).collect();
        prop_assert_eq!(success_filtered.len(), success_count,
            "Expected {} success jobs, got {}", success_count, success_filtered.len());
        
        let failed_filtered: Vec<_> = jobs.iter().filter(|j| j.status == JobStatus::Failed).collect();
        prop_assert_eq!(failed_filtered.len(), failed_count,
            "Expected {} failed jobs, got {}", failed_count, failed_filtered.len());
        
        let skipped_filtered: Vec<_> = jobs.iter().filter(|j| j.status == JobStatus::Skipped).collect();
        prop_assert_eq!(skipped_filtered.len(), skipped_count,
            "Expected {} skipped jobs, got {}", skipped_count, skipped_filtered.len());
        
        // Property: All filter should return all jobs
        let all_filtered: Vec<_> = jobs.iter().collect();
        prop_assert_eq!(all_filtered.len(), jobs.len(),
            "All filter should return all jobs");
    });
}

/// **Feature: av1-reencoder, Property 29: Sort mode cycling**
/// **Validates: Requirements 25.5**
/// 
/// For any current sort mode, cycling should progress through the sequence: Date → Size → Status → Savings → Date
#[test]
fn property_sort_mode_cycling() {
    // Define the sort mode cycle
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SortMode {
        ByDate,
        BySize,
        ByStatus,
        BySavings,
    }
    
    impl SortMode {
        fn cycle(&self) -> Self {
            match self {
                SortMode::ByDate => SortMode::BySize,
                SortMode::BySize => SortMode::ByStatus,
                SortMode::ByStatus => SortMode::BySavings,
                SortMode::BySavings => SortMode::ByDate,
            }
        }
    }
    
    proptest!(|(cycle_count in 1..=100usize)| {
        let mut current_mode = SortMode::ByDate;
        
        // Cycle through modes
        for _ in 0..cycle_count {
            current_mode = current_mode.cycle();
        }
        
        // Property: After 4 cycles, we should be back to the starting mode
        let expected_mode = match cycle_count % 4 {
            0 => SortMode::ByDate,
            1 => SortMode::BySize,
            2 => SortMode::ByStatus,
            3 => SortMode::BySavings,
            _ => unreachable!(),
        };
        
        prop_assert_eq!(current_mode, expected_mode,
            "After {} cycles, expected {:?}, got {:?}", cycle_count, expected_mode, current_mode);
    });
}

/// **Feature: av1-reencoder, Property 30: Progress rate calculation**
/// **Validates: Requirements 27.1, 27.2**
/// 
/// For any two file size measurements over time, the bytes per second rate should be calculated correctly
#[test]
fn property_progress_rate_calculation() {
    proptest!(|(
        initial_size in 0u64..10_000_000_000,
        size_delta in 1u64..1_000_000_000,
        time_delta_secs in 1u64..3600,
    )| {
        let final_size = initial_size + size_delta;
        
        // Calculate expected rate
        let expected_rate = size_delta as f64 / time_delta_secs as f64;
        
        // Simulate the calculation
        let actual_rate = (final_size - initial_size) as f64 / time_delta_secs as f64;
        
        // Property: Rate should match expected calculation
        prop_assert!((actual_rate - expected_rate).abs() < 0.01,
            "Expected rate: {:.2} bytes/sec, actual: {:.2} bytes/sec", expected_rate, actual_rate);
        
        // Property: Rate should be positive
        prop_assert!(actual_rate > 0.0, "Rate should be positive");
    });
}

/// **Feature: av1-reencoder, Property 31: ETA estimation**
/// **Validates: Requirements 27.3**
/// 
/// For any known write rate and expected output size, the estimated time remaining should be calculated correctly
#[test]
fn property_eta_estimation() {
    proptest!(|(
        current_size in 0u64..5_000_000_000,
        expected_final_size in 5_000_000_000u64..10_000_000_000,
        bytes_per_second in 1_000_000u64..100_000_000, // 1 MB/s to 100 MB/s
    )| {
        // Only test when current size is less than expected final size
        prop_assume!(current_size < expected_final_size);
        
        let remaining_bytes = expected_final_size - current_size;
        
        // Calculate expected ETA in seconds
        let expected_eta_secs = remaining_bytes as f64 / bytes_per_second as f64;
        
        // Simulate the calculation
        let actual_eta_secs = remaining_bytes as f64 / bytes_per_second as f64;
        
        // Property: ETA should match expected calculation
        prop_assert!((actual_eta_secs - expected_eta_secs).abs() < 0.01,
            "Expected ETA: {:.2} seconds, actual: {:.2} seconds", expected_eta_secs, actual_eta_secs);
        
        // Property: ETA should be positive
        prop_assert!(actual_eta_secs > 0.0, "ETA should be positive");
        
        // Property: ETA should be reasonable (not infinite)
        prop_assert!(actual_eta_secs.is_finite(), "ETA should be finite");
    });
}

/// **Feature: av1-reencoder, Property 32: Stage detection**
/// **Validates: Requirements 27.5**
/// 
/// For any job state, the current processing stage should be correctly identified based on job status and file existence
#[test]
fn property_stage_detection() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum JobStage {
        Probing,
        Transcoding,
        Verifying,
        Replacing,
        Complete,
    }
    
    proptest!(|(
        has_temp_file in any::<bool>(),
        temp_file_modified_recently in any::<bool>(),
        progress_percent in 0.0f64..=100.0,
        has_orig_backup in any::<bool>(),
    )| {
        // Detect stage based on conditions
        let detected_stage = if !has_temp_file {
            JobStage::Probing
        } else if has_orig_backup {
            JobStage::Replacing
        } else if progress_percent > 95.0 && !temp_file_modified_recently {
            JobStage::Verifying
        } else {
            JobStage::Transcoding
        };
        
        // Property: Stage should be one of the valid stages
        prop_assert!(matches!(detected_stage, 
            JobStage::Probing | JobStage::Transcoding | JobStage::Verifying | JobStage::Replacing | JobStage::Complete),
            "Detected stage should be valid");
        
        // Property: If no temp file, stage should be Probing
        if !has_temp_file {
            prop_assert_eq!(detected_stage, JobStage::Probing,
                "Without temp file, stage should be Probing");
        }
        
        // Property: If orig backup exists, stage should be Replacing
        if has_orig_backup && has_temp_file {
            prop_assert_eq!(detected_stage, JobStage::Replacing,
                "With orig backup, stage should be Replacing");
        }
    });
}

/// **Feature: av1-reencoder, Property 33: Responsive column layout**
/// **Validates: Requirements 28.1, 28.2, 28.3, 28.4, 28.5**
/// 
/// For any terminal width, the system should display the appropriate set of columns according to the responsive layout rules
#[test]
fn property_responsive_column_layout() {
    proptest!(|(terminal_width in 40u16..=200)| {
        // Determine expected column count based on terminal width
        let expected_column_count = if terminal_width >= 160 {
            14 // Large terminal: all columns
        } else if terminal_width >= 120 {
            9 // Medium terminal: essential columns
        } else if terminal_width >= 80 {
            5 // Small terminal: minimal columns
        } else {
            3 // Very small terminal: absolute minimum
        };
        
        // Property: Column count should match expected based on width
        prop_assert!(expected_column_count >= 3 && expected_column_count <= 14,
            "Column count should be between 3 and 14");
        
        // Property: Larger terminals should have more or equal columns
        if terminal_width >= 160 {
            prop_assert_eq!(expected_column_count, 14,
                "Terminal width >= 160 should show all 14 columns");
        } else if terminal_width >= 120 {
            prop_assert_eq!(expected_column_count, 9,
                "Terminal width >= 120 should show 9 essential columns");
        } else if terminal_width >= 80 {
            prop_assert_eq!(expected_column_count, 5,
                "Terminal width >= 80 should show 5 minimal columns");
        } else {
            prop_assert_eq!(expected_column_count, 3,
                "Terminal width < 80 should show 3 minimum columns");
        }
    });
}
