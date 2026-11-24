# Proposal

## Why

Adding or changing one adjustment still fans out across five parallel tables
that encode the same facts: dialog defaults, new-layer defaults, the direct
command table, the GPU kind ids, and the decoded-block kind map. Recent work
paid that cost. Color Lookup touched 19 files and Match Color 22. (The
Shadows/Highlights work landed the whole remaining adjustment family in one
commit, so its file count is not a per-kind figure.) The duplication is not
theoretical: a new Color Lookup layer's block stores the name `"Identity.CUBE"`,
which is not one of the built-in preset names, so `color_lookup_preset_index`
finds no preset and the block and the preset state diverge. The Properties page
falls back to the `None` default, which hides the drift from every existing test.
The GPU ids have no shared authority either, so a mismatch between the host table
and the WGSL branches silently drops the adjustment instead of failing.

## What Changes

- Introduce one typed adjustment kind for the document model's adjustment and
  fill taxonomy, so a kind is a compile-time value rather than a string in five
  places.
- Extend the engine adjustment catalogue so a kind's facts have one home. Each
  row carries its dialog-default builder, new-layer-default builder, direct
  constructor, and GPU route alongside the title, menu, and panel facts.
- Delete the duplicate tables: the app's sixteen-arm layer-default match, the
  codec's kind-keyed default matches, the six-arm direct table, the id half of
  the GPU kind map, the independent inline-else-LUT capability probe in the GPU
  plan, and the hand-written decoded-block kind maps in the shell and preset
  store.
- Generate `AdjustmentKind` and `ALL` from one declaration. Exhaustive conversion
  and GPU parameter matches make omitted classifications fail to compile, while
  a bidirectional catalogue test catches a missing row. The C++ dialog factory's
  if/else chain with a generic fallback stays runtime-only.
- Fix the Color Lookup drift: a new layer's block names the `None` preset, so
  its stored name resolves to `COLOR_LOOKUP_PRESETS[0]` and matches the dialog
  default.
- Drive the GPU parity case list's default-payload rows from the catalogue's GPU
  routes instead of a hand list, so a new GPU-applicable kind is covered
  automatically at its declared default. The non-default LUT cases (hand-built
  levels/exposure payloads and the curves/gradient-map fixtures) stay.

Deliberately not merged: dialog defaults and new-layer defaults stay distinct
facts. Their divergence is user-visible and intended (Brightness 20 vs 0, Hue
30 vs 0, Photo Filter's colour), so the change makes the divergence explicit
and tested rather than collapsing it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/adjustment-catalogue`: the catalogue gains row-backed default
  builders, a direct constructor, and a GPU route per entry. Generated `ALL`
  and bidirectional row coverage keep the taxonomy complete; the embedded kind
  map and preset kind resolution remain one mapping.
- `imaging/adjustment-layers`: a new layer's default block is a declared,
  per-kind fact distinct from the dialog default, and a new Color Lookup layer
  names its preset.

## Impact

- `pictura-core`: the typed kind and its conversions to and from the existing
  adjustment and fill models.
- `pictura-codec`: `layer_content/defaults.rs` exposes per-kind default-block
  builder hooks with unchanged payload bytes.
- `pictura-render`: the catalogue row's `GpuRoute`, the GPU plan's
  inline-versus-LUT eligibility, the preset store's kind mapping, and the parity
  test. `gpu/lut.rs::kind` (table shape 6/7) and
  `pictura_adjust::adjustment_lut` (table construction) are kept.
- `pictura-app`: the new-layer default match, the direct-command table, the
  decoded-kind map, and the control server's hand-listed kinds.
- Tests: `pictura-render` unit tests (row iteration), `pictura-app` Rust unit
  tests, the ignored GPU parity suite, `tst_adjustments_panel`,
  `tst_command_tree`, `tst_image_adjustments`, `tst_control_editing`, and the
  `adjustment_catalogue_*` cases.
- No PSD format, menu order, tile order, shader math, or parameter-range change.
