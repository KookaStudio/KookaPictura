# Proposal

## Why

The CPU compositor is the oracle and the fallback whenever the GPU declines a
stack. #1016 hoisted the per-layer constants, mask gates, and native planes, and
#1092 resolves the whole stack once per composite, but the inner loop still
evaluates per-pixel work that cannot change within a layer: a blend-mode `match`
per pixel, mask-gate dispatch and bounds checks per pixel, `Option` checks on
every channel sample, and up to five `u8` to `f32` true divisions per pixel.
#1016 deferred kernel specialisation until the plan existed and a measurement
asked for it; #1092 has landed, so the remaining cost is the loop itself.

## What Changes

- Select a row-span kernel per layer pass for the common case: 8-bit
  channel-plane source (no native samples), Normal blend mode, no Blend If, no
  text-gamma curve, no knockout cover, and either no mask or a mask resolved to
  a per-row coverage slice. The fast path applies to plain pixel content and to
  a smart-object or type layer composited from its raster proxy. The existing
  per-pixel kernel stays as the general fallback and the byte-identity
  reference.
- Ship three kernel shapes: a masked kernel, an alpha kernel, and a divide-free
  opaque kernel. The masked and alpha shapes keep the general formula's
  operation order and its three `ao` divisions; the opaque shape is selected
  only when the layer has no alpha plane and `passes_alpha()` holds.
- Materialise a mask's per-row coverage into an `f32` slice only for pixels
  whose source alpha after opacity and fill is greater than zero; skipped
  pixels stay `0.0` so the kernel's `as_ <= 0` skip leaves them unchanged. The
  slice uses the same gate evaluation as the per-pixel path, so plain,
  feathered, vector, and native masks stay byte-identical; the native mask
  keeps its `f32` precision.
- Replace the hot `u8 as f32 / 255.0` conversions with a shared 256-entry unit
  table in the general and fast channel-plane/mask sampling paths, producing
  the same bits without a division. Native `SampleRow` paths are out of scope;
  the fast set excludes native stores.
- Add a test-only `FORCE_GENERAL` switch beside `FORCE_SERIAL` in
  `composite_rows.rs` (the legacy switches stay in `composite_blend.rs`), and a
  parity matrix that forces the general path and compares final
  `PixelBuffer` bytes.
- Extend the ignored 4000² profile with kernel-eligible cases and record
  release before/after timings in the PR.
- No public API, compositor interface, output bytes, or GPU parity change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/cpu-composite-kernel`: the inner-loop requirement gains the
  per-pass row-span kernel selection, the per-row mask coverage slice, the
  shared unit table, the forced-general oracle, the override-selection
  scenario, and the requirement that every selected kernel stays byte-identical
  to the general path. The existing `One blend formula` requirement is modified
  to permit derived row-span kernels held to that parity.

## Impact

- `crates/pictura-render/src/composite_rows.rs` — kernel selection and the
  row-span loops.
- `crates/pictura-render/src/composite_blend.rs` — expose the effective blend
  state and resolve the per-row active-pixel mask coverage from the existing
  gates.
- `crates/pictura-render/src/composite.rs` — the shared `u8` unit table.
- `crates/pictura-render/src/tests/` — the parity matrix, the override and
  switch-composition tests, and the forced-general switch.
- `crates/pictura-render/examples/composite_profile.rs` — the kernel-eligible
  target profile cases.
- No new dependencies. No `docs/` change.
