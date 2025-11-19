# Channel Subsampling API - Documentation Index

This directory contains comprehensive planning documents for adding channel subsampling support to the exr crate.

## 📚 Document Overview

### 1. **IMPLEMENTATION_SUMMARY.md** - START HERE ⭐
**Quick executive summary** of the entire plan
- What's being added (3 API levels)
- Why it matters
- Quick code examples
- 5-phase roadmap
- Estimated effort: 46-60 hours

**Best for**: Getting oriented, understanding scope

---

### 2. **API_LEVELS_COMPARISON.md** - QUICK REFERENCE 🚀
**Side-by-side comparison** of all three API levels
- Code examples for each level
- When to use which level
- Filter performance comparison
- Migration guide
- Common patterns

**Best for**: Choosing API approach, quick examples

---

### 3. **SUBSAMPLING_API_PLAN.md** - DETAILED SPEC 📋
**Complete implementation specification** (longest document)
- Full API signatures with rustdoc
- Complete sinc filter implementation
- All integration points
- Testing strategy
- Documentation requirements
- Future extensions

**Best for**: Implementation details, copy-paste code

---

### 4. **FILTER_ARCHITECTURE.md** - TECHNICAL DEEP DIVE 🔬
**Mathematics and implementation details** for filters
- Sinc filter mathematics
- Lanczos windowing explanation
- Performance optimizations (SIMD, separable, etc.)
- Quality metrics (PSNR, SSIM)
- Edge case handling
- Benchmark strategy

**Best for**: Understanding filters, optimization, testing

---

## 🗂️ Navigation Guide

### "I want to understand the big picture"
→ Start with **IMPLEMENTATION_SUMMARY.md**

### "I want to see code examples"
→ Go to **API_LEVELS_COMPARISON.md**

### "I'm ready to implement"
→ Use **SUBSAMPLING_API_PLAN.md** as your guide

### "I need to understand the math/performance"
→ Read **FILTER_ARCHITECTURE.md**

### "I want everything"
→ Read in this order:
1. IMPLEMENTATION_SUMMARY.md (overview)
2. API_LEVELS_COMPARISON.md (examples)
3. SUBSAMPLING_API_PLAN.md (implementation)
4. FILTER_ARCHITECTURE.md (technical details)

---

## 🎯 Quick Start: 30-Second Summary

**Goal**: Add channel subsampling support to exr crate with sinc filters

**Three API Levels**:
- **High**: `write_yuv420_file(...)` - 2 lines of code
- **Mid**: `.with_downsampling_filter(SincFilter::lanczos2())` - builder pattern
- **Low**: `channel.sampling = Vec2(2,2)` - direct field access

**Core Feature**: Lanczos-windowed sinc filter for high-quality reconstruction/downsampling

**Effort**: 46-60 hours across 5 phases

**Risk**: Low (all additive, no breaking changes)

**Value**: Enables professional video/image workflows

---

## 📊 Implementation Status

### Current State (Before This Plan)
- ✅ Subsampling works internally (read/write)
- ✅ Tests pass
- ❌ No public API
- ❌ No filters

### After Phase 1 (4-6 hours)
- ✅ Low-level API
- ✅ Subsampling presets
- ❌ No filters yet

### After Phase 2 (12-16 hours)
- ✅ Low-level API
- ✅ Sinc filter implemented
- ❌ Not integrated yet

### After Phase 3 (16-20 hours)
- ✅ Low-level API
- ✅ Mid-level builder API
- ✅ Filters integrated
- ❌ No convenience functions

### After Phase 4 (6-8 hours)
- ✅ All three API levels
- ✅ Full functionality
- ❌ Needs documentation

### After Phase 5 (8-10 hours) - COMPLETE
- ✅ Everything
- ✅ Examples
- ✅ Documentation
- ✅ Ready to release

---

## 🔍 Key Locations in Codebase

### Files to Modify
```
src/image/mod.rs                  - Add AnyChannel::with_subsampling()
src/image/write/mod.rs            - Add write_yuv420_file(), builder extension
src/image/write/samples.rs        - Integrate downsampling filter
src/image/read/image.rs           - Add builder extension
src/image/read/samples.rs         - Integrate reconstruction filter
```

### Files to Create
```
src/image/reconstruct_filter.rs   - Filter implementations
examples/9_write_yuv_subsampled.rs
examples/10_read_yuv_with_reconstruction.rs
tests/filter_quality.rs
```

### Existing Files (Reference)
```
src/block/lines.rs                - LineIndex::lines_in_block() (critical function)
src/meta/attribute.rs             - ChannelDescription with sampling field
tests/subsampling.rs              - Existing tests
```

---

## 🧪 Testing Checklist

- [ ] Unit tests for sinc filter (correctness)
- [ ] Unit tests for Lanczos window
- [ ] Integration tests (round-trip with subsampling)
- [ ] Performance benchmarks (vs baseline)
- [ ] Quality metrics (PSNR, SSIM)
- [ ] Edge case handling (boundaries, odd dimensions)
- [ ] Existing test suite still passes
- [ ] New examples run successfully

---

## 📝 Documentation Checklist

- [ ] Rustdoc for all public APIs
- [ ] Module-level documentation
- [ ] Code examples in doc comments
- [ ] README updated
- [ ] CHANGELOG entry
- [ ] Migration guide (if needed)
- [ ] Performance characteristics documented
- [ ] Example programs work

---

## 🎬 Example Code (Teaser)

### High-Level API
```rust
use exr::prelude::*;

write_yuv420_file("out.exr", 1920, 1080,
    |x, y| luma(x, y),
    |x, y| u_chroma(x, y),
    |x, y| v_chroma(x, y)
).unwrap();
```

### Mid-Level API
```rust
use exr::prelude::*;
use exr::image::{subsampling, reconstruct_filter::SincFilter};

let channels = AnyChannels::sort(smallvec![
    AnyChannel::new("Y", y_samples),
    AnyChannel::with_subsampling("RY", u_samples, subsampling::YUV_420),
    AnyChannel::with_subsampling("BY", v_samples, subsampling::YUV_420),
]);

Image::from_layer(Layer::new(size, attrs, encoding, channels))
    .write()
    .with_downsampling_filter(SincFilter::lanczos2())
    .to_file("out.exr")
    .unwrap();
```

### Low-Level API
```rust
let mut u_channel = AnyChannel::new("RY", u_samples);
u_channel.sampling = Vec2(2, 2);
u_channel.quantize_linearly = true;
// ... build layer and write
```

---

## 🔗 External References

- [OpenEXR Specification](https://www.openexr.com/documentation/TechnicalIntroduction.pdf) - Section 4.2: Subsampling
- [Lanczos Resampling](https://en.wikipedia.org/wiki/Lanczos_resampling) - Filter theory
- [ImageMagick Resize Filters](https://imagemagick.org/Usage/filter/) - Industry comparison

---

## 💡 Design Philosophy

1. **Gradual complexity**: Simple for beginners, powerful for experts
2. **Opt-in filtering**: Performance choice, not forced
3. **Zero breaking changes**: All additive APIs
4. **Industry standard**: Lanczos filter (used by Photoshop, ffmpeg)
5. **Well tested**: Comprehensive unit and integration tests
6. **Well documented**: Examples for all three levels

---

## 🚦 Decision Matrix

| If you want... | Use this API level | See document |
|----------------|-------------------|--------------|
| Quick YUV export | High-level | API_LEVELS_COMPARISON.md |
| Custom filters | Mid-level | API_LEVELS_COMPARISON.md |
| Maximum control | Low-level | API_LEVELS_COMPARISON.md |
| Understand math | Any | FILTER_ARCHITECTURE.md |
| Implementation details | Any | SUBSAMPLING_API_PLAN.md |
| Overall roadmap | Any | IMPLEMENTATION_SUMMARY.md |

---

## ✅ Validation Checklist

Before starting implementation:
- [ ] Read IMPLEMENTATION_SUMMARY.md
- [ ] Review API_LEVELS_COMPARISON.md
- [ ] Understand SUBSAMPLING_API_PLAN.md phases
- [ ] Questions answered about filters (FILTER_ARCHITECTURE.md)

After Phase 1:
- [ ] Low-level API tests pass
- [ ] Examples demonstrate usage
- [ ] Documentation updated

After Phase 2:
- [ ] Filter unit tests pass
- [ ] Quality metrics acceptable
- [ ] Performance benchmarked

After Phase 3:
- [ ] Integration tests pass
- [ ] Round-trip works
- [ ] Existing tests still pass

After Phase 4:
- [ ] High-level functions work
- [ ] Validation logic correct
- [ ] Edge cases handled

After Phase 5:
- [ ] All documentation complete
- [ ] Examples tested
- [ ] Ready for review

---

## 📈 Success Metrics

### Functionality
- ✅ All three API levels implemented
- ✅ Sinc filter works correctly
- ✅ Round-trip tests pass
- ✅ Quality metrics meet targets (PSNR > 40dB)

### Code Quality
- ✅ 100% rustdoc coverage for public APIs
- ✅ Comprehensive test suite
- ✅ Benchmarks run successfully
- ✅ No regressions in existing tests

### Usability
- ✅ Examples work and are clear
- ✅ Documentation is complete
- ✅ API is intuitive
- ✅ Performance is acceptable

---

**Questions? Start with IMPLEMENTATION_SUMMARY.md**

**Ready to code? Go to SUBSAMPLING_API_PLAN.md**

**Need examples? See API_LEVELS_COMPARISON.md**

**Want technical details? Read FILTER_ARCHITECTURE.md**

---

*Generated as part of channel subsampling API planning for the exr crate*
*Total documentation: 4 files, ~3000 lines of specification and examples*
