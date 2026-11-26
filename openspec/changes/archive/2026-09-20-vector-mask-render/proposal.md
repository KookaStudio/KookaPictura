## Why

Roadmap P3 gap G6: the codec preserves the per-layer `vmsk` (vector mask) block
opaquely, and nothing renders it, so a shape layer — typically a solid/gradient
fill layer clipped by a `vmsk` — composites as a full-canvas fill instead of its
shape. This is the remaining grounded P3 "vector" kind. The block layout is
grounded on two independent implementations (psd-tools `psd/vector.py` and
ag-psd `additionalInfo.ts` `readVectorMask`) plus Adobe's published path-record
format, and the raster mask machinery the compositor already uses
(`mask_alpha`, the GPU mask plane) is the right place to fold it in.

## What Changes

- **Core model gains a derived vector-mask view.** `pictura-core::Layer` gains
  `vector_mask: Option<VectorMask>` (`VectorMask { subpaths, invert, disabled }`,
  `VectorSubpath { closed, operation, fill_rule, points }`). It is a read-derived
  view, not serialized: the raw `vmsk` block stays in `Layer.extra_blocks` and is
  re-emitted verbatim, so open→save is unchanged byte-for-byte.
- **A new codec module decodes and flattens `vmsk`.** A `resolve_vector_masks`
  post-pass (mirroring `resolve_smart_objects`) reads the preserved `vmsk` block,
  parses the version/flags header and the 26-byte path records, scales the 8.24
  fixed-point knots to document pixels, flattens each closed subpath's cubic
  segments to a polyline, and sets `Layer.vector_mask`.
- **A new render module samples the coverage.** The compositor multiplies the
  layer's source alpha by the vector coverage, combined with the existing raster
  mask. Coverage is even-odd by default (Adobe: "Paths use even/odd ruling"),
  non-zero only when the subpath's fill-rule field is ag-psd's marker `2`; it
  honors the `invert` flag and a `disable`d mask is a no-op.
- **The GPU path picks it up.** `mask_has_data` treats a live vector mask as
  mask data, so the GPU builds the existing per-pixel mask plane from the CPU
  `mask_alpha` and stays within the existing ±1 LSB CPU parity.
- **Fixture + oracles + render test.** A committed `vector_mask.psd` carries a
  closed rectangle path on a solid-fill layer and a second layer with the invert
  flag; psd-tools and ag-psd read it as independent structural oracles, and a
  render test proves the mask clips a fill layer.
- **No app change, no new dependency, no `docs/` change.** The app renders
  through the existing composite path, so no bridge/CPP/self-test code is needed.
- **BREAKING**: none. An existing file with a `vmsk` previously ignored now
  renders clipped; the block is still preserved byte-for-byte.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `psd-layer-io`: adds the `vmsk` decode into a derived view, its fixed-point
  scaling and fill-rule grounding, the malformed-block behavior, and the
  independent fixture oracles.
- `layer-compositing`: adds the vector-mask coverage requirement (multiplied
  with the raster mask, invert/disable, GPU mask-plane inclusion).

## Impact

- `crates/pictura-core/src/vector.rs` (new): `VectorMask`, `VectorSubpath`,
  `VectorFillRule`; re-exported from `crates/pictura-core/src/lib.rs`.
- `crates/pictura-core/src/lib.rs`: `Layer.vector_mask` field and
  `Layer::default`.
- Every exhaustive `Layer { … }` literal (about 32 sites across `pictura-core`,
  `pictura-codec`, `pictura-render`, `pictura-paint`, `pictura-app` tests):
  add `vector_mask: None`.
- `crates/pictura-codec/src/vector_mask.rs` (new): parse + flatten.
- `crates/pictura-codec/src/lib.rs`: `mod vector_mask;`.
- `crates/pictura-codec/src/read.rs`: call `resolve_vector_masks` after
  `resolve_smart_objects`.
- `crates/pictura-render/src/vector_mask.rs` (new): even-odd/non-zero coverage.
- `crates/pictura-render/src/lib.rs`: `mod vector_mask;`.
- `crates/pictura-render/src/composite.rs`: `mask_alpha` folds in vector
  coverage.
- `crates/pictura-render/src/gpu/backend.rs`: `mask_has_data` includes a live
  vector mask.
- `crates/pictura-render/src/tests/vector_mask.rs` (new) and
  `crates/pictura-render/src/tests/mod.rs`: render + fixture-decode tests.
- `crates/pictura-codec/tests/psd/vector_mask_oracle.rs` (new): psd-tools oracle.
- `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: a `vmsk` fixture test.
- `scripts/generate-fixtures.py`: a `vector_mask()` builder and a `FIXTURES`
  entry.
- `crates/pictura-codec/tests/fixtures/vector_mask.psd` (new committed
  fixture).
- `crates/pictura-app`, `CMakeLists.txt`, `docs/`: unchanged.

## Out of scope (deferred)

- **Open (unfilled) subpaths** contribute no coverage (they are stroke geometry).
- **`vscg` vector fill/stroke content** is ignored. The common shape layer's
  fill/stroke is a `SoCo`/`GdFl`/`PtFl` fill layer, which already renders; `vscg`
  is only needed for the shape's own live content and is deferred.
- **Multi-subpath boolean operations beyond union** (subtract `2`, intersect
  `3`, xor `0`) are not applied; every closed subpath contributes to one
  even-odd/non-zero fill.
- **`vsms` (the alternate vector-mask key)** stays opaque and unrendered.
- **Antialiasing, Density, Feathering**, `InitialFillRule`/Reveal-Hide-All, and
  the "Vector Mask Hides Effects" advanced option are not modeled.
- **Document resize/crop/orientation** do not transform a preserved vector mask.
- **Photoshop pixel parity** is not claimed beyond the oracle fixture; the
  decode is structural.
