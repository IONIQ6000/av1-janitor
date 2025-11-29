use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

// Public API types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbeResult {
    pub format: FormatInfo,
    pub video_streams: Vec<VideoStream>,
    pub audio_streams: Vec<AudioStream>,
    pub subtitle_streams: Vec<SubtitleStream>,
}

impl ProbeResult {
    /// Get the main video stream (prefers default, falls back to first)
    pub fn main_video_stream(&self) -> Option<&VideoStream> {
        select_main_video_stream(&self.video_streams)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormatInfo {
    pub duration: Option<f64>,
    pub size: u64,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoStream {
    pub index: usize,
    pub codec_name: String,
    pub width: i32,
    pub height: i32,
    pub bitrate: Option<u64>,
    pub frame_rate: Option<String>,
    pub pix_fmt: Option<String>,
    pub bit_depth: Option<u8>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioStream {
    pub index: usize,
    pub codec_name: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubtitleStream {
    pub index: usize,
    pub codec_name: String,
    pub language: Option<String>,
}

// Internal FFprobe JSON structures
#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: Option<FfprobeFormat>,
    streams: Option<Vec<FfprobeStream>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: usize,
    codec_type: String,
    codec_name: String,
    width: Option<i32>,
    height: Option<i32>,
    bit_rate: Option<String>,
    r_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<String>,
    disposition: Option<FfprobeDisposition>,
    tags: Option<FfprobeTags>,
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    default: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeTags {
    language: Option<String>,
}

/// Execute ffprobe on a file and parse the JSON output
pub async fn probe_file(path: &Path) -> Result<ProbeResult> {
    // Execute ffprobe with JSON output format
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path)
        .output()
        .await
        .context("Failed to execute ffprobe")?;

    // Check if ffprobe succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffprobe failed: {}", stderr);
    }

    // Parse JSON output
    let stdout = String::from_utf8(output.stdout).context("ffprobe output is not valid UTF-8")?;

    let ffprobe_output: FfprobeOutput =
        serde_json::from_str(&stdout).context("Failed to parse ffprobe JSON output")?;

    // Convert to our internal format
    parse_ffprobe_output(ffprobe_output)
}

/// Parse FFprobe output into our ProbeResult structure
fn parse_ffprobe_output(output: FfprobeOutput) -> Result<ProbeResult> {
    // Parse format information
    let format = if let Some(fmt) = output.format {
        FormatInfo {
            duration: fmt.duration.and_then(|d| d.parse::<f64>().ok()),
            size: fmt.size.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0),
            bitrate: fmt.bit_rate.and_then(|b| b.parse::<u64>().ok()),
        }
    } else {
        FormatInfo {
            duration: None,
            size: 0,
            bitrate: None,
        }
    };

    // Parse streams
    let streams = output.streams.unwrap_or_default();
    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    let mut subtitle_streams = Vec::new();

    for stream in streams {
        match stream.codec_type.as_str() {
            "video" => {
                if let (Some(width), Some(height)) = (stream.width, stream.height) {
                    video_streams.push(VideoStream {
                        index: stream.index,
                        codec_name: stream.codec_name.clone(),
                        width,
                        height,
                        bitrate: stream.bit_rate.and_then(|b| b.parse::<u64>().ok()),
                        frame_rate: stream.r_frame_rate.clone(),
                        pix_fmt: stream.pix_fmt.clone(),
                        bit_depth: stream
                            .bits_per_raw_sample
                            .and_then(|b| b.parse::<u8>().ok()),
                        is_default: stream
                            .disposition
                            .and_then(|d| d.default)
                            .map(|v| v == 1)
                            .unwrap_or(false),
                    });
                }
            }
            "audio" => {
                audio_streams.push(AudioStream {
                    index: stream.index,
                    codec_name: stream.codec_name.clone(),
                    language: stream.tags.and_then(|t| t.language),
                });
            }
            "subtitle" => {
                subtitle_streams.push(SubtitleStream {
                    index: stream.index,
                    codec_name: stream.codec_name.clone(),
                    language: stream.tags.and_then(|t| t.language),
                });
            }
            _ => {
                // Ignore other stream types (data, attachment, etc.)
            }
        }
    }

    Ok(ProbeResult {
        format,
        video_streams,
        audio_streams,
        subtitle_streams,
    })
}

/// Select the main video stream from a list of video streams
/// Prefers stream with default disposition, falls back to first stream
pub fn select_main_video_stream(streams: &[VideoStream]) -> Option<&VideoStream> {
    // First, try to find a stream with default disposition
    if let Some(default_stream) = streams.iter().find(|s| s.is_default) {
        return Some(default_stream);
    }

    // Fall back to first video stream
    streams.first()
}
