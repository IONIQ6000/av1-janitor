#[derive(Debug)]
pub enum SizeGateResult {
    Pass {
        savings_bytes: u64,
        compression_ratio: f64,
    },
    Fail {
        new_bytes: u64,
        threshold_bytes: u64,
    },
}

pub fn check_size_gate(original_bytes: u64, new_bytes: u64, max_ratio: f64) -> SizeGateResult {
    let threshold = (original_bytes as f64 * max_ratio) as u64;
    if new_bytes >= threshold {
        SizeGateResult::Fail {
            new_bytes,
            threshold_bytes: threshold,
        }
    } else {
        let savings = original_bytes - new_bytes;
        let ratio = (new_bytes as f64) / (original_bytes as f64);
        SizeGateResult::Pass {
            savings_bytes: savings,
            compression_ratio: ratio,
        }
    }
}
