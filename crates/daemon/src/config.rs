use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub library_roots: Vec<PathBuf>,
    pub min_bytes: u64,
    pub max_size_ratio: f64,
    pub scan_interval_secs: u64,
    pub job_state_dir: PathBuf,
    pub temp_output_dir: PathBuf,
    pub max_concurrent_jobs: usize,
    pub prefer_encoder: EncoderPreference,
    pub quality_tier: QualityTier,
    pub keep_original: bool,
    pub write_why_sidecars: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncoderPreference {
    Svt,
    Aom,
    Rav1e,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityTier {
    High,
    VeryHigh,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            library_roots: vec![PathBuf::from("/media")],
            min_bytes: 2_147_483_648, // 2 GiB
            max_size_ratio: 0.90,
            scan_interval_secs: 60,
            job_state_dir: PathBuf::from("/var/lib/av1d/jobs"),
            temp_output_dir: PathBuf::from("/var/lib/av1d/temp"),
            max_concurrent_jobs: 1,
            prefer_encoder: EncoderPreference::Svt,
            quality_tier: QualityTier::VeryHigh,
            keep_original: false,
            write_why_sidecars: true,
        }
    }
}

pub fn load_config(path: Option<&std::path::Path>) -> Result<DaemonConfig> {
    let config = if let Some(config_path) = path {
        if config_path.exists() {
            let contents = std::fs::read_to_string(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;

            toml::from_str::<DaemonConfig>(&contents)
                .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?
        } else {
            #[cfg(not(test))]
            tracing::warn!("Config file not found at {:?}, using defaults", config_path);
            DaemonConfig::default()
        }
    } else {
        #[cfg(not(test))]
        tracing::info!("No config path provided, using defaults");
        DaemonConfig::default()
    };

    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &DaemonConfig) -> Result<()> {
    if config.library_roots.is_empty() {
        anyhow::bail!("library_roots cannot be empty");
    }

    if config.max_size_ratio <= 0.0 || config.max_size_ratio > 1.0 {
        anyhow::bail!("max_size_ratio must be between 0.0 and 1.0 (exclusive of 0.0)");
    }

    if config.max_concurrent_jobs == 0 {
        anyhow::bail!("max_concurrent_jobs must be at least 1");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Property test generators
    fn arb_encoder_preference() -> impl Strategy<Value = EncoderPreference> {
        prop_oneof![
            Just(EncoderPreference::Svt),
            Just(EncoderPreference::Aom),
            Just(EncoderPreference::Rav1e),
        ]
    }

    fn arb_quality_tier() -> impl Strategy<Value = QualityTier> {
        prop_oneof![Just(QualityTier::High), Just(QualityTier::VeryHigh),]
    }

    fn arb_daemon_config() -> impl Strategy<Value = DaemonConfig> {
        (
            prop::collection::vec(any::<String>().prop_map(PathBuf::from), 1..5),
            1_000_000_u64..100_000_000_000_u64,
            0.01_f64..1.0_f64,
            1_u64..3600_u64,
            any::<String>().prop_map(PathBuf::from),
            any::<String>().prop_map(PathBuf::from),
            1_usize..32_usize,
            arb_encoder_preference(),
            arb_quality_tier(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(
                |(
                    library_roots,
                    min_bytes,
                    max_size_ratio,
                    scan_interval_secs,
                    job_state_dir,
                    temp_output_dir,
                    max_concurrent_jobs,
                    prefer_encoder,
                    quality_tier,
                    keep_original,
                    write_why_sidecars,
                )| {
                    DaemonConfig {
                        library_roots,
                        min_bytes,
                        max_size_ratio,
                        scan_interval_secs,
                        job_state_dir,
                        temp_output_dir,
                        max_concurrent_jobs,
                        prefer_encoder,
                        quality_tier,
                        keep_original,
                        write_why_sidecars,
                    }
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: av1-reencoder, Property 3: Configuration loading and application**
        /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
        ///
        /// For any valid DaemonConfig, serializing to TOML and then deserializing
        /// should produce an equivalent configuration.
        #[test]
        fn prop_config_round_trip(config in arb_daemon_config()) {
            // Serialize to TOML
            let toml_string = toml::to_string(&config)
                .expect("Failed to serialize config to TOML");

            // Write to temporary file
            let mut temp_file = NamedTempFile::new()
                .expect("Failed to create temp file");
            temp_file.write_all(toml_string.as_bytes())
                .expect("Failed to write to temp file");
            temp_file.flush()
                .expect("Failed to flush temp file");

            // Load config from file
            let loaded_config = load_config(Some(temp_file.path()))
                .expect("Failed to load config from file");

            // Verify equality
            prop_assert_eq!(config, loaded_config);
        }
    }

    // Unit tests for edge cases

    #[test]
    fn test_missing_config_file_uses_defaults() {
        let non_existent_path = PathBuf::from("/tmp/non_existent_config_12345.toml");
        let config = load_config(Some(&non_existent_path)).expect("Should load defaults");
        assert_eq!(config, DaemonConfig::default());
    }

    #[test]
    fn test_no_config_path_uses_defaults() {
        let config = load_config(None).expect("Should load defaults");
        assert_eq!(config, DaemonConfig::default());
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"this is not valid TOML {{{")
            .expect("Failed to write");
        temp_file.flush().expect("Failed to flush");

        let result = load_config(Some(temp_file.path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse TOML"));
    }

    #[test]
    fn test_partial_config_with_defaults() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let partial_toml = r#"
library_roots = ["/custom/path"]
min_bytes = 5000000000
"#;
        temp_file
            .write_all(partial_toml.as_bytes())
            .expect("Failed to write");
        temp_file.flush().expect("Failed to flush");

        let config = load_config(Some(temp_file.path())).expect("Should load partial config");

        // Check custom values
        assert_eq!(config.library_roots, vec![PathBuf::from("/custom/path")]);
        assert_eq!(config.min_bytes, 5_000_000_000);

        // Check defaults are used for missing fields
        assert_eq!(
            config.max_size_ratio,
            DaemonConfig::default().max_size_ratio
        );
        assert_eq!(
            config.scan_interval_secs,
            DaemonConfig::default().scan_interval_secs
        );
        assert_eq!(
            config.max_concurrent_jobs,
            DaemonConfig::default().max_concurrent_jobs
        );
    }

    #[test]
    fn test_validation_empty_library_roots() {
        let config = DaemonConfig {
            library_roots: vec![],
            ..Default::default()
        };

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("library_roots"));
    }

    #[test]
    fn test_validation_invalid_max_size_ratio_zero() {
        let config = DaemonConfig {
            max_size_ratio: 0.0,
            ..Default::default()
        };

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_size_ratio"));
    }

    #[test]
    fn test_validation_invalid_max_size_ratio_above_one() {
        let config = DaemonConfig {
            max_size_ratio: 1.5,
            ..Default::default()
        };

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_size_ratio"));
    }

    #[test]
    fn test_validation_zero_concurrent_jobs() {
        let config = DaemonConfig {
            max_concurrent_jobs: 0,
            ..Default::default()
        };

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_concurrent_jobs"));
    }

    #[test]
    fn test_encoder_preference_serialization() {
        // Test serialization through a wrapper struct
        #[derive(Serialize)]
        struct Wrapper {
            pref: EncoderPreference,
        }

        let svt = toml::to_string(&Wrapper {
            pref: EncoderPreference::Svt,
        })
        .unwrap();
        assert!(svt.contains("pref = \"svt\""));

        let aom = toml::to_string(&Wrapper {
            pref: EncoderPreference::Aom,
        })
        .unwrap();
        assert!(aom.contains("pref = \"aom\""));

        let rav1e = toml::to_string(&Wrapper {
            pref: EncoderPreference::Rav1e,
        })
        .unwrap();
        assert!(rav1e.contains("pref = \"rav1e\""));
    }

    #[test]
    fn test_quality_tier_serialization() {
        // Test serialization through a wrapper struct
        #[derive(Serialize)]
        struct Wrapper {
            tier: QualityTier,
        }

        let high = toml::to_string(&Wrapper {
            tier: QualityTier::High,
        })
        .unwrap();
        assert!(high.contains("tier = \"high\""));

        let very_high = toml::to_string(&Wrapper {
            tier: QualityTier::VeryHigh,
        })
        .unwrap();
        assert!(very_high.contains("tier = \"very_high\""));
    }
}
