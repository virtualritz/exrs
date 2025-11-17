//! ZIP byte reconstruction for DWA DC data processing.
//!
//! Based on OpenEXR's internal_zip.c

/// Apply delta decoding to a buffer.
/// Each byte is reconstructed as: buf[i] = buf[i-1] + buf[i] - 128
fn reconstruct(buf: &mut [u8]) {
    if buf.len() <= 1 {
        return;
    }

    for i in 1..buf.len() {
        // delta decode: t[i] = t[i-1] + t[i] - 128
        let prev = buf[i - 1];
        let curr = buf[i];
        buf[i] = prev.wrapping_add(curr).wrapping_sub(128);
    }
}

/// Interleave bytes from a deinterleaved buffer.
/// Input buffer is split in half: first half contains even indices,
/// second half contains odd indices.
fn interleave(out: &mut [u8], source: &[u8]) {
    let out_size = out.len();
    let mid = (out_size + 1) / 2;

    let t1 = &source[..mid];
    let t2 = &source[mid..];

    let mut i = 0;
    let mut idx1 = 0;
    let mut idx2 = 0;

    while i < out_size {
        if i < out_size {
            out[i] = t1[idx1];
            idx1 += 1;
            i += 1;
        }

        if i < out_size {
            out[i] = t2[idx2];
            idx2 += 1;
            i += 1;
        }
    }
}

/// Reconstruct bytes from ZIP-compressed data.
/// Applies delta decoding and byte interleaving.
///
/// This mirrors OpenEXR's internal_zip_reconstruct_bytes:
/// 1. Applies delta decoding (reconstruct)
/// 2. Applies byte interleaving
///
/// The `source` buffer is modified in-place during reconstruction,
/// then interleaved into `out`.
pub fn zip_reconstruct_bytes(out: &mut [u8], source: &mut [u8]) {
    assert_eq!(out.len(), source.len(), "Output and source buffers must have the same length");

    // Step 1: Apply delta decoding (reconstruct)
    reconstruct(source);

    // Step 2: Interleave bytes
    interleave(out, source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstruct() {
        // Test delta decoding
        let mut buf = vec![10u8, 130, 128, 129];
        reconstruct(&mut buf);

        // buf[0] = 10 (unchanged)
        // buf[1] = 10 + 130 - 128 = 12
        // buf[2] = 12 + 128 - 128 = 12
        // buf[3] = 12 + 129 - 128 = 13
        assert_eq!(buf, vec![10, 12, 12, 13]);
    }

    #[test]
    fn test_interleave() {
        // Test byte interleaving
        // Input: [a, b, c | d, e, f]
        // Output: [a, d, b, e, c, f]
        let source = vec![10, 20, 30, 40, 50, 60];
        let mut out = vec![0; 6];

        interleave(&mut out, &source);
        assert_eq!(out, vec![10, 40, 20, 50, 30, 60]);
    }

    #[test]
    fn test_interleave_odd_length() {
        // Test with odd length
        // Input: [a, b, c | d, e]
        // Output: [a, d, b, e, c]
        let source = vec![10, 20, 30, 40, 50];
        let mut out = vec![0; 5];

        interleave(&mut out, &source);
        assert_eq!(out, vec![10, 40, 20, 50, 30]);
    }

    #[test]
    fn test_zip_reconstruct_bytes() {
        // Combined test
        let mut source = vec![10, 130, 128, 129, 128, 128];
        let mut out = vec![0; 6];

        zip_reconstruct_bytes(&mut out, &mut source);

        // After reconstruct: [10, 12, 12, 13, 13, 13]
        // After interleave with mid=3: [10, 13, 12, 13, 12, 13]
        assert_eq!(out, vec![10, 13, 12, 13, 12, 13]);
    }
}
