use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::classify::SourceClassification;
use crate::probe::ProbeResult;
use crate::scan::CandidateFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    // Identity
    pub id: String,
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,

    // Status
    pub status: JobStatus,
    pub reason: Option<String>,

    // Size metrics
    pub original_bytes: Option<u64>,
    pub new_bytes: Option<u64>,

    // Source classification
    pub is_web_like: bool,

    // Video metadata
    pub video_codec: Option<String>,
    pub video_bitrate: Option<u64>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_frame_rate: Option<String>,

    // Encoding parameters
    pub crf_used: Option<u8>,
    pub preset_used: Option<u8>,
    pub encoder_used: Option<String>,

    // Additional metadata
    pub source_bit_depth: Option<u8>,
    pub source_pix_fmt: Option<String>,
    pub is_hdr: Option<bool>,

    // TUI-specific fields (optional, for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub av1_quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_bit_depth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub av1_profile: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_clip_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_clip_approved: Option<bool>,

    // Live progress (optional; populated during encode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<JobStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded_duration: Option<f64>, // seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>, // percent 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_est_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_duration: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

/// Live stage for UI progress; optional and best-effort
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStage {
    Probing,
    Encoding,
    Verifying,
    Replacing,
    Complete,
}

pub fn create_job(
    file: CandidateFile,
    probe: ProbeResult,
    classification: SourceClassification,
) -> Job {
    // Extract metadata from the main video stream
    let main_video = probe.main_video_stream();

    // Detect HDR from pixel format
    let is_hdr = main_video.and_then(|v| v.pix_fmt.as_ref()).map(|fmt| {
        fmt.contains("p010")
            || fmt.contains("p016")
            || fmt.contains("yuv420p10")
            || fmt.contains("yuv422p10")
            || fmt.contains("yuv444p10")
            || fmt.contains("yuv420p12")
    });

    Job {
        id: Uuid::new_v4().to_string(),
        source_path: file.path,
        output_path: None,
        created_at: Utc::now(),
        started_at: None,
        finished_at: None,
        status: JobStatus::Pending,
        reason: None,
        original_bytes: Some(file.size_bytes),
        new_bytes: None,
        is_web_like: matches!(
            classification.source_type,
            crate::classify::SourceType::WebLike
        ),
        video_codec: main_video.map(|v| v.codec_name.clone()),
        video_bitrate: main_video.and_then(|v| v.bitrate),
        video_width: main_video.map(|v| v.width),
        video_height: main_video.map(|v| v.height),
        video_frame_rate: main_video.and_then(|v| v.frame_rate.clone()),
        crf_used: None,
        preset_used: None,
        encoder_used: None,
        source_bit_depth: main_video.and_then(|v| v.bit_depth),
        source_pix_fmt: main_video.and_then(|v| v.pix_fmt.clone()),
        is_hdr,
        av1_quality: None,
        target_bit_depth: None,
        av1_profile: None,
        quality_tier: None,
        test_clip_path: None,
        test_clip_approved: None,
        stage: None,
        encoded_bytes: None,
        encoded_duration: None,
        progress: None,
        eta: None,
        output_est_bytes: None,
        speed_bps: None,
        original_duration: probe.format.duration,
    }
}

pub fn save_job(job: &Job, state_dir: &Path) -> Result<()> {
    use std::fs;
    use std::io::Write;

    // Ensure state directory exists
    fs::create_dir_all(state_dir)?;

    // Serialize job to JSON
    let json = serde_json::to_string_pretty(job)?;

    // Write atomically using a temporary file
    let job_file = state_dir.join(format!("{}.json", job.id));
    let temp_file = state_dir.join(format!("{}.json.tmp", job.id));

    // Write to temporary file
    let mut file = fs::File::create(&temp_file)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    drop(file);

    // Atomically rename temporary file to final name
    fs::rename(&temp_file, &job_file)?;

    Ok(())
}

pub fn load_all_jobs(state_dir: &Path) -> Result<Vec<Job>> {
    use std::fs;

    // If directory doesn't exist, return empty list
    if !state_dir.exists() {
        return Ok(vec![]);
    }

    let mut jobs = Vec::new();

    // Read all JSON files in the directory
    for entry in fs::read_dir(state_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip non-files and non-JSON files
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            if ext != "json" {
                continue;
            }
        } else {
            continue;
        }

        // Skip temporary files
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.ends_with(".tmp"))
            .unwrap_or(false)
        {
            continue;
        }

        // Read and deserialize job
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<Job>(&contents) {
                Ok(job) => jobs.push(job),
                Err(e) => {
                    eprintln!("Warning: Failed to parse job file {:?}: {}", path, e);
                    continue;
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read job file {:?}: {}", path, e);
                continue;
            }
        }
    }

    Ok(jobs)
}

pub fn update_job_status(job: &mut Job, status: JobStatus, state_dir: &Path) -> Result<()> {
    job.status = status;
    match status {
        JobStatus::Running => job.started_at = Some(Utc::now()),
        JobStatus::Success | JobStatus::Failed | JobStatus::Skipped => {
            job.finished_at = Some(Utc::now())
        }
        _ => {}
    }

    // Persist the updated job
    save_job(job, state_dir)?;

    Ok(())
}
