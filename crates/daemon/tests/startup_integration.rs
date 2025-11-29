use av1d_daemon::config::EncoderPreference;
use av1d_daemon::startup::*;

#[test]
fn test_encoder_selection_integration() {
    // Test encoder selection with all encoders available
    let all_encoders = vec![
        AvailableEncoder::SvtAv1,
        AvailableEncoder::LibaomAv1,
        AvailableEncoder::Librav1e,
    ];

    // Should select SVT-AV1 by default (hierarchy)
    let result = select_encoder(&all_encoders, EncoderPreference::Svt).unwrap();
    assert_eq!(result.encoder, AvailableEncoder::SvtAv1);
    assert_eq!(result.codec_name, "libsvtav1");

    // Should honor preference for AOM
    let result = select_encoder(&all_encoders, EncoderPreference::Aom).unwrap();
    assert_eq!(result.encoder, AvailableEncoder::LibaomAv1);
    assert_eq!(result.codec_name, "libaom-av1");

    // Test fallback when preferred not available
    let only_rav1e = vec![AvailableEncoder::Librav1e];
    let result = select_encoder(&only_rav1e, EncoderPreference::Svt).unwrap();
    assert_eq!(result.encoder, AvailableEncoder::Librav1e);
    assert_eq!(result.codec_name, "librav1e");

    // Test error on empty list
    let empty: Vec<AvailableEncoder> = vec![];
    assert!(select_encoder(&empty, EncoderPreference::Svt).is_err());
}

#[test]
fn test_encoder_hierarchy() {
    // Test that hierarchy is respected: SVT > AOM > rav1e

    // Only AOM and rav1e available, should pick AOM
    let aom_rav1e = vec![AvailableEncoder::LibaomAv1, AvailableEncoder::Librav1e];
    let result = select_encoder(&aom_rav1e, EncoderPreference::Svt).unwrap();
    assert_eq!(result.encoder, AvailableEncoder::LibaomAv1);

    // Only SVT and rav1e available, should pick SVT
    let svt_rav1e = vec![AvailableEncoder::SvtAv1, AvailableEncoder::Librav1e];
    let result = select_encoder(&svt_rav1e, EncoderPreference::Aom).unwrap();
    assert_eq!(result.encoder, AvailableEncoder::SvtAv1);
}
