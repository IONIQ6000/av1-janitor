pub mod aom;
pub mod common;
pub mod rav1e;
pub mod svt;

use crate::config::{DaemonConfig, QualityTier};
use crate::jobs::{save_job, Job, JobStage};
use crate::startup::SelectedEncoder;
use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

pub fn build_command(
    job: &Job,
    encoder: &SelectedEncoder,
    config: &DaemonConfig,
    output_path: &str,
) -> Vec<String> {
    use crate::startup::AvailableEncoder;

    // Calculate CRF based on video height and bitrate
    let height = job.video_height.unwrap_or(1080);
    let bitrate = job.video_bitrate;
    let crf = select_crf(height, bitrate, config.quality_tier);

    // Build command based on encoder type
    match encoder.encoder {
        AvailableEncoder::SvtAv1 => {
            let preset = select_preset(height, config.quality_tier);
            svt::build_svt_command(job, crf, preset, output_path)
        }
        AvailableEncoder::LibaomAv1 => aom::build_aom_command(job, crf, output_path),
        AvailableEncoder::Librav1e => rav1e::build_rav1e_command(job, crf, output_path),
    }
}

pub async fn execute_encode(
    job: &mut Job,
    command: Vec<String>,
    job_state_dir: &std::path::Path,
) -> Result<PathBuf> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    // Extract the output path from the command (last argument)
    let output_path = command
        .last()
        .ok_or_else(|| anyhow::anyhow!("Command has no output path"))?
        .clone();

    // Build the command with progress reporting
    let mut cmd = Command::new("ffmpeg");
    for arg in ["-progress", "pipe:1", "-nostats"]
        .iter()
        .map(|s| *s)
        .chain(command[1..].iter().map(|s| s.as_str()))
    {
        cmd.arg(arg);
    }

    // Capture stdout (progress) and stderr (errors)
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn ffmpeg: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;

    // Collect stderr for diagnostics
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut output = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            output.push(line);
        }
        output
    });

    // Progress parsing loop
    let mut reader = BufReader::new(stdout).lines();
    let mut total_size_bytes: Option<u64> = None;
    let mut out_time_secs: Option<f64> = None;
    let mut speed_x: Option<f64> = None;
    let mut last_save = Instant::now()
        .checked_sub(Duration::from_millis(750))
        .unwrap_or_else(Instant::now);

    job.stage = Some(JobStage::Encoding);
    save_job(job, job_state_dir)?;

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            match k {
                "out_time_ms" => {
                    if let Ok(ms) = v.parse::<u64>() {
                        out_time_secs = Some(ms as f64 / 1_000_000.0);
                    }
                }
                "out_time" => {
                    if out_time_secs.is_none() {
                        out_time_secs = parse_out_time(v);
                    }
                }
                "total_size" => {
                    if let Ok(sz) = v.parse::<u64>() {
                        total_size_bytes = Some(sz);
                    }
                }
                "speed" => {
                    let clean = v.trim_end_matches('x');
                    if let Ok(s) = clean.parse::<f64>() {
                        speed_x = Some(s);
                    }
                }
                "progress" if v == "end" => break,
                _ => {}
            }
        }

        if last_save.elapsed() >= Duration::from_millis(750) {
            update_job_progress(job, out_time_secs, total_size_bytes, speed_x, job_state_dir)?;
            last_save = Instant::now();
        }
    }

    // Final progress update and mark verifying
    update_job_progress(job, out_time_secs, total_size_bytes, speed_x, job_state_dir)?;
    job.stage = Some(JobStage::Verifying);
    save_job(job, job_state_dir)?;

    let status = child
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for ffmpeg: {}", e))?;

    let stderr_lines = stderr_task
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read stderr: {}", e))?;

    if !status.success() {
        let error_msg = format!(
            "FFmpeg failed with exit code: {:?}\nStderr:\n{}",
            status.code(),
            stderr_lines.join("\n")
        );
        return Err(anyhow::anyhow!(error_msg));
    }

    Ok(PathBuf::from(output_path))
}

pub fn select_crf(height: i32, _bitrate: Option<u64>, quality_tier: QualityTier) -> u8 {
    // Quality-first defaults; lower CRF = higher quality (18 is near-lossless, 23 is high quality)
    let base_crf: u8 = match height {
        h if h >= 2160 => 18, // 4K: CRF 18 (near-lossless)
        h if h >= 1440 => 19, // 1440p: CRF 19
        h if h >= 1080 => 20, // 1080p: CRF 20
        _ => 21,              // 720p and below: CRF 21
    };

    let crf = match quality_tier {
        QualityTier::High => base_crf,
        // Maximum quality: 2 steps lower CRF
        QualityTier::VeryHigh => base_crf.saturating_sub(2),
    };

    // Don't increase CRF for low bitrate sources - maintain quality
    // The user wants quality, not size optimization
    crf
}

pub fn select_preset(height: i32, quality_tier: QualityTier) -> u8 {
    // Ultra-slow presets for maximum quality
    // Lower preset = slower but higher quality (0 is slowest/best, 13 is fastest/worst)
    let base_preset = match height {
        h if h >= 2160 => 1, // 4K: Preset 1 (extremely slow, max quality)
        h if h >= 1440 => 2, // 1440p: Preset 2
        h if h >= 1080 => 2, // 1080p: Preset 2
        _ => 3,              // 720p and below: Preset 3
    };

    match quality_tier {
        QualityTier::High => base_preset,
        // Maximum quality: 2 steps slower for VeryHigh (clamped at 0)
        QualityTier::VeryHigh => base_preset.saturating_sub(2),
    }
}

fn parse_out_time(val: &str) -> Option<f64> {
    let parts: Vec<&str> = val.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h = parts.get(0)?.parse::<f64>().ok()?;
    let m = parts.get(1)?.parse::<f64>().ok()?;
    let s = parts.get(2)?.parse::<f64>().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn update_job_progress(
    job: &mut Job,
    out_time_secs: Option<f64>,
    total_size_bytes: Option<u64>,
    speed_x: Option<f64>,
    job_state_dir: &std::path::Path,
) -> Result<()> {
    if let Some(sz) = total_size_bytes {
        job.encoded_bytes = Some(sz);
    }
    if let Some(ots) = out_time_secs {
        job.encoded_duration = Some(ots);
    }

    if let (Some(ots), Some(total_dur)) = (out_time_secs, job.original_duration) {
        if total_dur > 0.0 {
            let pct = (ots / total_dur * 100.0).clamp(0.0, 100.0);
            job.progress = Some(pct);
        }
    }

    if let (Some(ots), Some(total_dur), Some(speed)) =
        (out_time_secs, job.original_duration, speed_x)
    {
        if speed > 0.0 && total_dur > ots {
            let remaining = (total_dur - ots).max(0.0);
            let seconds_left = remaining / speed;
            job.eta =
                Some(Utc::now() + ChronoDuration::milliseconds((seconds_left * 1000.0) as i64));
        } else {
            job.eta = None;
        }
        if let Some(bytes) = job.encoded_bytes {
            if ots > 0.0 {
                job.speed_bps = Some((bytes as f64) / ots);
            }
        }
    }

    if let (Some(bytes), Some(pct)) = (job.encoded_bytes, job.progress) {
        if pct > 0.1 {
            let est = (bytes as f64 / (pct / 100.0)) as u64;
            job.output_est_bytes = Some(est);
        }
    }

    save_job(job, job_state_dir)?;
    Ok(())
}

/// JobExecutor manages concurrent encoding jobs with a configurable limit
pub struct JobExecutor {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl JobExecutor {
    /// Create a new JobExecutor with the specified maximum concurrent jobs
    pub fn new(max_concurrent_jobs: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent_jobs)),
            max_concurrent: max_concurrent_jobs,
        }
    }

    /// Get the maximum number of concurrent jobs
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Get the number of available slots
    pub fn available_slots(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Execute a job with concurrency limiting
    /// This will wait until a slot is available before executing
    pub async fn execute_job<F, Fut>(&self, job_fn: F) -> Result<PathBuf>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<PathBuf>>,
    {
        // Acquire a permit from the semaphore
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire semaphore permit: {}", e))?;

        // Execute the job while holding the permit
        let result = job_fn().await;

        // Permit is automatically released when _permit is dropped
        result
    }

    /// Execute an encoding job with concurrency limiting
    pub async fn execute_encode_job(
        &self,
        job: &mut Job,
        command: Vec<String>,
        job_state_dir: &std::path::Path,
    ) -> Result<PathBuf> {
        let command_clone = command.clone();
        let job_clone = job.clone();
        let state_dir = job_state_dir.to_path_buf();

        self.execute_job(|| async move {
            let mut job_mut = job_clone;
            execute_encode(&mut job_mut, command_clone, &state_dir).await
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = JobExecutor::new(2);
        assert_eq!(executor.max_concurrent(), 2);
        assert_eq!(executor.available_slots(), 2);
    }

    #[tokio::test]
    async fn test_executor_slot_management() {
        let executor = Arc::new(JobExecutor::new(2));

        // Initially, all slots should be available
        assert_eq!(executor.available_slots(), 2);

        // Spawn two tasks that hold permits
        let executor1 = executor.clone();
        let executor2 = executor.clone();

        let (tx1, rx1) = tokio::sync::oneshot::channel();
        let (tx2, rx2) = tokio::sync::oneshot::channel();

        let task1 = tokio::spawn(async move {
            executor1
                .execute_job(|| async move {
                    // Hold the permit until signaled
                    rx1.await.ok();
                    Ok(PathBuf::from("test1"))
                })
                .await
        });

        let task2 = tokio::spawn(async move {
            executor2
                .execute_job(|| async move {
                    // Hold the permit until signaled
                    rx2.await.ok();
                    Ok(PathBuf::from("test2"))
                })
                .await
        });

        // Give tasks time to acquire permits
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // All slots should be taken
        assert_eq!(executor.available_slots(), 0);

        // Release one task
        tx1.send(()).ok();
        task1.await.ok();

        // Give time for permit to be released
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // One slot should be available
        assert_eq!(executor.available_slots(), 1);

        // Release second task
        tx2.send(()).ok();
        task2.await.ok();

        // Give time for permit to be released
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // All slots should be available again
        assert_eq!(executor.available_slots(), 2);
    }
}
