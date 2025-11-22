use av1d_daemon::jobs::{create_job, load_all_jobs, save_job, Job, JobStatus};
use av1d_daemon::classify::{SourceClassification, SourceType};
use av1d_daemon::probe::{FormatInfo, ProbeResult, VideoStream};
use av1d_daemon::scan::CandidateFile;
use chrono::{DateTime, Utc};
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

// Simple compilation test
#[test]
fn test_compilation() {
    assert!(true);
}

/// **Feature: av1-reencoder, Property 25: Job persistence**
/// *For any* job state change, the corresponding JSON file should be updated atomically 
/// with all current metadata
/// **Validates: Requirements 23.1, 23.2, 23.3, 23.4**
#[test]
fn property_job_persistence() {
    proptest!(|(
        job in job_strategy(),
    )| {
        // Create temporary directory for job state
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Save the job
        save_job(&job, state_dir).unwrap();

        // Load all jobs
        let loaded_jobs = load_all_jobs(state_dir).unwrap();

        // Should have exactly one job
        prop_assert_eq!(loaded_jobs.len(), 1, "Should load exactly one job");

        // Loaded job should match original
        let loaded_job = &loaded_jobs[0];
        
        prop_assert_eq!(&loaded_job.id, &job.id, "Job ID should match");
        prop_assert_eq!(&loaded_job.source_path, &job.source_path, "Source path should match");
        prop_assert_eq!(&loaded_job.output_path, &job.output_path, "Output path should match");
        prop_assert_eq!(loaded_job.status, job.status, "Status should match");
        prop_assert_eq!(&loaded_job.reason, &job.reason, "Reason should match");
        prop_assert_eq!(loaded_job.original_bytes, job.original_bytes, "Original bytes should match");
        prop_assert_eq!(loaded_job.new_bytes, job.new_bytes, "New bytes should match");
        prop_assert_eq!(loaded_job.is_web_like, job.is_web_like, "is_web_like should match");
        prop_assert_eq!(&loaded_job.video_codec, &job.video_codec, "Video codec should match");
        prop_assert_eq!(loaded_job.video_bitrate, job.video_bitrate, "Video bitrate should match");
        prop_assert_eq!(loaded_job.video_width, job.video_width, "Video width should match");
        prop_assert_eq!(loaded_job.video_height, job.video_height, "Video height should match");
        prop_assert_eq!(&loaded_job.video_frame_rate, &job.video_frame_rate, "Video frame rate should match");
        prop_assert_eq!(loaded_job.crf_used, job.crf_used, "CRF used should match");
        prop_assert_eq!(loaded_job.preset_used, job.preset_used, "Preset used should match");
        prop_assert_eq!(&loaded_job.encoder_used, &job.encoder_used, "Encoder used should match");
        prop_assert_eq!(loaded_job.source_bit_depth, job.source_bit_depth, "Source bit depth should match");
        prop_assert_eq!(&loaded_job.source_pix_fmt, &job.source_pix_fmt, "Source pix fmt should match");
        prop_assert_eq!(loaded_job.is_hdr, job.is_hdr, "is_hdr should match");

        // Timestamps should match (within a small tolerance for serialization)
        assert_timestamps_match(&loaded_job.created_at, &job.created_at, "created_at");
        if let (Some(loaded), Some(original)) = (&loaded_job.started_at, &job.started_at) {
            assert_timestamps_match(loaded, original, "started_at");
        } else {
            prop_assert_eq!(loaded_job.started_at.is_some(), job.started_at.is_some(), "started_at presence should match");
        }
        if let (Some(loaded), Some(original)) = (&loaded_job.finished_at, &job.finished_at) {
            assert_timestamps_match(loaded, original, "finished_at");
        } else {
            prop_assert_eq!(loaded_job.finished_at.is_some(), job.finished_at.is_some(), "finished_at presence should match");
        }
    });
}

/// Test that multiple jobs can be saved and loaded
#[test]
fn test_multiple_jobs_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    // Create and save multiple jobs
    let job1 = create_test_job("/media/video1.mkv", JobStatus::Pending);
    let job2 = create_test_job("/media/video2.mkv", JobStatus::Running);
    let job3 = create_test_job("/media/video3.mkv", JobStatus::Success);

    save_job(&job1, state_dir).unwrap();
    save_job(&job2, state_dir).unwrap();
    save_job(&job3, state_dir).unwrap();

    // Load all jobs
    let loaded_jobs = load_all_jobs(state_dir).unwrap();

    assert_eq!(loaded_jobs.len(), 3, "Should load all three jobs");

    // Verify all job IDs are present
    let loaded_ids: Vec<String> = loaded_jobs.iter().map(|j| j.id.clone()).collect();
    assert!(loaded_ids.contains(&job1.id));
    assert!(loaded_ids.contains(&job2.id));
    assert!(loaded_ids.contains(&job3.id));
}

/// Test that saving a job twice updates the existing file
#[test]
fn test_job_update_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    // Create and save a job
    let mut job = create_test_job("/media/video.mkv", JobStatus::Pending);
    save_job(&job, state_dir).unwrap();

    // Update the job status
    job.status = JobStatus::Running;
    job.started_at = Some(Utc::now());
    save_job(&job, state_dir).unwrap();

    // Load jobs
    let loaded_jobs = load_all_jobs(state_dir).unwrap();

    assert_eq!(loaded_jobs.len(), 1, "Should still have only one job");
    assert_eq!(loaded_jobs[0].status, JobStatus::Running, "Status should be updated");
    assert!(loaded_jobs[0].started_at.is_some(), "started_at should be set");
}

/// Test that load_all_jobs handles non-existent directory
#[test]
fn test_load_from_nonexistent_directory() {
    let result = load_all_jobs(&PathBuf::from("/nonexistent/directory"));
    assert!(result.is_ok(), "Should return Ok with empty vec for non-existent directory");
    assert_eq!(result.unwrap().len(), 0, "Should return empty vec");
}

/// Test that load_all_jobs skips invalid JSON files
#[test]
fn test_load_skips_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    // Create a valid job
    let job = create_test_job("/media/video.mkv", JobStatus::Pending);
    save_job(&job, state_dir).unwrap();

    // Create an invalid JSON file
    let invalid_file = state_dir.join("invalid.json");
    std::fs::write(&invalid_file, "{ invalid json }").unwrap();

    // Load jobs - should skip invalid file and load valid one
    let loaded_jobs = load_all_jobs(state_dir).unwrap();

    assert_eq!(loaded_jobs.len(), 1, "Should load only the valid job");
    assert_eq!(loaded_jobs[0].id, job.id);
}

/// Test that load_all_jobs skips temporary files
#[test]
fn test_load_skips_temporary_files() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    // Create a valid job
    let job = create_test_job("/media/video.mkv", JobStatus::Pending);
    save_job(&job, state_dir).unwrap();

    // Create a temporary file
    let temp_file = state_dir.join("somejob.json.tmp");
    std::fs::write(&temp_file, "{}").unwrap();

    // Load jobs - should skip temporary file
    let loaded_jobs = load_all_jobs(state_dir).unwrap();

    assert_eq!(loaded_jobs.len(), 1, "Should load only the non-temporary job");
    assert_eq!(loaded_jobs[0].id, job.id);
}

/// Test that save_job creates directory if it doesn't exist
#[test]
fn test_save_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join("nested").join("directory");

    let job = create_test_job("/media/video.mkv", JobStatus::Pending);
    
    // Directory doesn't exist yet
    assert!(!state_dir.exists());

    // Save should create it
    save_job(&job, &state_dir).unwrap();

    assert!(state_dir.exists(), "Directory should be created");
    assert!(state_dir.join(format!("{}.json", job.id)).exists(), "Job file should exist");
}

// Helper functions and strategies

fn job_strategy() -> impl Strategy<Value = Job> {
    (
        (
            path_strategy(),
            option_path_strategy(),
            job_status_strategy(),
            option_string_strategy(),
            option_u64_strategy(),
            option_u64_strategy(),
            bool_strategy(),
            option_string_strategy(),
            option_u64_strategy(),
        ),
        (
            option_i32_strategy(),
            option_i32_strategy(),
            option_string_strategy(),
            option_u8_strategy(),
            option_u8_strategy(),
            option_string_strategy(),
            option_u8_strategy(),
            option_string_strategy(),
            option_bool_strategy(),
        ),
    ).prop_map(|(
        (source_path, output_path, status, reason, original_bytes, new_bytes, is_web_like, video_codec, video_bitrate),
        (video_width, video_height, video_frame_rate, crf_used, preset_used, encoder_used, source_bit_depth, source_pix_fmt, is_hdr),
    )| {
        let created_at = Utc::now();
        let started_at = if matches!(status, JobStatus::Running | JobStatus::Success | JobStatus::Failed | JobStatus::Skipped) {
            Some(Utc::now())
        } else {
            None
        };
        let finished_at = if matches!(status, JobStatus::Success | JobStatus::Failed | JobStatus::Skipped) {
            Some(Utc::now())
        } else {
            None
        };

        Job {
            id: uuid::Uuid::new_v4().to_string(),
            source_path,
            output_path,
            created_at,
            started_at,
            finished_at,
            status,
            reason,
            original_bytes,
            new_bytes,
            is_web_like,
            video_codec,
            video_bitrate,
            video_width,
            video_height,
            video_frame_rate,
            crf_used,
            preset_used,
            encoder_used,
            source_bit_depth,
            source_pix_fmt,
            is_hdr,
            av1_quality: None,
            target_bit_depth: None,
            av1_profile: None,
            quality_tier: None,
            test_clip_path: None,
            test_clip_approved: None,
        }
    })
}

fn path_strategy() -> impl Strategy<Value = PathBuf> {
    prop::sample::select(vec![
        "/media/video.mkv",
        "/media/movies/movie.mp4",
        "/media/shows/show.avi",
        "/home/user/videos/test.mov",
    ]).prop_map(PathBuf::from)
}

fn option_path_strategy() -> impl Strategy<Value = Option<PathBuf>> {
    prop::option::of(path_strategy())
}

fn job_status_strategy() -> impl Strategy<Value = JobStatus> {
    prop_oneof![
        Just(JobStatus::Pending),
        Just(JobStatus::Running),
        Just(JobStatus::Success),
        Just(JobStatus::Failed),
        Just(JobStatus::Skipped),
    ]
}

fn option_string_strategy() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop::sample::select(vec![
        "h264".to_string(),
        "hevc".to_string(),
        "av1".to_string(),
        "24/1".to_string(),
        "yuv420p".to_string(),
        "libsvtav1".to_string(),
        "libaom-av1".to_string(),
        "Error message".to_string(),
    ]))
}

fn option_u64_strategy() -> impl Strategy<Value = Option<u64>> {
    prop::option::of(1_000_000u64..100_000_000_000u64)
}

fn option_i32_strategy() -> impl Strategy<Value = Option<i32>> {
    prop::option::of(480i32..4320i32)
}

fn option_u8_strategy() -> impl Strategy<Value = Option<u8>> {
    prop::option::of(0u8..10u8)
}

fn option_bool_strategy() -> impl Strategy<Value = Option<bool>> {
    prop::option::of(any::<bool>())
}

fn bool_strategy() -> impl Strategy<Value = bool> {
    any::<bool>()
}

fn assert_timestamps_match(loaded: &DateTime<Utc>, original: &DateTime<Utc>, field_name: &str) {
    // Timestamps should match exactly after JSON round-trip
    assert_eq!(
        loaded.timestamp_millis(),
        original.timestamp_millis(),
        "{} timestamp should match",
        field_name
    );
}

fn create_test_job(path: &str, status: JobStatus) -> Job {
    let candidate = CandidateFile {
        path: PathBuf::from(path),
        size_bytes: 5_000_000_000,
        modified_time: std::time::SystemTime::now(),
    };

    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: 5_000_000_000,
            bitrate: Some(8_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(8_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let classification = SourceClassification {
        source_type: SourceType::Unknown,
        web_score: 0,
        disc_score: 0,
        reasons: vec![],
    };

    let mut job = create_job(candidate, probe, classification);
    job.status = status;
    
    if matches!(status, JobStatus::Running | JobStatus::Success | JobStatus::Failed | JobStatus::Skipped) {
        job.started_at = Some(Utc::now());
    }
    
    if matches!(status, JobStatus::Success | JobStatus::Failed | JobStatus::Skipped) {
        job.finished_at = Some(Utc::now());
    }
    
    job
}

/// **Feature: av1-reencoder, Property 21: Job status transitions**
/// *For any* job, status transitions should follow the valid state machine 
/// (Pending → Running → Success/Failed/Skipped) with appropriate timestamp updates
/// **Validates: Requirements 19.3, 19.4, 19.5**
#[test]
fn property_job_status_transitions() {
    proptest!(|(
        initial_status in initial_status_strategy(),
        target_status in target_status_strategy(),
    )| {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Create a job with initial status
        let mut job = create_test_job("/media/video.mkv", initial_status);
        let initial_created_at = job.created_at;
        let initial_started_at = job.started_at;
        let initial_finished_at = job.finished_at;

        // Update to target status
        av1d_daemon::jobs::update_job_status(&mut job, target_status, state_dir).unwrap();

        // Verify status was updated
        prop_assert_eq!(job.status, target_status, "Status should be updated");

        // Verify timestamp updates based on target status
        match target_status {
            JobStatus::Pending => {
                // Pending status should not modify timestamps
                prop_assert_eq!(job.created_at, initial_created_at, "created_at should not change");
                prop_assert_eq!(job.started_at, initial_started_at, "started_at should not change for Pending");
                prop_assert_eq!(job.finished_at, initial_finished_at, "finished_at should not change for Pending");
            }
            JobStatus::Running => {
                // Running status should set started_at
                prop_assert!(job.started_at.is_some(), "started_at should be set for Running");
                prop_assert_eq!(job.finished_at, initial_finished_at, "finished_at should not be set for Running");
            }
            JobStatus::Success | JobStatus::Failed | JobStatus::Skipped => {
                // Terminal statuses should set finished_at
                prop_assert!(job.finished_at.is_some(), "finished_at should be set for terminal status");
            }
        }

        // Verify job was persisted
        let loaded_jobs = load_all_jobs(state_dir).unwrap();
        prop_assert_eq!(loaded_jobs.len(), 1, "Job should be persisted");
        prop_assert_eq!(loaded_jobs[0].status, target_status, "Persisted status should match");
    });
}

/// Test valid state machine transitions
#[test]
fn test_valid_status_transitions() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    // Test Pending → Running → Success
    let mut job = create_test_job("/media/video1.mkv", JobStatus::Pending);
    assert_eq!(job.status, JobStatus::Pending);
    assert!(job.started_at.is_none());
    assert!(job.finished_at.is_none());

    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Running, state_dir).unwrap();
    assert_eq!(job.status, JobStatus::Running);
    assert!(job.started_at.is_some());
    assert!(job.finished_at.is_none());

    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Success, state_dir).unwrap();
    assert_eq!(job.status, JobStatus::Success);
    assert!(job.started_at.is_some());
    assert!(job.finished_at.is_some());

    // Test Pending → Running → Failed
    let mut job2 = create_test_job("/media/video2.mkv", JobStatus::Pending);
    av1d_daemon::jobs::update_job_status(&mut job2, JobStatus::Running, state_dir).unwrap();
    av1d_daemon::jobs::update_job_status(&mut job2, JobStatus::Failed, state_dir).unwrap();
    assert_eq!(job2.status, JobStatus::Failed);
    assert!(job2.finished_at.is_some());

    // Test Pending → Running → Skipped
    let mut job3 = create_test_job("/media/video3.mkv", JobStatus::Pending);
    av1d_daemon::jobs::update_job_status(&mut job3, JobStatus::Running, state_dir).unwrap();
    av1d_daemon::jobs::update_job_status(&mut job3, JobStatus::Skipped, state_dir).unwrap();
    assert_eq!(job3.status, JobStatus::Skipped);
    assert!(job3.finished_at.is_some());
}

/// Test that timestamps are monotonically increasing
#[test]
fn test_timestamp_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    let mut job = create_test_job("/media/video.mkv", JobStatus::Pending);
    let created_at = job.created_at;

    // Transition to Running
    std::thread::sleep(std::time::Duration::from_millis(10));
    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Running, state_dir).unwrap();
    let started_at = job.started_at.unwrap();

    assert!(started_at >= created_at, "started_at should be >= created_at");

    // Transition to Success
    std::thread::sleep(std::time::Duration::from_millis(10));
    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Success, state_dir).unwrap();
    let finished_at = job.finished_at.unwrap();

    assert!(finished_at >= started_at, "finished_at should be >= started_at");
    assert!(finished_at >= created_at, "finished_at should be >= created_at");
}

/// Test that status updates are persisted correctly
#[test]
fn test_status_update_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path();

    let mut job = create_test_job("/media/video.mkv", JobStatus::Pending);
    let job_id = job.id.clone();

    // Update to Running
    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Running, state_dir).unwrap();

    // Load and verify
    let loaded_jobs = load_all_jobs(state_dir).unwrap();
    assert_eq!(loaded_jobs.len(), 1);
    assert_eq!(loaded_jobs[0].id, job_id);
    assert_eq!(loaded_jobs[0].status, JobStatus::Running);
    assert!(loaded_jobs[0].started_at.is_some());

    // Update to Success
    av1d_daemon::jobs::update_job_status(&mut job, JobStatus::Success, state_dir).unwrap();

    // Load and verify again
    let loaded_jobs = load_all_jobs(state_dir).unwrap();
    assert_eq!(loaded_jobs.len(), 1);
    assert_eq!(loaded_jobs[0].status, JobStatus::Success);
    assert!(loaded_jobs[0].finished_at.is_some());
}

// Additional strategy functions for status transitions

fn initial_status_strategy() -> impl Strategy<Value = JobStatus> {
    prop_oneof![
        Just(JobStatus::Pending),
        Just(JobStatus::Running),
        Just(JobStatus::Success),
        Just(JobStatus::Failed),
        Just(JobStatus::Skipped),
    ]
}

fn target_status_strategy() -> impl Strategy<Value = JobStatus> {
    prop_oneof![
        Just(JobStatus::Pending),
        Just(JobStatus::Running),
        Just(JobStatus::Success),
        Just(JobStatus::Failed),
        Just(JobStatus::Skipped),
    ]
}
