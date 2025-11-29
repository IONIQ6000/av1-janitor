use av1d_daemon::probe::{select_main_video_stream, VideoStream};

/// Test stream selection with empty stream list
#[test]
fn test_empty_stream_list() {
    let streams: Vec<VideoStream> = Vec::new();
    let selected = select_main_video_stream(&streams);
    assert!(
        selected.is_none(),
        "Should return None for empty stream list"
    );
}

/// Test stream selection with single stream
#[test]
fn test_single_stream() {
    let streams = vec![VideoStream {
        index: 0,
        codec_name: "h264".to_string(),
        width: 1920,
        height: 1080,
        bitrate: Some(5_000_000),
        frame_rate: Some("24/1".to_string()),
        pix_fmt: Some("yuv420p".to_string()),
        bit_depth: Some(8),
        is_default: false,
    }];

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
        },
    ];

    let selected = select_main_video_stream(&streams);
    assert!(selected.is_some());
    assert_eq!(
        selected.unwrap().index,
        1,
        "Should select stream with default disposition"
    );
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
        },
    ];

    let selected = select_main_video_stream(&streams);
    assert!(selected.is_some());
    assert_eq!(
        selected.unwrap().index,
        0,
        "Should select first stream when no default"
    );
}

/// Test parsing of various video codecs
#[test]
fn test_various_codecs() {
    let codecs = vec!["h264", "hevc", "vp9", "av1", "mpeg2video", "mpeg4"];

    for codec in codecs {
        let streams = vec![VideoStream {
            index: 0,
            codec_name: codec.to_string(),
            width: 1920,
            height: 1080,
            bitrate: Some(5_000_000),
            frame_rate: Some("24/1".to_string()),
            pix_fmt: Some("yuv420p".to_string()),
            bit_depth: Some(8),
            is_default: false,
        }];

        let selected = select_main_video_stream(&streams);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().codec_name, codec);
    }
}
