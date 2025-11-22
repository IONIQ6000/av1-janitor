use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(name = "av1d")]
#[command(about = "AV1 Re-encoding Daemon", long_about = None)]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with timestamps and levels
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_ansi(true)
        .init();
    
    info!("AV1 Re-encoding Daemon v{}", env!("CARGO_PKG_VERSION"));
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Load configuration
    info!("Loading configuration...");
    let config = match av1d_daemon::config::load_config(args.config.as_deref()) {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };
    
    // Display configuration summary
    info!("Library roots: {:?}", config.library_roots);
    info!("Min file size: {} bytes ({} GB)", 
        config.min_bytes, 
        config.min_bytes as f64 / 1_073_741_824.0
    );
    info!("Max size ratio: {}", config.max_size_ratio);
    info!("Scan interval: {} seconds", config.scan_interval_secs);
    info!("Max concurrent jobs: {}", config.max_concurrent_jobs);
    info!("Job state directory: {:?}", config.job_state_dir);
    info!("Temp output directory: {:?}", config.temp_output_dir);
    info!("Keep original files: {}", config.keep_original);
    info!("Write why sidecars: {}", config.write_why_sidecars);
    
    // Run startup validation
    info!("Running startup validation...");
    
    // Check FFmpeg version
    info!("Checking FFmpeg version...");
    let _ffmpeg_version = match av1d_daemon::startup::check_ffmpeg_version() {
        Ok(version) => {
            info!("FFmpeg version: {}.{}.{}", version.0, version.1, version.2);
            if version.0 < 8 {
                error!("FFmpeg version {} is too old. Version 8.0 or higher is required.", version.0);
                return Err(anyhow::anyhow!("FFmpeg version too old"));
            }
            version
        }
        Err(e) => {
            error!("Failed to check FFmpeg version: {}", e);
            return Err(e);
        }
    };
    
    // Detect available encoders
    info!("Detecting available AV1 encoders...");
    let available_encoders = match av1d_daemon::startup::detect_available_encoders() {
        Ok(encoders) => {
            info!("Available encoders: {:?}", encoders);
            if encoders.is_empty() {
                error!("No AV1 encoders found. Please install libsvtav1, libaom-av1, or librav1e.");
                return Err(anyhow::anyhow!("No AV1 encoders available"));
            }
            encoders
        }
        Err(e) => {
            error!("Failed to detect encoders: {}", e);
            return Err(e);
        }
    };
    
    // Select encoder based on preference and availability
    info!("Selecting encoder...");
    let selected_encoder = match av1d_daemon::startup::select_encoder(&available_encoders, config.prefer_encoder) {
        Ok(encoder) => {
            info!("Selected encoder: {:?} ({})", encoder.encoder, encoder.codec_name);
            encoder
        }
        Err(e) => {
            error!("Failed to select encoder: {}", e);
            return Err(e);
        }
    };
    
    info!("Startup validation complete");
    info!("Starting daemon main loop...");
    
    // Run the daemon main loop
    if let Err(e) = av1d_daemon::run_daemon_loop(config, selected_encoder).await {
        error!("Daemon loop error: {}", e);
        return Err(e);
    }
    
    Ok(())
}
