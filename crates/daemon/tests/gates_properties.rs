use av1d_daemon::config::{DaemonConfig, EncoderPreference, QualityTier};
use av1d_daemon::gates::{check_gates, GateResult, SkipReason};
use av1d_daemon::probe::{AudioStream, FormatInfo, ProbeResult, VideoStream};
use av1d_daemon::scan::CandidateFile;
use av1d_daemon::sidecars::create_skip_marker;
use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tempfile::TempDir;

/// **Feature: av1-reencoder, Property 9: Size threshold enforcement**
/// *For any* file size and min_bytes threshold, files at or below the threshold should be skipped with appropriate sidecar files
/// **Validates: Requirements 8.1, 8.2, 8.3, 8.4**
#[test]
fn property_size_threshold_enforcement() {
    proptest!(|(
        file_size in 0u64..10_000_000_000u64, // 0 to 10GB
        min_bytes in 0u64..10_000_000_000u64,
    )| {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test.mkv");

        // Create a dummy video file
        fs::write(&video_path, vec![0u8; 100]).unwrap();

        // Create candidate file with specified size
        let candidate = CandidateFile {
            path: video_path.clone(),
            size_bytes: file_size,
            modified_time: SystemTime::now(),
        };

        // Create probe result with at least one video stream (not AV1)
        let probe = ProbeResult {
            format: FormatInfo {
                duration: Some(3600.0),
                size: file_size,
                bitrate: Some(5_000_000),
            },
            video_streams: vec![VideoStream {
                index: 0,
                codec_name: "h264".to_string(),
                width: 1920,
                height: 1080,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: true,
            }],
            audio_streams: vec![],
            subtitle_streams: vec![],
        };

        // Create config with specified min_bytes
        let config = DaemonConfig {
            library_roots: vec![],
            min_bytes,
            max_size_ratio: 0.9,
            scan_interval_secs: 60,
            job_state_dir: PathBuf::from("/tmp"),
            temp_output_dir: PathBuf::from("/tmp"),
            max_concurrent_jobs: 1,
            prefer_encoder: EncoderPreference::Svt,
            quality_tier: QualityTier::High,
            keep_original: false,
            write_why_sidecars: true,
        };

        // Check gates
        let result = check_gates(&candidate, &probe, &config);

        // Property: Files at or below min_bytes should be skipped
        if file_size <= min_bytes {
            prop_assert!(
                matches!(result, GateResult::Skip(SkipReason::TooSmall)),
                "File with size {} should be skipped when min_bytes is {}",
                file_size, min_bytes
            );
        } else {
            // File is large enough, should not be skipped for size reasons
            // (might be skipped for other reasons, but not TooSmall)
            prop_assert!(
                !matches!(result, GateResult::Skip(SkipReason::TooSmall)),
                "File with size {} should not be skipped for size when min_bytes is {}",
                file_size, min_bytes
            );
        }
    });
}

/// Test boundary condition: file size exactly equal to min_bytes
#[test]
fn test_size_threshold_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    let min_bytes = 1_000_000_000u64; // 1GB

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: min_bytes, // Exactly equal
        modified_time: SystemTime::now(),
    };

    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: min_bytes,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    // File size exactly equal to min_bytes should be skipped (<=)
    assert!(
        matches!(result, GateResult::Skip(SkipReason::TooSmall)),
        "File with size exactly equal to min_bytes should be skipped"
    );
}

/// Test that files just above threshold pass the size gate
#[test]
fn test_size_just_above_threshold() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    let min_bytes = 1_000_000_000u64; // 1GB

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: min_bytes + 1, // Just above threshold
        modified_time: SystemTime::now(),
    };

    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: min_bytes + 1,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    // Should not be skipped for size reasons
    assert!(
        !matches!(result, GateResult::Skip(SkipReason::TooSmall)),
        "File just above min_bytes should not be skipped for size"
    );
}

/// **Feature: av1-reencoder, Property 10: AV1 codec detection and skip**
/// *For any* video with codec name "av1", the system should skip processing and create appropriate sidecar files
/// **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
#[test]
fn property_av1_codec_detection_and_skip() {
    proptest!(|(
        codec_name in prop::sample::select(vec![
            "av1", "AV1", "Av1", "aV1", // Various cases
            "h264", "h265", "hevc", "vp9", "mpeg4", "mpeg2video" // Non-AV1 codecs
        ]),
        file_size in 3_000_000_000u64..10_000_000_000u64, // Large enough to pass size gate
    )| {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test.mkv");
        fs::write(&video_path, vec![0u8; 100]).unwrap();

        let candidate = CandidateFile {
            path: video_path.clone(),
            size_bytes: file_size,
            modified_time: SystemTime::now(),
        };

        // Create probe result with specified codec
        let probe = ProbeResult {
            format: FormatInfo {
                duration: Some(3600.0),
                size: file_size,
                bitrate: Some(5_000_000),
            },
            video_streams: vec![VideoStream {
                index: 0,
                codec_name: codec_name.to_string(),
                width: 1920,
                height: 1080,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: true,
            }],
            audio_streams: vec![],
            subtitle_streams: vec![],
        };

        let config = DaemonConfig {
            library_roots: vec![],
            min_bytes: 1_000_000_000, // 1GB - below our file size
            max_size_ratio: 0.9,
            scan_interval_secs: 60,
            job_state_dir: PathBuf::from("/tmp"),
            temp_output_dir: PathBuf::from("/tmp"),
            max_concurrent_jobs: 1,
            prefer_encoder: EncoderPreference::Svt,
            quality_tier: QualityTier::High,
            keep_original: false,
            write_why_sidecars: true,
        };

        let result = check_gates(&candidate, &probe, &config);

        // Property: Files with AV1 codec (case-insensitive) should be skipped
        if codec_name.to_lowercase() == "av1" {
            prop_assert!(
                matches!(result, GateResult::Skip(SkipReason::AlreadyAv1)),
                "File with codec {} should be skipped as already AV1",
                codec_name
            );
        } else {
            // Non-AV1 codecs should not be skipped for this reason
            prop_assert!(
                !matches!(result, GateResult::Skip(SkipReason::AlreadyAv1)),
                "File with codec {} should not be skipped as AV1",
                codec_name
            );
        }
    });
}

/// Test that files with no video streams are skipped
#[test]
fn test_no_video_streams() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: 5_000_000_000,
        modified_time: SystemTime::now(),
    };

    // Probe result with no video streams
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 5_000_000_000,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![], // No video streams
        audio_streams: vec![AudioStream {
            index: 0,
            codec_name: "aac".to_string(),
            language: Some("eng".to_string()),
        }],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes: 1_000_000_000,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    assert!(
        matches!(result, GateResult::Skip(SkipReason::NoVideo)),
        "File with no video streams should be skipped"
    );
}

/// Test that files with skip markers are skipped
#[test]
fn test_skip_marker_gate() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    // Create skip marker
    create_skip_marker(&video_path).unwrap();

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: 5_000_000_000,
        modified_time: SystemTime::now(),
    };

    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 5_000_000_000,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes: 1_000_000_000,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    assert!(
        matches!(result, GateResult::Skip(SkipReason::HasSkipMarker)),
        "File with skip marker should be skipped"
    );
}

/// Test gate evaluation order: skip marker is checked first
#[test]
fn test_gate_evaluation_order() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    // Create skip marker
    create_skip_marker(&video_path).unwrap();

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: 100, // Too small
        modified_time: SystemTime::now(),
    };

    // Probe with no video streams
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 100,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![], // No video
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes: 1_000_000_000,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    // Should return HasSkipMarker, not NoVideo or TooSmall
    // This tests that skip marker is checked first
    assert!(
        matches!(result, GateResult::Skip(SkipReason::HasSkipMarker)),
        "Skip marker should be checked before other gates"
    );
}

/// Test that valid files pass all gates
#[test]
fn test_valid_file_passes_gates() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: 5_000_000_000, // Large enough
        modified_time: SystemTime::now(),
    };

    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 5_000_000_000,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(), // Not AV1
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes: 1_000_000_000,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    assert!(
        matches!(result, GateResult::Pass),
        "Valid file should pass all gates"
    );
}

/// Test multiple video streams - should check first stream
#[test]
fn test_multiple_video_streams() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mkv");
    fs::write(&video_path, vec![0u8; 100]).unwrap();

    let candidate = CandidateFile {
        path: video_path.clone(),
        size_bytes: 5_000_000_000,
        modified_time: SystemTime::now(),
    };

    // Multiple video streams, first one is AV1
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 5_000_000_000,
            bitrate: Some(5_000_000),
        },
        video_streams: vec![
            VideoStream {
                index: 0,
                codec_name: "av1".to_string(), // First stream is AV1
                width: 1920,
                height: 1080,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: true,
            },
            VideoStream {
                index: 1,
                codec_name: "h264".to_string(), // Second stream is not
                width: 1920,
                height: 1080,
                bitrate: Some(3_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: false,
            },
        ],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let config = DaemonConfig {
        library_roots: vec![],
        min_bytes: 1_000_000_000,
        max_size_ratio: 0.9,
        scan_interval_secs: 60,
        job_state_dir: PathBuf::from("/tmp"),
        temp_output_dir: PathBuf::from("/tmp"),
        max_concurrent_jobs: 1,
        prefer_encoder: EncoderPreference::Svt,
        quality_tier: QualityTier::High,
        keep_original: false,
        write_why_sidecars: true,
    };

    let result = check_gates(&candidate, &probe, &config);

    // Should be skipped because first stream is AV1
    assert!(
        matches!(result, GateResult::Skip(SkipReason::AlreadyAv1)),
        "File with AV1 as first video stream should be skipped"
    );
}
