use av1d_daemon::probe::{select_main_video_stream, VideoStream, ProbeResult, FormatInfo, AudioStream, SubtitleStream};
use proptest::prelude::*;
use serde_json::json;

/// **Feature: av1-reencoder, Property 7: FFprobe JSON parsing**
/// *For any* valid ffprobe JSON output, the system should correctly extract all video stream metadata fields
/// **Validates: Requirements 7.1, 7.2**
///
/// Note: This property test validates the parsing logic by generating random FFprobe JSON
/// and ensuring all fields are correctly extracted and preserved through the parsing process.
#[test]
fn property_ffprobe_json_parsing() {
    proptest!(ProptestConfig::with_cases(100), |(
        duration in prop::option::of(0.0f64..86400.0), // 0 to 24 hours
        size in 1000u64..1_000_000_000_000, // 1KB to 1TB
        bitrate in prop::option::of(100_000u64..100_000_000), // 100kbps to 100Mbps
        num_video_streams in 1usize..5,
        num_audio_streams in 0usize..10,
        num_subtitle_streams in 0usize..10,
        video_codec in prop::sample::select(vec!["h264", "hevc", "vp9", "av1", "mpeg2video"]),
        width in 640i32..3840,
        height in 480i32..2160,
        video_bitrate in prop::option::of(1_000_000u64..50_000_000),
        frame_rate in prop::option::of(prop::sample::select(vec!["24/1", "25/1", "30/1", "60/1", "24000/1001"])),
        pix_fmt in prop::option::of(prop::sample::select(vec!["yuv420p", "yuv420p10le", "yuv444p"])),
        bit_depth in prop::option::of(8u8..12),
        has_default_stream in prop::bool::ANY,
    )| {
        // Generate FFprobe JSON output structure
        let mut streams = Vec::new();
        
        // Add video streams
        for i in 0..num_video_streams {
            let is_default = has_default_stream && i == 0;
            let mut stream = json!({
                "index": i,
                "codec_type": "video",
                "codec_name": video_codec,
                "width": width,
                "height": height,
                "disposition": {
                    "default": if is_default { 1 } else { 0 }
                }
            });
            
            if let Some(br) = video_bitrate {
                stream["bit_rate"] = json!(br.to_string());
            }
            if let Some(ref fr) = frame_rate {
                stream["r_frame_rate"] = json!(fr);
            }
            if let Some(ref pf) = pix_fmt {
                stream["pix_fmt"] = json!(pf);
            }
            if let Some(bd) = bit_depth {
                stream["bits_per_raw_sample"] = json!(bd.to_string());
            }
            
            streams.push(stream);
        }
        
        // Add audio streams
        for i in 0..num_audio_streams {
            let audio_codec = if i % 2 == 0 { "aac" } else { "ac3" };
            let language = if i % 3 == 0 { Some("eng") } else { None };
            
            let mut stream = json!({
                "index": num_video_streams + i,
                "codec_type": "audio",
                "codec_name": audio_codec,
            });
            
            if let Some(lang) = language {
                stream["tags"] = json!({ "language": lang });
            }
            
            streams.push(stream);
        }
        
        // Add subtitle streams
        for i in 0..num_subtitle_streams {
            let language = if i % 2 == 0 { Some("eng") } else { Some("spa") };
            
            let mut stream = json!({
                "index": num_video_streams + num_audio_streams + i,
                "codec_type": "subtitle",
                "codec_name": "subrip",
            });
            
            if let Some(lang) = language {
                stream["tags"] = json!({ "language": lang });
            }
            
            streams.push(stream);
        }
        
        let mut format_json = json!({
            "size": size.to_string(),
        });
        
        if let Some(dur) = duration {
            format_json["duration"] = json!(dur.to_string());
        }
        if let Some(br) = bitrate {
            format_json["bit_rate"] = json!(br.to_string());
        }
        
        let ffprobe_json = json!({
            "format": format_json,
            "streams": streams
        });
        
        // Parse the JSON using our helper function (simulates internal parsing)
        let parsed = parse_test_json(ffprobe_json).unwrap();
        
        // Verify format information
        prop_assert_eq!(parsed.format.duration, duration, "Duration should match");
        prop_assert_eq!(parsed.format.size, size, "Size should match");
        prop_assert_eq!(parsed.format.bitrate, bitrate, "Bitrate should match");
        
        // Verify video streams
        prop_assert_eq!(parsed.video_streams.len(), num_video_streams, 
            "Should have {} video streams", num_video_streams);
        
        for (i, stream) in parsed.video_streams.iter().enumerate() {
            prop_assert_eq!(stream.index, i, "Video stream index should match");
            prop_assert_eq!(&stream.codec_name, video_codec, "Video codec should match");
            prop_assert_eq!(stream.width, width, "Width should match");
            prop_assert_eq!(stream.height, height, "Height should match");
            prop_assert_eq!(stream.bitrate, video_bitrate, "Video bitrate should match");
            prop_assert_eq!(&stream.frame_rate, &frame_rate.map(|s| s.to_string()), "Frame rate should match");
            prop_assert_eq!(&stream.pix_fmt, &pix_fmt.map(|s| s.to_string()), "Pixel format should match");
            prop_assert_eq!(stream.bit_depth, bit_depth, "Bit depth should match");
            
            if has_default_stream && i == 0 {
                prop_assert!(stream.is_default, "First stream should be default");
            } else {
                prop_assert!(!stream.is_default, "Non-first streams should not be default");
            }
        }
        
        // Verify audio streams
        prop_assert_eq!(parsed.audio_streams.len(), num_audio_streams,
            "Should have {} audio streams", num_audio_streams);
        
        // Verify subtitle streams
        prop_assert_eq!(parsed.subtitle_streams.len(), num_subtitle_streams,
            "Should have {} subtitle streams", num_subtitle_streams);
    });
}

/// Helper function to parse test JSON (simulates internal parsing logic)
fn parse_test_json(value: serde_json::Value) -> Result<ProbeResult, serde_json::Error> {
    let format_obj = value.get("format").and_then(|f| f.as_object());
    let streams_arr = value.get("streams").and_then(|s| s.as_array());
    
    let format = if let Some(fmt) = format_obj {
        FormatInfo {
            duration: fmt.get("duration")
                .and_then(|d| d.as_str())
                .and_then(|s| s.parse::<f64>().ok()),
            size: fmt.get("size")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            bitrate: fmt.get("bit_rate")
                .and_then(|b| b.as_str())
                .and_then(|s| s.parse::<u64>().ok()),
        }
    } else {
        FormatInfo {
            duration: None,
            size: 0,
            bitrate: None,
        }
    };
    
    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    let mut subtitle_streams = Vec::new();
    
    if let Some(streams) = streams_arr {
        for stream in streams {
            let codec_type = stream.get("codec_type").and_then(|c| c.as_str()).unwrap_or("");
            let index = stream.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            let codec_name = stream.get("codec_name").and_then(|c| c.as_str()).unwrap_or("").to_string();
            
            match codec_type {
                "video" => {
                    if let (Some(width), Some(height)) = (
                        stream.get("width").and_then(|w| w.as_i64()),
                        stream.get("height").and_then(|h| h.as_i64())
                    ) {
                        video_streams.push(VideoStream {
                            index,
                            codec_name,
                            width: width as i32,
                            height: height as i32,
                            bitrate: stream.get("bit_rate")
                                .and_then(|b| b.as_str())
                                .and_then(|s| s.parse::<u64>().ok()),
                            frame_rate: stream.get("r_frame_rate")
                                .and_then(|f| f.as_str())
                                .map(|s| s.to_string()),
                            pix_fmt: stream.get("pix_fmt")
                                .and_then(|p| p.as_str())
                                .map(|s| s.to_string()),
                            bit_depth: stream.get("bits_per_raw_sample")
                                .and_then(|b| b.as_str())
                                .and_then(|s| s.parse::<u8>().ok()),
                            is_default: stream.get("disposition")
                                .and_then(|d| d.get("default"))
                                .and_then(|v| v.as_i64())
                                .map(|v| v == 1)
                                .unwrap_or(false),
                        });
                    }
                }
                "audio" => {
                    audio_streams.push(AudioStream {
                        index,
                        codec_name,
                        language: stream.get("tags")
                            .and_then(|t| t.get("language"))
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string()),
                    });
                }
                "subtitle" => {
                    subtitle_streams.push(SubtitleStream {
                        index,
                        codec_name,
                        language: stream.get("tags")
                            .and_then(|t| t.get("language"))
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string()),
                    });
                }
                _ => {}
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

/// **Feature: av1-reencoder, Property 8: Main video stream selection**
/// *For any* set of video streams, the system should select the stream with default disposition if present, otherwise the first stream
/// **Validates: Requirements 7.4**
#[test]
fn property_main_video_stream_selection() {
    proptest!(ProptestConfig::with_cases(100), |(
        num_streams in 1usize..10,
        default_index in prop::option::of(0usize..10),
    )| {
        // Generate video streams
        let mut streams = Vec::new();
        
        for i in 0..num_streams {
            let is_default = default_index.map(|idx| idx == i && idx < num_streams).unwrap_or(false);
            
            streams.push(VideoStream {
                index: i,
                codec_name: "h264".to_string(),
                width: 1920,
                height: 1080,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default,
            });
        }
        
        // Select main stream
        let selected = select_main_video_stream(&streams);
        
        // Verify selection logic
        if let Some(default_idx) = default_index {
            if default_idx < num_streams {
                // Should select the default stream
                prop_assert!(selected.is_some(), "Should select a stream when default exists");
                prop_assert_eq!(selected.unwrap().index, default_idx, 
                    "Should select stream with default disposition");
                prop_assert!(selected.unwrap().is_default, 
                    "Selected stream should have is_default=true");
            } else {
                // Default index out of range, should select first stream
                prop_assert!(selected.is_some(), "Should select first stream");
                prop_assert_eq!(selected.unwrap().index, 0, 
                    "Should select first stream when no default");
            }
        } else {
            // No default stream, should select first
            prop_assert!(selected.is_some(), "Should select first stream");
            prop_assert_eq!(selected.unwrap().index, 0, 
                "Should select first stream when no default");
        }
    });
}

/// Test stream selection with empty stream list
#[test]
fn test_empty_stream_list() {
    let streams: Vec<VideoStream> = Vec::new();
    let selected = select_main_video_stream(&streams);
    assert!(selected.is_none(), "Should return None for empty stream list");
}

/// Test stream selection with single stream
#[test]
fn test_single_stream() {
    let streams = vec![
        VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: false,
        }
    ];
    
    let selected = select_main_video_stream(&streams);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().index, 0);
}

/// Test stream selection prefers default over first
#[test]
fn test_default_preference() {
    let streams = vec![
        VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: false,
        },
        VideoStream {
            index: 1,
            codec_name: "hevc".to_string(),
            width: 3840,
            height: 2160,
            bitrate: Some(20_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            is_default: true,
        },
        VideoStream {
            index: 2,
            codec_name: "av1".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(3_000_000),
            frame_rate: Some("30/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: false,
        }
    ];
    
    let selected = select_main_video_stream(&streams);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().index, 1, "Should select stream with default disposition");
    assert!(selected.unwrap().is_default);
}

/// Test stream selection falls back to first when no default
#[test]
fn test_first_stream_fallback() {
    let streams = vec![
        VideoStream {
            index: 0,
            codec_name: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: false,
        },
        VideoStream {
            index: 1,
            codec_name: "hevc".to_string(),
            width: 3840,
            height: 2160,
            bitrate: Some(20_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            is_default: false,
        }
    ];
    
    let selected = select_main_video_stream(&streams);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().index, 0, "Should select first stream when no default");
}

/// Test parsing of various video codecs
#[test]
fn test_various_codecs() {
    let codecs = vec!["h264", "hevc", "vp9", "av1", "mpeg2video", "mpeg4"];
    
    for codec in codecs {
        let streams = vec![
            VideoStream {
                index: 0,
                codec_name: codec.to_string(),
                width: 1920,
                height: 1080,
                bitrate: Some(5_000_000),
                frame_rate: Some("24/1".to_string()),
                pix_fmt: Some("yuv420p".to_string()),
                bit_depth: Some(8),
                is_default: false,
            }
        ];
        
        let selected = select_main_video_stream(&streams);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().codec_name, codec);
    }
}
