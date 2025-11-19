# Sinc Filter Architecture & Implementation Details

## Filter Pipeline Overview

```
WRITE PATH (Downsampling):
┌─────────────────┐
│ User provides   │
│ FULL resolution │  1920x1080 pixels
│ Y, U, V data    │
└────────┬────────┘
         │
         ├──────────────────────────────┬────────────────────────┐
         │                              │                        │
         ▼                              ▼                        ▼
┌────────────────┐            ┌─────────────────┐     ┌─────────────────┐
│ Y channel      │            │ U channel       │     │ V channel       │
│ 1920x1080      │            │ 1920x1080       │     │ 1920x1080       │
│ sampling: 1x1  │            │ sampling: 2x2   │     │ sampling: 2x2   │
└────────┬───────┘            └────────┬────────┘     └────────┬────────┘
         │                             │                       │
         │ No filtering                │ SINC DOWNSAMPLE       │ SINC DOWNSAMPLE
         │                             │                       │
         ▼                             ▼                       ▼
┌────────────────┐            ┌─────────────────┐     ┌─────────────────┐
│ Y: 1920x1080   │            │ U: 960x540      │     │ V: 960x540      │
│ 2,073,600      │            │ 518,400 samples │     │ 518,400 samples │
│ samples        │            │ (1/4 size)      │     │ (1/4 size)      │
└────────┬───────┘            └────────┬────────┘     └────────┬────────┘
         │                             │                       │
         └──────────────┬──────────────┴───────────────────────┘
                        ▼
                ┌───────────────┐
                │ Write to EXR  │
                │ Y: 2MB        │
                │ U: 0.5MB      │
                │ V: 0.5MB      │
                │ Total: 3MB    │
                └───────────────┘


READ PATH (Reconstruction):
┌────────────────┐
│ Read from EXR  │
│ Y: 1920x1080   │
│ U: 960x540     │
│ V: 960x540     │
└────────┬───────┘
         │
         ├──────────────────────────────┬────────────────────────┐
         │                              │                        │
         ▼                              ▼                        ▼
┌────────────────┐            ┌─────────────────┐     ┌─────────────────┐
│ Y channel      │            │ U channel       │     │ V channel       │
│ 1920x1080      │            │ 960x540         │     │ 960x540         │
│ (full res)     │            │ (subsampled)    │     │ (subsampled)    │
└────────┬───────┘            └────────┬────────┘     └────────┬────────┘
         │                             │                       │
         │ No filtering                │ SINC RECONSTRUCT      │ SINC RECONSTRUCT
         │                             │                       │
         ▼                             ▼                       ▼
┌────────────────┐            ┌─────────────────┐     ┌─────────────────┐
│ Y: 1920x1080   │            │ U: 1920x1080    │     │ V: 1920x1080    │
│ Ready to use   │            │ Reconstructed   │     │ Reconstructed   │
└────────────────┘            └─────────────────┘     └─────────────────┘
```

## Sinc Filter Mathematics

### Core Sinc Function
```
sinc(x) = sin(πx) / (πx)

Properties:
- sinc(0) = 1
- sinc(n) = 0 for integer n ≠ 0
- Infinite support (extends to ±∞)
```

### Lanczos Window
```
lanczos_a(x) = sinc(x) × sinc(x/a)    for |x| < a
             = 0                       for |x| ≥ a

where:
- a = number of lobes (typically 2 or 3)
- Limits the infinite sinc to finite support
- Reduces ringing artifacts
```

### Reconstruction Formula
```
For output position (x, y):

1. Map to subsampled space:
   sub_x = x / sampling_x
   sub_y = y / sampling_y

2. Sample neighbors within radius 'a':
   for each (sx, sy) in [sub_x-a .. sub_x+a]:
       weight = lanczos(sub_x - sx) × lanczos(sub_y - sy)
       sum += subsampled[sy][sx] × weight
       weight_sum += weight

3. Normalize:
   result = sum / weight_sum
```

## Implementation Details

### Filter Kernel Size

```
Lanczos2 (2 lobes):
┌─────────────────────────────┐
│ Kernel size: 4×4            │
│ Read samples: 16 per output │
│ Memory: Moderate            │
│ Quality: Excellent          │
└─────────────────────────────┘

Lanczos3 (3 lobes):
┌─────────────────────────────┐
│ Kernel size: 6×6            │
│ Read samples: 36 per output │
│ Memory: Higher              │
│ Quality: Exceptional        │
└─────────────────────────────┘
```

### Example: Upsampling 2×2 to Full Resolution

```
Subsampled data (2×2):
┌───┬───┐
│ A │ B │  A=0.2, B=0.4
├───┼───┤  C=0.6, D=0.8
│ C │ D │
└───┴───┘

Full resolution output (4×4):
┌────┬────┬────┬────┐
│ a₀ │ a₁ │ a₂ │ a₃ │
├────┼────┼────┼────┤
│ b₀ │ b₁ │ b₂ │ b₃ │  Each aᵢ calculated from
├────┼────┼────┼────┤  weighted combination of A,B,C,D
│ c₀ │ c₁ │ c₂ │ c₃ │  using sinc filter
├────┼────┼────┼────┤
│ d₀ │ d₁ │ d₂ │ d₃ │
└────┴────┴────┴────┘

For pixel a₁ (position x=1, y=0):
  sub_x = 1 / 2 = 0.5
  sub_y = 0 / 2 = 0.0

  Weights (Lanczos2):
    A (0,0): lanczos(0.5) × lanczos(0.0) = 0.605 × 1.0   = 0.605
    B (1,0): lanczos(0.5) × lanczos(0.0) = 0.605 × 1.0   = 0.605
    C (0,1): lanczos(0.5) × lanczos(1.0) = 0.605 × 0.0   = 0.0
    D (1,1): lanczos(0.5) × lanczos(1.0) = 0.605 × 0.0   = 0.0

  Result:
    a₁ = (A×0.605 + B×0.605) / (0.605 + 0.605)
       = (0.2×0.605 + 0.4×0.605) / 1.21
       = 0.363 / 1.21
       = 0.3  (linear interpolation between A and B)
```

## Code Structure

### Trait Definitions
```rust
// src/image/reconstruct_filter.rs

pub trait ReconstructionFilter: Send + Sync {
    fn reconstruct(
        &self,
        subsampled_data: &[f32],     // Input: subsampled buffer
        subsampled_size: Vec2<usize>, // Input dimensions
        output_pos: Vec2<usize>,      // Where to reconstruct
        sampling: Vec2<usize>,        // Subsampling rate
    ) -> f32;                         // Reconstructed value
}

pub trait DownsamplingFilter: Send + Sync {
    fn downsample(
        &self,
        full_data: &[f32],           // Input: full-res buffer
        full_size: Vec2<usize>,       // Input dimensions
        output_pos: Vec2<usize>,      // Subsampled position
        sampling: Vec2<usize>,        // Subsampling rate
    ) -> f32;                         // Downsampled value
}
```

### Integration Points

```
┌─────────────────────────────────────────────────────────┐
│ FlatSamplesReader                                       │
│                                                         │
│  fn read_line(&mut self, line: LineRef) -> Result<()>  │
│      ↓                                                  │
│  If reconstruction_filter.is_some():                    │
│      ↓                                                  │
│  For each pixel in line:                                │
│      value = filter.reconstruct(                        │
│          self.samples,                                  │
│          self.subsampled_resolution,                    │
│          output_position,                               │
│          self.sampling                                  │
│      )                                                  │
│      write value to output line                         │
│  Else:                                                  │
│      Direct copy (existing behavior)                    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ FlatSamplesWriter                                       │
│                                                         │
│  fn extract_line(&self, line: LineRefMut)              │
│      ↓                                                  │
│  If downsampling_filter.is_some():                      │
│      ↓                                                  │
│  For each subsampled pixel:                             │
│      value = filter.downsample(                         │
│          self.samples,                                  │
│          self.full_resolution,                          │
│          subsampled_position,                           │
│          self.sampling                                  │
│      )                                                  │
│      write value to output line                         │
│  Else:                                                  │
│      Direct copy from subsampled buffer (existing)      │
└─────────────────────────────────────────────────────────┘
```

## Performance Optimization Opportunities

### 1. Separable Filter
```
2D convolution:     O(n² × kernel²) per pixel
Separable (1D×1D):  O(2 × n × kernel) per pixel

Example for 1920×1080, 2×2 subsampling, Lanczos2:
  2D:        960×540 × 16 samples = 8.3M operations
  Separable: 960×540 × 8 samples  = 4.1M operations (2× faster)
```

### 2. SIMD Acceleration
```rust
// Process 4 pixels at once with AVX
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

unsafe fn sinc_simd(x: __m128) -> __m128 {
    // Vectorized sinc calculation
    // 4× speedup on compatible hardware
}
```

### 3. Precomputed Kernel
```rust
struct PrecomputedKernel {
    weights: Vec<f32>,        // Precomputed for common ratios
    kernel_size: usize,
}

// For fixed subsampling (e.g., always 2×2):
// - Compute weights once at initialization
// - Reuse for all pixels
// - ~3× faster than computing on the fly
```

### 4. Parallel Processing
```rust
// Already supported via rayon feature
#[cfg(feature = "rayon")]
use rayon::prelude::*;

lines.par_iter_mut().for_each(|line| {
    apply_filter(line);  // Each line processes in parallel
});
```

## Quality Metrics

### PSNR (Peak Signal-to-Noise Ratio)
```
Higher is better (typically 30-50 dB for good quality)

Box filter:      ~32 dB
Lanczos2:        ~42 dB  ⭐ Recommended
Lanczos3:        ~44 dB
```

### SSIM (Structural Similarity Index)
```
Range: 0.0 (worst) to 1.0 (perfect)

Box filter:      ~0.92
Lanczos2:        ~0.97  ⭐ Recommended
Lanczos3:        ~0.98
```

## Edge Cases & Handling

### 1. Image Boundaries
```rust
// Clamp coordinates to valid range
let sx = sx.max(0).min(subsampled_size.x() - 1);
let sy = sy.max(0).min(subsampled_size.y() - 1);

// Or use mirror/wrap boundary conditions
```

### 2. Odd Dimensions
```rust
// Validate before processing
if width % sampling.x() != 0 {
    return Err(Error::invalid("Width not aligned to subsampling"));
}
```

### 3. Zero Weight Sum
```rust
if weight_sum > 0.0 {
    sum / weight_sum
} else {
    // Fallback to nearest neighbor
    nearest_sample(subsampled_data, output_pos, sampling)
}
```

### 4. Numerical Stability
```rust
// Avoid division by very small numbers in sinc
fn sinc(x: f32) -> f32 {
    if x.abs() < 0.001 {  // Near zero
        1.0
    } else {
        let pi_x = std::f32::consts::PI * x;
        pi_x.sin() / pi_x
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_sinc_zero() {
    assert_eq!(SincFilter::sinc(0.0), 1.0);
}

#[test]
fn test_sinc_integer() {
    assert_eq!(SincFilter::sinc(1.0), 0.0);
    assert_eq!(SincFilter::sinc(2.0), 0.0);
}

#[test]
fn test_lanczos_window() {
    let filter = SincFilter::lanczos2();
    assert_eq!(filter.lanczos(3.0), 0.0);  // Outside 2-lobe radius
}
```

### Integration Tests
```rust
#[test]
fn test_reconstruct_identity() {
    // No subsampling should be identity
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let filter = SincFilter::lanczos2();

    let result = filter.reconstruct(
        &data,
        Vec2(2, 2),
        Vec2(0, 0),
        Vec2(1, 1),  // No subsampling
    );

    assert_eq!(result, 1.0);
}

#[test]
fn test_downsample_upsample_roundtrip() {
    // Downsample then upsample should be close to original
    let original = create_test_image(100, 100);
    let filter = SincFilter::lanczos3();

    let downsampled = downsample_image(&original, Vec2(2, 2), &filter);
    let reconstructed = upsample_image(&downsampled, Vec2(2, 2), &filter);

    let psnr = calculate_psnr(&original, &reconstructed);
    assert!(psnr > 40.0);  // High quality
}
```

### Benchmark Tests
```rust
#[bench]
fn bench_sinc_lanczos2_1080p(b: &mut Bencher) {
    let data = vec![0.5_f32; 960 * 540];  // Subsampled 1080p
    let filter = SincFilter::lanczos2();

    b.iter(|| {
        filter.reconstruct(
            &data,
            Vec2(960, 540),
            Vec2(100, 100),
            Vec2(2, 2),
        )
    });
}
```

## References

### Academic
- Smith, J.O. (2007). "Mathematics of the Discrete Fourier Transform (DFT)" - Chapter on windowing
- Turkowski, K. (1990). "Filters for Common Resampling Tasks" - Apple Technical Report

### Implementation Examples
- ImageMagick: Uses Lanczos for high-quality resize
- FFmpeg: libswscale uses Lanczos for video scaling
- Photoshop: Bicubic/Lanczos for image resize

### OpenEXR Specification
- Section 4.2: Channel Subsampling
- https://www.openexr.com/documentation/TechnicalIntroduction.pdf

---

**See also:**
- `SUBSAMPLING_API_PLAN.md` - Complete implementation plan
- `API_LEVELS_COMPARISON.md` - API usage examples
- `IMPLEMENTATION_SUMMARY.md` - Executive overview
