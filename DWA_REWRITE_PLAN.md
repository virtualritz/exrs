# DWA Decompression Rewrite Plan

## Problem Summary

All DWAA/DWAB pixels decompress as 0.0 instead of expected values (e.g., RGB~0.02-0.03, Alpha=1.0).

**Root Cause**: Fundamental architectural mismatch with OpenEXR reference implementation.

## Completed Work

### ✅ Infrastructure (Committed: e7e719a)
- **src/compression/dwa/zip.rs**: ZIP byte reconstruction (`zip_reconstruct_bytes`)
  - Delta decoding: `buf[i] = buf[i-1] + buf[i] - 128`
  - Byte interleaving: splits buffer in half, interleaves even/odd bytes
- **src/compression/dwa/lut.rs**: Exact OpenEXR toLinear LUT (65536-entry u16→u16 table)
- **Module integration**: Added to dwa.rs mod declarations

## Core Issues

### 1. DC Data Not Reconstructed
**Location**: src/compression/dwa.rs:103-111
```rust
let dc_data = if header.dc_compressed_size > 0 {
    let compressed = read_bytes(&mut reader, header.dc_compressed_size)?;
    let decompressed = decompress_zip(&compressed, header.dc_uncompressed_size * 2)?;
    // TODO: Apply byte-delta decoding (zip_reconstruct_bytes from OpenEXR)
    decompressed
}
```

**Fix**: Must apply `zip::zip_reconstruct_bytes` after ZIP decompression:
```rust
let dc_data = if header.dc_compressed_size > 0 {
    let compressed = read_bytes(&mut reader, header.dc_compressed_size)?;
    let mut scratch = decompress_zip(&compressed, header.dc_uncompressed_size * 2)?;
    let mut reconstructed = vec![0u8; scratch.len()];
    zip::zip_reconstruct_bytes(&mut reconstructed, &mut scratch);
    reconstructed
} else {
    Vec::new()
};
```

### 2. Channel-by-Channel Processing (WRONG Architecture)
**Location**: src/compression/dwa.rs:127-170

Current flow:
```
For each channel:
  decode_lossy_dct_channel() → Vec<f32>
Apply inverse CSC
For each channel:
  Write channel data sequentially to output
```

**Problem**:
- Processes whole channels into f32 buffers
- Writes channels sequentially (wrong interleaving)
- Applies transforms multiple times (double rounding)
- Values become too small (~0.0001) and round to zero

### 3. OpenEXR Architecture (CORRECT)
**Reference**: internal_dwa_decoder.h:288-740

```
For each 8-row block:
  For each 8x8 DCT block in row:
    Read DC (u16) + AC (u16[63]) coefficients
    Un-zigzag to normal order
    Apply inverse DCT → spatial values (f32)
    Convert to half-float (u16 bit pattern)
    Store in rowBlock[comp][blockx * 64]

  Apply inverse CSC if RGB triplet (operates on rowBlock)

  For each row in block:
    For each channel:
      Read 8 values from rowBlock[comp][(y & 0x7) * 8]
      Apply toLinear LUT: linear_u16 = TO_LINEAR_LUT[nonlinear_u16]
      Write linear u16 values to channel row
```

**Key Points**:
- Data stays as u16 (half bits) after IDCT
- toLinear LUT applied exactly once during row write
- Rows assembled in proper channel-interleaved order
- RLE channels processed per-row, not as monolithic blocks

## Required Changes

### Phase 1: Fix DC Reconstruction ✅ (Infrastructure Ready)
Apply `zip::zip_reconstruct_bytes` to DC data (5-line fix at dwa.rs:103-111)

### Phase 2: Rewrite Core Decompression Loop (MAJOR)
**Target**: src/compression/dwa.rs:127-227 (entire decompression logic)

**New Architecture**:
```rust
// Per-channel state tracking
struct ChannelDecodeState {
    scheme: CompressionScheme,
    resolution: Vec2<usize>,
    sample_type: SampleType,
    bytes_per_sample: usize,
    // Cursors for streaming data
    lossy_dct_state: Option<LossyDctState>,
    rle_cursor: usize,
    unknown_cursor: usize,
}

struct LossyDctState {
    dc_reader: Cursor<&[u8]>,
    ac_reader: Cursor<&[u8]>,
    row_block: [u16; 64], // Decoded DCT block in half format
}

// Main decompression loop
for scanline in rectangle.position.y()..rectangle.end.y() {
    for channel_idx in 0..channels.len() {
        let state = &mut channel_states[channel_idx];

        match state.scheme {
            LossyDct => {
                // Decode 8x8 blocks for this row
                // Keep as u16 throughout
                decode_dct_row(state, scanline, &ac_data, &dc_data);
            }
            Rle => {
                // Read from rle_data cursor
                read_rle_row(state, &rle_data);
            }
            Unknown => {
                // Read from unknown_data cursor
                read_unknown_row(state, &unknown_data);
            }
        }

        // Apply toLinear LUT and write to output
        write_channel_row(output, state, scanline, &lut::TO_LINEAR_LUT);
    }
}
```

### Phase 3: Fix AC/DC Coefficient Handling
**Location**: src/compression/dwa.rs:256-311 (`decode_lossy_dct_channel`)

**Current Issues**:
- Treats DC/AC as f16, converts to f32 → correct interpretation, wrong type
- Uses `INVERSE_ZIGZAG_ORDER` incorrectly

**Fix** (following internal_dwa_decoder.h:400-470):
```rust
// Pre-zero 64-element block
let mut halfZigBlock = [0u16; 64];

// DC coefficient (zigzag position 0)
let dc_bits = read_u16_le(dc_reader)?;
halfZigBlock[0] = dc_bits;

// AC coefficients (zigzag positions 1-63)
let mut dct_comp = 1;
while dct_comp < 64 {
    let val = read_u16_le(ac_reader)?;
    if (val & 0xff00) == 0xff00 {
        let count = (val & 0xff) as usize;
        dct_comp += if count == 0 { 64 } else { count }; // count=0 = end of block
    } else {
        halfZigBlock[dct_comp] = val;
        last_non_zero = dct_comp;
        dct_comp += 1;
    }
}

// Un-zigzag: halfZigBlock is IN zigzag order, convert to normal order
let mut dct_coeffs = [0.0f32; 64];
for i in 0..64 {
    dct_coeffs[i] = f16::from_bits(halfZigBlock[i]).to_f32();
}

// Apply inverse DCT
let spatial_f32 = inverse_dct_8x8(&dct_coeffs, last_non_zero);

// Convert to half (u16 bit pattern) - NO toLinear yet!
let spatial_u16: [u16; 64] = spatial_f32.map(|v| f16::from_f32(v).to_bits());
```

### Phase 4: Apply toLinear LUT During Write
**Location**: New function in rewritten decompression

```rust
fn write_channel_row(
    output: &mut [u8],
    channel_state: &ChannelDecodeState,
    scanline: usize,
    to_linear_lut: &[u16; 65536],
) {
    let row_start = scanline * channel_state.resolution.x();
    let output_offset = channel_state.output_base + row_start * channel_state.bytes_per_sample;

    for x in 0..channel_state.resolution.x() {
        let nonlinear_u16 = channel_state.row_buffer[x];

        // Apply toLinear LUT: u16 → u16
        let linear_u16 = to_linear_lut[nonlinear_u16 as usize];

        // Write as little-endian bytes
        let bytes = linear_u16.to_le_bytes();
        output[output_offset + x * 2] = bytes[0];
        output[output_offset + x * 2 + 1] = bytes[1];
    }
}
```

## Implementation Strategy

### Quick Win (1 hour)
1. Apply DC ZIP reconstruction fix (Phase 1)
2. Test if DC values improve (may partially fix issue)

### Major Rewrite (6-8 hours)
3. Implement `ChannelDecodeState` struct
4. Rewrite main loop to process row-by-row (Phase 2)
5. Fix AC/DC handling to match reference (Phase 3)
6. Integrate toLinear LUT in write path (Phase 4)
7. Run tests, iterate until all 7 validations pass

## Testing

**Command**: `cargo test --test test_dwa -- --nocapture`

**Current State**: All 7 tests fail - pixels are 0.0
**Target State**: All 7 tests pass with lossy tolerance (epsilon=0.06, max_diff=0.1)

**Test Files**:
- tests/test_dwa.rs: 7 validation tests
- tests/images/valid/custom/compression_methods/{f16,f32}/dwaa.exr
- tests/images/valid/custom/compression_methods/{f16,f32}/dwab.exr

## Reference Files

**OpenEXR C Reference**:
- `/home/user/openexr-reference/src/lib/OpenEXRCore/internal_dwa_decoder.h:288-740` - Main decompression
- `/home/user/openexr-reference/src/lib/OpenEXRCore/internal_dwa_decoder.h:107-167` - AC RLE reading
- `/home/user/openexr-reference/src/lib/OpenEXRCore/internal_zip.c:130-238` - ZIP reconstruction
- `/home/user/openexr-reference/src/lib/OpenEXR/dwaLookups.cpp:296-347` - toLinear generation

**Rust Implementation**:
- src/compression/dwa.rs - Main decompression (NEEDS REWRITE)
- src/compression/dwa/zip.rs - ZIP reconstruction (READY)
- src/compression/dwa/lut.rs - toLinear LUT (READY)
- src/compression/dwa/dct.rs - Inverse DCT (OK, may need u16 variant)
- src/compression/dwa/nonlinear.rs - OLD approach (DELETE after rewrite)

## Branch Info

**Branch**: `claude/dwaa-dwab-compression-plan-011CV4cVygC8NG5HSSva11KV`
**Latest Commit**: e7e719a "Add ZIP reconstruction and OpenEXR toLinear LUT for DWA"
**Status**: ZIP/LUT infrastructure ready, awaiting decompression rewrite
