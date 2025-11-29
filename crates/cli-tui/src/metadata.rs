/// Metadata validation and formatting utilities for the TUI
///
/// This module provides helper functions for:
/// - Validating job metadata completeness
/// - Formatting optional values consistently
/// - Identifying missing metadata fields
use crate::models::Job;
use std::fmt::Display;

/// Check if job has all metadata needed for savings estimation
///
/// Required fields for estimation:
/// - original_bytes: Size of the original file
/// - video_codec: Source codec (affects compression potential)
/// - video_width: Video width in pixels
/// - video_height: Video height in pixels
/// - video_bitrate: Source bitrate
/// - video_frame_rate: Frame rate for duration calculations
pub fn has_estimation_metadata(job: &Job) -> bool {
    job.original_bytes.is_some()
        && job.video_codec.is_some()
        && job.video_width.is_some()
        && job.video_height.is_some()
        && job.video_bitrate.is_some()
        && job.video_frame_rate.is_some()
}

/// Check if job has complete video metadata
///
/// Complete metadata includes all estimation fields plus:
/// - source_bit_depth: Bit depth of source video
/// - source_pix_fmt: Pixel format
/// - is_hdr: HDR status
pub fn has_complete_video_metadata(job: &Job) -> bool {
    has_estimation_metadata(job)
        && job.source_bit_depth.is_some()
        && job.source_pix_fmt.is_some()
        && job.is_hdr.is_some()
}

/// Get list of missing metadata fields for a job
///
/// Returns a vector of field names that are missing and required for estimation.
/// Field abbreviations:
/// - "orig": original_bytes
/// - "codec": video_codec
/// - "w": video_width
/// - "h": video_height
/// - "br": video_bitrate
/// - "fps": video_frame_rate
pub fn get_missing_metadata_fields(job: &Job) -> Vec<&'static str> {
    let mut missing = Vec::new();

    if job.original_bytes.is_none() {
        missing.push("orig");
    }
    if job.video_codec.is_none() {
        missing.push("codec");
    }
    if job.video_width.is_none() {
        missing.push("w");
    }
    if job.video_height.is_none() {
        missing.push("h");
    }
    if job.video_bitrate.is_none() {
        missing.push("br");
    }
    if job.video_frame_rate.is_none() {
        missing.push("fps");
    }

    missing
}

/// Format optional value with fallback
///
/// Returns the formatted value if Some, otherwise returns the fallback string.
///
/// # Examples
/// ```
/// let value: Option<i32> = Some(42);
/// assert_eq!(format_optional(value, "-"), "42");
///
/// let value: Option<i32> = None;
/// assert_eq!(format_optional(value, "-"), "-");
/// ```
pub fn format_optional<T: Display>(value: Option<T>, fallback: &str) -> String {
    match value {
        Some(v) => v.to_string(),
        None => fallback.to_string(),
    }
}

/// Format file size with fallback
///
/// Returns human-readable size if Some, otherwise returns the fallback string.
/// Uses decimal units (GB, MB, KB) for consistency with storage industry standards.
pub fn format_size_optional(bytes: Option<u64>, fallback: &str) -> String {
    match bytes {
        Some(b) => {
            use humansize::{format_size, DECIMAL};
            format_size(b, DECIMAL)
        }
        None => fallback.to_string(),
    }
}

/// Format percentage with fallback
///
/// Returns formatted percentage (e.g., "45.2%") if Some, otherwise returns fallback.
/// Clamps percentage to 0-100 range for display.
pub fn format_percentage_optional(value: Option<f64>, fallback: &str) -> String {
    match value {
        Some(pct) => {
            let clamped = pct.max(0.0).min(100.0);
            format!("{:.1}%", clamped)
        }
        None => fallback.to_string(),
    }
}

/// Format missing metadata indicator
///
/// Creates a string showing which specific fields are missing.
/// Format: "-field1,field2,field3"
///
/// # Examples
/// ```
/// let missing = vec!["orig", "codec", "w"];
/// assert_eq!(format_missing_metadata(&missing), "-orig,codec,w");
///
/// let missing = vec![];
/// assert_eq!(format_missing_metadata(&missing), "");
/// ```
pub fn format_missing_metadata(missing_fields: &[&str]) -> String {
    if missing_fields.is_empty() {
        String::new()
    } else {
        format!("-{}", missing_fields.join(","))
    }
}

/// Format codec name consistently (uppercase)
///
/// Returns uppercase codec name if Some, otherwise returns fallback.
/// Ensures consistent display of codec names across the UI.
pub fn format_codec(codec: Option<&str>, fallback: &str) -> String {
    match codec {
        Some(c) => c.to_uppercase(),
        None => fallback.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::JobStatus;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_job_with_metadata(
        has_orig: bool,
        has_codec: bool,
        has_width: bool,
        has_height: bool,
        has_bitrate: bool,
        has_fps: bool,
    ) -> Job {
        Job {
            id: "test-job".to_string(),
            source_path: PathBuf::from("/test/video.mkv"),
            output_path: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            status: JobStatus::Pending,
            reason: None,
            original_bytes: if has_orig { Some(10_000_000_000) } else { None },
            new_bytes: None,
            is_web_like: false,
            video_codec: if has_codec {
                Some("hevc".to_string())
            } else {
                None
            },
            video_bitrate: if has_bitrate { Some(10_000_000) } else { None },
            video_width: if has_width { Some(1920) } else { None },
            video_height: if has_height { Some(1080) } else { None },
            video_frame_rate: if has_fps {
                Some("24/1".to_string())
            } else {
                None
            },
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
    }

    #[test]
    fn test_has_estimation_metadata_complete() {
        let job = create_test_job_with_metadata(true, true, true, true, true, true);
        assert!(has_estimation_metadata(&job));
    }

    #[test]
    fn test_has_estimation_metadata_missing_fields() {
        // Missing original_bytes
        let job = create_test_job_with_metadata(false, true, true, true, true, true);
        assert!(!has_estimation_metadata(&job));

        // Missing codec
        let job = create_test_job_with_metadata(true, false, true, true, true, true);
        assert!(!has_estimation_metadata(&job));

        // Missing width
        let job = create_test_job_with_metadata(true, true, false, true, true, true);
        assert!(!has_estimation_metadata(&job));
    }

    #[test]
    fn test_get_missing_metadata_fields() {
        // All fields present
        let job = create_test_job_with_metadata(true, true, true, true, true, true);
        assert_eq!(get_missing_metadata_fields(&job), Vec::<&str>::new());

        // Missing original_bytes
        let job = create_test_job_with_metadata(false, true, true, true, true, true);
        assert_eq!(get_missing_metadata_fields(&job), vec!["orig"]);

        // Missing multiple fields
        let job = create_test_job_with_metadata(false, false, true, true, false, true);
        assert_eq!(
            get_missing_metadata_fields(&job),
            vec!["orig", "codec", "br"]
        );
    }

    #[test]
    fn test_format_optional() {
        assert_eq!(format_optional(Some(42), "-"), "42");
        assert_eq!(format_optional(Some("test"), "-"), "test");
        assert_eq!(format_optional(None::<i32>, "-"), "-");
        assert_eq!(
            format_optional(None::<String>, "(not available)"),
            "(not available)"
        );
    }

    #[test]
    fn test_format_percentage_optional() {
        assert_eq!(format_percentage_optional(Some(45.67), "-"), "45.7%");
        assert_eq!(format_percentage_optional(Some(100.0), "-"), "100.0%");
        assert_eq!(format_percentage_optional(Some(0.0), "-"), "0.0%");
        assert_eq!(format_percentage_optional(None, "-"), "-");

        // Test clamping
        assert_eq!(format_percentage_optional(Some(150.0), "-"), "100.0%");
        assert_eq!(format_percentage_optional(Some(-10.0), "-"), "0.0%");
    }

    #[test]
    fn test_format_missing_metadata() {
        assert_eq!(format_missing_metadata(&[]), "");
        assert_eq!(format_missing_metadata(&["orig"]), "-orig");
        assert_eq!(
            format_missing_metadata(&["orig", "codec", "w"]),
            "-orig,codec,w"
        );
    }

    #[test]
    fn test_format_codec() {
        assert_eq!(format_codec(Some("hevc"), "-"), "HEVC");
        assert_eq!(format_codec(Some("h264"), "-"), "H264");
        assert_eq!(format_codec(Some("av1"), "-"), "AV1");
        assert_eq!(format_codec(None, "-"), "-");
    }
}
