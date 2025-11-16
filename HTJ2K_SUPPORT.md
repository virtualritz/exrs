# HTJ2K Support in exrs

This document describes the HTJ2K (High-Throughput JPEG 2000) compression support added to the exrs library.

## Overview

HTJ2K is a lossy compression format for EXR files that provides:
- High compression ratios with quality control
- Fast encoding/decoding performance
- Resolution scalability for progressive image loading

This implementation is based on the [lossy-j2k-exr](https://github.com/sandflow/lossy-j2k-exr) project.

## Status

⚠️ **Current Implementation Status: Framework Only**

The HTJ2K support is currently implemented as a framework with the following structure in place:
- `HTJ2K32` - 32 scan line blocks
- `HTJ2K256` - 256 scan line blocks (more efficient for full-frame operations)

However, the actual compression/decompression functionality is **not yet fully implemented**. The framework returns appropriate error messages indicating that the feature requires additional implementation work.

## Enabling HTJ2K Support

HTJ2K support is available as an optional feature that requires C library dependencies via the `jpeg2k` crate.

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

Both `HTJ2K32` and `HTJ2K256` accept an optional quality parameter (QStep):

```rust
Compression::HTJ2K32(None)          // Default quality
Compression::HTJ2K32(Some(0.01))    // Custom quality (lower = better quality)
Compression::HTJ2K256(Some(0.05))   // Higher value = more compression
```

According to the HTJ2K specification, the quality parameter should be larger than `1/2^(sample_depth)`.

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

1. **Module**: `src/compression/htj2k.rs` - HTJ2K compression/decompression functions
2. **Enum Variants**: `Compression::HTJ2K32(Option<f32>)` and `Compression::HTJ2K256(Option<f32>)`
3. **Feature Flag**: `htj2k` - Optional feature for enabling HTJ2K support
4. **Dependencies**: `jpeg2k` crate (wraps OpenJPEG with HTJ2K support)

## Future Work

To complete the HTJ2K implementation, the following work is needed:

1. **Data Conversion**: Implement conversion between EXR channel data format and JPEG 2000 image format
2. **Encoding**: Integrate with jpeg2k crate's encoding API with proper quality parameters
3. **Decoding**: Integrate with jpeg2k crate's decoding API with resolution level support
4. **Testing**: Add comprehensive tests with sample HTJ2K-compressed EXR files
5. **Pure Rust**: Consider migrating to a pure Rust HTJ2K implementation when available (e.g., [htj2k-rs](https://gitlab.com/wg1/htj2k-rs))

## Without the htj2k Feature

When the `htj2k` feature is not enabled, attempting to use HTJ2K compression will return a clear error message:

```
HTJ2K compression is not available. Enable the 'htj2k' feature flag to use HTJ2K compression.
Note: This adds C library dependencies via the jpeg2k crate.
```

## References

- [lossy-j2k-exr GitHub Repository](https://github.com/sandflow/lossy-j2k-exr)
- [OpenJPH - Open-source HTJ2K Implementation](https://github.com/aous72/OpenJPH)
- [HTJ2K Specification](https://jpeg.org/jpeg2000/htj2k.html)
- [OpenEXR Documentation](https://openexr.com/)

## License

The HTJ2K support follows the same BSD-3-Clause license as the exrs library.
