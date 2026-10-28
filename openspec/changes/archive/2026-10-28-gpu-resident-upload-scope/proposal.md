# Proposal

## Why

GPU resident uploads are keyed by the composite region, so every present during a
stroke adds a new entry that no later composite can hit. The store grows toward its
2 GB budget and evicts the whole-document uploads the next full composite needs, so
a stroke leaves every subsequent full composite re-uploading every layer (#975).

## What Changes

- Key a layer's source upload by its content stamps, layer rect, document size,
  and grayscale flag instead of the composite region. A region composite looks up
  that document-scoped key, so an unchanged layer hits the upload a previous full
  composite kept; on a miss it still assembles only the region it needs.
- Retain an upload only when the composite covers the whole document. Region-sized
  source and mask uploads are built and dropped, because their bytes are
  addressable only by that region.
- Stop retaining constant-coverage masks. They are rebuilt row-wise from the
  layer geometry, and keeping one byte per pixel per maskless layer crowds the
  source budget.
- Keep sampled masks cached only for whole-document composites. The shader reads
  the mask by region-local index, so a document-sized mask cannot serve a region
  composite without a shader layout change (out of scope).
- Bound the resident store by bytes and entry count, with a keyed lookup and a
  running byte total. Build but do not retain an upload larger than the byte
  budget. Do not add a dependency.
- Expose `resident_entries` and `resident_bytes` gauges through
  `pictura_render::metrics::Counters` and the app metrics JSON.
- Add an ignored device test in the `gpu-parity` job that proves 100 moving region
  composites hit the retained sources rather than growing the store, that a keyed
  sub-region miss retains nothing, and that a following full composite hits every
  unchanged layer, plus resident-store unit tests that need no device.

Composite output is unchanged: the source keys guard byte identity, the shader
still samples sources by document pixel, and the existing parity suites must stay
within ±1 LSB (region slices byte-identical).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/gpu-transfer`: the resident-upload requirement changes from
  region/geometry keys with least-recently-used eviction to document-scoped
  source keys, whole-document-only retention, sampled-mask-only coverage, and a
  bounded insertion-order store; adds resident usage gauges.

## Impact

- `crates/pictura-render/src/gpu/resident.rs`: the key shapes, the store, and its
  unit tests.
- `crates/pictura-render/src/gpu/backend.rs`: `build_source` looks up the
  document-scoped key and chooses the layout for a hit or a region-sized build;
  `build_mask` keeps coverage only for whole-document composites.
- `crates/pictura-render/src/gpu/mod.rs`: the test-only `run_profile` mask call
  site and the module docs.
- `crates/pictura-render/src/metrics.rs` and the app metrics JSON: two gauges.
- `crates/pictura-render/tests/gpu_parity/region.rs`: stamped region scenes and
  the resident device test.
- No new dependencies. Additive `Counters` fields; the compositor API is
  unchanged. No app-side caching.
