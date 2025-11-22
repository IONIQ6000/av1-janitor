// SVT-AV1 encoder command builder

use crate::jobs::Job;
use super::common::{stream_mapping_flags, websafe_input_flags, pad_filter, pad_filter_value};

pub fn build_svt_command(job: &Job, crf: u8, preset: u8, output_path: &str) -> Vec<String> {
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
    
    // Add SVT-AV1 encoder parameters
    command.push("-c:v".to_string());
    command.push("libsvtav1".to_string());
    command.push("-crf".to_string());
    command.push(crf.to_string());
    command.push("-preset".to_string());
    command.push(preset.to_string());
    command.push("-threads".to_string());
    command.push("0".to_string());
    command.push("-svtav1-params".to_string());
    command.push("lp=0".to_string());
    
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
