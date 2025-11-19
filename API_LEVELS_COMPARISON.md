# Channel Subsampling API - Three Levels Comparison

## Quick Overview

The plan adds channel subsampling support at **three distinct levels** matching the exr crate's existing architecture:

```
┌────────────────────────────────────────────────────────────────┐
│ Level 1: HIGH-LEVEL (Simple Functions)                        │
│ - write_yuv420_file(), write_yuv422_file()                    │
│ - Automatic sinc filtering                                    │
│ - Best for: Quick start, common use cases                     │
└────────────────────────────────────────────────────────────────┘
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Level 2: MID-LEVEL (Builder Pattern)                          │
│ - .with_downsampling_filter(SincFilter::lanczos2())          │
│ - .with_reconstruction_filter(SincFilter::lanczos3())        │
│ - AnyChannel::with_subsampling(name, data, sampling)         │
│ - Best for: Flexibility with safety                           │
└────────────────────────────────────────────────────────────────┘
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Level 3: LOW-LEVEL (Direct Field Access)                      │
│ - channel.sampling = Vec2(2, 2)                               │
│ - Manual filter implementation                                 │
│ - Best for: Maximum control, custom workflows                 │
└────────────────────────────────────────────────────────────────┘
```

## Side-by-Side Comparison

### Writing a YUV 4:2:0 Image

#### Level 1: High-Level (2 lines)
```rust
use exr::prelude::*;

write_yuv420_file("output.exr", 1920, 1080,
    |x, y| y_value,  // Luma
    |x, y| u_value,  // U chroma (auto-subsampled with sinc filter)
    |x, y| v_value   // V chroma (auto-subsampled with sinc filter)
).unwrap();
```
**Pros**: Simple, safe, automatic filtering
**Cons**: Less flexible, preset only

---

#### Level 2: Mid-Level (15 lines)
```rust
use exr::prelude::*;
use exr::image::{subsampling, reconstruct_filter::SincFilter};

let channels = AnyChannels::sort(smallvec![
    AnyChannel::new("Y", y_samples),
    AnyChannel::with_subsampling("RY", u_samples, subsampling::YUV_420),
    AnyChannel::with_subsampling("BY", v_samples, subsampling::YUV_420),
]);

let layer = Layer::new((width, height), attributes, encoding, channels);

Image::from_layer(layer)
    .write()
    .with_downsampling_filter(SincFilter::lanczos3())  // Custom filter
    .to_file("output.exr")
    .unwrap();
```
**Pros**: Flexible, explicit control, custom filters
**Cons**: More verbose

---

#### Level 3: Low-Level (20 lines)
```rust
use exr::prelude::*;

let mut y_chan = AnyChannel::new("Y", y_samples);
let mut u_chan = AnyChannel::new("RY", u_samples);
let mut v_chan = AnyChannel::new("BY", v_samples);

// Direct field manipulation
u_chan.sampling = Vec2(2, 2);
v_chan.sampling = Vec2(2, 2);
u_chan.quantize_linearly = true;
v_chan.quantize_linearly = true;

let channels = AnyChannels::sort(smallvec![y_chan, u_chan, v_chan]);
let layer = Layer::new((width, height), attributes, encoding, channels);

// Manual filtering if needed
// ... custom filter implementation ...

Image::from_layer(layer)
    .write()
    .to_file("output.exr")
    .unwrap();
```
**Pros**: Maximum control, custom workflows, no overhead
**Cons**: Manual filter implementation, more code

---

### Reading Subsampled Data

#### Level 1: High-Level
```rust
// Future API (if we add convenience function)
let image = read_yuv420_file("input.exr").unwrap();
// Automatically reconstructs to full resolution with sinc filter
```

---

#### Level 2: Mid-Level
```rust
use exr::image::reconstruct_filter::SincFilter;

let image: AnyImage = read()
    .no_deep_data()
    .largest_resolution_level()
    .all_channels()
    .with_reconstruction_filter(SincFilter::lanczos2())  // Upsample with sinc
    .all_layers()
    .from_file("input.exr")
    .unwrap();
```

---

#### Level 3: Low-Level
```rust
// Read raw subsampled data (no automatic reconstruction)
let image: AnyImage = read()
    .no_deep_data()
    .largest_resolution_level()
    .all_channels()
    .all_layers()
    .from_file("input.exr")
    .unwrap();

// Manually inspect subsampling
for layer in &image.layer_data {
    for channel in &layer.channel_data.list {
        if channel.sampling != Vec2(1, 1) {
            println!("Channel '{}' is subsampled: {:?}",
                channel.name, channel.sampling);
            // Implement custom reconstruction...
        }
    }
}
```

## Filter Options

### Sinc Filter (Lanczos)
```rust
SincFilter::lanczos2()  // 2 lobes - good balance (recommended)
SincFilter::lanczos3()  // 3 lobes - higher quality
SincFilter::new(4)      // 4 lobes - maximum quality
```

**Quality**: ⭐⭐⭐⭐⭐ Excellent
**Speed**: ⭐⭐⭐ Moderate
**Use for**: Professional content, final output

### Box Filter
```rust
BoxFilter
```

**Quality**: ⭐⭐⭐ Adequate
**Speed**: ⭐⭐⭐⭐ Fast
**Use for**: Quick previews, non-critical content

### Nearest Neighbor
```rust
NearestFilter
```

**Quality**: ⭐⭐ Poor (blocky)
**Speed**: ⭐⭐⭐⭐⭐ Very fast
**Use for**: Debugging, pixel art

## Common Subsampling Patterns

```rust
use exr::image::subsampling;

subsampling::NONE      // Vec2(1, 1) - 4:4:4 (no subsampling)
subsampling::YUV_422   // Vec2(2, 1) - Horizontal 2x (professional video)
subsampling::YUV_420   // Vec2(2, 2) - 2x2 (consumer video, JPEG)
subsampling::YUV_411   // Vec2(4, 1) - Horizontal 4x (old DV)
```

## When to Use Each Level

| Use Case | Recommended Level | Why |
|----------|------------------|-----|
| Quick YUV video export | **Level 1** | Minimal code, automatic best practices |
| Custom channel arrangement | **Level 2** | Flexible but safe, explicit filters |
| Research/experimentation | **Level 2** | Try different filters easily |
| Performance-critical app | **Level 3** | No overhead, manual optimization |
| Reading legacy files | **Level 2** or **3** | Inspect metadata, custom handling |
| Batch processing | **Level 2** | Balance of control and simplicity |
| Library/framework author | **Level 3** | Maximum control for wrapping |

## Migration Path

### Existing Code (No Changes Required)
```rust
// This still works exactly as before
let image = read_all_data_from_file("input.exr").unwrap();
```

### Gradual Adoption
```rust
// Step 1: Start with low-level API (just add sampling)
let mut channel = AnyChannel::new("RY", data);
channel.sampling = Vec2(2, 2);  // Manual subsampling

// Step 2: Switch to mid-level (add convenience constructor)
let channel = AnyChannel::with_subsampling("RY", data, subsampling::YUV_420);

// Step 3: Add filtering when needed
image.write()
    .with_downsampling_filter(SincFilter::lanczos2())
    .to_file("output.exr")
```

## Performance Characteristics

| Operation | No Filter | Box Filter | Sinc Filter (2-lobe) | Sinc Filter (3-lobe) |
|-----------|-----------|------------|----------------------|----------------------|
| **Read Speed** | 100% (baseline) | 105% | 125% | 145% |
| **Write Speed** | 100% (baseline) | 108% | 130% | 155% |
| **Memory** | 1x | 1.2x | 2.5x | 3.0x |
| **Quality** | Raw/Blocky | Adequate | Excellent | Exceptional |

*Estimates based on typical chroma subsampling scenarios*

## Implementation Priority

```
Phase 1: Low-Level API          ⚡ Quick win (4-6 hours)
    ↓
Phase 2: Sinc Filter           🔧 Core feature (12-16 hours)
    ↓
Phase 3: Filter Integration    🔗 Complex (16-20 hours)
    ↓
Phase 4: High-Level API        ✨ Nice to have (6-8 hours)
    ↓
Phase 5: Docs & Examples       📚 Essential (8-10 hours)
```

**Total estimated effort**: 46-60 hours for complete implementation

## Key Design Decisions

1. **Default behavior**: No filtering (backward compatible, opt-in)
2. **Recommended filter**: Lanczos2 (good balance of quality/performance)
3. **API consistency**: Follows existing builder pattern
4. **No breaking changes**: All additions are new APIs
5. **Both directions**: Filters for both reading (reconstruction) and writing (downsampling)

## Questions & Answers

**Q: Can I use subsampling without filters?**
A: Yes! Just set the `sampling` field or use `with_subsampling()`. Filters are optional.

**Q: What if I want a custom filter?**
A: Implement the `ReconstructionFilter` or `DownsamplingFilter` trait.

**Q: Will this slow down my code?**
A: Only if you enable filtering. Without filters, minimal overhead (~2-5%).

**Q: Can I use this with deep data or tiled images?**
A: No, subsampling is restricted to flat, scan-line images per OpenEXR spec.

**Q: Is the sinc filter high quality?**
A: Yes, Lanczos-windowed sinc is industry standard for high-quality resampling.

---

**For complete implementation details, see**: `SUBSAMPLING_API_PLAN.md`
