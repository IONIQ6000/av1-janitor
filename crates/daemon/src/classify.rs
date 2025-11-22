use std::path::Path;
use crate::probe::{ProbeResult, select_main_video_stream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    WebLike,
    DiscLike,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SourceClassification {
    pub source_type: SourceType,
    pub web_score: i32,
    pub disc_score: i32,
    pub reasons: Vec<String>,
}

/// Classify a video source as WebLike, DiscLike, or Unknown based on path keywords and metadata
pub fn classify_source(path: &Path, probe: &ProbeResult) -> SourceClassification {
    let mut web_score = 0;
    let mut disc_score = 0;
    let mut reasons = Vec::new();

    // Convert path to string for keyword matching
    let path_str = path.to_string_lossy().to_uppercase();

    // WebLike keyword detection (+10 each)
    let web_keywords = [
        "WEB", "WEBRIP", "WEBDL", "WEB-DL", 
        "NF", "AMZN", "DSNP", "HULU", "ATVP"
    ];
    
    for keyword in &web_keywords {
        if path_str.contains(keyword) {
            web_score += 10;
            reasons.push(format!("Path contains WebLike keyword: {}", keyword));
            break; // Only count once for path keywords
        }
    }

    // DiscLike keyword detection (+10 each)
    let disc_keywords = [
        "BLURAY", "BLU-RAY", "REMUX", "BDMV", "UHD"
    ];
    
    for keyword in &disc_keywords {
        if path_str.contains(keyword) {
            disc_score += 10;
            reasons.push(format!("Path contains DiscLike keyword: {}", keyword));
            break; // Only count once for path keywords
        }
    }

    // Get main video stream for bitrate analysis
    if let Some(video_stream) = select_main_video_stream(&probe.video_streams) {
        let height = video_stream.height;
        
        // Determine bitrate to use (prefer stream bitrate, fall back to format bitrate)
        let bitrate = video_stream.bitrate.or(probe.format.bitrate);

        if let Some(br) = bitrate {
            // Bitrate-based scoring for WebLike
            if height >= 2160 && br < 10_000_000 {
                // 2160p with bitrate < 10 Mbps
                web_score += 5;
                reasons.push(format!("Low bitrate for 2160p: {} bps", br));
            } else if height >= 1080 && height < 2160 && br < 5_000_000 {
                // 1080p with bitrate < 5 Mbps
                web_score += 5;
                reasons.push(format!("Low bitrate for 1080p: {} bps", br));
            }

            // Bitrate-based scoring for DiscLike
            if height >= 2160 && br > 40_000_000 {
                // 2160p with bitrate > 40 Mbps
                disc_score += 5;
                reasons.push(format!("High bitrate for 2160p: {} bps", br));
            } else if height >= 1080 && height < 2160 && br > 15_000_000 {
                // 1080p with bitrate > 15 Mbps
                disc_score += 5;
                reasons.push(format!("High bitrate for 1080p: {} bps", br));
            }
        }

        // Codec-based scoring
        if video_stream.codec_name.to_lowercase() == "vp9" {
            web_score += 5;
            reasons.push("Codec is VP9 (web-typical)".to_string());
        }
    }

    // File size-based scoring for DiscLike
    let file_size_gb = probe.format.size as f64 / (1024.0 * 1024.0 * 1024.0);
    if file_size_gb > 20.0 {
        disc_score += 5;
        reasons.push(format!("Large file size: {:.2} GB", file_size_gb));
    }

    // Determine final classification
    let source_type = if web_score > disc_score {
        SourceType::WebLike
    } else if disc_score > web_score {
        SourceType::DiscLike
    } else {
        SourceType::Unknown
    };

    SourceClassification {
        source_type,
        web_score,
        disc_score,
        reasons,
    }
}
