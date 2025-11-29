use crate::config::DaemonConfig;
use crate::probe::ProbeResult;
use crate::scan::CandidateFile;
use crate::sidecars::has_skip_marker;

#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Pass,
    Skip(SkipReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkipReason {
    NoVideo,
    TooSmall,
    AlreadyAv1,
    HasSkipMarker,
}

/// Evaluate all gates to determine if a file should be encoded
/// Gates are checked in order:
/// 1. Skip marker exists
/// 2. No video streams
/// 3. File size too small
/// 4. Already encoded in AV1
pub fn check_gates(file: &CandidateFile, probe: &ProbeResult, config: &DaemonConfig) -> GateResult {
    // Gate 1: Check for skip marker
    if has_skip_marker(&file.path) {
        return GateResult::Skip(SkipReason::HasSkipMarker);
    }

    // Gate 2: Check for video streams
    if probe.video_streams.is_empty() {
        return GateResult::Skip(SkipReason::NoVideo);
    }

    // Gate 3: Check file size against minimum threshold
    if file.size_bytes <= config.min_bytes {
        return GateResult::Skip(SkipReason::TooSmall);
    }

    // Gate 4: Check if already encoded in AV1
    // Check the first video stream (or main video stream)
    if let Some(video_stream) = probe.video_streams.first() {
        if video_stream.codec_name.to_lowercase() == "av1" {
            return GateResult::Skip(SkipReason::AlreadyAv1);
        }
    }

    // All gates passed
    GateResult::Pass
}
