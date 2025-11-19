# Channel Subsampling API Plan for EXR Crate

## Executive Summary

This plan details the API additions needed to expose channel subsampling functionality to users of the exr crate. The implementation is **already complete internally** - subsampling works correctly for reading/writing, but lacks user-facing APIs. This plan adds both **low-level control** for advanced users and **high-level convenience** with sinc filter support.

## Current State

### ✅ Already Implemented (Internal)
- Metadata storage: `ChannelDescription::sampling: Vec2<usize>`
- Read path: `FlatSamplesReader` with subsampling-aware buffer allocation
- Write path: `FlatSamplesWriter` with subsampled buffer extraction
- Block iteration: `LineIndex::lines_in_block()` properly handles Y-line skipping and X-sample reduction
- Byte calculations: All compression methods account for subsampling
- Comprehensive test coverage in `tests/subsampling.rs`

### ❌ Missing (User-Facing)
- Public API for setting subsampling on channels
- Sinc filter implementation
- Reconstruction filter application during reads
- Downsampling filter application during writes
- Helper functions for common presets (4:2:0, 4:2:2, etc.)
- Builder pattern extensions for easy configuration

## API Design Philosophy

Following the crate's existing three-tier architecture:

1. **Layer 1 (High-Level)**: Simple functions with automatic sinc filtering
2. **Layer 2 (Mid-Level)**: Builder pattern with explicit filter control
3. **Layer 3 (Low-Level)**: Direct field access for maximum control

## Proposed API Additions

### Part 1: Low-Level API (Direct Control)

#### 1.1 AnyChannel Constructor with Subsampling
**Location**: `src/image/mod.rs`

```rust
impl<'s, SampleData: 's> AnyChannel<SampleData> {
    /// Create a new channel with custom subsampling.
    ///
    /// # Arguments
    /// * `name` - Channel name (e.g., "RY", "BY" for chroma)
    /// * `sample_data` - The pixel data
    /// * `sampling` - Subsampling rate (e.g., Vec2(2,2) for 4:2:0 chroma)
    ///
    /// # Constraints
    /// - Only works with flat, scan-line based images
    /// - Data window must be aligned to sampling boundaries
    /// - Will fail validation if used with tiled or deep images
    pub fn with_subsampling(
        name: impl Into<Text>,
        sample_data: SampleData,
        sampling: Vec2<usize>,
    ) -> Self
    where
        SampleData: WritableSamples<'s>,
    {
        let name: Text = name.into();

        AnyChannel {
            quantize_linearly: ChannelDescription::guess_quantization_linearity(&name),
            name,
            sample_data,
            sampling,
        }
    }

    /// Builder method to modify subsampling on an existing channel.
    pub fn set_subsampling(mut self, sampling: Vec2<usize>) -> Self {
        self.sampling = sampling;
        self
    }
}
```

#### 1.2 Common Subsampling Presets
**Location**: `src/image/mod.rs`

```rust
/// Common chroma subsampling patterns used in video and image compression.
pub mod subsampling {
    use crate::math::Vec2;

    /// 4:4:4 - No subsampling (full resolution for all channels)
    pub const NONE: Vec2<usize> = Vec2(1, 1);

    /// 4:2:2 - Horizontal 2x subsampling (common in professional video)
    pub const YUV_422: Vec2<usize> = Vec2(2, 1);

    /// 4:2:0 - 2x2 subsampling (common in consumer video, JPEG)
    pub const YUV_420: Vec2<usize> = Vec2(2, 2);

    /// 4:1:1 - Horizontal 4x subsampling (old DV format)
    pub const YUV_411: Vec2<usize> = Vec2(4, 1);
}
```

**Example Usage**:
```rust
use exr::prelude::*;
use exr::image::subsampling;

let y_channel = AnyChannel::new("Y", y_samples);
let u_channel = AnyChannel::with_subsampling("RY", u_samples, subsampling::YUV_420);
let v_channel = AnyChannel::with_subsampling("BY", v_samples, subsampling::YUV_420);
```

### Part 2: Sinc Filter Implementation

#### 2.1 Filter Module Structure
**Location**: `src/image/reconstruct_filter.rs` (new file)

```rust
//! Reconstruction and downsampling filters for subsampled channels.
//!
//! When reading subsampled channels, reconstruction filters upsample the data
//! to full resolution. When writing with subsampling, downsampling filters
//! reduce the resolution while preserving quality.

use crate::math::Vec2;

/// Trait for upsampling filters used when reading subsampled channels.
pub trait ReconstructionFilter: Send + Sync {
    /// Apply the filter to reconstruct a full-resolution sample.
    ///
    /// # Arguments
    /// * `subsampled_data` - The subsampled input buffer
    /// * `subsampled_size` - Dimensions of the subsampled data
    /// * `output_pos` - Position in full-resolution output
    /// * `sampling` - Subsampling rate
    ///
    /// # Returns
    /// The reconstructed sample value at the output position
    fn reconstruct(
        &self,
        subsampled_data: &[f32],
        subsampled_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32;
}

/// Trait for downsampling filters used when writing subsampled channels.
pub trait DownsamplingFilter: Send + Sync {
    /// Apply the filter to downsample full-resolution data.
    ///
    /// # Arguments
    /// * `full_data` - The full-resolution input buffer
    /// * `full_size` - Dimensions of the full-resolution data
    /// * `output_pos` - Position in subsampled output
    /// * `sampling` - Subsampling rate
    ///
    /// # Returns
    /// The downsampled sample value at the output position
    fn downsample(
        &self,
        full_data: &[f32],
        full_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32;
}

/// Sinc-based reconstruction filter (Lanczos-style windowed sinc).
///
/// Provides high-quality upsampling for subsampled channels.
/// This is the recommended filter for chroma upsampling.
#[derive(Debug, Clone, Copy)]
pub struct SincFilter {
    /// Number of lobes (2 or 3 typical, higher = sharper but more ringing)
    pub lobes: usize,
}

impl SincFilter {
    /// Create a new sinc filter with the specified number of lobes.
    ///
    /// Common values:
    /// - 2 lobes: Good balance of quality and performance (Lanczos2)
    /// - 3 lobes: Higher quality, more computation (Lanczos3)
    pub fn new(lobes: usize) -> Self {
        Self { lobes }
    }

    /// Standard 2-lobe Lanczos filter (recommended default)
    pub fn lanczos2() -> Self {
        Self { lobes: 2 }
    }

    /// High-quality 3-lobe Lanczos filter
    pub fn lanczos3() -> Self {
        Self { lobes: 3 }
    }

    /// Sinc function: sin(πx) / (πx)
    #[inline]
    fn sinc(x: f32) -> f32 {
        if x.abs() < 0.001 {
            1.0
        } else {
            let pi_x = std::f32::consts::PI * x;
            pi_x.sin() / pi_x
        }
    }

    /// Lanczos window function
    #[inline]
    fn lanczos(&self, x: f32) -> f32 {
        let a = self.lobes as f32;
        if x.abs() < a {
            Self::sinc(x) * Self::sinc(x / a)
        } else {
            0.0
        }
    }
}

impl ReconstructionFilter for SincFilter {
    fn reconstruct(
        &self,
        subsampled_data: &[f32],
        subsampled_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        // Map output position to subsampled coordinate space
        let sub_x = output_pos.x() as f32 / sampling.x() as f32;
        let sub_y = output_pos.y() as f32 / sampling.y() as f32;

        let radius = self.lobes;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        // Sample neighboring subsampled pixels
        let x_start = (sub_x - radius as f32).floor() as i32;
        let x_end = (sub_x + radius as f32).ceil() as i32;
        let y_start = (sub_y - radius as f32).floor() as i32;
        let y_end = (sub_y + radius as f32).ceil() as i32;

        for sy in y_start..=y_end {
            if sy < 0 || sy >= subsampled_size.y() as i32 {
                continue;
            }

            for sx in x_start..=x_end {
                if sx < 0 || sx >= subsampled_size.x() as i32 {
                    continue;
                }

                let dx = sub_x - sx as f32;
                let dy = sub_y - sy as f32;

                let weight_x = self.lanczos(dx);
                let weight_y = self.lanczos(dy);
                let weight = weight_x * weight_y;

                let idx = sy as usize * subsampled_size.x() + sx as usize;
                sum += subsampled_data[idx] * weight;
                weight_sum += weight;
            }
        }

        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            // Fallback to nearest neighbor
            let sx = sub_x.round().max(0.0).min((subsampled_size.x() - 1) as f32) as usize;
            let sy = sub_y.round().max(0.0).min((subsampled_size.y() - 1) as f32) as usize;
            subsampled_data[sy * subsampled_size.x() + sx]
        }
    }
}

impl DownsamplingFilter for SincFilter {
    fn downsample(
        &self,
        full_data: &[f32],
        full_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        // Map subsampled position back to full resolution
        let full_x = (output_pos.x() * sampling.x()) as f32 + (sampling.x() as f32 / 2.0);
        let full_y = (output_pos.y() * sampling.y()) as f32 + (sampling.y() as f32 / 2.0);

        // Use anti-aliasing kernel scaled by sampling rate
        let radius_x = self.lobes * sampling.x();
        let radius_y = self.lobes * sampling.y();

        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        let x_start = (full_x - radius_x as f32).floor() as i32;
        let x_end = (full_x + radius_x as f32).ceil() as i32;
        let y_start = (full_y - radius_y as f32).floor() as i32;
        let y_end = (full_y + radius_y as f32).ceil() as i32;

        for fy in y_start..=y_end {
            if fy < 0 || fy >= full_size.y() as i32 {
                continue;
            }

            for fx in x_start..=x_end {
                if fx < 0 || fx >= full_size.x() as i32 {
                    continue;
                }

                let dx = (full_x - fx as f32) / sampling.x() as f32;
                let dy = (full_y - fy as f32) / sampling.y() as f32;

                let weight_x = self.lanczos(dx);
                let weight_y = self.lanczos(dy);
                let weight = weight_x * weight_y;

                let idx = fy as usize * full_size.x() + fx as usize;
                sum += full_data[idx] * weight;
                weight_sum += weight;
            }
        }

        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            // Fallback to box filter (simple average)
            let mut box_sum = 0.0;
            let mut box_count = 0;

            let box_x_start = output_pos.x() * sampling.x();
            let box_x_end = (output_pos.x() + 1) * sampling.x();
            let box_y_start = output_pos.y() * sampling.y();
            let box_y_end = (output_pos.y() + 1) * sampling.y();

            for y in box_y_start..box_y_end {
                if y >= full_size.y() { break; }
                for x in box_x_start..box_x_end {
                    if x >= full_size.x() { break; }
                    box_sum += full_data[y * full_size.x() + x];
                    box_count += 1;
                }
            }

            if box_count > 0 {
                box_sum / box_count as f32
            } else {
                0.0
            }
        }
    }
}

/// Simple box filter (averaging) - faster but lower quality.
#[derive(Debug, Clone, Copy)]
pub struct BoxFilter;

impl ReconstructionFilter for BoxFilter {
    fn reconstruct(
        &self,
        subsampled_data: &[f32],
        subsampled_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        let sx = output_pos.x() / sampling.x();
        let sy = output_pos.y() / sampling.y();

        if sx < subsampled_size.x() && sy < subsampled_size.y() {
            subsampled_data[sy * subsampled_size.x() + sx]
        } else {
            0.0
        }
    }
}

impl DownsamplingFilter for BoxFilter {
    fn downsample(
        &self,
        full_data: &[f32],
        full_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        let x_start = output_pos.x() * sampling.x();
        let x_end = (output_pos.x() + 1) * sampling.x();
        let y_start = output_pos.y() * sampling.y();
        let y_end = (output_pos.y() + 1) * sampling.y();

        let mut sum = 0.0;
        let mut count = 0;

        for y in y_start..y_end {
            if y >= full_size.y() { break; }
            for x in x_start..x_end {
                if x >= full_size.x() { break; }
                sum += full_data[y * full_size.x() + x];
                count += 1;
            }
        }

        if count > 0 {
            sum / count as f32
        } else {
            0.0
        }
    }
}

/// Nearest-neighbor filter - fastest but lowest quality.
#[derive(Debug, Clone, Copy)]
pub struct NearestFilter;

impl ReconstructionFilter for NearestFilter {
    fn reconstruct(
        &self,
        subsampled_data: &[f32],
        subsampled_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        let sx = (output_pos.x() / sampling.x()).min(subsampled_size.x() - 1);
        let sy = (output_pos.y() / sampling.y()).min(subsampled_size.y() - 1);
        subsampled_data[sy * subsampled_size.x() + sx]
    }
}

impl DownsamplingFilter for NearestFilter {
    fn downsample(
        &self,
        full_data: &[f32],
        full_size: Vec2<usize>,
        output_pos: Vec2<usize>,
        sampling: Vec2<usize>,
    ) -> f32 {
        let fx = (output_pos.x() * sampling.x()).min(full_size.x() - 1);
        let fy = (output_pos.y() * sampling.y()).min(full_size.y() - 1);
        full_data[fy * full_size.x() + fx]
    }
}
```

### Part 3: High-Level API (Builder Pattern with Filters)

#### 3.1 Writer API Extension
**Location**: `src/image/write/mod.rs`

```rust
impl<'img, L, F> WriteImageWithOptions<'img, L, F> {
    /// Enable automatic subsampling with sinc filter during write.
    ///
    /// Channels with subsampling will be automatically downsampled from
    /// full-resolution data using the specified filter.
    ///
    /// # Example
    /// ```no_run
    /// use exr::prelude::*;
    /// use exr::image::reconstruct_filter::SincFilter;
    ///
    /// image.write()
    ///     .with_downsampling_filter(SincFilter::lanczos2())
    ///     .to_file("output.exr")
    ///     .unwrap();
    /// ```
    pub fn with_downsampling_filter<Filter>(
        self,
        filter: Filter,
    ) -> WriteImageWithOptions<'img, L, impl Fn(f64)>
    where
        Filter: DownsamplingFilter + 'static,
    {
        // Store filter in WriteImageWithOptions for use during write
        // Implementation would extend WriteImageWithOptions struct
        todo!("Integrate filter into write path")
    }
}
```

#### 3.2 Reader API Extension
**Location**: `src/image/read/image.rs`

```rust
impl<ChannelMode, LayerMode> ReadImage<ChannelMode, LayerMode> {
    /// Apply reconstruction filter when reading subsampled channels.
    ///
    /// This will upsample any subsampled channels to full resolution
    /// using the specified filter (e.g., sinc filter for high quality).
    ///
    /// # Example
    /// ```no_run
    /// use exr::prelude::*;
    /// use exr::image::reconstruct_filter::SincFilter;
    ///
    /// let image = read()
    ///     .no_deep_data()
    ///     .largest_resolution_level()
    ///     .all_channels()
    ///     .with_reconstruction_filter(SincFilter::lanczos3())
    ///     .all_layers()
    ///     .all_attributes()
    ///     .from_file("input.exr")
    ///     .unwrap();
    /// ```
    pub fn with_reconstruction_filter<Filter>(
        self,
        filter: Filter,
    ) -> ReadImage<ChannelMode, LayerMode>
    where
        Filter: ReconstructionFilter + 'static,
    {
        // Store filter for use during read
        todo!("Integrate filter into read path")
    }
}
```

### Part 4: Convenience Functions (Highest Level)

#### 4.1 Simple YUV 4:2:0 Write Function
**Location**: `src/image/write/mod.rs`

```rust
/// Write a YUV 4:2:0 subsampled image with automatic sinc filtering.
///
/// This is a convenience function for writing chroma-subsampled images,
/// commonly used in video and photography. The chroma channels (U, V) are
/// automatically downsampled to 1/4 the resolution (half width, half height)
/// using a high-quality sinc filter.
///
/// # Arguments
/// * `path` - Output file path
/// * `width` - Image width (must be even for 4:2:0)
/// * `height` - Image height (must be even for 4:2:0)
/// * `y_fn` - Function returning luma value for position (x, y)
/// * `u_fn` - Function returning U chroma value for position (x, y)
/// * `v_fn` - Function returning V chroma value for position (x, y)
///
/// # Example
/// ```no_run
/// use exr::prelude::*;
///
/// write_yuv420_file(
///     "output.exr",
///     1920,
///     1080,
///     |x, y| { /* luma */ 0.5 },
///     |x, y| { /* U chroma */ 0.0 },
///     |x, y| { /* V chroma */ 0.0 },
/// ).unwrap();
/// ```
pub fn write_yuv420_file<Y, U, V>(
    path: impl AsRef<std::path::Path>,
    width: usize,
    height: usize,
    y_fn: impl Sync + Fn(usize, usize) -> Y,
    u_fn: impl Sync + Fn(usize, usize) -> U,
    v_fn: impl Sync + Fn(usize, usize) -> V,
) -> UnitResult
where
    Y: IntoSample,
    U: IntoSample,
    V: IntoSample,
{
    use crate::image::reconstruct_filter::SincFilter;
    use crate::image::subsampling;

    // Validate dimensions
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Error::invalid("YUV 4:2:0 requires even width and height"));
    }

    // Create full-resolution samples for all channels
    let y_samples = create_samples(width, height, y_fn);
    let u_samples = create_samples(width, height, u_fn);
    let v_samples = create_samples(width, height, v_fn);

    // Create channels with appropriate subsampling
    let channels = AnyChannels::sort(smallvec![
        AnyChannel::new("Y", y_samples),
        AnyChannel::with_subsampling("RY", u_samples, subsampling::YUV_420),
        AnyChannel::with_subsampling("BY", v_samples, subsampling::YUV_420),
    ]);

    let layer = Layer::new(
        (width, height),
        LayerAttributes::named(""),
        Encoding::FAST_LOSSLESS,
        channels,
    );

    Image::from_layer(layer)
        .write()
        .with_downsampling_filter(SincFilter::lanczos2())
        .to_file(path)
}

/// Helper function to create sample data from a function
fn create_samples<T: IntoSample>(
    width: usize,
    height: usize,
    sample_fn: impl Fn(usize, usize) -> T,
) -> FlatSamples {
    // Implementation details...
    todo!()
}
```

#### 4.2 Simple YUV 4:2:2 Write Function
**Location**: `src/image/write/mod.rs`

```rust
/// Write a YUV 4:2:2 subsampled image with automatic sinc filtering.
///
/// Professional video format with horizontal chroma subsampling.
/// Chroma channels are half the width but full height.
pub fn write_yuv422_file<Y, U, V>(
    path: impl AsRef<std::path::Path>,
    width: usize,
    height: usize,
    y_fn: impl Sync + Fn(usize, usize) -> Y,
    u_fn: impl Sync + Fn(usize, usize) -> U,
    v_fn: impl Sync + Fn(usize, usize) -> V,
) -> UnitResult
where
    Y: IntoSample,
    U: IntoSample,
    V: IntoSample,
{
    use crate::image::reconstruct_filter::SincFilter;
    use crate::image::subsampling;

    if width % 2 != 0 {
        return Err(Error::invalid("YUV 4:2:2 requires even width"));
    }

    let y_samples = create_samples(width, height, y_fn);
    let u_samples = create_samples(width, height, u_fn);
    let v_samples = create_samples(width, height, v_fn);

    let channels = AnyChannels::sort(smallvec![
        AnyChannel::new("Y", y_samples),
        AnyChannel::with_subsampling("RY", u_samples, subsampling::YUV_422),
        AnyChannel::with_subsampling("BY", v_samples, subsampling::YUV_422),
    ]);

    let layer = Layer::new(
        (width, height),
        LayerAttributes::named(""),
        Encoding::FAST_LOSSLESS,
        channels,
    );

    Image::from_layer(layer)
        .write()
        .with_downsampling_filter(SincFilter::lanczos2())
        .to_file(path)
}
```

### Part 5: Documentation and Examples

#### 5.1 New Example: Write Subsampled Image
**Location**: `examples/9_write_yuv_subsampled.rs`

```rust
//! Write a YUV image with chroma subsampling (4:2:0)

extern crate exr;

fn main() {
    use exr::prelude::*;
    use exr::image::reconstruct_filter::SincFilter;
    use exr::image::subsampling;

    println!("Writing YUV 4:2:0 subsampled image...");

    let width = 1920;
    let height = 1080;

    // Method 1: High-level convenience function
    write_yuv420_file(
        "test_yuv420_simple.exr",
        width,
        height,
        |x, y| {
            // Luma (full resolution)
            (y as f32 / height as f32) as f16
        },
        |x, y| {
            // U chroma (will be subsampled to 960x540)
            (x as f32 / width as f32 - 0.5) as f16
        },
        |x, y| {
            // V chroma (will be subsampled to 960x540)
            0.0_f16
        },
    ).unwrap();

    println!("Method 1 complete: test_yuv420_simple.exr");

    // Method 2: Mid-level with explicit channel construction
    let y_data: Vec<f16> = (0..width * height)
        .map(|i| {
            let y = i / width;
            (y as f32 / height as f32) as f16
        })
        .collect();

    let u_data: Vec<f16> = (0..width * height)
        .map(|i| {
            let x = i % width;
            (x as f32 / width as f32 - 0.5) as f16
        })
        .collect();

    let v_data: Vec<f16> = vec![f16::ZERO; width * height];

    let channels = AnyChannels::sort(smallvec![
        AnyChannel::new("Y", FlatSamples::F16(y_data)),
        AnyChannel::with_subsampling(
            "RY",
            FlatSamples::F16(u_data),
            subsampling::YUV_420
        ),
        AnyChannel::with_subsampling(
            "BY",
            FlatSamples::F16(v_data),
            subsampling::YUV_420
        ),
    ]);

    let layer = Layer::new(
        (width, height),
        LayerAttributes::named(""),
        Encoding::FAST_LOSSLESS,
        channels,
    );

    Image::from_layer(layer)
        .write()
        .with_downsampling_filter(SincFilter::lanczos3())
        .to_file("test_yuv420_explicit.exr")
        .unwrap();

    println!("Method 2 complete: test_yuv420_explicit.exr");

    // Method 3: Low-level with direct field access
    let mut y_chan = AnyChannel::new("Y", FlatSamples::F16(vec![f16::ZERO; width * height]));
    let mut u_chan = AnyChannel::new("RY", FlatSamples::F16(vec![f16::ZERO; width * height]));
    let mut v_chan = AnyChannel::new("BY", FlatSamples::F16(vec![f16::ZERO; width * height]));

    // Directly set subsampling (low-level control)
    u_chan.sampling = Vec2(2, 2);
    v_chan.sampling = Vec2(2, 2);
    u_chan.quantize_linearly = true;
    v_chan.quantize_linearly = true;

    let channels = AnyChannels::sort(smallvec![y_chan, u_chan, v_chan]);

    let layer = Layer::new(
        (width, height),
        LayerAttributes::named(""),
        Encoding::FAST_LOSSLESS,
        channels,
    );

    Image::from_layer(layer)
        .write()
        .to_file("test_yuv420_lowlevel.exr")
        .unwrap();

    println!("Method 3 complete: test_yuv420_lowlevel.exr");
    println!("All methods complete!");
}
```

#### 5.2 New Example: Read Subsampled Image with Filter
**Location**: `examples/10_read_yuv_with_reconstruction.rs`

```rust
//! Read a YUV subsampled image and reconstruct full resolution with sinc filter

extern crate exr;

fn main() {
    use exr::prelude::*;
    use exr::image::reconstruct_filter::SincFilter;

    // Method 1: Automatic reconstruction with sinc filter
    let image: AnyImage = read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .with_reconstruction_filter(SincFilter::lanczos3())
        .all_layers()
        .all_attributes()
        .from_file("tests/images/valid/openexr/LuminanceChroma/Flowers.exr")
        .unwrap();

    println!("Read image with automatic upsampling:");
    println!("  Layers: {}", image.layer_data.len());

    for (i, layer) in image.layer_data.iter().enumerate() {
        println!("  Layer {}: {}x{}", i, layer.size.width(), layer.size.height());

        for channel in &layer.channel_data.list {
            println!("    Channel '{}': sampling={:?}", channel.name, channel.sampling);
        }
    }

    // Method 2: Read raw subsampled data (no reconstruction)
    let raw_image: AnyImage = read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        // No reconstruction filter - channels remain subsampled
        .all_layers()
        .all_attributes()
        .from_file("tests/images/valid/openexr/LuminanceChroma/Flowers.exr")
        .unwrap();

    println!("\nRead raw subsampled data:");
    for layer in &raw_image.layer_data {
        for channel in &layer.channel_data.list {
            if let FlatSamples::F16(data) = &get_channel_samples(channel) {
                println!("    Channel '{}': {} samples (subsampled)",
                    channel.name, data.len());
            }
        }
    }
}

fn get_channel_samples(channel: &AnyChannel<FlatSamples>) -> &FlatSamples {
    &channel.sample_data
}
```

## Implementation Phases

### Phase 1: Low-Level API (Highest Priority)
**Estimated effort**: 4-6 hours

1. Add `AnyChannel::with_subsampling()` constructor
2. Add `AnyChannel::set_subsampling()` builder method
3. Add `subsampling` module with common presets
4. Add tests demonstrating low-level API usage
5. Update documentation

**Why first**: Provides immediate value with minimal code changes. The sampling field is already public, this just adds convenience.

### Phase 2: Sinc Filter Implementation (Core Feature)
**Estimated effort**: 12-16 hours

1. Create `src/image/reconstruct_filter.rs` module
2. Implement `ReconstructionFilter` and `DownsamplingFilter` traits
3. Implement `SincFilter` with Lanczos windowing
4. Implement `BoxFilter` and `NearestFilter` for comparison
5. Add comprehensive unit tests for filters
6. Benchmark filter performance

**Why second**: Core functionality needed for high-level API. Self-contained module.

### Phase 3: Filter Integration (Medium Priority)
**Estimated effort**: 16-20 hours

1. Extend `FlatSamplesReader` to support reconstruction filters
2. Extend `FlatSamplesWriter` to support downsampling filters
3. Add filter options to `WriteImageWithOptions`
4. Add filter options to `ReadImage` builder
5. Update `SamplesReader` and `SamplesWriter` traits
6. Integration tests with real EXR files

**Why third**: Requires filter implementation. Most complex integration work.

### Phase 4: High-Level Convenience API (Nice to Have)
**Estimated effort**: 6-8 hours

1. Implement `write_yuv420_file()` function
2. Implement `write_yuv422_file()` function
3. Add `create_samples()` helper
4. Add validation for dimension requirements
5. Tests for convenience functions

**Why fourth**: Builds on all previous work. Provides best UX but not essential.

### Phase 5: Documentation & Examples (Final Polish)
**Estimated effort**: 8-10 hours

1. Write `examples/9_write_yuv_subsampled.rs`
2. Write `examples/10_read_yuv_with_reconstruction.rs`
3. Update crate-level documentation
4. Add module-level documentation for new modules
5. Update README with subsampling examples
6. Create benchmarks for filter performance

**Why last**: Demonstrates complete functionality. Essential for users but not for implementation.

## Testing Strategy

### Unit Tests
- Filter correctness (known input/output pairs)
- Edge cases (image boundaries, sampling factors)
- Performance benchmarks

### Integration Tests
- Round-trip tests with subsampling
- Read existing subsampled EXR files
- Write and verify subsampled files
- Filter quality comparison (PSNR, SSIM metrics)

### Test Files
Use existing OpenEXR test suite:
- `tests/images/valid/openexr/LuminanceChroma/Garden.exr`
- `tests/images/valid/openexr/LuminanceChroma/Flowers.exr`

## Compatibility Considerations

### Breaking Changes
**None.** All additions are new APIs, existing code continues to work.

### Deprecations
**None planned.** Direct field access remains valid for advanced use.

### Version Target
- Minimum: 1.x (current major version)
- Recommend: Bump minor version for new feature

## Performance Implications

### Memory
- Filters require temporary buffers for filtered samples
- Sinc filter: ~2-3x memory overhead during filtering
- Box filter: minimal overhead

### Speed
- Sinc filtering adds ~20-40% overhead to read/write
- Users can opt-in (default: no filtering, direct subsampled access)
- Filtering can be parallelized with rayon

### Optimization Opportunities
- SIMD for sinc calculations
- Separable filters (1D horizontal, then 1D vertical)
- Cached filter kernel weights

## Documentation Requirements

### Public API Docs
- All public functions need rustdoc
- Examples in doc comments
- Link to OpenEXR specification

### Module Documentation
- Explain subsampling concepts (4:2:0, 4:2:2, etc.)
- Filter theory (why sinc, what are lobes)
- Performance trade-offs

### README Updates
- Add subsampling to feature list
- Link to examples
- Show quick-start code snippet

## Future Extensions (Out of Scope)

### Not in This Plan
- Deep data subsampling (complex, rare use case)
- Tiled image subsampling (spec restriction)
- GPU-accelerated filtering (significant complexity)
- Other filter types (Mitchell, Catmull-Rom, etc.) - can be added later

### Potential Future Work
- Separate RGB vs YUV color space conversion utilities
- Automatic chroma placement (MPEG1 vs MPEG2 vs JPEG)
- Perceptual quality metrics (SSIM, MS-SSIM)
- Filter kernel caching for batch processing

## Summary

This plan provides a **complete roadmap** for exposing channel subsampling in the exr crate:

✅ **Low-level API**: Direct control with `AnyChannel::with_subsampling()` and presets
✅ **Mid-level API**: Builder pattern with explicit filter selection
✅ **High-level API**: Simple functions like `write_yuv420_file()` with automatic filtering
✅ **Sinc filter**: High-quality Lanczos-windowed sinc for reconstruction/downsampling
✅ **Alternative filters**: Box and nearest-neighbor for comparison
✅ **Examples**: Comprehensive examples showing all three API levels
✅ **Tests**: Unit tests for filters, integration tests for round-trips
✅ **Documentation**: Full rustdoc coverage and user guide updates

The implementation follows the crate's existing architecture patterns and maintains backward compatibility while providing both power and convenience.
