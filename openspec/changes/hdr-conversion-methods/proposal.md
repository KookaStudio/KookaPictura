# Proposal

## Why

The 32-bit to 16/8-bit HDR Conversion path implements only Exposure & Gamma. CS6's HDR Conversion dialog offers four tone-mapping methods (Local Adaptation, Equalize Histogram, Exposure and Gamma, Highlight Compression), and the parity audit records the other three as divergent (`IMG-5`, issue #557). A 32-bit document therefore cannot be converted with the automatic methods the specification documents.

## What Changes

- Add an f32 tone-map method enum and implementations for Equalize Histogram, Highlight Compression, and Local Adaptation alongside the existing Exposure & Gamma operator.
- Extend `pictura_render::convert_depth` to take the selected method while keeping the existing exposure/gamma entry point for callers that use it.
- Expose the four methods in the HDR Conversion dialog, with per-method parameter pages. Local Adaptation reuses the nine numeric controls already implemented for `Image > Adjustments > HDR Toning` through a shared numeric-control widget; the HDR Toning preset menu and preview stay in that dialog. The Local Adaptation Toning Curve/Corner is a recorded ceiling, not part of this change.
- Use the current composite when the retained 32-bit store is stale after an edit, so a conversion never tone-maps pre-edit pixels.
- Keep the conversion gate, quantization, retained-store rebuild, and layer-store clearing unchanged.
- Record the new methods as behavioral parity with inferred kernels: the CS6 kernels are closed, and only the Exposure & Gamma formula is fixed by the specification.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `color/hdr-conversion`: the conversion operator accepts a tone-mapping method; the dialog exposes all four CS6 methods; Local Adaptation receives f32 HDR samples.

## Impact

- `crates/pictura-adjust`: new tone-map methods and parameters; `hdr_toning` split into submodules to stay under the file-size cap.
- `crates/pictura-render`: `document_ops/depth.rs` method plumbing; native-depth tests.
- `crates/pictura-app`: conversion bridge signature, HDR Conversion dialog, shared Local Adaptation controls extracted from the HDR Toning dialog.
- New Qt Test suite `tst_hdr_conversion`; updates to `tst_image_mode` and the existing conversion self-test call.
- No new dependencies. The three new kernels are approximate and must not be described as verified CS6 parity.
