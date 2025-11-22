use av1d_daemon::classify::{classify_source, SourceType};
use av1d_daemon::probe::{FormatInfo, ProbeResult, VideoStream};
use proptest::prelude::*;
use std::path::PathBuf;

/// **Feature: av1-reencoder, Property 11: Source classification scoring**
/// *For any* file path and video metadata, the classification system should correctly score 
/// WebLike and DiscLike indicators and assign the appropriate classification
/// **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8**
#[test]
fn property_source_classification_scoring() {
    proptest!(|(
        path_type in path_type_strategy(),
        resolution in resolution_strategy(),
        bitrate_type in bitrate_type_strategy(),
        codec in codec_strategy(),
        file_size_gb in file_size_strategy()
    )| {
        // Build path based on path_type
        let path = match &path_type {
            PathType::WebLike(keyword) => PathBuf::from(format!("/media/shows/{}_show.mkv", keyword)),
            PathType::DiscLike(keyword) => PathBuf::from(format!("/media/movies/{}_movie.mkv", keyword)),
            PathType::Neutral => PathBuf::from("/media/video/movie.mkv"),
        };

        // Build probe result
        let (width, height) = resolution;
        let bitrate = match bitrate_type {
            BitrateType::Low => Some(calculate_low_bitrate(height)),
            BitrateType::High => Some(calculate_high_bitrate(height)),
            BitrateType::Medium => Some(calculate_medium_bitrate(height)),
            BitrateType::None => None,
        };

        let file_size_bytes = (file_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;

        let probe = ProbeResult {
            format: FormatInfo {
                duration: Some(7200.0),
                size: file_size_bytes,
                bitrate,
            },
            video_streams: vec![VideoStream {
                index: 0,
                codec_name: codec.clone(),
                width,
                height,
                bitrate,
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: true,
            }],
            audio_streams: vec![],
            subtitle_streams: vec![],
        };

        // Classify
        let classification = classify_source(&path, &probe);

        // Verify scoring logic
        let expected_web_score = calculate_expected_web_score(&path_type, &bitrate_type, height, &codec);
        let expected_disc_score = calculate_expected_disc_score(&path_type, &bitrate_type, height, file_size_gb);

        prop_assert_eq!(classification.web_score, expected_web_score,
            "Web score mismatch for path={:?}, bitrate_type={:?}, height={}, codec={}",
            path, bitrate_type, height, codec);

        prop_assert_eq!(classification.disc_score, expected_disc_score,
            "Disc score mismatch for path={:?}, bitrate_type={:?}, height={}, file_size_gb={}",
            path, bitrate_type, height, file_size_gb);

        // Verify classification type matches scores
        let expected_type = if expected_web_score > expected_disc_score {
            SourceType::WebLike
        } else if expected_disc_score > expected_web_score {
            SourceType::DiscLike
        } else {
            SourceType::Unknown
        };

        prop_assert_eq!(classification.source_type, expected_type,
            "Classification type should be {:?} for web_score={} disc_score={}",
            expected_type, expected_web_score, expected_disc_score);

        // Verify reasons are populated when scores are non-zero
        if expected_web_score > 0 || expected_disc_score > 0 {
            prop_assert!(!classification.reasons.is_empty(),
                "Reasons should be populated when scores are non-zero");
        }
    });
}

#[derive(Debug, Clone)]
enum PathType {
    WebLike(String),
    DiscLike(String),
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BitrateType {
    Low,
    Medium,
    High,
    None,
}

fn path_type_strategy() -> impl Strategy<Value = PathType> {
    prop_oneof![
        prop::sample::select(vec!["WEB", "WEBRip", "WEBDL", "WEB-DL", "NF", "AMZN", "DSNP", "HULU", "ATVP"])
            .prop_map(|s| PathType::WebLike(s.to_string())),
        prop::sample::select(vec!["BluRay", "Blu-ray", "Remux", "BDMV", "UHD"])
            .prop_map(|s| PathType::DiscLike(s.to_string())),
        Just(PathType::Neutral),
    ]
}

fn resolution_strategy() -> impl Strategy<Value = (i32, i32)> {
    prop_oneof![
        Just((1920, 1080)),  // 1080p
        Just((3840, 2160)),  // 2160p (4K)
        Just((2560, 1440)),  // 1440p
        Just((1280, 720)),   // 720p
    ]
}

fn bitrate_type_strategy() -> impl Strategy<Value = BitrateType> {
    prop_oneof![
        Just(BitrateType::Low),
        Just(BitrateType::Medium),
        Just(BitrateType::High),
        Just(BitrateType::None),
    ]
}

fn codec_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("h264".to_string()),
        Just("hevc".to_string()),
        Just("vp9".to_string()),
        Just("av1".to_string()),
    ]
}

fn file_size_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![
        (1.0..10.0),   // Small files (1-10 GB)
        (10.0..20.0),  // Medium files (10-20 GB)
        (20.0..50.0),  // Large files (20-50 GB)
    ]
}

fn calculate_low_bitrate(height: i32) -> u64 {
    match height {
        h if h >= 2160 => 8_000_000,   // 8 Mbps for 4K (below 10 Mbps threshold)
        h if h >= 1080 => 3_000_000,   // 3 Mbps for 1080p (below 5 Mbps threshold)
        _ => 1_000_000,                // 1 Mbps for lower resolutions
    }
}

fn calculate_medium_bitrate(height: i32) -> u64 {
    match height {
        h if h >= 2160 => 25_000_000,  // 25 Mbps for 4K
        h if h >= 1080 => 8_000_000,   // 8 Mbps for 1080p
        _ => 3_000_000,                // 3 Mbps for lower resolutions
    }
}

fn calculate_high_bitrate(height: i32) -> u64 {
    match height {
        h if h >= 2160 => 50_000_000,  // 50 Mbps for 4K (above 40 Mbps threshold)
        h if h >= 1080 => 20_000_000,  // 20 Mbps for 1080p (above 15 Mbps threshold)
        _ => 10_000_000,               // 10 Mbps for lower resolutions
    }
}

fn calculate_expected_web_score(
    path_type: &PathType,
    bitrate_type: &BitrateType,
    height: i32,
    codec: &str,
) -> i32 {
    let mut score = 0;

    // Path keyword scoring
    if matches!(path_type, PathType::WebLike(_)) {
        score += 10;
    }

    // Bitrate scoring (only for 1080p and 2160p)
    if *bitrate_type == BitrateType::Low {
        if height >= 1080 {
            score += 5;
        }
    }

    // Codec scoring
    if codec.to_lowercase() == "vp9" {
        score += 5;
    }

    score
}

fn calculate_expected_disc_score(
    path_type: &PathType,
    bitrate_type: &BitrateType,
    height: i32,
    file_size_gb: f64,
) -> i32 {
    let mut score = 0;

    // Path keyword scoring
    if matches!(path_type, PathType::DiscLike(_)) {
        score += 10;
    }

    // Bitrate scoring (only for 1080p and 2160p)
    if *bitrate_type == BitrateType::High {
        if height >= 1080 {
            score += 5;
        }
    }

    // File size scoring
    if file_size_gb > 20.0 {
        score += 5;
    }

    score
}

/// Test that WebLike keywords are detected case-insensitively
#[test]
fn test_weblike_keyword_case_insensitivity() {
    let probe = create_minimal_probe(1920, 1080, Some(8_000_000), 5.0);

    let paths = vec![
        "/media/show.WEB.mkv",
        "/media/show.web.mkv",
        "/media/show.Web.mkv",
        "/media/show.WEBRIP.mkv",
        "/media/show.webrip.mkv",
    ];

    for path_str in paths {
        let path = PathBuf::from(path_str);
        let classification = classify_source(&path, &probe);
        assert!(
            classification.web_score > 0,
            "Path {} should have positive web score",
            path_str
        );
    }
}

/// Test that DiscLike keywords are detected case-insensitively
#[test]
fn test_disclike_keyword_case_insensitivity() {
    let probe = create_minimal_probe(1920, 1080, Some(20_000_000), 25.0);

    let paths = vec![
        "/media/movie.BLURAY.mkv",
        "/media/movie.bluray.mkv",
        "/media/movie.BluRay.mkv",
        "/media/movie.REMUX.mkv",
        "/media/movie.remux.mkv",
    ];

    for path_str in paths {
        let path = PathBuf::from(path_str);
        let classification = classify_source(&path, &probe);
        assert!(
            classification.disc_score > 0,
            "Path {} should have positive disc score",
            path_str
        );
    }
}

/// Test classification with no indicators (should be Unknown)
#[test]
fn test_neutral_classification() {
    let probe = create_minimal_probe(1920, 1080, Some(8_000_000), 5.0);
    let path = PathBuf::from("/media/video/movie.mkv");

    let classification = classify_source(&path, &probe);

    assert_eq!(classification.source_type, SourceType::Unknown);
    assert_eq!(classification.web_score, 0);
    assert_eq!(classification.disc_score, 0);
}

/// Test that VP9 codec increases web score
#[test]
fn test_vp9_codec_scoring() {
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: 5_000_000_000,
            bitrate: Some(8_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "vp9".to_string(),
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

    let path = PathBuf::from("/media/video/movie.mkv");
    let classification = classify_source(&path, &probe);

    assert_eq!(classification.web_score, 5, "VP9 codec should add 5 to web score");
}

/// Test that large file size increases disc score
#[test]
fn test_large_file_size_scoring() {
    let file_size_bytes = (25.0 * 1024.0 * 1024.0 * 1024.0) as u64; // 25 GB
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: file_size_bytes,
            bitrate: Some(20_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "hevc".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(20_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let path = PathBuf::from("/media/video/movie.mkv");
    let classification = classify_source(&path, &probe);

    // Should get +5 for high bitrate (>15 Mbps for 1080p) and +5 for large file (>20GB)
    assert_eq!(classification.disc_score, 10, "Large file (>20GB) and high bitrate should add 10 to disc score");
}

/// Test combined scoring: WebLike path + low bitrate
#[test]
fn test_combined_weblike_scoring() {
    let probe = create_minimal_probe(2160, 3840, Some(8_000_000), 5.0);
    let path = PathBuf::from("/media/shows/Show.WEBRip.mkv");

    let classification = classify_source(&path, &probe);

    assert_eq!(classification.source_type, SourceType::WebLike);
    assert_eq!(classification.web_score, 15); // 10 (path) + 5 (low bitrate)
    assert_eq!(classification.disc_score, 0);
}

/// Test combined scoring: DiscLike path + high bitrate + large file
#[test]
fn test_combined_disclike_scoring() {
    let file_size_bytes = (30.0 * 1024.0 * 1024.0 * 1024.0) as u64; // 30 GB
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: file_size_bytes,
            bitrate: Some(50_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "hevc".to_string(),
            width: 3840,
            height: 2160,
            bitrate: Some(50_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(10),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let path = PathBuf::from("/media/movies/Movie.BluRay.Remux.mkv");
    let classification = classify_source(&path, &probe);

    assert_eq!(classification.source_type, SourceType::DiscLike);
    assert_eq!(classification.disc_score, 20); // 10 (path) + 5 (high bitrate) + 5 (large file)
    assert_eq!(classification.web_score, 0);
}

/// Test that no video streams results in Unknown classification
#[test]
fn test_no_video_streams() {
    let probe = ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: 5_000_000_000,
            bitrate: Some(8_000_000),
        },
        video_streams: vec![],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    let path = PathBuf::from("/media/shows/Show.WEBRip.mkv");
    let classification = classify_source(&path, &probe);

    // Should still detect path keyword
    assert_eq!(classification.web_score, 10);
    assert_eq!(classification.disc_score, 0);
    assert_eq!(classification.source_type, SourceType::WebLike);
}

/// Test bitrate thresholds for 1080p
#[test]
fn test_1080p_bitrate_thresholds() {
    // Low bitrate (< 5 Mbps) should increase web score
    let probe_low = create_minimal_probe(1920, 1080, Some(4_000_000), 5.0);
    let path = PathBuf::from("/media/video/movie.mkv");
    let classification_low = classify_source(&path, &probe_low);
    assert_eq!(classification_low.web_score, 5);

    // High bitrate (> 15 Mbps) should increase disc score
    let probe_high = create_minimal_probe(1920, 1080, Some(20_000_000), 5.0);
    let classification_high = classify_source(&path, &probe_high);
    assert_eq!(classification_high.disc_score, 5);

    // Medium bitrate should not affect scores
    let probe_medium = create_minimal_probe(1920, 1080, Some(10_000_000), 5.0);
    let classification_medium = classify_source(&path, &probe_medium);
    assert_eq!(classification_medium.web_score, 0);
    assert_eq!(classification_medium.disc_score, 0);
}

/// Test bitrate thresholds for 2160p (4K)
#[test]
fn test_2160p_bitrate_thresholds() {
    // Low bitrate (< 10 Mbps) should increase web score
    let probe_low = create_minimal_probe(3840, 2160, Some(8_000_000), 5.0);
    let path = PathBuf::from("/media/video/movie.mkv");
    let classification_low = classify_source(&path, &probe_low);
    assert_eq!(classification_low.web_score, 5);

    // High bitrate (> 40 Mbps) should increase disc score
    let probe_high = create_minimal_probe(3840, 2160, Some(50_000_000), 5.0);
    let classification_high = classify_source(&path, &probe_high);
    assert_eq!(classification_high.disc_score, 5);

    // Medium bitrate should not affect scores
    let probe_medium = create_minimal_probe(3840, 2160, Some(25_000_000), 5.0);
    let classification_medium = classify_source(&path, &probe_medium);
    assert_eq!(classification_medium.web_score, 0);
    assert_eq!(classification_medium.disc_score, 0);
}

/// Helper function to create a minimal ProbeResult for testing
fn create_minimal_probe(width: i32, height: i32, bitrate: Option<u64>, file_size_gb: f64) -> ProbeResult {
    let file_size_bytes = (file_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;
    ProbeResult {
        format: FormatInfo {
            duration: Some(7200.0),
            size: file_size_bytes,
            bitrate,
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width,
            height,
            bitrate,
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    }
}
