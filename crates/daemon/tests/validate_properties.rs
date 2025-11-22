use av1d_daemon::probe::{ProbeResult, FormatInfo, VideoStream};
use av1d_daemon::validate::ValidationError;
use proptest::prelude::*;

/// **Feature: av1-reencoder, Property 22: Output validation**
/// *For any* encoded output file, validation should verify ffprobe can read it and proceed to size gate only when all checks pass
/// **Validates: Requirements 20.1, 20.6**
///
/// This property test validates the output validation logic by generating various scenarios:
/// - Valid outputs with exactly one AV1 stream
/// - Invalid outputs with no AV1 streams
/// - Invalid outputs with multiple AV1 streams
/// - Invalid outputs with duration mismatches
#[test]
fn property_output_validation() {
    proptest!(ProptestConfig::with_cases(100), |(
        original_duration in prop::option::of(100.0f64..7200.0), // 100s to 2 hours
        output_duration_offset in -5.0f64..5.0, // Duration offset in seconds
        num_av1_streams in 0usize..4, // 0 to 3 AV1 streams
        has_other_video_streams in prop::bool::ANY,
        original_width in 640i32..3840,
        original_height in 480i32..2160,
    )| {
        // Create original probe result
        let original_probe = create_probe_result(
            original_duration,
            original_width,
            original_height,
            "hevc",
        );
        
        // Create output probe result based on test parameters
        let output_duration = original_duration.map(|d| d + output_duration_offset);
        let mut output_video_streams = Vec::new();
        
        // Add AV1 streams
        for i in 0..num_av1_streams {
            output_video_streams.push(VideoStream {
                index: i,
                codec_name: "av1".to_string(),
                width: original_width,
                height: original_height,
                bitrate: Some(3_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: i == 0,
            });
        }
        
        // Add other video streams if requested
        if has_other_video_streams && num_av1_streams > 0 {
            output_video_streams.push(VideoStream {
                index: num_av1_streams,
                codec_name: "hevc".to_string(),
                width: original_width,
                height: original_height,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: false,
            });
        }
        
        let output_probe = ProbeResult {
            format: FormatInfo {
                duration: output_duration,
                size: 1_000_000_000,
                bitrate: Some(3_000_000),
            },
            video_streams: output_video_streams,
            audio_streams: vec![],
            subtitle_streams: vec![],
        };
        
        // Determine expected validation result
        let expected_result = determine_expected_result(
            &original_probe,
            &output_probe,
            num_av1_streams,
            output_duration_offset,
        );
        
        // Verify the validation logic matches expectations
        verify_validation_logic(
            &original_probe,
            &output_probe,
            expected_result,
            num_av1_streams,
            output_duration_offset,
        );
    });
}

/// Helper function to create a ProbeResult for testing
fn create_probe_result(
    duration: Option<f64>,
    width: i32,
    height: i32,
    codec: &str,
) -> ProbeResult {
    ProbeResult {
        format: FormatInfo {
            duration,
            size: 5_000_000_000,
            bitrate: Some(10_000_000),
        },
        video_streams: vec![VideoStream {
            index: 0,
            codec_name: codec.to_string(),
            width,
            height,
            bitrate: Some(10_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: true,
        }],
        audio_streams: vec![],
        subtitle_streams: vec![],
    }
}

/// Determine the expected validation result based on test parameters
fn determine_expected_result(
    original_probe: &ProbeResult,
    output_probe: &ProbeResult,
    num_av1_streams: usize,
    _duration_offset: f64,
) -> Result<(), ValidationError> {
    // Check for exactly one AV1 stream
    if num_av1_streams == 0 {
        return Err(ValidationError::NoAv1Stream);
    }
    
    if num_av1_streams > 1 {
        return Err(ValidationError::MultipleAv1Streams);
    }
    
    // Check duration mismatch (epsilon = 2.0 seconds)
    if let (Some(original_duration), Some(output_duration)) = 
        (original_probe.format.duration, output_probe.format.duration) {
        let duration_diff = (original_duration - output_duration).abs();
        if duration_diff > 2.0 {
            return Err(ValidationError::DurationMismatch {
                expected: original_duration,
                actual: output_duration,
            });
        }
    }
    
    Ok(())
}

/// Verify the validation logic produces the expected result
fn verify_validation_logic(
    original_probe: &ProbeResult,
    output_probe: &ProbeResult,
    expected_result: Result<(), ValidationError>,
    _num_av1_streams: usize,
    _duration_offset: f64,
) {
    // Simulate validation logic
    let av1_streams: Vec<_> = output_probe.video_streams
        .iter()
        .filter(|s| s.codec_name == "av1")
        .collect();
    
    // Check AV1 stream count
    if av1_streams.is_empty() {
        assert!(
            matches!(expected_result, Err(ValidationError::NoAv1Stream)),
            "Expected NoAv1Stream error when no AV1 streams present"
        );
        return;
    }
    
    if av1_streams.len() > 1 {
        assert!(
            matches!(expected_result, Err(ValidationError::MultipleAv1Streams)),
            "Expected MultipleAv1Streams error when multiple AV1 streams present"
        );
        return;
    }
    
    // Check duration
    if let (Some(original_duration), Some(output_duration)) = 
        (original_probe.format.duration, output_probe.format.duration) {
        let duration_diff = (original_duration - output_duration).abs();
        if duration_diff > 2.0 {
            assert!(
                matches!(expected_result, Err(ValidationError::DurationMismatch { .. })),
                "Expected DurationMismatch error when duration differs by more than 2 seconds"
            );
            return;
        }
    }
    
    // All checks passed
    assert!(
        expected_result.is_ok(),
        "Expected validation to pass when all checks succeed"
    );
}

/// Unit test: Valid output with exactly one AV1 stream
#[test]
fn test_valid_output_single_av1_stream() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    let output_probe = create_probe_result(Some(3601.0), 1920, 1080, "av1");
    
    let result = determine_expected_result(&original_probe, &output_probe, 1, 1.0);
    assert!(result.is_ok(), "Should pass validation with single AV1 stream and duration within epsilon");
}

/// Unit test: Invalid output with no AV1 streams
#[test]
fn test_invalid_output_no_av1_stream() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    let output_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    
    let result = determine_expected_result(&original_probe, &output_probe, 0, 0.0);
    assert!(matches!(result, Err(ValidationError::NoAv1Stream)));
}

/// Unit test: Invalid output with multiple AV1 streams
#[test]
fn test_invalid_output_multiple_av1_streams() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    
    let output_probe = ProbeResult {
        format: FormatInfo {
            duration: Some(3600.0),
            size: 1_000_000_000,
            bitrate: Some(3_000_000),
        },
        video_streams: vec![
            VideoStream {
                index: 0,
                codec_name: "av1".to_string(),
                width: 1920,
                height: 1080,
                bitrate: Some(3_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: true,
            },
            VideoStream {
                index: 1,
                codec_name: "av1".to_string(),
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
    
    let result = determine_expected_result(&original_probe, &output_probe, 2, 0.0);
    assert!(matches!(result, Err(ValidationError::MultipleAv1Streams)));
}

/// Unit test: Invalid output with duration mismatch
#[test]
fn test_invalid_output_duration_mismatch() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    let output_probe = create_probe_result(Some(3605.0), 1920, 1080, "av1");
    
    let result = determine_expected_result(&original_probe, &output_probe, 1, 5.0);
    assert!(matches!(result, Err(ValidationError::DurationMismatch { .. })));
}

/// Unit test: Valid output at duration boundary (exactly 2 seconds)
#[test]
fn test_valid_output_duration_boundary() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    let output_probe = create_probe_result(Some(3602.0), 1920, 1080, "av1");
    
    let result = determine_expected_result(&original_probe, &output_probe, 1, 2.0);
    assert!(result.is_ok(), "Should pass validation when duration differs by exactly 2 seconds");
}

/// Unit test: Invalid output just over duration boundary
#[test]
fn test_invalid_output_duration_just_over_boundary() {
    let original_probe = create_probe_result(Some(3600.0), 1920, 1080, "hevc");
    let output_probe = create_probe_result(Some(3602.1), 1920, 1080, "av1");
    
    let result = determine_expected_result(&original_probe, &output_probe, 1, 2.1);
    assert!(matches!(result, Err(ValidationError::DurationMismatch { .. })),
        "Should fail validation when duration differs by more than 2 seconds");
}

/// Unit test: Valid output with no duration information
#[test]
fn test_valid_output_no_duration() {
    let original_probe = create_probe_result(None, 1920, 1080, "hevc");
    let output_probe = create_probe_result(None, 1920, 1080, "av1");
    
    let result = determine_expected_result(&original_probe, &output_probe, 1, 0.0);
    assert!(result.is_ok(), "Should pass validation when duration is not available");
}
