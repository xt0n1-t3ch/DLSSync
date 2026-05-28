/// Shannon entropy of a byte slice in bits/byte (0.0-8.0). Packed or encrypted
/// data trends above 7.5; normal compiled code sits in 4.0-6.5. Empty input is
/// 0.0.
pub fn shannon(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let len = bytes.len() as f64;
    let mut entropy = 0.0;
    for &c in counts.iter() {
        if c == 0 {
            continue;
        }
        let p = c as f64 / len;
        entropy -= p * p.log2();
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(shannon(&[]), 0.0);
    }

    #[test]
    fn uniform_single_byte_is_zero() {
        assert_eq!(shannon(&[0x41; 4096]), 0.0);
    }

    #[test]
    fn full_byte_range_is_eight() {
        let all: Vec<u8> = (0..=255u8).cycle().take(256 * 16).collect();
        let e = shannon(&all);
        assert!((e - 8.0).abs() < 1e-9, "expected ~8.0, got {e}");
    }

    #[test]
    fn two_equiprobable_symbols_is_one() {
        let mut v = vec![0u8; 2048];
        for (i, b) in v.iter_mut().enumerate() {
            *b = if i % 2 == 0 { 0x00 } else { 0xFF };
        }
        let e = shannon(&v);
        assert!((e - 1.0).abs() < 1e-9, "expected ~1.0, got {e}");
    }
}
