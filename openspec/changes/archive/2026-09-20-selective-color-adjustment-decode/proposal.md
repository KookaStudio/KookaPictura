## Why

Roadmap P3 gap G8: the codec preserves the `selc` (Selective Color) block and
`ADJUSTMENT_KEYS` already lists it (`crates/pictura-codec/src/common.rs:49`), so
a real block is stored on `Layer.adjustment`; but `pictura-render`'s
`decode_adjustment` has no `selc` arm, so a Selective Color adjustment layer
composites as a no-op (documented in `composite.rs` and asserted by
`deferred_keys_still_none`). `pictura-adjust` has no Selective Color op at all,
even though `docs/04-image-ops/adjustments/selective-color.md` (`ADJ-023`)
defines the operation and the codec already classifies the key. The block layout
was previously ambiguous; it is now settled by a three-way agreement (libpsd,
ag-psd, psd-tools — see the design). The missing pieces are the engine op and
kernel, the decode, an encoder so the app can author the layer, a committed
fixture plus independent oracles, and the drop of `selc` from the deferred set.

## What Changes

- **Codec classification needs no code change.** `*b"selc"` is already in
  `ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:49`). The codec
  round-trip whitelist (`crates/pictura-codec/src/tests.rs:432`) does **not**
  yet carry a `selc` case, so one is added to exercise read/write equality.
- `pictura-adjust` gains `Adjustment::SelectiveColor(SelectiveColorParams {
  method: SelectiveColorMethod, ranges: [SelectiveRange; 9] })`, where each of
  the nine ranges (reds, yellows, greens, cyans, blues, magentas, whites,
  neutrals, blacks, in that order) holds `c`/`m`/`y`/`k` `i16` corrections in
  `-100..=100`. A kernel implements libpsd's integer pipeline: profile-free
  RGB→CMYK→RGB, the six hue families with a 30-wide core and a 30 feather each
  side, and whites/neutrals/blacks selected by the black ink. An all-zero set is
  the exact identity. The conversion's imprecision is marked a `ponytail:`
  ceiling and no Photoshop pixel parity is claimed.
- `crates/pictura-render/src/selective_color.rs` (new, mirroring
  `curves.rs`/`color_balance.rs`, keeping `composite.rs` inside budget) gains
  `decode_selective_color(&[u8]) -> Option<Adjustment>` and
  `pub fn encode_selective_color(...) -> AdjustmentData`. The decode arm is
  wired in `composite.rs` (`b"selc"`); the doc-comment key list moves `selc`
  from the deferred to the committed set, and `tests/adjustment.rs` drops `selc`
  from `deferred_keys_still_none`.
  - The payload is a `u16` version (must be 1), a `u16` correction method (`0`
    relative, nonzero absolute), and **10 plates × 4 `i16`** corrections (cyan,
    magenta, yellow, black). Plate 0 is Photoshop's reserved/ignored plate and
    is skipped without validation; plates 1..9 are the nine named ranges. A
    payload shorter than 84 bytes, a version other than 1, or a range correction
    outside `-100..=100` returns `None`. Trailing bytes are ignored.
- A committed fixture `crates/pictura-codec/tests/fixtures/selective_color.psd`
  carries a relative `selc` block and an absolute `selc` block with fixed
  nine-range values, authored in `scripts/generate-fixtures.py` with psd-tools'
  `SelectiveColor` (its 10-plate read/write framing is correct). An ag-psd
  oracle reads the fixture with `node` + the `ag-psd` npm package and asserts
  the method and the nine named ranges; it self-skips when `node`/`ag-psd` is
  absent. A psd-tools partial/field check asserts the version, method, and the
  nine plates it reads framingly. Regeneration is byte-stable and no existing
  golden changes.
- The app can author one: `helpers.rs::adjustment_layer` gains a
  `"selective-color"` arm writing the neutral default (relative method, all nine
  ranges zero), and `panel_group_menu.cpp` gains an `adjustment:selective-color`
  entry under the Adjustments panel menu. The `Selective Color` command-tree
  leaves already exist. One C++ self-test check (**exit code 296**, the next
  free code) verifies the kind, the neutral composite, and the menu row.
- Tests prove the kernel's known values, that decode no longer no-ops, that
  malformed payloads are rejected, that the encoder round-trips, and that an
  ag-psd read of the fixture reports the authored ranges.
- **BREAKING**: none. No dependency changes; no codec layout change.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `adjustment-layer-rendering`: the committed-decode requirement adds `selc`, the
  deferred-payload requirement drops it, and new requirements cover the `selc`
  encoder, its ag-psd oracle, and the app authoring path.
- `image-adjustments`: a new `Selective Color adjustment` requirement, and the
  validation, alpha-preservation, and oracle-classification requirements widen
  to the new destructive variant.

## Impact

- `crates/pictura-adjust/src/types.rs`: `SelectiveColorParams`,
  `SelectiveRange`, `SelectiveColorMethod`, and the `Adjustment` variant.
- `crates/pictura-adjust/src/color.rs`: the `selective_color` kernel plus the
  profile-free RGB→CMYK→RGB and hue helpers.
- `crates/pictura-adjust/src/apply.rs`: the `Adjustment::SelectiveColor` arm.
- `crates/pictura-adjust/src/lib.rs`: the new type re-exports.
- `crates/pictura-adjust/src/tests.rs`: known-value kernel tests and the new
  variant in the alpha-preservation list.
- `crates/pictura-adjust/tests/oracle.rs`: the `SelectiveColor` no-equivalent
  mapping row and the widened `NO_EQUIVALENT` list.
- `crates/pictura-render/src/selective_color.rs`: new decode/encode module with
  its own decode/reject/round-trip tests (kept here because
  `tests/adjustment.rs` is at 1398/1400 LOC).
- `crates/pictura-render/src/composite.rs`: the `selc` match arm and the
  doc-comment key list.
- `crates/pictura-render/src/lib.rs`: the `encode_selective_color` re-export.
- `crates/pictura-render/src/tests/adjustment.rs`: drop `selc` from
  `deferred_keys_still_none`.
- `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: a psd-tools partial read of
  the fixture's `selc` block and a pure-Rust fixture-decode check.
- `crates/pictura-codec/src/tests.rs`: a `selc` case in the adjustment
  round-trip whitelist.
- `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: a new ag-psd oracle over the
  fixture (self-skips without `node`/`ag-psd`).
- `crates/pictura-codec/tests/fixtures/selective_color.psd`: new committed
  fixture.
- `scripts/generate-fixtures.py`: a `selective_color` builder and its
  registration in `FIXTURES`.
- `crates/pictura-app/src/cxxqt_object/helpers.rs`: a `"selective-color"` arm in
  `adjustment_layer`.
- `crates/pictura-app/cpp/panels/panel_group_menu.cpp`: the
  `adjustment:selective-color` menu entry.
- `crates/pictura-app/cpp/selftest_layers_adjustments.cpp`: one new
  `lpr_selective_color` check (exit code 296); the file is at 156 LOC and has
  ample budget.
- `crates/pictura-app/cpp/command_tree.cpp`: unchanged — the
  `Image > Adjustments > Selective Color` and
  `Layer > New Adjustment Layer > Selective Color` leaves already exist
  (`command_tree.cpp:221`, `:346`).
- GPU path: `gpu/mod.rs::adjustment_params` has no Selective Color shader, so a
  document containing one keeps falling back to the CPU composite.

## Out of scope (deferred)

- **The reserved plate 0.** It is skipped on read and written as zeroes.
- **CMYK / Lab / Grayscale document modes and a profile-based CMYK path.** The
  kernel is profile-free RGB, matching the document modes the codec reads; the
  working-space-dependent path in `docs/04-image-ops/adjustments/selective-color.md`
  is not implemented.
- **Photoshop family-membership curves.** The libpsd hue/tonal windows are an
  approximation, marked as such; a CS6 calibration sweep would be needed for
  parity.
- **Photo Filter version 3 (`phfl`), Color Lookup (`clrL`).** They stay no-ops.
- **A Selective Color GPU shader and an authoring UI beyond the neutral
  default.** Editing the nine-by-four grid is a separate UI change.
- **Pixel parity against Photoshop.** The decoding is structural; the render
  contract belongs to `image-adjustments`/`pictura-adjust`.
