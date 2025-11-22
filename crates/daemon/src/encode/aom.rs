// libaom-av1 encoder command builder

use crate::jobs::Job;
use super::common::{stream_mapping_flags, websafe_input_flags, pad_filter, pad_filter_value};

pub fn build_aom_command(job: &Job, crf: u8, output_path: &str) -> Vec<String> {
    let mut command = vec!["ffmpeg".to_string(), "-hide_banner".to_string(), "-y".to_string()];
    
    // Add WebSafe input flags if source is WebLike
    if job.is_web_like {
        command.extend(websafe_input_flags());
    }
    
    // Add input file
    command.push("-i".to_string());
    command.push(job.source_path.to_string_lossy().to_string());
    
    // Add stream mapping flags
    command.extend(stream_mapping_flags());
    
    // Add pad filter if needed
    let width = job.video_width.unwrap_or(1920);
    let height = job.video_height.unwrap_or(1080);
    if let Some(filter_flag) = pad_filter(width, height, job.is_web_like) {
        command.push(filter_flag);
        command.push(pad_filter_value());
    }
    
    // Add libaom-av1 encoder parameters
    command.push("-c:v".to_string());
    command.push("libaom-av1".to_string());
    command.push("-b:v".to_string());
    command.push("0".to_string());
    command.push("-crf".to_string());
    command.push(crf.to_string());
    
    // Add cpu-used based on resolution
    let cpu_used = select_cpu_used(height);
    command.push("-cpu-used".to_string());
    command.push(cpu_used.to_string());
    
    // Add row-based multithreading
    command.push("-row-mt".to_string());
    command.push("1".to_string());
    
    // Add tile configuration based on resolution
    let tiles = select_tiles(height);
    command.push("-tiles".to_string());
    command.push(tiles.to_string());
    
    // Copy audio and subtitle streams
    command.push("-c:a".to_string());
    command.push("copy".to_string());
    command.push("-c:s".to_string());
    command.push("copy".to_string());
    
    // Add max muxing queue size
    command.push("-max_muxing_queue_size".to_string());
    command.push("2048".to_string());
    
    // Add output path
    command.push(output_path.to_string());
    
    command
}

pub fn select_tiles(height: i32) -> &'static str {
    match height {
        h if h > 2160 => "3x2",
        h if h > 1080 => "2x2",
        _ => "2x1",
    }
}

pub fn select_cpu_used(height: i32) -> u8 {
    match height {
        h if h > 1080 => 3,
        _ => 4,
    }
}
