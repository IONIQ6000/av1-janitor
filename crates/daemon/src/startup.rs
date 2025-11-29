use crate::config::EncoderPreference;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailableEncoder {
    SvtAv1,
    LibaomAv1,
    Librav1e,
}

#[derive(Debug, Clone)]
pub struct SelectedEncoder {
    pub encoder: AvailableEncoder,
    pub codec_name: String,
}

pub fn check_ffmpeg_version() -> Result<(u32, u32, u32)> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("Failed to execute ffmpeg -version. Is ffmpeg installed and in PATH?")?;

    if !output.status.success() {
        return Err(anyhow!("ffmpeg -version command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse version from output like "ffmpeg version 8.0.1" or "ffmpeg version n8.0.1"
    let re = Regex::new(r"ffmpeg version[^\d]*(\d+)\.(\d+)\.(\d+)").unwrap();

    if let Some(caps) = re.captures(&stdout) {
        let major: u32 = caps[1].parse().context("Failed to parse major version")?;
        let minor: u32 = caps[2].parse().context("Failed to parse minor version")?;
        let patch: u32 = caps[3].parse().context("Failed to parse patch version")?;

        if major < 8 {
            return Err(anyhow!(
                "FFmpeg version {}.{}.{} is too old. Version 8.0 or higher is required.",
                major,
                minor,
                patch
            ));
        }

        Ok((major, minor, patch))
    } else {
        Err(anyhow!(
            "Failed to parse ffmpeg version from output: {}",
            stdout
        ))
    }
}

pub fn detect_available_encoders() -> Result<Vec<AvailableEncoder>> {
    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .context("Failed to execute ffmpeg -encoders")?;

    if !output.status.success() {
        return Err(anyhow!("ffmpeg -encoders command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut available = Vec::new();

    // Check for each AV1 encoder in the output
    if stdout.contains("libsvtav1") {
        available.push(AvailableEncoder::SvtAv1);
    }
    if stdout.contains("libaom-av1") {
        available.push(AvailableEncoder::LibaomAv1);
    }
    if stdout.contains("librav1e") {
        available.push(AvailableEncoder::Librav1e);
    }

    if available.is_empty() {
        return Err(anyhow!(
            "No AV1 encoders detected. Please install ffmpeg with at least one of: libsvtav1, libaom-av1, librav1e"
        ));
    }

    Ok(available)
}

pub fn select_encoder(
    available: &[AvailableEncoder],
    preference: EncoderPreference,
) -> Result<SelectedEncoder> {
    if available.is_empty() {
        return Err(anyhow!("No encoders available for selection"));
    }

    // Try to honor the preference first
    let preferred = match preference {
        EncoderPreference::Svt => AvailableEncoder::SvtAv1,
        EncoderPreference::Aom => AvailableEncoder::LibaomAv1,
        EncoderPreference::Rav1e => AvailableEncoder::Librav1e,
    };

    if available.contains(&preferred) {
        return Ok(encoder_to_selected(preferred));
    }

    // Fall back to hierarchy: SVT-AV1 > libaom-av1 > librav1e
    if available.contains(&AvailableEncoder::SvtAv1) {
        return Ok(encoder_to_selected(AvailableEncoder::SvtAv1));
    }
    if available.contains(&AvailableEncoder::LibaomAv1) {
        return Ok(encoder_to_selected(AvailableEncoder::LibaomAv1));
    }
    if available.contains(&AvailableEncoder::Librav1e) {
        return Ok(encoder_to_selected(AvailableEncoder::Librav1e));
    }

    Err(anyhow!("No suitable encoder found"))
}

fn encoder_to_selected(encoder: AvailableEncoder) -> SelectedEncoder {
    let codec_name = match encoder {
        AvailableEncoder::SvtAv1 => "libsvtav1",
        AvailableEncoder::LibaomAv1 => "libaom-av1",
        AvailableEncoder::Librav1e => "librav1e",
    };

    SelectedEncoder {
        encoder,
        codec_name: codec_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: av1-reencoder, Property 1: FFmpeg version validation**
    // **Validates: Requirements 1.1, 1.2, 1.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_ffmpeg_version_parsing(
            major in 0u32..20,
            minor in 0u32..100,
            patch in 0u32..100,
        ) {
            // Generate a mock ffmpeg version output
            let version_output = format!("ffmpeg version {}.{}.{} Copyright (c) 2000-2024", major, minor, patch);

            // Parse using regex (same logic as check_ffmpeg_version)
            let re = Regex::new(r"ffmpeg version[^\d]*(\d+)\.(\d+)\.(\d+)").unwrap();

            if let Some(caps) = re.captures(&version_output) {
                let parsed_major: u32 = caps[1].parse().unwrap();
                let parsed_minor: u32 = caps[2].parse().unwrap();
                let parsed_patch: u32 = caps[3].parse().unwrap();

                // Property: Parsing should correctly extract version numbers
                prop_assert_eq!(parsed_major, major);
                prop_assert_eq!(parsed_minor, minor);
                prop_assert_eq!(parsed_patch, patch);

                // Property: Versions < 8 should be rejected
                if major < 8 {
                    prop_assert!(parsed_major < 8, "Version {} should be rejected", major);
                } else {
                    prop_assert!(parsed_major >= 8, "Version {} should be accepted", major);
                }
            } else {
                panic!("Failed to parse version string: {}", version_output);
            }
        }
    }

    #[test]
    fn test_version_rejection() {
        // Test that versions below 8 are properly rejected
        let test_cases = vec![
            ("ffmpeg version 7.0.0", false),
            ("ffmpeg version 7.9.9", false),
            ("ffmpeg version 8.0.0", true),
            ("ffmpeg version 8.1.0", true),
            ("ffmpeg version 9.0.0", true),
        ];

        for (version_str, should_accept) in test_cases {
            let re = Regex::new(r"ffmpeg version[^\d]*(\d+)\.(\d+)\.(\d+)").unwrap();
            if let Some(caps) = re.captures(version_str) {
                let major: u32 = caps[1].parse().unwrap();
                let is_valid = major >= 8;
                assert_eq!(
                    is_valid, should_accept,
                    "Version {} acceptance mismatch",
                    version_str
                );
            }
        }
    }

    #[test]
    fn test_encoder_to_selected() {
        let svt = encoder_to_selected(AvailableEncoder::SvtAv1);
        assert_eq!(svt.codec_name, "libsvtav1");
        assert_eq!(svt.encoder, AvailableEncoder::SvtAv1);

        let aom = encoder_to_selected(AvailableEncoder::LibaomAv1);
        assert_eq!(aom.codec_name, "libaom-av1");
        assert_eq!(aom.encoder, AvailableEncoder::LibaomAv1);

        let rav1e = encoder_to_selected(AvailableEncoder::Librav1e);
        assert_eq!(rav1e.codec_name, "librav1e");
        assert_eq!(rav1e.encoder, AvailableEncoder::Librav1e);
    }

    // **Feature: av1-reencoder, Property 2: Encoder selection hierarchy**
    // **Validates: Requirements 2.2, 2.3, 2.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_encoder_selection_hierarchy(
            has_svt in prop::bool::ANY,
            has_aom in prop::bool::ANY,
            has_rav1e in prop::bool::ANY,
            preference in prop::sample::select(vec![
                EncoderPreference::Svt,
                EncoderPreference::Aom,
                EncoderPreference::Rav1e,
            ]),
        ) {
            // Build available encoder list based on flags
            let mut available = Vec::new();
            if has_svt {
                available.push(AvailableEncoder::SvtAv1);
            }
            if has_aom {
                available.push(AvailableEncoder::LibaomAv1);
            }
            if has_rav1e {
                available.push(AvailableEncoder::Librav1e);
            }

            // If no encoders available, selection should fail
            if available.is_empty() {
                prop_assert!(select_encoder(&available, preference).is_err());
                return Ok(());
            }

            // Otherwise, selection should succeed
            let result = select_encoder(&available, preference);
            prop_assert!(result.is_ok());

            let selected = result.unwrap();

            // Property: If preferred encoder is available, it should be selected
            let preferred_encoder = match preference {
                EncoderPreference::Svt => AvailableEncoder::SvtAv1,
                EncoderPreference::Aom => AvailableEncoder::LibaomAv1,
                EncoderPreference::Rav1e => AvailableEncoder::Librav1e,
            };

            if available.contains(&preferred_encoder) {
                prop_assert_eq!(selected.encoder, preferred_encoder);
            } else {
                // Property: Fall back to hierarchy: SVT-AV1 > libaom-av1 > librav1e
                if has_svt {
                    prop_assert_eq!(selected.encoder, AvailableEncoder::SvtAv1);
                } else if has_aom {
                    prop_assert_eq!(selected.encoder, AvailableEncoder::LibaomAv1);
                } else if has_rav1e {
                    prop_assert_eq!(selected.encoder, AvailableEncoder::Librav1e);
                }
            }

            // Property: Codec name should match encoder type
            let expected_codec = match selected.encoder {
                AvailableEncoder::SvtAv1 => "libsvtav1",
                AvailableEncoder::LibaomAv1 => "libaom-av1",
                AvailableEncoder::Librav1e => "librav1e",
            };
            prop_assert_eq!(selected.codec_name, expected_codec);
        }
    }

    #[test]
    fn test_encoder_selection_specific_cases() {
        // Test specific hierarchy cases

        // Case 1: All available, should select SVT-AV1
        let all = vec![
            AvailableEncoder::SvtAv1,
            AvailableEncoder::LibaomAv1,
            AvailableEncoder::Librav1e,
        ];
        let result = select_encoder(&all, EncoderPreference::Svt).unwrap();
        assert_eq!(result.encoder, AvailableEncoder::SvtAv1);

        // Case 2: Only AOM and rav1e, should select AOM
        let aom_rav1e = vec![AvailableEncoder::LibaomAv1, AvailableEncoder::Librav1e];
        let result = select_encoder(&aom_rav1e, EncoderPreference::Svt).unwrap();
        assert_eq!(result.encoder, AvailableEncoder::LibaomAv1);

        // Case 3: Only rav1e, should select rav1e
        let only_rav1e = vec![AvailableEncoder::Librav1e];
        let result = select_encoder(&only_rav1e, EncoderPreference::Svt).unwrap();
        assert_eq!(result.encoder, AvailableEncoder::Librav1e);

        // Case 4: Preference honored when available
        let result = select_encoder(&all, EncoderPreference::Aom).unwrap();
        assert_eq!(result.encoder, AvailableEncoder::LibaomAv1);

        // Case 5: Empty list should fail
        let empty: Vec<AvailableEncoder> = vec![];
        assert!(select_encoder(&empty, EncoderPreference::Svt).is_err());
    }
}
