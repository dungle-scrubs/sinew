# Performance Notes for Sinew

Research notes on GPU rendering, Rust performance, and high-performance UI frameworks.

## Why Current Implementation is Slow

The scroll lag in the demo panel is **not** from slow frame rendering (frames are 0.3-0.6ms). The issues are:

1. **Manual scroll handling** - Reimplementing `scrollWheel:` instead of using native `NSScrollView`
2. **CPU-based text rendering** - Core Text creates `CFMutableAttributedString` + `CTLine` on every draw
3. **No GPU acceleration** - All rendering happens on CPU via Core Graphics

---

## Zed's GPUI Architecture

Source: [Leveraging Rust and the GPU to render user interfaces at 120 FPS](https://zed.dev/blog/videogame)

### Core Philosophy
- Treat UI composition like a **game engine**: frame scheduling, GPU batching, minimal CPU overhead
- Target: **120 FPS** (8.33ms per frame budget)
- Everything renders on GPU - shadows, quads, paths, underlines, sprites, glyphs

### Rendering Pipeline

#### Signed Distance Functions (SDFs)
- Foundation for rendering all primitives
- Mathematical functions returning distance to object edge
- Efficient rounded rectangles by adjusting corner radius calculations
- Implemented in **Metal Shader Language** (macOS), Vulkan (Linux), DirectX 11 (Windows)

#### Instanced Rendering (Batching)
- Draw multiple UI elements in **single draw calls**
- Focus on specific primitives only: rectangles, shadows, text, icons, images
- Drawing order: shadows → rectangles → glyphs → icons → images

#### Layering
- Z-index positioning through **stacking contexts**
- Enables proper occlusion without per-pixel depth testing

### Text Rendering

1. **Shaping**: OS APIs handle text shaping (guarantees native consistency)
2. **Caching**: Text-font pairs cached to shaped glyphs; reused across frames
3. **Rasterization**: Glyphs rasterize to alpha-channel only on CPU (enables dynamic tinting)
4. **Glyph Atlas**: Bin-packing algorithm stores rasterized variants on GPU
5. **Sub-pixel positioning**: Up to 16 glyph variants per character for accuracy

### Drop Shadows
- Evan Wallace's mathematical approximation for closed-form shadow calculations
- Separates Gaussian blur: exact convolution on one axis, sampled curve on other
- Avoids expensive per-pixel sampling

### State Management
- **Centralized ownership**: All state in entities owned by GPUI's `AppContext`
- **Element trait**: Abstracts rendering complexity
- **Layout flow**: Constraints down, sizes up through element trees
- **Scene structure**: Platform-neutral primitive collection before GPU submission

### Frame-by-Frame Optimization
- Unused text-font pairs deleted from caches each frame
- Only changed content triggers reshaping
- Amortization keeps expensive ops proportional to actual changes

Source: [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)

---

## Vello: Alternative GPU 2D Renderer

Source: [Vello GitHub](https://github.com/linebender/vello)

### Overview
- GPU compute-centric 2D renderer from Linebender project
- Uses **wgpu** for GPU access (cross-platform)
- Three implementations: full GPU, full CPU, hybrid

### Key Innovation: Prefix-Sum Algorithms
- Traditional renderers do sorting/clipping on CPU or with temp textures
- Vello uses **prefix-sum algorithms** to parallelize sequential work
- Offloads to GPU with minimal temporary buffers
- Requires GPU with **compute shader support**

### Performance (2025 Benchmarks)
- vello-cpu: Second fastest CPU renderer (behind Blend2D)
- Beats Skia and Cairo in many benchmarks
- 30% improvement with new overdraw handling

### Current Status
- Alpha state, actively developed
- Used as rendering backend for **Xilem** (Rust GUI toolkit)

---

## Rust Performance Optimization Techniques

Source: [The Rust Performance Book](https://nnethercote.github.io/perf-book/general-tips.html)

### Memory Management

#### Borrowing over Cloning
```rust
// Prefer this
fn process(data: &str) { ... }

// Over this
fn process(data: String) { ... }
```

#### Cow (Clone on Write)
```rust
use std::borrow::Cow;

fn process<'a>(input: Cow<'a, str>) -> Cow<'a, str> {
    if needs_modification(&input) {
        Cow::Owned(modify(input.into_owned()))
    } else {
        input  // Zero-copy borrow
    }
}
```

#### SmartString for Short Strings
- Inline storage for strings up to 23 bytes (64-bit)
- 2.5x faster than String for short strings
- Zero-cost conversion to/from String when needed

### Arena Allocators

Source: [Bumpalo](https://github.com/fitzgen/bumpalo)

Best for:
- Many small objects with similar lifetimes
- Parsers, compilers, VMs, game engines
- Frame-based UI rendering (allocate per frame, drop all at once)

```rust
use bumpalo::Bump;

let bump = Bump::new();
let layout = bump.alloc(Layout { ... });
// All allocations freed when bump is dropped
```

### Compiler Optimizations

#### Link-Time Optimization (LTO)
```toml
# Cargo.toml
[profile.release]
lto = true
```

#### Custom Allocators
- **jemalloc**: Reduces fragmentation, high concurrency
- **mimalloc**: Microsoft's allocator, very fast

### Data Layout & Cache

- Minimize cache misses: keep related data together
- Minimize branch mispredictions: order cases by frequency
- Use compact representations with lookup tables for outliers

### Hot Path Optimization

1. **Lazy computation**: Defer until needed
2. **Special cases**: Handle 0, 1, 2 element collections specially
3. **Local caches**: Small caches before frequently accessed structures
4. **Eliminate over add**: Remove unnecessary work rather than adding optimizations

---

## GPU Rendering Options for Sinew

### Option 1: GPUI (Zed's Framework)
**Pros:**
- Battle-tested at 120 FPS
- Complete solution (windowing, input, rendering)
- Text rendering solved

**Cons:**
- Tied to Zed development
- Would require full rewrite
- Pre-1.0, breaking changes

### Option 2: Vello + Winit
**Pros:**
- Modern, pure Rust
- Actively developed by Linebender
- Cross-platform via wgpu

**Cons:**
- Alpha state
- Need to handle text rendering separately
- Requires compute shader support

### Option 3: wgpu Direct
**Pros:**
- Full control
- Cross-platform (Metal, Vulkan, DX12, WebGPU)
- No framework overhead

**Cons:**
- Most work to implement
- Need to build text rendering, layout, etc.

### Option 4: Core Animation (CALayer)
**Pros:**
- Native macOS GPU compositing
- Keeps existing NSView architecture
- CATextLayer for GPU text

**Cons:**
- macOS only
- Limited to what CA supports
- Still not true GPU rendering

### Recommendation
For a menu bar app like Sinew, **Vello + Winit** is the best balance:
- Active development, good documentation
- Pure Rust, cross-platform potential
- Reasonable scope for integration

---

## Implementation Strategy for GPU Rendering

### Phase 1: Window System
1. Replace NSWindow/NSView with **winit** window
2. Set up **wgpu** surface and device
3. Basic clear color rendering

### Phase 2: Primitive Rendering
1. Integrate **vello** for 2D rendering
2. Port rectangle/rounded rect rendering
3. Port color/gradient fills

### Phase 3: Text Rendering
1. Use **parley** (Linebender's text layout)
2. Or use **cosmic-text** for shaping
3. Integrate with vello's glyph rendering

### Phase 4: Component System
1. Port Component trait to vello scenes
2. Implement measure/draw with vello primitives
3. GPU-accelerated scroll via transform

### Phase 5: Polish
1. Smooth scroll physics
2. Animations via GPU transforms
3. Performance profiling and optimization

---

## References

- [Zed Blog: GPU Rendering at 120 FPS](https://zed.dev/blog/videogame)
- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)
- [Vello GitHub](https://github.com/linebender/vello)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Bumpalo Arena Allocator](https://github.com/fitzgen/bumpalo)
- [SmartString Crate](https://docs.rs/smartstring)
