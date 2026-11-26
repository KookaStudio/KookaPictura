## Why

Roadmap P3 gap G8: the codec preserves the `mixr` (Channel Mixer) block and
`ADJUSTMENT_KEYS` already lists it, so a real block is stored on
`Layer.adjustment`; but `pictura-render`'s `decode_adjustment` has no `mixr`
arm, so a Channel Mixer adjustment layer composites as a no-op. `pictura-adjust`
already implements `Adjustment::ChannelMixer` and
`docs/04-image-ops/adjustments/channel-mixer.md` defines the operation. The
block layout is **not** what psd-tools models: psd-tools' `ChannelMixer.read`
reads only `2H` + `5h` (the red row) and hides the rest in `unknown`, so it
cannot author a faithful block. The independent ground truth is ag-psd, whose
`readMixrChannel`/`mixr` handler and `ChannelMixerChannel` define the full
4-channel layout. The missing pieces are the decode, an encoder so the app can
create a layer, a committed fixture plus an ag-psd oracle, and the panel wiring.

## What Changes

- **Codec classification needs no code change.** `*b"mixr"` is already in
  `ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:49`), so the block is
  already classified. The codec round-trip test's case list
  (`crates/pictura-codec/src/tests.rs:432`) gains a `mixr` payload so the
  whitelist is exercised by read/write equality.
- `crates/pictura-render/src/channel_mixer.rs` (new, mirroring
  `color_balance.rs`) gains `decode_channel_mixer(&[u8]) -> Option<Adjustment>`
  and `pub fn encode_channel_mixer(...) -> AdjustmentData`. The decode arm is
  wired in `composite.rs`; the doc-comment key list moves `mixr` from the
  deferred to the committed set, and `tests/adjustment.rs` drops `mixr` from its
  deferred-key assertion.
  - The payload is a `u16` version (must be 1), a `u16` monochrome flag, then
    the channels: when not monochrome `red`, `green`, `blue`, and always `gray`.
    Each channel is three big-endian `i16` source percentages, two reserved
    bytes, and one big-endian `i16` constant percentage.
  - Non-monochrome maps the three RGB rows to `red`/`green`/`blue` and their
    constants to `constant`; monochrome maps the `gray` row to `red` and its
    constant to `constant[0]` (the op reads only those). A version other than 1,
    a payload shorter than its declared channels, or any percentage/constant
    outside `-200..=200` returns `None`. Trailing bytes are ignored.
- `pictura-render` gains `encode_channel_mixer(...)`, re-exported from the crate
  root, that clamps every weight to `-200..=200` and builds the block the
  decoder reads.
- A committed fixture `crates/pictura-codec/tests/fixtures/channel_mixer.psd`
  carries a non-monochrome and a monochrome `mixr` block with known values,
  authored in `scripts/generate-fixtures.py` from **hand-built raw `mixr`
  bytes** (psd-tools passes raw `bytes` through verbatim; its typed writer is
  wrong). An ag-psd oracle reads the fixture with `node` + the `ag-psd` npm
  package and asserts `monochrome`/`red`/`green`/`blue`/`gray`; it self-skips
  when `node`/`ag-psd` is absent. A cheap psd-tools partial check (version,
  monochrome, red row) runs whenever psd-tools is present.
- The app can create one: `helpers.rs::adjustment_layer` gains a
  `"channel-mixer"` arm authoring the neutral identity default (monochrome off;
  `[100,0,0]`/`[0,100,0]`/`[0,0,100]`; constants `[0,0,0]`), and
  `panel_group_menu.cpp` gains an `adjustment:channel-mixer` entry under the
  Adjustments panel menu. The `Channel Mixer` command-tree leaves already exist.
- Tests prove the decode no longer no-ops, malformed payloads are rejected, the
  encoder round-trips, and an ag-psd read of the fixture reports the authored
  fields.
- **BREAKING**: none. No dependency changes; no `pictura-adjust` change.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `adjustment-layer-rendering`: the committed-decode requirement adds `mixr`,
  the deferred-payload requirement drops it, and new requirements cover the
  `mixr` encoder, its ag-psd oracle, and the app authoring path.

## Impact

- `crates/pictura-codec/src/tests.rs`: a `mixr` case in the adjustment
  round-trip whitelist.
- `crates/pictura-codec/tests/fixtures/channel_mixer.psd`: new committed fixture.
- `scripts/generate-fixtures.py`: a `_mixr_data` raw-byte helper and the
  `channel_mixer` fixture builder.
- `crates/pictura-render/src/channel_mixer.rs`: new decode/encode module.
- `crates/pictura-render/src/composite.rs`: the `mixr` match arm and the
  doc-comment key list.
- `crates/pictura-render/src/lib.rs`: the `encode_channel_mixer` re-export.
- `crates/pictura-render/src/tests/adjustment.rs`: decode, malformed-rejection,
  encoder round-trip, fixture-decode, and composite tests; drop `mixr` from the
  deferred set.
- `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: a psd-tools partial read
  of the fixture's `mixr` block (optional, always-run-with-psd-tools).
- `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: new ag-psd oracle over the
  fixture (self-skips without `node`/`ag-psd`).
- `crates/pictura-app/src/cxxqt_object/helpers.rs`: a `"channel-mixer"` arm in
  `adjustment_layer`.
- `crates/pictura-app/cpp/panels/panel_group_menu.cpp`: the
  `adjustment:channel-mixer` menu entry.
- `crates/pictura-app/cpp/selftest_layers_adjustments.{cpp,h}`: a new
  sub-runner holding the adjustment checks moved out of
  `selftest_layers_controls.cpp` (which is at 1185/1200 and cannot take another
  check) plus the new `lpr_channel_mixer` check (exit code **294**). Registered
  in `CMakeLists.txt` and called from `selftest_layers_controls.cpp`.
- `crates/pictura-app/cpp/command_tree.cpp`: unchanged — the
  `Image > Adjustments > Channel Mixer` and
  `Layer > New Adjustment Layer > Channel Mixer` leaves already exist
  (`command_tree.cpp:215`, `:339`).
- GPU path: `gpu/mod.rs::adjustment_params` has no Channel Mixer shader, so a
  document containing one continues to fall back to the CPU composite. Only the
  CPU path gains decoding.

## Out of scope (deferred)

- **Curves (`curv`), Selective Color (`selc`), Color Lookup (`clrL`), and a
  version-3 `phfl`.** They stay no-ops; their models are ungrounded or have no
  committed op.
- **CMYK / non-RGB mixer semantics.** The engine op and this decode are RGB
  only, matching the document modes the codec reads.
- **A Channel Mixer GPU shader.** Documents with one keep falling back to the
  CPU path, as today.
- **Pixel parity against Photoshop.** The decoding is structural; the render
  contract belongs to `image-adjustments`/`pictura-adjust`, and the existing
  IM-based op caveat is unchanged.
