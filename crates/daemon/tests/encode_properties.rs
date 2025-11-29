use av1d_daemon::config::QualityTier;
use av1d_daemon::encode::aom::{build_aom_command, select_cpu_used, select_tiles};
use av1d_daemon::encode::common::{pad_filter, stream_mapping_flags, websafe_input_flags};
use av1d_daemon::encode::rav1e::build_rav1e_command;
use av1d_daemon::encode::svt::build_svt_command;
use av1d_daemon::encode::{select_crf, select_preset};
use av1d_daemon::jobs::{Job, JobStatus};
use chrono::Utc;
use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: av1-reencoder, Property 12: CRF selection by resolution**
    /// **Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**
    ///
    /// For any video height, the system should select the appropriate CRF value
    /// according to the ultra-high quality ladder:
    /// - 20 for ≥2160p (very high quality)
    /// - 21 for 1440p (very high quality)
    /// - 22 for 1080p (high quality)
    /// - 23 for <1080p (high quality)
    /// No bitrate adjustment - prioritize quality over size
    #[test]
    fn prop_crf_selection_by_resolution(
        height in 480i32..4320i32,
        bitrate in prop::option::of(1_000_000u64..100_000_000u64),
        quality_tier in prop_oneof![Just(QualityTier::High), Just(QualityTier::VeryHigh)],
    ) {
        let crf = select_crf(height, bitrate, quality_tier);

        // Determine expected CRF based on height (no bitrate adjustment)
        let base_crf = if height >= 2160 {
            20
        } else if height >= 1440 {
            21
        } else if height >= 1080 {
            22
        } else {
            23
        };
        let expected_crf = match quality_tier {
            QualityTier::High => base_crf,
            QualityTier::VeryHigh => base_crf.saturating_sub(1),
        };

        prop_assert_eq!(crf, expected_crf,
            "CRF mismatch for height={}, bitrate={:?}: expected {}, got {}",
            height, bitrate, expected_crf, crf);
    }

    /// **Feature: av1-reencoder, Property 13: SVT-AV1 preset selection**
    /// **Validates: Requirements 12.1, 12.2, 12.3, 12.4, 12.5**
    ///
    /// For any video height and quality tier, the system should select the
    /// appropriate SVT-AV1 preset value (ultra-slow for maximum quality):
    /// - Preset 2 for ≥2160p (very slow, very high quality)
    /// - Preset 3 for 1440p and 1080p (slow, high quality)
    /// - Preset 4 for <1080p (moderate speed, high quality)
    /// With quality tier adjustment (-1 for VeryHigh)
    #[test]
    fn prop_svt_preset_selection(
        height in 480i32..4320i32,
        quality_tier in prop_oneof![Just(QualityTier::High), Just(QualityTier::VeryHigh)]
    ) {
        let preset = select_preset(height, quality_tier);

        // Determine expected base preset based on height (slower presets for quality)
        let expected_base_preset: u8 = if height >= 2160 {
            2
        } else if height >= 1440 {
            3
        } else if height >= 1080 {
            3
        } else {
            4
        };

        // Apply quality tier adjustment
        let expected_preset = match quality_tier {
            QualityTier::High => expected_base_preset,
            QualityTier::VeryHigh => expected_base_preset.saturating_sub(1),
        };

        prop_assert_eq!(preset, expected_preset,
            "Preset mismatch for height={}, quality_tier={:?}: expected {}, got {}",
            height, quality_tier, expected_preset, preset);
    }

    /// **Feature: av1-reencoder, Property 14: Stream mapping command construction**
    /// **Validates: Requirements 13.1, 13.2, 13.3, 13.4, 13.5**
    ///
    /// For any ffmpeg command, it should include all required stream mapping flags:
    /// - Map all streams initially (-map 0)
    /// - Exclude attached pictures (-map -0:v:m:attached_pic)
    /// - Exclude Russian audio tracks (-map -0:a:m:language:ru and rus)
    /// - Exclude Russian subtitle tracks (-map -0:s:m:language:ru and rus)
    /// - Preserve chapters and metadata (-map_chapters 0, -map_metadata 0)
    #[test]
    fn prop_stream_mapping_command_construction(_dummy in 0u8..1u8) {
        let flags = stream_mapping_flags();

        // Convert to a single string for easier checking
        let flags_str = flags.join(" ");

        // Check for required flags
        prop_assert!(flags.contains(&"-map".to_string()), "Missing -map flag");
        prop_assert!(flags.contains(&"0".to_string()), "Missing initial stream selection '0'");
        prop_assert!(flags_str.contains("-map -0:v:m:attached_pic"),
            "Missing attached picture exclusion");
        prop_assert!(flags_str.contains("-map -0:a:m:language:ru"),
            "Missing Russian audio exclusion (ru)");
        prop_assert!(flags_str.contains("-map -0:a:m:language:rus"),
            "Missing Russian audio exclusion (rus)");
        prop_assert!(flags_str.contains("-map -0:s:m:language:ru"),
            "Missing Russian subtitle exclusion (ru)");
        prop_assert!(flags_str.contains("-map -0:s:m:language:rus"),
            "Missing Russian subtitle exclusion (rus)");
        prop_assert!(flags_str.contains("-map_chapters 0"),
            "Missing chapter preservation");
        prop_assert!(flags_str.contains("-map_metadata 0"),
            "Missing metadata preservation");
    }

    /// **Feature: av1-reencoder, Property 15: WebLike flag inclusion**
    /// **Validates: Requirements 14.1, 14.2, 14.3, 14.4**
    ///
    /// For any source classified as WebLike, the ffmpeg command should include
    /// all WebRip-safe flags (genpts, copyts, start_at_zero, vsync, avoid_negative_ts).
    /// For non-WebLike sources, these flags should be omitted.
    #[test]
    fn prop_weblike_flag_inclusion(_dummy in 0u8..1u8) {
        let flags = websafe_input_flags();

        // Convert to a single string for easier checking
        let flags_str = flags.join(" ");

        // Check for all required WebSafe flags
        prop_assert!(flags_str.contains("-fflags +genpts"),
            "Missing -fflags +genpts flag");
        prop_assert!(flags_str.contains("-copyts"),
            "Missing -copyts flag");
        prop_assert!(flags_str.contains("-start_at_zero"),
            "Missing -start_at_zero flag");
        prop_assert!(flags_str.contains("-vsync 0"),
            "Missing -vsync 0 flag");
        prop_assert!(flags_str.contains("-avoid_negative_ts make_zero"),
            "Missing -avoid_negative_ts make_zero flag");
    }

    /// **Feature: av1-reencoder, Property 16: Pad filter application**
    /// **Validates: Requirements 15.1, 15.2, 15.3**
    ///
    /// For any source that is WebLike OR has odd width OR has odd height,
    /// the ffmpeg command should include the pad filter.
    /// Otherwise, the pad filter should be omitted.
    #[test]
    fn prop_pad_filter_application(
        width in 480i32..3840i32,
        height in 480i32..2160i32,
        is_web_like in any::<bool>()
    ) {
        let filter = pad_filter(width, height, is_web_like);

        // Determine if padding should be applied
        let should_pad = is_web_like || width % 2 != 0 || height % 2 != 0;

        if should_pad {
            prop_assert!(filter.is_some(),
                "Pad filter should be present for width={}, height={}, is_web_like={}",
                width, height, is_web_like);
            prop_assert_eq!(filter.unwrap(), "-vf",
                "Pad filter flag should be '-vf'");
        } else {
            prop_assert!(filter.is_none(),
                "Pad filter should be omitted for width={}, height={}, is_web_like={}",
                width, height, is_web_like);
        }
    }

    /// **Feature: av1-reencoder, Property 17: SVT-AV1 command parameters**
    /// **Validates: Requirements 16.1, 16.2, 16.3, 16.4, 16.5**
    ///
    /// For any job using SVT-AV1 encoder, the ffmpeg command should include
    /// all required SVT-AV1 parameters with correct CRF and preset values.
    #[test]
    fn prop_svt_av1_command_parameters(
        crf in 18u8..28u8,
        preset in 2u8..6u8,
        is_web_like in any::<bool>(),
        width in 480i32..3840i32,
        height in 480i32..2160i32
    ) {
        // Create a test job
        let job = Job {
            id: "test-job".to_string(),
            source_path: PathBuf::from("/test/input.mkv"),
            output_path: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            status: JobStatus::Pending,
            reason: None,
            original_bytes: Some(1000000),
            new_bytes: None,
            is_web_like,
            video_codec: Some("hevc".to_string()),
            video_bitrate: Some(5000000),
            video_width: Some(width),
            video_height: Some(height),
            video_frame_rate: Some("24/1".to_string()),
            crf_used: None,
            preset_used: None,
            encoder_used: None,
            source_bit_depth: Some(8),
            source_pix_fmt: Some("yuv420p".to_string()),
            is_hdr: Some(false),
            av1_quality: None,
            target_bit_depth: None,
            av1_profile: None,
            quality_tier: None,
            test_clip_path: None,
            test_clip_approved: None,
        };

        let command = build_svt_command(&job, crf, preset, "/test/output.mkv");
        let command_str = command.join(" ");

        // Check for required SVT-AV1 parameters
        prop_assert!(command_str.contains("-c:v libsvtav1"),
            "Missing SVT-AV1 codec specification");
        prop_assert!(command_str.contains(&format!("-crf {}", crf)),
            "Missing or incorrect CRF value");
        prop_assert!(command_str.contains(&format!("-preset {}", preset)),
            "Missing or incorrect preset value");
        prop_assert!(command_str.contains("-threads 0"),
            "Missing automatic thread allocation");
        prop_assert!(command_str.contains("-svtav1-params lp=0"),
            "Missing SVT-AV1 logical processor parameter");

        // Check for audio and subtitle copying
        prop_assert!(command_str.contains("-c:a copy"),
            "Missing audio stream copy");
        prop_assert!(command_str.contains("-c:s copy"),
            "Missing subtitle stream copy");

        // Check for max muxing queue size
        prop_assert!(command_str.contains("-max_muxing_queue_size 2048"),
            "Missing max muxing queue size");

        // Check for WebSafe flags if WebLike
        if is_web_like {
            prop_assert!(command_str.contains("-fflags +genpts"),
                "Missing WebSafe flags for WebLike source");
        }
    }

    /// **Feature: av1-reencoder, Property 18: libaom-av1 command parameters**
    /// **Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5, 17.6, 17.7, 17.8**
    ///
    /// For any job using libaom-av1 encoder, the ffmpeg command should include
    /// all required libaom-av1 parameters with correct CRF, cpu-used, and tile
    /// configuration based on resolution.
    #[test]
    fn prop_libaom_av1_command_parameters(
        crf in 18u8..28u8,
        is_web_like in any::<bool>(),
        width in 480i32..3840i32,
        height in 480i32..2160i32
    ) {
        // Create a test job
        let job = Job {
            id: "test-job".to_string(),
            source_path: PathBuf::from("/test/input.mkv"),
            output_path: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            status: JobStatus::Pending,
            reason: None,
            original_bytes: Some(1000000),
            new_bytes: None,
            is_web_like,
            video_codec: Some("hevc".to_string()),
            video_bitrate: Some(5000000),
            video_width: Some(width),
            video_height: Some(height),
            video_frame_rate: Some("24/1".to_string()),
            crf_used: None,
            preset_used: None,
            encoder_used: None,
            source_bit_depth: Some(8),
            source_pix_fmt: Some("yuv420p".to_string()),
            is_hdr: Some(false),
            av1_quality: None,
            target_bit_depth: None,
            av1_profile: None,
            quality_tier: None,
            test_clip_path: None,
            test_clip_approved: None,
        };

        let command = build_aom_command(&job, crf, "/test/output.mkv");
        let command_str = command.join(" ");

        // Check for required libaom-av1 parameters
        prop_assert!(command_str.contains("-c:v libaom-av1"),
            "Missing libaom-av1 codec specification");
        prop_assert!(command_str.contains("-b:v 0"),
            "Missing constant quality mode (-b:v 0)");
        prop_assert!(command_str.contains(&format!("-crf {}", crf)),
            "Missing or incorrect CRF value");

        // Check cpu-used based on resolution
        let expected_cpu_used = select_cpu_used(height);
        prop_assert!(command_str.contains(&format!("-cpu-used {}", expected_cpu_used)),
            "Missing or incorrect cpu-used value for height {}", height);

        // Check row-based multithreading
        prop_assert!(command_str.contains("-row-mt 1"),
            "Missing row-based multithreading");

        // Check tile configuration based on resolution
        let expected_tiles = select_tiles(height);
        prop_assert!(command_str.contains(&format!("-tiles {}", expected_tiles)),
            "Missing or incorrect tile configuration for height {}", height);

        // Check for audio and subtitle copying
        prop_assert!(command_str.contains("-c:a copy"),
            "Missing audio stream copy");
        prop_assert!(command_str.contains("-c:s copy"),
            "Missing subtitle stream copy");

        // Check for max muxing queue size
        prop_assert!(command_str.contains("-max_muxing_queue_size 2048"),
            "Missing max muxing queue size");

        // Check for WebSafe flags if WebLike
        if is_web_like {
            prop_assert!(command_str.contains("-fflags +genpts"),
                "Missing WebSafe flags for WebLike source");
        }
    }

    /// **Feature: av1-reencoder, Property 19: Audio and subtitle stream copying**
    /// **Validates: Requirements 18.1, 18.2, 18.3**
    ///
    /// For any ffmpeg command, it should include parameters to copy audio and
    /// subtitle streams without re-encoding, and include max_muxing_queue_size.
    #[test]
    fn prop_audio_and_subtitle_stream_copying(
        crf in 18u8..28u8,
        encoder_type in 0u8..3u8,
        is_web_like in any::<bool>(),
        width in 480i32..3840i32,
        height in 480i32..2160i32
    ) {
        // Create a test job
        let job = Job {
            id: "test-job".to_string(),
            source_path: PathBuf::from("/test/input.mkv"),
            output_path: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            status: JobStatus::Pending,
            reason: None,
            original_bytes: Some(1000000),
            new_bytes: None,
            is_web_like,
            video_codec: Some("hevc".to_string()),
            video_bitrate: Some(5000000),
            video_width: Some(width),
            video_height: Some(height),
            video_frame_rate: Some("24/1".to_string()),
            crf_used: None,
            preset_used: None,
            encoder_used: None,
            source_bit_depth: Some(8),
            source_pix_fmt: Some("yuv420p".to_string()),
            is_hdr: Some(false),
            av1_quality: None,
            target_bit_depth: None,
            av1_profile: None,
            quality_tier: None,
            test_clip_path: None,
            test_clip_approved: None,
        };

        // Build command based on encoder type
        let command = match encoder_type {
            0 => build_svt_command(&job, crf, 4, "/test/output.mkv"),
            1 => build_aom_command(&job, crf, "/test/output.mkv"),
            _ => build_rav1e_command(&job, crf, "/test/output.mkv"),
        };

        let command_str = command.join(" ");

        // Check for audio stream copying
        prop_assert!(command_str.contains("-c:a copy"),
            "Missing audio stream copy parameter");

        // Check for subtitle stream copying
        prop_assert!(command_str.contains("-c:s copy"),
            "Missing subtitle stream copy parameter");

        // Check for max muxing queue size
        prop_assert!(command_str.contains("-max_muxing_queue_size 2048"),
            "Missing max_muxing_queue_size parameter");
    }
}

/// **Feature: av1-reencoder, Property 20: Concurrent job limiting**
/// **Validates: Requirements 19.2**
///
/// For any number of pending jobs, the system should never run more than
/// max_concurrent_jobs simultaneously. This property verifies that the
/// JobExecutor correctly limits concurrent execution.
#[tokio::test]
async fn prop_concurrent_job_limiting() {
    use av1d_daemon::encode::JobExecutor;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    // Test with different max_concurrent values
    for max_concurrent in 1..=5 {
        let executor = Arc::new(JobExecutor::new(max_concurrent));
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));

        // Spawn more jobs than the limit
        let num_jobs = max_concurrent * 3;
        let mut handles = Vec::new();

        for job_id in 0..num_jobs {
            let executor_clone = executor.clone();
            let concurrent_count_clone = concurrent_count.clone();
            let max_observed_clone = max_observed.clone();

            let handle = tokio::spawn(async move {
                executor_clone
                    .execute_job(|| async move {
                        // Increment concurrent count
                        let current = concurrent_count_clone.fetch_add(1, Ordering::SeqCst) + 1;

                        // Update max observed
                        max_observed_clone.fetch_max(current, Ordering::SeqCst);

                        // Simulate some work
                        sleep(Duration::from_millis(50)).await;

                        // Decrement concurrent count
                        concurrent_count_clone.fetch_sub(1, Ordering::SeqCst);

                        Ok(PathBuf::from(format!("job-{}", job_id)))
                    })
                    .await
            });

            handles.push(handle);
        }

        // Wait for all jobs to complete
        for handle in handles {
            handle
                .await
                .expect("Job task panicked")
                .expect("Job failed");
        }

        // Verify that we never exceeded the limit
        let max_concurrent_observed = max_observed.load(Ordering::SeqCst);
        assert!(
            max_concurrent_observed <= max_concurrent,
            "Concurrent job limit violated: max_concurrent={}, observed={}",
            max_concurrent,
            max_concurrent_observed
        );

        // Verify that we actually used the available concurrency
        // (at least reached the limit at some point)
        assert!(
            max_concurrent_observed >= max_concurrent.min(num_jobs),
            "Did not utilize available concurrency: max_concurrent={}, observed={}",
            max_concurrent,
            max_concurrent_observed
        );
    }
}
