## Why

Roadmap P3 gap G8: the codec preserves the `curv` (Curves) block and
`ADJUSTMENT_KEYS` already lists it (`crates/pictura-codec/src/common.rs:48`),
so a real block is stored on `Layer.adjustment`; but `pictura-render`'s
`decode_adjustment` has no `curv` arm, so a Curves adjustment layer composites
as a no-op (documented in `composite.rs` and asserted by
`deferred_keys_still_none`). `pictura-adjust` already implements
`Adjustment::Curves` and `docs/04-image-ops/adjustments/curves.md` defines the
operation, but the existing `CurvesParams` carries a single composite `points`
curve while Photoshop stores a curve per color channel. The PSD block layout is
grounded on **ag-psd** (`readCurveChannel`/`curv` handler), which reads the
`u16` channel bitmask; psd-tools' `Curves.read` reads only the version-1
prefix (`BUF("BHI")`, `version in (1, 4)`) and treats the duplicate `Crv `
version-4 section as an opaque marker. The missing pieces are the decode, a
per-channel model and kernel, an encoder so the block can round-trip, a
committed fixture plus an ag-psd oracle, and the drop of `curv` from the
deferred set.

## What Changes

- **Codec classification needs no code change.** `*b"curv"` is already in
  `ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:48`) and the codec
  round-trip whitelist already carries a `curv` case
  (`crates/pictura-codec/src/tests.rs:440`). The block is already preserved and
  exercised by read/write equality; `common.rs` and `tests.rs` are unchanged.
- `pictura-adjust`'s `CurvesParams` (`crates/pictura-adjust/src/types.rs:19`)
  gains `red`, `green`, and `blue` as `Option<Vec<(u8, u8)>>` alongside the
  existing composite `points`. The kernel (`crates/pictura-adjust/src/tonal.rs:40`)
  SHALL build a LUT per present per-channel curve, apply each to its own color
  plane, then apply the composite `points` LUT to all three color channels. The
  order is a **documented, marked assumption** (`ponytail:` ceiling): it is not
  provable without Photoshop. Existing constructors and tests are updated
  mechanically; `points`-only behavior is unchanged.
- `crates/pictura-render/src/curves.rs` (new, mirroring `channel_mixer.rs`)
  gains `decode_curves(&[u8]) -> Option<Adjustment>` and
  `pub fn encode_curves(...) -> AdjustmentData`. The decode arm is wired in
  `composite.rs` (`b"curv"`); the doc-comment key list moves `curv` from the
  deferred to the committed set, and `tests/adjustment.rs` drops `curv` from
  `deferred_keys_still_none`.
  - The payload is a `u8` ignored byte, a `u16` version (must be 1), a `u16`
    ignored word, and a `u16` channel bitmask (`1` rgb, `2` red, `4` green, `8`
    blue); for each set bit, in the order rgb, red, green, blue, a `u16` node
    count followed by that many `(i16 output, i16 input)` pairs.
  - A bitmask with an unknown bit or no channels, a node count outside
    `2..=14`, a coordinate outside `0..=255`, or non-strictly-increasing
    inputs returns `None`. The version-4 duplicate `Crv ` section and all
    trailing bytes are ignored.
- `pictura-render` gains `encode_curves(...)`, re-exported from the crate root,
  that writes the version-1 bitmask block the decoder reads and omits the
  duplicate `Crv ` section.
- A committed fixture `crates/pictura-codec/tests/fixtures/curves.psd` carries a
  composite-only `curv` block and a per-channel `curv` block with fixed values,
  authored in `scripts/generate-fixtures.py` from **hand-built raw bytes**
  (including the version-4 duplicate section, so the ignore path is exercised).
  An ag-psd oracle reads the fixture with `node` + the `ag-psd` npm package and
  asserts `rgb`/`red`/`green`/`blue`; it self-skips when `node`/`ag-psd` is
  absent. A cheap psd-tools partial check (version, bitmask) runs whenever
  psd-tools is present. Regeneration is byte-stable.
- The app is **unchanged**: this change is decode/render only. No
  `pictura-app` bridge kind, panel entry, or self-test check is added; the
  destructive `Image > Adjustments > Curves` path already exists separately and
  authoring a Curves adjustment layer is a deferred follow-up.
- Tests prove the decode no longer no-ops, malformed payloads are rejected, the
  encoder round-trips, the per-channel kernel changes only its channel, and an
  ag-psd read of the fixture reports the authored curves.
- **BREAKING**: none at the file-format level; `CurvesParams` gains fields, so
  struct literals in the engine and app are updated mechanically. No dependency
  changes; no codec layout change.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `adjustment-layer-rendering`: the committed-decode requirement adds `curv`,
  the deferred-payload requirement drops it, and new requirements cover the
  `curv` encoder and its ag-psd oracle.
- `image-adjustments`: the `Curves adjustment` requirement gains optional
  per-channel curves applied before the composite curve.

## Impact

- `crates/pictura-adjust/src/types.rs`: `CurvesParams` gains optional
  per-channel curves.
- `crates/pictura-adjust/src/tonal.rs`: the `curves` kernel applies the
  per-channel curves then the composite curve.
- `crates/pictura-adjust/src/tests.rs`: existing `CurvesParams` literals updated
  mechanically; new per-channel kernel tests.
- `crates/pictura-render/src/curves.rs`: new decode/encode module.
- `crates/pictura-render/src/composite.rs`: the `curv` match arm and the
  doc-comment key list (keeping the file inside its size budget).
- `crates/pictura-render/src/lib.rs`: the `encode_curves` re-export.
- `crates/pictura-render/src/tests/adjustment.rs`: decode, malformed-rejection,
  encoder round-trip, fixture-decode, per-channel composite tests; drop `curv`
  from `deferred_keys_still_none`.
- `crates/pictura-render/tests/oracle/adjustment_oracle.rs`: a psd-tools partial read
  of the fixture's `curv` block (optional, always-run-with-psd-tools).
- `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: a second ag-psd oracle over the
  new fixture (self-skips without `node`/`ag-psd`).
- `crates/pictura-codec/tests/fixtures/curves.psd`: new committed fixture.
- `scripts/generate-fixtures.py`: a `_curv_data` raw-byte helper and the
  `curves` fixture builder; register `"curves.psd": curves` in `FIXTURES`.
- `crates/pictura-adjust`: per-channel curve validation uses the existing op
  contract (2..=14 points, strictly increasing inputs).
- GPU path: `gpu/mod.rs::adjustment_params` has no Curves shader, so a document
  containing one continues to fall back to the CPU composite. A `curv` layer
  previously decoded to `None` and was rejected; it now decodes and is still
  rejected by `adjustment_params`, so the fallback outcome is identical.
- `crates/pictura-app`: unchanged.

## Out of scope (deferred)

- **The legacy `is_map` 256-byte bitmap curve variant.** The decoder supports
  only the node-list form; an `is_map` block stays unread.
- **The duplicate `Crv ` version-4 section.** It is ignored on read and not
  written.
- **Selective Color (`selc`), Color Lookup (`clrL`), and a version-3 `phfl`.**
  They stay no-ops.
- **CMYK / Lab / Grayscale per-channel semantics and Pencil-mode smoothing.**
  The engine op and this decode are RGB only, matching the document modes the
  codec reads.
- **A Curves GPU shader and an app authoring path (panel entry / self-test).**
  Documents with one keep falling back to the CPU path, as today.
- **Pixel parity against Photoshop.** The decoding is structural; the render
  contract belongs to `image-adjustments`/`pictura-adjust`, and the existing
  no-ImageMagick-equivalent caveat is unchanged.
