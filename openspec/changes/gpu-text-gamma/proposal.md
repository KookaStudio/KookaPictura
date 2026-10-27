# Proposal

## Why

`check_supported` declines the whole document for the GPU whenever a visible type
layer exists and Blend Text Colors Using Gamma is on. The setting defaults on at
1.45, and the usual Photoshop-saved type layer already carries a raster proxy, so
any document with text composites on the CPU: about 1064 ms instead of about
190 ms for a 4000² composite (`docs/dev/app-bugs-investigation.md:326`). The
curve is alpha-only and color/blend-mode independent, so the shader can apply it
exactly as the CPU does.

## What Changes

- The GPU compositor applies the current text-gamma curve to a type layer's
  proxied pixels: the shader curves `source alpha × opacity × Fill` before the
  mask, using the same 4097-entry lookup table as the CPU oracle.
- A per-dispatch flag and a storage-buffer binding carry the curve; the table is
  uploaded from `text_gamma::current_curve()` and bound for every blend dispatch.
- `check_supported` no longer declines a type layer because the gamma is on.
  `ContentRoute::TypeSource` (a type layer with no raster proxy) still declines.
- The GPU decline counter's `unsupported_text` bucket becomes `type_source`, so
  the remaining proxy-less declines stay measurable. `text_gamma` has no
  producer after the gate is removed and is not kept as a dead bucket.
- The GPU parity suite gains proxied-type scenes at γ 1.00, 1.45, 2.20, and off,
  with opacity, Fill < 255, and a mask.
- The stale behavior line in `docs/dev/STATE.md` is updated.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/text-gamma-blend`: the GPU no longer refuses a type layer while
  the gamma is on; it applies the curve to a proxied type layer and keeps the
  proxy-less decline.
- `compositing/gpu-compositing`: text gamma stops being a GPU eligibility
  attribute; a proxied type layer stays eligible and is covered by the parity
  contract.

## Impact

- `crates/pictura-render/src/gpu/mod.rs` (eligibility and error text)
- `crates/pictura-render/src/gpu/backend.rs` (params, LUT binding, dispatch)
- `crates/pictura-render/src/gpu/shader.rs` (curve application)
- `crates/pictura-render/src/text_gamma.rs` (table accessor)
- `crates/pictura-render/src/metrics.rs` (decline reason)
- `crates/pictura-render/src/gpu/eligibility_tests.rs`,
  `src/tests/text_gamma.rs`, `src/tests/composite.rs`, and
  `tests/gpu_parity/` (tests)
- `docs/dev/STATE.md` (stale GPU behavior note)
- No Rust type or signature change: `GpuError` variants, `composite_gpu`,
  `composite_active`, and `Backend` stay as they are. The public metrics decline
  key `unsupported_text` becomes `type_source`, so the control-server JSON key
  changes with it.
