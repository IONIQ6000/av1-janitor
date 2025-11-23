pub mod svt;
pub mod aom;
pub mod rav1e;
pub mod common;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::config::{DaemonConfig, QualityTier};
use crate::jobs::Job;
use crate::startup::SelectedEncoder;

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
    let crf = select_crf(height, bitrate);
    
    // Build command based on encoder type
    match encoder.encoder {
        AvailableEncoder::SvtAv1 => {
            let preset = select_preset(height, config.quality_tier);
            svt::build_svt_command(job, crf, preset, output_path)
        }
        AvailableEncoder::LibaomAv1 => {
            aom::build_aom_command(job, crf, output_path)
        }
        AvailableEncoder::Librav1e => {
            rav1e::build_rav1e_command(job, crf, output_path)
        }
    }
}

pub async fn execute_encode(_job: &mut Job, command: Vec<String>) -> Result<PathBuf> {
    use tokio::process::Command;
    use tokio::io::{AsyncBufReadExt, BufReader};
    
    // Extract the output path from the command (last argument)
    let output_path = command.last()
        .ok_or_else(|| anyhow::anyhow!("Command has no output path"))?
        .clone();
    
    // Build the command
    let mut cmd = Command::new("ffmpeg");
    for arg in &command[1..] {  // Skip "ffmpeg" itself
        cmd.arg(arg);
    }
    
    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    
    // Spawn the process
    let mut child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn ffmpeg: {}", e))?;
    
    // Get handles to stdout and stderr
    let stdout = child.stdout.take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    let stderr = child.stderr.take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;
    
    // Spawn tasks to read stdout and stderr
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut output = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            output.push(line);
        }
        output
    });
    
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut output = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            output.push(line);
        }
        output
    });
    
    // Wait for the process to complete
    let status = child.wait().await
        .map_err(|e| anyhow::anyhow!("Failed to wait for ffmpeg: {}", e))?;
    
    // Collect output
    let stdout_lines = stdout_task.await
        .map_err(|e| anyhow::anyhow!("Failed to read stdout: {}", e))?;
    let stderr_lines = stderr_task.await
        .map_err(|e| anyhow::anyhow!("Failed to read stderr: {}", e))?;
    
    // Check if the process succeeded
    if !status.success() {
        let error_msg = format!(
            "FFmpeg failed with exit code: {:?}\nStderr:\n{}",
            status.code(),
            stderr_lines.join("\n")
        );
        return Err(anyhow::anyhow!(error_msg));
    }
    
    // Log output for debugging (optional)
    if !stdout_lines.is_empty() {
        tracing::debug!("FFmpeg stdout: {}", stdout_lines.join("\n"));
    }
    if !stderr_lines.is_empty() {
        tracing::debug!("FFmpeg stderr: {}", stderr_lines.join("\n"));
    }
    
    Ok(PathBuf::from(output_path))
}

pub fn select_crf(height: i32, _bitrate: Option<u64>) -> u8 {
    // Ultra-high quality settings - prioritize quality over speed/size
    // Lower CRF = higher quality (18 is near-lossless, 23 is high quality)
    let base_crf = match height {
        h if h >= 2160 => 20, // 4K: CRF 20 (very high quality)
        h if h >= 1440 => 21, // 1440p: CRF 21 (very high quality)
        h if h >= 1080 => 22, // 1080p: CRF 22 (high quality)
        _ => 23,              // 720p and below: CRF 23 (high quality)
    };
    
    // Don't increase CRF for low bitrate sources - maintain quality
    // The user wants quality, not size optimization
    base_crf
}

pub fn select_preset(height: i32, quality_tier: QualityTier) -> u8 {
    // Ultra-slow presets for maximum quality
    // Lower preset = slower but higher quality (0 is slowest/best, 13 is fastest/worst)
    let base_preset = match height {
        h if h >= 2160 => 2, // 4K: Preset 2 (very slow, very high quality)
        h if h >= 1440 => 3, // 1440p: Preset 3 (slow, high quality)
        h if h >= 1080 => 3, // 1080p: Preset 3 (slow, high quality)
        _ => 4,              // 720p and below: Preset 4 (moderate speed, high quality)
    };
    
    match quality_tier {
        QualityTier::High => base_preset,
        QualityTier::VeryHigh => base_preset.saturating_sub(1), // Even slower for VeryHigh
    }
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
        let _permit = self.semaphore.acquire().await
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
    ) -> Result<PathBuf> {
        let command_clone = command.clone();
        let job_clone = job.clone();
        
        self.execute_job(|| async move {
            let mut job_mut = job_clone;
            execute_encode(&mut job_mut, command_clone).await
        }).await
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
            executor1.execute_job(|| async move {
                // Hold the permit until signaled
                rx1.await.ok();
                Ok(PathBuf::from("test1"))
            }).await
        });
        
        let task2 = tokio::spawn(async move {
            executor2.execute_job(|| async move {
                // Hold the permit until signaled
                rx2.await.ok();
                Ok(PathBuf::from("test2"))
            }).await
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
