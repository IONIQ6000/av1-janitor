use av1d_daemon::size_gate::{check_size_gate, SizeGateResult};
use proptest::prelude::*;

/// **Feature: av1-reencoder, Property 23: Size gate enforcement**
/// *For any* original and output file sizes, the size gate should reject outputs ≥ (original × max_size_ratio) and create appropriate sidecar files
/// **Validates: Requirements 21.1, 21.2, 21.3, 21.4, 21.5, 21.6**
#[test]
fn property_size_gate_enforcement() {
    proptest!(|(
        original_bytes in 1_000_000u64..100_000_000_000u64, // 1MB to 100GB
        new_bytes in 1_000_000u64..100_000_000_000u64,
        max_ratio in 0.5f64..1.0f64, // Typical range: 50% to 100%
    )| {
        let result = check_size_gate(original_bytes, new_bytes, max_ratio);

        let threshold = (original_bytes as f64 * max_ratio) as u64;

        // Property: Files at or above threshold should fail
        if new_bytes >= threshold {
            prop_assert!(
                matches!(result, SizeGateResult::Fail { .. }),
                "Output size {} should fail when threshold is {} (original: {}, ratio: {})",
                new_bytes, threshold, original_bytes, max_ratio
            );

            // Verify the fail result contains correct values
            if let SizeGateResult::Fail { new_bytes: nb, threshold_bytes: tb } = result {
                prop_assert_eq!(nb, new_bytes, "Fail result should contain correct new_bytes");
                prop_assert_eq!(tb, threshold, "Fail result should contain correct threshold_bytes");
            }
        } else {
            // Files below threshold should pass
            prop_assert!(
                matches!(result, SizeGateResult::Pass { .. }),
                "Output size {} should pass when threshold is {} (original: {}, ratio: {})",
                new_bytes, threshold, original_bytes, max_ratio
            );

            // Verify the pass result contains correct calculations
            if let SizeGateResult::Pass { savings_bytes, compression_ratio } = result {
                let expected_savings = original_bytes - new_bytes;
                let expected_ratio = (new_bytes as f64) / (original_bytes as f64);

                prop_assert_eq!(
                    savings_bytes, expected_savings,
                    "Savings should be original - new: {} - {} = {}",
                    original_bytes, new_bytes, expected_savings
                );

                // Allow small floating point error
                prop_assert!(
                    (compression_ratio - expected_ratio).abs() < 0.0001,
                    "Compression ratio {} should be close to expected {}",
                    compression_ratio, expected_ratio
                );
            }
        }
    });
}

/// Test boundary condition: output size exactly equal to threshold
#[test]
fn test_size_gate_boundary_equal() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let max_ratio = 0.9;
    let threshold = (original_bytes as f64 * max_ratio) as u64; // 9GB
    let new_bytes = threshold; // Exactly at threshold

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // At threshold should fail (>=)
    assert!(
        matches!(result, SizeGateResult::Fail { .. }),
        "Output size exactly at threshold should fail"
    );

    if let SizeGateResult::Fail {
        new_bytes: nb,
        threshold_bytes: tb,
    } = result
    {
        assert_eq!(nb, new_bytes);
        assert_eq!(tb, threshold);
    }
}

/// Test boundary condition: output size just below threshold
#[test]
fn test_size_gate_boundary_below() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let max_ratio = 0.9;
    let threshold = (original_bytes as f64 * max_ratio) as u64; // 9GB
    let new_bytes = threshold - 1; // Just below threshold

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // Just below threshold should pass
    assert!(
        matches!(result, SizeGateResult::Pass { .. }),
        "Output size just below threshold should pass"
    );

    if let SizeGateResult::Pass {
        savings_bytes,
        compression_ratio,
    } = result
    {
        assert_eq!(savings_bytes, original_bytes - new_bytes);
        let expected_ratio = (new_bytes as f64) / (original_bytes as f64);
        assert!((compression_ratio - expected_ratio).abs() < 0.0001);
    }
}

/// Test boundary condition: output size just above threshold
#[test]
fn test_size_gate_boundary_above() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let max_ratio = 0.9;
    let threshold = (original_bytes as f64 * max_ratio) as u64; // 9GB
    let new_bytes = threshold + 1; // Just above threshold

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // Just above threshold should fail
    assert!(
        matches!(result, SizeGateResult::Fail { .. }),
        "Output size just above threshold should fail"
    );
}

/// Test that excellent compression passes
#[test]
fn test_excellent_compression() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 3_000_000_000u64; // 3GB (70% savings)
    let max_ratio = 0.9;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    assert!(
        matches!(result, SizeGateResult::Pass { .. }),
        "Excellent compression should pass"
    );

    if let SizeGateResult::Pass {
        savings_bytes,
        compression_ratio,
    } = result
    {
        assert_eq!(savings_bytes, 7_000_000_000);
        assert!((compression_ratio - 0.3).abs() < 0.0001);
    }
}

/// Test that minimal compression passes if under threshold
#[test]
fn test_minimal_compression() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 8_900_000_000u64; // 8.9GB (11% savings)
    let max_ratio = 0.9;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // 8.9GB < 9GB threshold, should pass
    assert!(
        matches!(result, SizeGateResult::Pass { .. }),
        "Minimal but sufficient compression should pass"
    );
}

/// Test that size increase fails
#[test]
fn test_size_increase() {
    let original_bytes = 5_000_000_000u64; // 5GB
    let new_bytes = 6_000_000_000u64; // 6GB (size increased!)
    let max_ratio = 0.9;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // 6GB > 4.5GB threshold, should fail
    assert!(
        matches!(result, SizeGateResult::Fail { .. }),
        "Size increase should fail"
    );
}

/// Test with max_ratio = 1.0 (no compression required)
#[test]
fn test_max_ratio_one() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 9_999_999_999u64; // Just under original
    let max_ratio = 1.0;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // Should pass since new < original and threshold = original
    assert!(
        matches!(result, SizeGateResult::Pass { .. }),
        "With max_ratio=1.0, any size reduction should pass"
    );
}

/// Test with max_ratio = 1.0 and equal sizes
#[test]
fn test_max_ratio_one_equal_size() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 10_000_000_000u64; // Same size
    let max_ratio = 1.0;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // Should fail since new >= threshold (both are 10GB)
    assert!(
        matches!(result, SizeGateResult::Fail { .. }),
        "With max_ratio=1.0 and equal sizes, should fail"
    );
}

/// Test with very strict max_ratio
#[test]
fn test_strict_max_ratio() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 6_000_000_000u64; // 6GB (40% savings)
    let max_ratio = 0.5; // Requires 50% or more savings

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // 6GB > 5GB threshold, should fail
    assert!(
        matches!(result, SizeGateResult::Fail { .. }),
        "With strict max_ratio=0.5, 40% savings should fail"
    );
}

/// Test with very strict max_ratio that passes
#[test]
fn test_strict_max_ratio_passes() {
    let original_bytes = 10_000_000_000u64; // 10GB
    let new_bytes = 4_000_000_000u64; // 4GB (60% savings)
    let max_ratio = 0.5; // Requires 50% or more savings

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    // 4GB < 5GB threshold, should pass
    assert!(
        matches!(result, SizeGateResult::Pass { .. }),
        "With strict max_ratio=0.5, 60% savings should pass"
    );
}

/// Test compression ratio calculation accuracy
#[test]
fn test_compression_ratio_accuracy() {
    let original_bytes = 8_000_000_000u64;
    let new_bytes = 2_000_000_000u64;
    let max_ratio = 0.9;

    let result = check_size_gate(original_bytes, new_bytes, max_ratio);

    if let SizeGateResult::Pass {
        savings_bytes,
        compression_ratio,
    } = result
    {
        assert_eq!(savings_bytes, 6_000_000_000);
        // 2GB / 8GB = 0.25
        assert!((compression_ratio - 0.25).abs() < 0.0001);
    } else {
        panic!("Expected Pass result");
    }
}

/// Test savings calculation accuracy
#[test]
fn test_savings_calculation() {
    let test_cases = vec![
        (10_000_000_000u64, 3_000_000_000u64, 7_000_000_000u64),
        (5_000_000_000u64, 4_000_000_000u64, 1_000_000_000u64),
        (1_000_000_000u64, 500_000_000u64, 500_000_000u64),
    ];

    for (original, new, expected_savings) in test_cases {
        let result = check_size_gate(original, new, 0.9);

        if let SizeGateResult::Pass { savings_bytes, .. } = result {
            assert_eq!(
                savings_bytes, expected_savings,
                "Savings calculation incorrect for original={}, new={}",
                original, new
            );
        } else {
            panic!(
                "Expected Pass result for original={}, new={}",
                original, new
            );
        }
    }
}
