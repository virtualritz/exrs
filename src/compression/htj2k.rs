//! HTJ2K (High-Throughput JPEG 2000) compression for EXR files.
//!
//! This compression method uses lossy JPEG 2000 codec which provides lossy compression
//! with quality control via rate parameter and resolution scalability.
//!
//! **Note**: This implementation uses standard JPEG 2000 (J2K) compression via numcodecs-jpeg2000.
//! True HTJ2K (with high-throughput block coder) requires OpenJPH bindings which don't exist yet.
//! The compression characteristics are very similar - same wavelet transforms, just different
//! block coding algorithm. True HTJ2K encoding support will be added when OpenJPH Rust bindings
//! or OpenJPEG encoding support becomes available.
//!
//! HTJ2K support is optional and requires the `htj2k` feature flag, which adds
//! C library dependencies via the numcodecs-jpeg2000 crate (which uses OpenJPEG).
//!
//! Based on: https://github.com/sandflow/lossy-j2k-exr

use super::*;
use crate::error::{Result, Error};

#[cfg(feature = "htj2k")]
use numcodecs_jpeg2000::{Jpeg2000Codec, Jpeg2000CompressionMode, Jpeg2000CodecError};

#[cfg(feature = "htj2k")]
use ndarray::Array;

#[cfg(feature = "htj2k")]
use numcodecs::Codec;

/// Compress pixel data using lossy JPEG 2000 compression.
///
/// The quality parameter controls the compression level. If None, uses a default quality.
/// The parameter represents the compression rate (e.g., 10.0 for 10x compression).
/// Higher values = more compression, lower quality.
#[cfg(feature = "htj2k")]
pub fn compress(
    _channels: &ChannelList,
    uncompressed_ne: ByteVec,
    rectangle: IntegerBounds,
    quality: Option<f32>,
) -> Result<ByteVec> {
    if uncompressed_ne.is_empty() {
        return Ok(Vec::new());
    }

    // Determine compression mode based on quality parameter
    // Default to 10x compression rate if no quality specified
    let compression_rate = quality.unwrap_or(10.0);
    let mode = if compression_rate == 0.0 {
        Jpeg2000CompressionMode::Lossless
    } else {
        Jpeg2000CompressionMode::Rate { rate: compression_rate }
    };

    let codec = Jpeg2000Codec { mode, version: Default::default() };

    let width = rectangle.size.0;
    let height = rectangle.size.1;
    let total_pixels = width * height;

    // Determine bytes per sample from the data size
    let bytes_per_sample = uncompressed_ne.len() / total_pixels;

    let result = if bytes_per_sample == 2 {
        // F16 or U16 data
        compress_u16_data(&codec, &uncompressed_ne, width, height)
    } else if bytes_per_sample == 4 {
        // F32 or U32 data
        compress_i32_data(&codec, &uncompressed_ne, width, height)
    } else {
        return Err(Error::invalid(format!(
            "Unsupported bytes per sample for JPEG 2000: {} (expected 2 or 4)",
            bytes_per_sample
        )));
    };

    result.map_err(|e| Error::invalid(format!("JPEG 2000 compression failed: {}", e)))
}

#[cfg(feature = "htj2k")]
fn compress_u16_data(
    codec: &Jpeg2000Codec,
    data: &[u8],
    width: usize,
    height: usize,
) -> std::result::Result<Vec<u8>, numcodecs_jpeg2000::Jpeg2000CodecError> {
    use numcodecs::AnyCowArray;
    use lebe::io::ReadPrimitive;

    // Convert bytes to u16 array
    let mut u16_data = Vec::with_capacity(width * height);
    let mut reader = data;

    for _ in 0..(width * height) {
        let value = u16::read_from_native_endian(&mut reader)
            .map_err(|_| numcodecs_jpeg2000::Jpeg2000CodecError::UnsupportedDtype(
                numcodecs::AnyArrayDType::U16
            ))?;
        u16_data.push(value);
    }

    // Create 2D array
    let array = Array::from_shape_vec((height, width), u16_data)
        .map_err(|_| Jpeg2000CodecError::UnsupportedDtype(numcodecs::AnyArrayDType::U16))?;

    // Encode
    let encoded = codec.encode(AnyCowArray::U16(array.into_dyn().into()))?;

    // Extract bytes
    Ok(encoded.as_bytes().to_vec())
}

#[cfg(feature = "htj2k")]
fn compress_i32_data(
    codec: &Jpeg2000Codec,
    data: &[u8],
    width: usize,
    height: usize,
) -> std::result::Result<Vec<u8>, numcodecs_jpeg2000::Jpeg2000CodecError> {
    use numcodecs::AnyCowArray;
    use lebe::io::ReadPrimitive;

    // Convert bytes to i32 array (reinterpret f32 as i32 bit pattern)
    let mut i32_data = Vec::with_capacity(width * height);
    let mut reader = data;

    for _ in 0..(width * height) {
        let value = u32::read_from_native_endian(&mut reader)
            .map_err(|_| numcodecs_jpeg2000::Jpeg2000CodecError::UnsupportedDtype(
                numcodecs::AnyArrayDType::I32
            ))?;
        // Reinterpret as signed for JPEG 2000
        i32_data.push(value as i32);
    }

    // Create 2D array
    let array = Array::from_shape_vec((height, width), i32_data)
        .map_err(|_| Jpeg2000CodecError::UnsupportedDtype(numcodecs::AnyArrayDType::I32))?;

    // Encode
    let encoded = codec.encode(AnyCowArray::I32(array.into_dyn().into()))?;

    // Extract bytes
    Ok(encoded.as_bytes().to_vec())
}

/// Decompress JPEG 2000 compressed pixel data.
///
/// Supports resolution level skipping for progressive decoding (future enhancement).
#[cfg(feature = "htj2k")]
pub fn decompress(
    _channels: &ChannelList,
    compressed_le: ByteVec,
    _rectangle: IntegerBounds,
    expected_byte_size: usize,
    _pedantic: bool,
    _resolution_level: Option<u32>,
) -> Result<ByteVec> {
    if compressed_le.is_empty() {
        return Ok(Vec::new());
    }

    // Create a default codec for decompression (mode doesn't matter for decoding)
    let codec = Jpeg2000Codec {
        mode: Jpeg2000CompressionMode::Lossless,
        version: Default::default(),
    };

    // Decode the data
    use numcodecs::AnyCowArray;
    let compressed_array = Array::from_shape_vec(compressed_le.len(), compressed_le.to_vec())
        .map_err(|e| Error::invalid(format!("Failed to create array from compressed data: {}", e)))?;

    let decoded = codec.decode(AnyCowArray::U8(compressed_array.into_dyn().into()))
        .map_err(|e| Error::invalid(format!("JPEG 2000 decompression failed: {}", e)))?;

    // Convert decoded data back to bytes
    let result_bytes = decoded.as_bytes().to_vec();

    // Verify size matches expected
    if result_bytes.len() != expected_byte_size {
        return Err(Error::invalid(format!(
            "Decompressed size mismatch: expected {}, got {}",
            expected_byte_size,
            result_bytes.len()
        )));
    }

    Ok(result_bytes)
}

/// Stub implementation when htj2k feature is not enabled.
#[cfg(not(feature = "htj2k"))]
pub fn compress(
    _channels: &ChannelList,
    _uncompressed_ne: ByteVec,
    _rectangle: IntegerBounds,
    _quality: Option<f32>,
) -> Result<ByteVec> {
    Err(Error::unsupported(
        "HTJ2K compression is not available. \
         Enable the 'htj2k' feature flag to use HTJ2K compression. \
         Note: This adds C library dependencies via the numcodecs-jpeg2000 crate."
    ))
}

/// Stub implementation when htj2k feature is not enabled.
#[cfg(not(feature = "htj2k"))]
pub fn decompress(
    _channels: &ChannelList,
    _compressed_le: ByteVec,
    _rectangle: IntegerBounds,
    _expected_byte_size: usize,
    _pedantic: bool,
    _resolution_level: Option<u32>,
) -> Result<ByteVec> {
    Err(Error::unsupported(
        "HTJ2K decompression is not available. \
         Enable the 'htj2k' feature flag to use HTJ2K compression. \
         Note: This adds C library dependencies via the numcodecs-jpeg2000 crate."
    ))
}
