//! Tests for HTJ2K compression framework
//!
//! Note: These tests verify the framework is correctly wired up,
//! but do not test actual compression/decompression since that
//! functionality is not yet implemented.

use exr::prelude::*;
use exr::prelude::pixel_vec::PixelVec;

#[test]
fn test_htj2k_enum_variants() {
    // Verify HTJ2K enum variants exist and have correct properties
    use exr::compression::Compression;

    let htj2k32 = Compression::HTJ2K32(None);
    let htj2k256 = Compression::HTJ2K256(Some(0.01));

    // Check scan lines per block
    assert_eq!(htj2k32.scan_lines_per_block(), 32);
    assert_eq!(htj2k256.scan_lines_per_block(), 256);

    // Check deep data support (HTJ2K doesn't support deep data)
    assert!(!htj2k32.supports_deep_data());
    assert!(!htj2k256.supports_deep_data());

    // Check lossiness (HTJ2K is lossy)
    assert!(!htj2k32.is_lossless_for(exr::meta::attribute::SampleType::F32));
    assert!(!htj2k32.is_lossless_for(exr::meta::attribute::SampleType::F16));
    assert!(!htj2k256.is_lossless_for(exr::meta::attribute::SampleType::U32));

    // Check that it may lose data
    assert!(htj2k32.may_loose_data());
    assert!(htj2k256.may_loose_data());

    // Check NaN support
    assert!(htj2k32.supports_nan());
    assert!(htj2k256.supports_nan());
    assert!(htj2k32.preserves_nan_bits());
    assert!(htj2k256.preserves_nan_bits());
}

#[test]
fn test_htj2k_display() {
    use exr::compression::Compression;

    let htj2k32 = Compression::HTJ2K32(None);
    let htj2k256 = Compression::HTJ2K256(Some(0.01));

    // Check Display trait implementation
    assert_eq!(format!("{}", htj2k32), "htj2k 32 compression");
    assert_eq!(format!("{}", htj2k256), "htj2k 256 compression");
}

#[test]
fn test_htj2k_serialization() {
    use exr::compression::Compression;
    
    // Test that HTJ2K variants can be serialized
    let htj2k32 = Compression::HTJ2K32(None);
    let htj2k256 = Compression::HTJ2K256(Some(0.05));

    // Serialize to bytes
    let mut buffer32 = Vec::new();
    htj2k32.write(&mut buffer32).unwrap();
    assert_eq!(buffer32, vec![11]); // HTJ2K32 should be byte ID 11

    let mut buffer256 = Vec::new();
    htj2k256.write(&mut buffer256).unwrap();
    assert_eq!(buffer256, vec![10]); // HTJ2K256 should be byte ID 10

    // Deserialize from bytes
    let mut cursor32 = std::io::Cursor::new(&buffer32[..]);
    let decoded32 = Compression::read(&mut cursor32).unwrap();
    assert_eq!(decoded32, Compression::HTJ2K32(None)); // Quality is not serialized

    let mut cursor256 = std::io::Cursor::new(&buffer256[..]);
    let decoded256 = Compression::read(&mut cursor256).unwrap();
    assert_eq!(decoded256, Compression::HTJ2K256(None)); // Quality is not serialized
}

#[cfg(not(feature = "htj2k"))]
#[test]
fn test_htj2k_disabled_without_feature() {
    // When htj2k feature is disabled, attempting to use it should give a clear error
    use exr::prelude::*;
    use exr::image::write::WritableImage;

    // Create a simple RGBA image
    let pixels = vec![(1.0_f32, 1.0_f32, 1.0_f32, 1.0_f32); 16 * 16];

    let image = Image::from_encoded_channels(
        (16, 16),
        Encoding {
            compression: exr::compression::Compression::HTJ2K32(None),
            ..Default::default()
        },
        SpecificChannels::rgba(
            PixelVec::new(Vec2(16, 16), pixels)
        )
    );

    // Try to write with HTJ2K compression
    let result = image.write().to_buffered(std::io::Cursor::new(Vec::new()));

    // Should fail with appropriate error message
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("htj2k") &&
        (error_msg.contains("not available") ||
         error_msg.contains("not yet fully implemented") ||
         error_msg.contains("cannot be compressed")),
        "Expected HTJ2K error, got: {}",
        error_msg
    );
}

#[cfg(feature = "htj2k")]
#[test]
fn test_htj2k_enabled_returns_not_implemented() {
    // When htj2k feature is enabled, it should return "not yet fully implemented"
    use exr::prelude::*;
    use exr::image::write::WritableImage;

    // Create a simple RGBA image
    let pixels = vec![(1.0_f32, 1.0_f32, 1.0_f32, 1.0_f32); 16 * 16];

    let image = Image::from_encoded_channels(
        (16, 16),
        Encoding {
            compression: exr::compression::Compression::HTJ2K32(None),
            ..Default::default()
        },
        SpecificChannels::rgba(
            PixelVec::new(Vec2(16, 16), pixels)
        )
    );

    // Try to write with HTJ2K compression
    let result = image.write().to_buffered(std::io::Cursor::new(Vec::new()));

    // Should fail with "not yet fully implemented" or "cannot be compressed" error
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not yet fully implemented") || error_msg.contains("cannot be compressed"),
        "Expected HTJ2K compression error, got: {}",
        error_msg
    );
}

#[cfg(feature = "htj2k")]
#[test]
fn test_htj2k_quality_parameters() {
    use exr::compression::Compression;

    // Test different quality parameters
    let default_quality = Compression::HTJ2K32(None);
    let low_quality = Compression::HTJ2K32(Some(0.1));   // More compression
    let high_quality = Compression::HTJ2K32(Some(0.001)); // Less compression

    // All should have same block size
    assert_eq!(default_quality.scan_lines_per_block(), 32);
    assert_eq!(low_quality.scan_lines_per_block(), 32);
    assert_eq!(high_quality.scan_lines_per_block(), 32);

    // But different quality values (when implemented, these would affect compression ratio)
    match default_quality {
        Compression::HTJ2K32(None) => {},
        _ => panic!("Expected HTJ2K32(None)"),
    }

    match low_quality {
        Compression::HTJ2K32(Some(q)) if (q - 0.1).abs() < 0.0001 => {},
        _ => panic!("Expected HTJ2K32(Some(0.1))"),
    }

    match high_quality {
        Compression::HTJ2K32(Some(q)) if (q - 0.001).abs() < 0.00001 => {},
        _ => panic!("Expected HTJ2K32(Some(0.001))"),
    }
}

// This test demonstrates what would need to be done for full implementation
#[cfg(feature = "htj2k")]
#[test]
#[ignore] // Ignored because compression is not yet implemented
fn test_htj2k_roundtrip_when_implemented() {
    // This test shows what a full roundtrip test would look like
    // Once compression/decompression is implemented, remove #[ignore]

    use exr::prelude::*;

    // Load a test image
    let original = exr::image::read::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .all_layers()
        .all_attributes()
        .from_file("tests/images/valid/custom/compression_methods/f32/uncompressed.exr")
        .expect("Failed to load test image");

    // Compress with HTJ2K
    // Note: This would require re-encoding the image with HTJ2K compression
    // For now, this is a placeholder showing the intended API
    let mut compressed_buffer = Vec::new();
    // original.write().to_buffered(...) would fail since original uses a different compression
    // This demonstrates where the compression would happen

    // Decompress
    let decompressed = exr::image::read::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .all_layers()
        .all_attributes()
        .from_buffered(std::io::Cursor::new(&compressed_buffer))
        .expect("Failed to decompress");

    // Verify metadata matches
    assert_eq!(original.layer_data.len(), decompressed.layer_data.len());

    // For lossy compression, we would check that the difference is within acceptable bounds
    // rather than exact equality
    println!("Compressed size: {} bytes", compressed_buffer.len());
    println!("Original would have been approximately: {} bytes",
             original.layer_data[0].size.0 * original.layer_data[0].size.1 * 4 * 4); // Rough estimate
}
