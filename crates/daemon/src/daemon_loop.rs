use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error, debug};

use crate::config::DaemonConfig;
use crate::startup::SelectedEncoder;
use crate::scan::{scan_libraries, CandidateFile};
use crate::stable::check_stability;
use crate::probe::probe_file;
use crate::classify::classify_source;
use crate::gates::{check_gates, GateResult};
use crate::jobs::{JobStatus, create_job, save_job, update_job_status};
use crate::encode::{build_command, execute_encode, JobExecutor};
use crate::validate::validate_output;
use crate::size_gate::{check_size_gate, SizeGateResult};
use crate::replace::atomic_replace;
use crate::sidecars::{has_skip_marker, create_skip_marker, write_why_file};

/// Main daemon loop that orchestrates the entire encoding workflow
pub async fn run_daemon_loop(
    config: DaemonConfig,
    encoder: SelectedEncoder,
) -> Result<()> {
    info!("Starting daemon main loop");
    info!("Scan interval: {} seconds", config.scan_interval_secs);
    info!("Max concurrent jobs: {}", config.max_concurrent_jobs);
    info!("Selected encoder: {:?}", encoder.encoder);
    
    // Create job executor for managing concurrent encoding jobs
    let executor = JobExecutor::new(config.max_concurrent_jobs);
    
    // Ensure job state directory exists
    std::fs::create_dir_all(&config.job_state_dir)?;
    std::fs::create_dir_all(&config.temp_output_dir)?;
    
    loop {
        info!("Starting scan cycle");
        
        // Scan all library roots for video files
        match scan_libraries(&config.library_roots) {
            Ok(candidates) => {
                info!("Found {} candidate files", candidates.len());
                
                // Process each candidate file
                for candidate in candidates {
                    if let Err(e) = process_candidate(
                        candidate,
                        &config,
                        &encoder,
                        &executor,
                    ).await {
                        error!("Error processing candidate: {}", e);
                        // Continue with next file
                    }
                }
            }
            Err(e) => {
                error!("Error scanning libraries: {}", e);
                // Continue to next scan cycle
            }
        }
        
        info!("Scan cycle complete, waiting {} seconds", config.scan_interval_secs);
        sleep(Duration::from_secs(config.scan_interval_secs)).await;
    }
}

/// Process a single candidate file through the entire workflow
async fn process_candidate(
    candidate: CandidateFile,
    config: &DaemonConfig,
    encoder: &SelectedEncoder,
    executor: &JobExecutor,
) -> Result<()> {
    let path = &candidate.path;
    debug!("Processing candidate: {:?}", path);
    
    // Step 1: Check for skip marker
    if has_skip_marker(path) {
        debug!("File has skip marker, skipping: {:?}", path);
        return Ok(());
    }
    
    // Step 2: Check file stability
    debug!("Checking file stability: {:?}", path);
    match check_stability(&candidate, Duration::from_secs(10)).await {
        Ok(is_stable) => {
            if !is_stable {
                debug!("File is not stable, skipping for this cycle: {:?}", path);
                return Ok(());
            }
        }
        Err(e) => {
            warn!("Error checking file stability for {:?}: {}", path, e);
            return Ok(());
        }
    }
    
    // Step 3: Probe file metadata
    debug!("Probing file: {:?}", path);
    let probe_result = match probe_file(path).await {
        Ok(result) => result,
        Err(e) => {
            warn!("Failed to probe file {:?}: {}", path, e);
            create_skip_marker(path)?;
            if config.write_why_sidecars {
                write_why_file(path, &format!("Probe failed: {}", e))?;
            }
            return Ok(());
        }
    };
    
    // Step 4: Classify source
    debug!("Classifying source: {:?}", path);
    let classification = classify_source(path, &probe_result);
    debug!("Classification: {:?}", classification.source_type);
    
    // Step 5: Check gates
    debug!("Checking gates: {:?}", path);
    match check_gates(&candidate, &probe_result, config) {
        GateResult::Pass => {
            debug!("Gates passed: {:?}", path);
        }
        GateResult::Skip(reason) => {
            info!("File skipped due to gate: {:?} - {:?}", path, reason);
            create_skip_marker(path)?;
            if config.write_why_sidecars {
                write_why_file(path, &format!("{:?}", reason))?;
            }
            return Ok(());
        }
    }
    
    // Step 6: Create job
    let mut job = create_job(candidate.clone(), probe_result.clone(), classification);
    
    // Populate video metadata from probe result
    if let Some(main_stream) = probe_result.main_video_stream() {
        job.video_codec = Some(main_stream.codec_name.clone());
        job.video_bitrate = main_stream.bitrate;
        job.video_width = Some(main_stream.width);
        job.video_height = Some(main_stream.height);
        job.video_frame_rate = main_stream.frame_rate.clone();
        job.source_bit_depth = main_stream.bit_depth;
        job.source_pix_fmt = main_stream.pix_fmt.clone();
    }
    
    // Save initial job state
    save_job(&job, &config.job_state_dir)?;
    info!("Created job {} for {:?}", job.id, path);
    
    // Step 7: Execute encoding
    // Update job status to running
    update_job_status(&mut job, JobStatus::Running, &config.job_state_dir)?;
    
    // Generate output path
    let output_path = config.temp_output_dir.join(format!("{}.mkv", job.id));
    job.output_path = Some(output_path.clone());
    
    // Build FFmpeg command
    let command = build_command(&job, encoder, config, output_path.to_str().unwrap());
    
    // Store encoding parameters in job
    job.encoder_used = Some(encoder.codec_name.clone());
    job.crf_used = Some(crate::encode::select_crf(
        job.video_height.unwrap_or(1080),
        job.video_bitrate,
    ));
    if matches!(encoder.encoder, crate::startup::AvailableEncoder::SvtAv1) {
        job.preset_used = Some(crate::encode::select_preset(
            job.video_height.unwrap_or(1080),
            config.quality_tier,
        ));
    }
    
    save_job(&job, &config.job_state_dir)?;
    
    info!("Starting encoding for job {}: {:?}", job.id, path);
    debug!("FFmpeg command: {:?}", command);
    
    // Execute encoding with concurrency limiting
    let encode_result = executor.execute_job(|| {
        let cmd = command.clone();
        let mut job_clone = job.clone();
        async move {
            execute_encode(&mut job_clone, cmd).await
        }
    }).await;
    
    let encoded_path = match encode_result {
        Ok(path) => path,
        Err(e) => {
            error!("Encoding failed for job {}: {}", job.id, e);
            job.reason = Some(format!("Encoding failed: {}", e));
            update_job_status(&mut job, JobStatus::Failed, &config.job_state_dir)?;
            return Ok(());
        }
    };
    
    info!("Encoding complete for job {}", job.id);
    
    // Step 8: Validate output
    debug!("Validating output: {:?}", encoded_path);
    let _output_probe = match validate_output(&encoded_path, &probe_result).await {
        Ok(result) => result,
        Err(e) => {
            error!("Output validation failed for job {}: {}", job.id, e);
            job.reason = Some(format!("Validation failed: {}", e));
            update_job_status(&mut job, JobStatus::Failed, &config.job_state_dir)?;
            
            // Clean up failed output
            if let Err(cleanup_err) = std::fs::remove_file(&encoded_path) {
                warn!("Failed to clean up invalid output {:?}: {}", encoded_path, cleanup_err);
            }
            
            return Ok(());
        }
    };
    
    info!("Output validation passed for job {}", job.id);
    
    // Step 9: Check size gate
    let output_size = std::fs::metadata(&encoded_path)?.len();
    job.new_bytes = Some(output_size);
    
    debug!("Checking size gate: original={}, new={}, ratio={}",
        job.original_bytes.unwrap_or(0),
        output_size,
        config.max_size_ratio
    );
    
    match check_size_gate(
        job.original_bytes.unwrap_or(0),
        output_size,
        config.max_size_ratio,
    ) {
        SizeGateResult::Pass { savings_bytes, compression_ratio } => {
            info!(
                "Size gate passed for job {}: saved {} bytes ({:.2}% compression)",
                job.id,
                savings_bytes,
                (1.0 - compression_ratio) * 100.0
            );
        }
        SizeGateResult::Fail { new_bytes, threshold_bytes } => {
            warn!(
                "Size gate failed for job {}: {} bytes >= {} bytes threshold",
                job.id,
                new_bytes,
                threshold_bytes
            );
            
            job.reason = Some(format!(
                "Size gate failed: {} bytes >= {} bytes threshold",
                new_bytes,
                threshold_bytes
            ));
            update_job_status(&mut job, JobStatus::Skipped, &config.job_state_dir)?;
            
            // Clean up output
            if let Err(cleanup_err) = std::fs::remove_file(&encoded_path) {
                warn!("Failed to clean up oversized output {:?}: {}", encoded_path, cleanup_err);
            }
            
            // Create skip marker and why file
            create_skip_marker(path)?;
            if config.write_why_sidecars {
                write_why_file(path, &job.reason.as_ref().unwrap())?;
            }
            
            return Ok(());
        }
    }
    
    // Step 10: Atomic replacement
    info!("Replacing original file for job {}", job.id);
    match atomic_replace(path, &encoded_path, config.keep_original).await {
        Ok(()) => {
            info!("Successfully replaced {:?}", path);
            update_job_status(&mut job, JobStatus::Success, &config.job_state_dir)?;
        }
        Err(e) => {
            error!("Failed to replace file for job {}: {}", job.id, e);
            job.reason = Some(format!("Replacement failed: {}", e));
            update_job_status(&mut job, JobStatus::Failed, &config.job_state_dir)?;
            
            // Keep the output file for manual inspection
            warn!("Output file preserved at {:?} for manual inspection", encoded_path);
            
            return Ok(());
        }
    }
    
    info!("Job {} completed successfully", job.id);
    Ok(())
}
