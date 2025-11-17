# HTJ2K Support in exrs

This document describes the HTJ2K (High-Throughput JPEG 2000) compression support added to the exrs library.

## Overview

HTJ2K is a lossy compression format for EXR files that provides:
- High compression ratios with quality control
- Fast encoding/decoding performance
- Resolution scalability for progressive image loading

This implementation is based on the [lossy-j2k-exr](https://github.com/sandflow/lossy-j2k-exr) project.

## Status

✅ **Current Implementation Status: WORKING**

The HTJ2K support is fully implemented and functional with the following features:
- `HTJ2K32` - 32 scan line blocks
- `HTJ2K256` - 256 scan line blocks (more efficient for full-frame operations)
- Working lossy JPEG 2000 compression/decompression via numcodecs-jpeg2000
- Support for U16/F16 and I32/F32 data types
- Configurable compression rates and lossless mode

**Technical Note**: This implementation uses standard JPEG 2000 (J2K) via OpenJPEG, not true HTJ2K with the high-throughput block coder. The compression characteristics are very similar (same wavelet transforms), just with a different block coding algorithm. True HTJ2K encoding will be available when OpenJPH Rust bindings or OpenJPEG HTJ2K encoding support becomes available. OpenJPEG 2.5+ already has HTJ2K *decoding* support.

## Enabling HTJ2K Support

HTJ2K support is available as an optional feature that requires C library dependencies via the `numcodecs-jpeg2000` crate (which provides OpenJPEG bindings).

### Add to Cargo.toml

```toml
[dependencies]
exr = { version = "1.74", features = ["htj2k"] }
```

### Usage Example

```rust
use exr::prelude::*;
use exr::compression::Compression;

// Create an image with HTJ2K compression
let image = // ... your image data
    .with_encoding(
        Compression::HTJ2K256(Some(0.01)) // Quality parameter (QStep)
    );

// Write the image
image.write_to_file("output.exr", WriteOptions::default())?;
```

## Compression Parameters

Both `HTJ2K32` and `HTJ2K256` accept an optional compression rate parameter:

```rust
Compression::HTJ2K32(None)          // Default 10x compression rate
Compression::HTJ2K32(Some(10.0))    // 10x compression (moderate quality loss)
Compression::HTJ2K32(Some(5.0))     // 5x compression (less quality loss)
Compression::HTJ2K256(Some(0.0))    // Lossless compression
```

**Parameter Meaning**:
- `None`: Uses default 10x compression rate
- `Some(rate)`: Compression rate (higher = more compression, lower quality)
- `Some(0.0)`: Lossless mode (no quality loss)

## Compression Characteristics

### HTJ2K32
- **Block Size**: 32 scan lines
- **Best For**: Partial frame access, streaming
- **Lossiness**: Lossy for all sample types
- **Deep Data**: Not supported
- **NaN Support**: Preserves NaN values but may alter bit patterns

### HTJ2K256
- **Block Size**: 256 scan lines
- **Best For**: Full frame operations, maximum compression
- **Lossiness**: Lossy for all sample types
- **Deep Data**: Not supported
- **NaN Support**: Preserves NaN values but may alter bit patterns

## Implementation Details

The HTJ2K support integrates with the exrs compression system through:

1. **Module**: `src/compression/htj2k.rs` - JPEG 2000 compression/decompression functions
2. **Enum Variants**: `Compression::HTJ2K32(Option<f32>)` and `Compression::HTJ2K256(Option<f32>)`
3. **Feature Flag**: `htj2k` - Optional feature for enabling HTJ2K support
4. **Dependencies**:
   - `numcodecs-jpeg2000` - JPEG 2000 codec implementation
   - `numcodecs` - Codec API
   - `ndarray` - Multi-dimensional array support
   - `openjpeg-sys` (transitive) - OpenJPEG FFI bindings

### Data Type Support

- **U16/F16**: 2 bytes per sample - compressed as U16
- **I32/F32**: 4 bytes per sample - compressed as I32 (f32 reinterpreted as bits)

## Future Work

To enhance the HTJ2K implementation:

1. **True HTJ2K**: Migrate to OpenJPH bindings when available for true high-throughput block coding
2. **Resolution Scalability**: Implement resolution level skipping for progressive decoding
3. **Multi-Channel Optimization**: Optimize handling of multi-channel images (currently treats all channels as single image)
4. **Chroma Subsampling**: Add support for chroma subsampling in RGB images
5. **Pure Rust**: Consider migrating to a pure Rust HTJ2K implementation when available (e.g., [htj2k-rs](https://gitlab.com/wg1/htj2k-rs))
6. **Performance**: Profile and optimize compression/decompression speed

## Without the htj2k Feature

When the `htj2k` feature is not enabled, attempting to use HTJ2K compression will return a clear error message:

```
HTJ2K compression is not available. Enable the 'htj2k' feature flag to use HTJ2K compression.
Note: This adds C library dependencies via the numcodecs-jpeg2000 crate.
```

## Testing

The implementation includes comprehensive tests in `tests/htj2k_framework.rs`:
- Enum variant properties verification
- Serialization/deserialization
- Feature flag behavior
- Quality parameter handling

Run tests with:
```bash
cargo test --features htj2k --test htj2k_framework
```

## References

- [lossy-j2k-exr GitHub Repository](https://github.com/sandflow/lossy-j2k-exr)
- [OpenJPH - Open-source HTJ2K Implementation](https://github.com/aous72/OpenJPH)
- [HTJ2K Specification](https://jpeg.org/jpeg2000/htj2k.html)
- [OpenEXR Documentation](https://openexr.com/)

## License

The HTJ2K support follows the same BSD-3-Clause license as the exrs library.
