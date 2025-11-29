// Common FFmpeg command components

/// Returns stream mapping flags that:
/// - Select all streams initially
/// - Exclude attached pictures
/// - Exclude Russian audio tracks
/// - Exclude Russian subtitle tracks
/// - Preserve chapters and metadata
pub fn stream_mapping_flags() -> Vec<String> {
    vec![
        "-map".to_string(),
        "0".to_string(),
        "-map".to_string(),
        "-0:v:m:attached_pic".to_string(),
        "-map".to_string(),
        "-0:a:m:language:ru".to_string(),
        "-map".to_string(),
        "-0:a:m:language:rus".to_string(),
        "-map".to_string(),
        "-0:s:m:language:ru".to_string(),
        "-map".to_string(),
        "-0:s:m:language:rus".to_string(),
        "-map_chapters".to_string(),
        "0".to_string(),
        "-map_metadata".to_string(),
        "0".to_string(),
    ]
}

/// Returns WebSafe input flags for web sources to handle timestamp issues
pub fn websafe_input_flags() -> Vec<String> {
    vec![
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-copyts".to_string(),
        "-start_at_zero".to_string(),
        "-vsync".to_string(),
        "0".to_string(),
        "-avoid_negative_ts".to_string(),
        "make_zero".to_string(),
    ]
}

/// Returns pad filter for WebLike sources or odd dimensions
/// Returns None if dimensions are even and not WebLike
pub fn pad_filter(width: i32, height: i32, is_web_like: bool) -> Option<String> {
    let needs_padding = is_web_like || width % 2 != 0 || height % 2 != 0;

    if needs_padding {
        Some("-vf".to_string())
    } else {
        None
    }
}

/// Returns the pad filter value string
pub fn pad_filter_value() -> String {
    "pad=ceil(iw/2)*2:ceil(ih/2)*2,setsar=1".to_string()
}
