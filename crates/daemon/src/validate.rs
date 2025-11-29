use crate::probe::{probe_file, ProbeResult};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid(ProbeResult),
    Invalid(ValidationError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    ProbeFailure(String),
    NoAv1Stream,
    MultipleAv1Streams,
    DurationMismatch { expected: f64, actual: f64 },
}

/// Validate encoded output file
///
/// Checks:
/// 1. FFprobe can read the file
/// 2. Exactly one AV1 video stream exists
/// 3. Duration matches original within 2 seconds
pub async fn validate_output(
    output_path: &Path,
    original_probe: &ProbeResult,
) -> Result<ValidationResult> {
    // Execute ffprobe on output file
    let output_probe = match probe_file(output_path).await {
        Ok(probe) => probe,
        Err(e) => {
            return Ok(ValidationResult::Invalid(ValidationError::ProbeFailure(
                e.to_string(),
            )));
        }
    };

    // Check for exactly one AV1 video stream
    let av1_streams: Vec<_> = output_probe
        .video_streams
        .iter()
        .filter(|s| s.codec_name == "av1")
        .collect();

    if av1_streams.is_empty() {
        return Ok(ValidationResult::Invalid(ValidationError::NoAv1Stream));
    }

    if av1_streams.len() > 1 {
        return Ok(ValidationResult::Invalid(
            ValidationError::MultipleAv1Streams,
        ));
    }

    // Check duration matches original within epsilon (2 seconds)
    if let (Some(original_duration), Some(output_duration)) =
        (original_probe.format.duration, output_probe.format.duration)
    {
        let duration_diff = (original_duration - output_duration).abs();
        if duration_diff > 2.0 {
            return Ok(ValidationResult::Invalid(
                ValidationError::DurationMismatch {
                    expected: original_duration,
                    actual: output_duration,
                },
            ));
        }
    }

    // All validation checks passed
    Ok(ValidationResult::Valid(output_probe))
}
