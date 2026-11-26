## 1. Crate and contract

- [x] 1.1 Add `crates/pictura-render` depending on `pictura-core`
- [x] 1.2 Define `composite_rgba(&Document) -> PixelBuffer` returning planar straight-alpha RGBA8 at document resolution
- [x] 1.3 Implement the `Canvas` `f32` accumulator and clamp-and-round 8-bit write-back
- [x] 1.4 Walk the layer stack bottom-to-top, skipping invisible layers

## 2. Blend kernels

- [x] 2.1 Implement the 27-mode dispatch and separable per-channel formulas
- [x] 2.2 Add the W3C non-separable helpers (`lum`, `sat`, `set_lum`, `set_sat`, `clip_color`) for Hue/Saturation/Color/Luminosity
- [x] 2.3 Implement the Photoshop-only extended modes and clamps (Divide-by-zero, Subtract, Hard Mix, Darker/Lighter Color)
- [x] 2.4 Add Dissolve as a deterministic per-pixel noise threshold

## 3. Compositing pipeline

- [x] 3.1 Apply layer opacity and raster mask to source alpha before compositing
- [x] 3.2 Implement the W3C source-over equation with premultiplied intermediates
- [x] 3.3 Clip layers to the canvas and handle negative, offset, and zero-area rectangles

## 4. Groups

- [x] 4.1 Composite isolated groups into a private transparent buffer and blend them as one layer
- [x] 4.2 Implement true pass-through for groups with opacity 255 and no mask
- [x] 4.3 Fall back to isolated `Normal` for pass-through groups with opacity or a mask, documented as an approximation

## 5. W3C unit tests

- [x] 5.1 Add one unit test per mode at `Cb = 0.25, Cs = 0.75`
- [x] 5.2 Add hand-computed chromatic tests for the non-separable modes
- [x] 5.3 Add pipeline tests for multi-layer source-over, transparent backdrop, opacity, masking, groups, clipping, Dissolve, and grayscale

## 6. ImageMagick oracle

- [x] 6.1 Write `scripts/im_compose.py` with scenes, the PSD-to-ImageMagick operator mapping, and `gen`/`check`/`list`/`compose` commands
- [x] 6.2 Generate and commit the 8×8 interleaved RGBA fixtures under `crates/pictura-render/tests/fixtures/`
- [x] 6.3 Add `tests/oracle/main.rs` diffing the solid, ramp, and alpha scenes against the fixtures with per-mode tolerance
- [x] 6.4 Document the operator mapping, scenes, and unsupported modes in `crates/pictura-render/tests/README.md`
