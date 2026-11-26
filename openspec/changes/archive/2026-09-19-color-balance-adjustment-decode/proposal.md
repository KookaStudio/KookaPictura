## Why

Roadmap P3 gap G8: the codec preserves the `blnc` (Color Balance) block, but
`ADJUSTMENT_KEYS` does not list it, so the block lands in `extra_blocks` and the
layer's `adjustment` stays `None`; even if it were classified,
`pictura-render`'s `decode_adjustment` has no `blnc` arm, so a Color Balance
adjustment layer composites as a no-op. `pictura-adjust` already implements
`Adjustment::ColorBalance` and `docs/04-image-ops/adjustments/color-balance.md`
defines the operation. psd-tools 1.19 reads and writes the `blnc` fixed struct,
so the payload has an independent reference; the missing pieces are the codec
key, the decode, an encoder so the app can create a layer, and the panel wiring.

## What Changes

- `crates/pictura-codec/src/common.rs` accepts `blnc` in `ADJUSTMENT_KEYS` (20
  keys become 21), so a real Color Balance block is classified as an adjustment
  and preserved as `AdjustmentData` instead of an opaque extra block.
- `decode_adjustment` in `crates/pictura-render/src/composite.rs` gains a
  `decode_color_balance` helper and a `blnc` arm producing
  `Adjustment::ColorBalance(ColorBalanceParams { shadows, midtones, highlights,
  preserve_luminosity })`.
  - The payload is nine big-endian `i16` shifts in band order (shadows,
    midtones, highlights, three each), each in `-100..=100`, followed by a `u8`
    luminosity flag and padding to a 4-byte boundary.
  - A truncated payload or a shift outside `-100..=100` returns `None`; it never
    panics and never errors. The trailing pad is ignored.
- `pictura-render` gains `pub fn encode_color_balance(shadows: [f64; 3],
  midtones: [f64; 3], highlights: [f64; 3], preserve_luminosity: bool) ->
  AdjustmentData`, re-exported from the crate root, that builds the `blnc` block
  the decoder reads and clamps each shift to `-100..=100`.
- The app can create one: `helpers.rs::adjustment_layer` gains a
  `"color-balance"` arm authoring the neutral Photoshop default (all shifts
  zero, luminosity preservation on) and `panel_group_menu.cpp` gains an
  `adjustment:color-balance` entry under the Adjustments panel menu.
- Tests prove the decode no longer no-ops, malformed payloads are rejected, the
  encoder round-trips, and an independent psd-tools read of the encoder's bytes
  reports the same `shadows`/`midtones`/`highlights`/`luminosity`.
- **BREAKING**: none. No fixture changes.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `adjustment-layer-rendering`: the committed-decode requirement adds `blnc`; a
  new requirement covers the `blnc` encoder and its psd-tools parity; another
  covers the app authoring path.

## Impact

- `crates/pictura-codec/src/common.rs`: `blnc` in `ADJUSTMENT_KEYS`.
- `crates/pictura-codec/src/tests.rs`: the round-trip key/bytes cases gain a
  `blnc` payload.
- `crates/pictura-render/src/composite.rs`: `decode_color_balance`,
  `encode_color_balance`, the `blnc` match arm, the doc-comment key list, and
  unit tests.
- `crates/pictura-render/src/lib.rs`: the `encode_color_balance` re-export.
- `crates/pictura-render/src/tests/adjustment.rs`: a Color Balance round-trip,
  a malformed-payload rejection, and a composite test.
- `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: one psd-tools parity test
  over `encode_color_balance`'s bytes (new file).
- `crates/pictura-app/src/cxxqt_object/helpers.rs`: a `"color-balance"` arm in
  `adjustment_layer`.
- `crates/pictura-app/cpp/panels/panel_group_menu.cpp`: the
  `adjustment:color-balance` menu entry.
- `crates/pictura-app/cpp/selftest_layers_controls.cpp`: one new check (exit
  code 293) that the kind is added and reported as an adjustment and that the
  Adjustments panel offers `Color Balance`.
- No new dependency: `pictura-adjust`'s Color Balance op already exists, so no
  `pictura-adjust` change and no `image-adjustments` spec change.
- GPU path: `gpu/mod.rs::adjustment_params` has no Color Balance shader, so a
  document containing one continues to fall back to the CPU composite. Only the
  CPU path gains decoding.

## Out of scope (deferred)

- **Curves (`curv`), Channel Mixer (`mixr`), Selective Color (`selc`), Color
  Lookup (`clrL`), a real Photoshop `SoCo`/`PtFl` descriptor, and a version-3
  `phfl`.** They stay no-ops; their models are ungrounded or have no committed
  op.
- **Per-band `preserve_luminosity` fidelity.** The decoded flag is handed to the
  existing engine op unchanged; how faithfully that op reproduces Photoshop's
  band weighting is owned by `pictura-adjust`.
- **A Color Balance GPU shader.** Documents with one keep falling back to the
  CPU path, as today.
