## Context

`pictura-codec` preserves the `curv` (Curves) block verbatim, and
`ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:48`) already lists
`*b"curv"`, so `is_adjustment_key` is true and the block is stored on
`Layer.adjustment` rather than in `extra_blocks`. The codec's adjustment
round-trip whitelist already carries a `curv` case
(`crates/pictura-codec/src/tests.rs:440`), so read/write equality is already
exercised. Independently, `pictura-render`'s `decode_adjustment`
(`crates/pictura-render/src/composite.rs:338`) has no `curv` arm, so a Curves
adjustment layer composites as a no-op; `deferred_keys_still_none`
(`crates/pictura-render/src/tests/adjustment.rs:882`) asserts exactly that.
`pictura-adjust` already implements the operation: `CurvesParams`
(`crates/pictura-adjust/src/types.rs:19`), `Adjustment::Curves`
(`types.rs:156`), the kernel (`crates/pictura-adjust/src/tonal.rs:40`), and its
tests (`src/tests.rs:153`, `:176`). That op models one composite curve applied
per color channel; Photoshop stores a curve per channel.

The payload layout is grounded on **ag-psd** (`readCurveChannel` at
`additionalInfo.ts:2554` and the `curv` handler at `:2571`, confirmed against
the installed `node_modules/ag-psd` copy):

```
u8    ignored (0)
u16   version == 1
u16   ignored (0)
u16   channel bitmask   (1 = rgb, 2 = red, 4 = green, 8 = blue)
for each set bit, in the order rgb, red, green, blue:
    u16 node count
    node count x (i16 output, i16 input)
then   a duplicate `Crv ` version-4 section (ignored)
then   any trailing bytes (ignored)
```

psd-tools' `Curves.read` (`psd_tools/psd/adjustments.py`) reads the same v1
prefix as `BUF("BHI")` with `version in (1, 4)`; its `count` for a v1 payload is
`bin(count_map).count("1")` — i.e. the low 16 bits of `count_map` are the
bitmask — and its `CurvesExtraMarker` (`4sHI`: `Crv `, `u16` version, `u32`
count) is the duplicate section. ag-psd widens the reader's `output`/`input`
from `i16`, so coordinates are validated to `0..=255`; ag-psd's writer emits the
duplicate section, psd-tools' writer emits it only when `extra` is set. Values
are `0..=255`.

## Goals / Non-Goals

**Goals:**

- Extend `pictura_adjust::CurvesParams` with optional per-channel curves and
  apply a per-channel curve to its own channel before the composite curve, so a
  decoded Curves layer renders through the existing adjustment composite path.
- Decode a `curv` payload into `Adjustment::Curves`, and provide
  `encode_curves` so the block round-trips.
- Commit a fixture carrying known composite-only and per-channel `curv` blocks
  and prove it with an independent ag-psd oracle (plus a cheap psd-tools partial
  check).
- Drop `curv` from the deferred set in the doc comment and the test.

**Non-Goals:**

- The legacy `is_map` 256-byte bitmap curve form. It stays unread.
- The duplicate `Crv ` version-4 section: ignored on read, not written.
- A Curves GPU shader. Documents with one keep falling back to the CPU path.
- App authoring: no bridge kind, panel entry, or self-test check.
- CMYK/Lab/Grayscale per-channel semantics and Pencil-mode smoothing.
- Verified pixel parity against Photoshop. The decoding is structural.
- The other still-deferred keys (`selc`, `clrL`, version-3 `phfl`).

## Decisions

### D1. The `curv` byte layout (ag-psd is the structural reference)

Decode follows ag-psd's v1 block:

```
u8 ignored, u16 version == 1, u16 ignored, u16 bitmask
per set bit (rgb, red, green, blue): u16 count, count x (i16 output, i16 input)
```

The header is 7 bytes; `version` sits at offset 1, the bitmask at offset 5. The
parser reads each present channel's node count and points with the existing
`be_u16`/`be_i16` helpers (`crates/pictura-render/src/composite.rs:392`). It
requires `version == 1`, a nonzero bitmask with no bit outside `0b1111`, a node
count in `2..=14` per present channel, every coordinate in `0..=255`, and
strictly increasing inputs. Any violation is `None`. Every byte after the last
channel is ignored, which subsumes the duplicate `Crv ` version-4 section and
any pad.

The `2..=14` node-count bound is an intended engine-op contract, matching
`curve_lut` in `pictura-adjust`; psd-tools also permits up to 19, so a legacy
file carrying 15–19 points decodes to `None` (a no-op), which is the recorded
ceiling rather than a parity claim.

**Why the v1 bitmask and not the `is_map` form?** The `is_map` form is an
undocumented 256-byte bitmap lookup that ag-psd itself only references in a
commented-out block; no fixture exercises it. The node-list form is what
Photoshop writes for a normal Curves layer and what both oracles read, so it is
the honest supported set.

**Why reject rather than clamp the duplicate `Crv ` section?** The duplicate is
not consumed at all; it is skipped as trailing bytes. Only the v1 section is
parsed.

### D2. Mapping a decoded block to `Adjustment::Curves`

The composite `points` are the `rgb` channel's points, converted from the disk
`(output, input)` pair order to the model's `(input, output)` order. `red`,
`green`, and `blue` are `Some(points)` when the corresponding bit is set and
`None` otherwise. When the `rgb` bit is clear (a per-channel-only block),
`points` is the identity endpoints `[(0, 0), (255, 255)]`, so the composite
stage is a no-op and only the per-channel curves change the image; the kernel
requires a composite curve because `points` stays a required field.

### D3. Extend `CurvesParams` with optional per-channel curves

```rust
pub struct CurvesParams {
    pub points: Vec<(u8, u8)>,
    pub red: Option<Vec<(u8, u8)>>,
    pub green: Option<Vec<(u8, u8)>>,
    pub blue: Option<Vec<(u8, u8)>>,
}
```

`points` remains the composite RGB curve, so every existing constructor and
test is updated mechanically with `red: None, green: None, blue: None` and the
existing behavior is bit-identical. The change is small (three `Option` fields)
and keeps one struct rather than a parallel type.

### D4. Kernel order: per-channel curves, then the composite curve

The kernel applies each present per-channel curve to its own plane, then applies
the composite `points` LUT to all three planes:

```
for (plane, curve) in [(r, red), (g, green), (b, blue)]:
    if curve: map plane through LUT(curve)
map all three planes through LUT(points)
```

`planes_mut` (`crates/pictura-adjust/src/common.rs:34`) splits the three color
planes so a single plane can be remapped; the composite LUT reuses the existing
`map_lut` (`common.rs:41`). Building a LUT is factored out of the current
`curves` body so both stages share the monotone-Hermite code and the 2..=14 /
strictly-increasing validation.

**ponytail: the per-channel-then-composite order is an assumption.** Photoshop's
documented model applies the composite curve to the already channel-adjusted
result (the composite is a per-channel curve applied to all channels), but the
composition order is not published and there is no real Photoshop fixture with a
`curv` block to check it against. This is recorded as a ceiling, not a verified
parity claim; revisit if a CS6/CC Curves baseline appears.

### D5. New `curves` module, wired from `composite.rs`

Decode and encode live in a new `crates/pictura-render/src/curves.rs` module,
mirroring `channel_mixer.rs`, with the arm
`b"curv" => crate::curves::decode_curves(&data.data)` added to
`decode_adjustment`. `composite.rs` is already near the 1200 cap, and the
`blnc`/`mixr` codec pairs set the module precedent; keeping the new logic in a
sibling keeps `composite.rs` inside budget. `encode_curves` is re-exported from
`lib.rs` beside `encode_channel_mixer`.

### D6. The encoder writes the version-1 bitmask section only

`encode_curves(points: &[(u8, u8)], red: Option<&[(u8, u8)]>, green:
Option<&[(u8, u8)]>, blue: Option<&[(u8, u8)]>) -> AdjustmentData` writes the
`u8`/`u16`/`u16`/`u16` header, sets a bit for each present non-empty channel,
then writes each present channel's `u16` count and `(output, input)` pairs in
the order rgb, red, green, blue. It does not write the duplicate `Crv `
version-4 section — our decoder and both oracles accept the v1-only block, and
reproducing Photoshop's duplicate is deferred. Coordinates are already `u8`, so
no clamping is needed; the encoder writes the curves it is given and the
round-trip contract is defined for the op-valid form (2..=14 strictly-increasing
points).

**Alternative considered:** have the encoder emit an ag-psd-shaped duplicate
section. Rejected as scope: nothing consumes it, and writing extra bytes would
need its own oracle.

### D7. Fixture with hand-built raw bytes; ag-psd is the oracle

`scripts/generate-fixtures.py` gains a `_curv_data(...)` helper that hand-builds
the bytes (`struct.pack(">BHHH", 0, 1, 0, bitmask)`, then per set bit
`struct.pack(">H", n)` and per node `struct.pack(">hh", output, input)`), then
appends a duplicate `Crv ` version-4 section (signature, version 4, a `u16`
zero, the channel count, and each channel) so the ignore path is exercised, and
a `curves()` builder with a Base layer plus:- `Curves` (composite-only): bitmask `1`, points `(0,0)`, `(64,32)`, `(192,224)`,
  `(255,255)` written as `(output, input)`.
- `Curves Channels` (per-channel): bitmask `1|2|4|8`; rgb identity `(0,0)`,
  `(255,255)`; red `(0,0)`,`(128,255)`,`(255,255)`; green `(0,0)`,`(64,16)`,
  `(255,255)`; blue `(0,255)`,`(255,0)`.

Passing a plain `bytes` as `TaggedBlock.data` stores it verbatim (the
`channel_mixer` fixture established this), so the block round-trips through
psd-tools byte-for-byte. The duplicate marker is three `u16`s (`>HHH`), not
`struct.pack(">HHI", …)`: psd-tools reads it as `4sHI` (version + `u32` count),
so its `u32` count is the `u16` zero followed by the `u16` channel count.
Confirmed empirically: reopening the fixture parses the v1 prefix
(`version=1`, `count_map=1`/`15`) and the duplicate as a `CurvesExtraMarker`
(`version=4`) carrying the authored channel items, and regeneration is
byte-stable.

The oracle in `crates/pictura-codec/tests/psd/agpsd_oracle.rs` adds a second test
that runs `node -e "<script>" <fixture>` with
`require('ag-psd').readPsd(bytes, { skipLayerImageData: true,
skipCompositeImageData: true })` and prints one whitespace-separated line per
`curves` adjustment child (each channel's node count and `output,input` pairs,
in rgb/red/green/blue order). Rust parses the tokens (no JSON, no new dependency)
and asserts the authored curves. The test self-skips with a clear message when
`node` or `ag-psd` is unavailable, exactly like the psd-tools/magick oracles.
`.github/workflows/ci.yml`'s `oracles` job already installs `ag-psd`, so no CI
change is needed.

### D8. A cheap psd-tools partial check always runs

`crates/pictura-render/tests/oracle/adjustment_oracle.rs` gains a third test that feeds
the fixture's composite-only `curv` bytes to
`psd_tools.psd.adjustments.Curves.read` and asserts `version == 1`, the
`count_map` low-16-bit bitmask, and the first curve's points equal the authored
rgb points. This is honest about psd-tools' limits (it does not name the
per-channel rows) and runs whenever psd-tools is present; ag-psd carries the
full-field proof. Because the fixture includes the duplicate `Crv ` section that
psd-tools knows as `CurvesExtraMarker`, the read also exercises that framing.

### D9. The app is unchanged (decode/render only)

No `crates/pictura-app` change is made. There is no Curves adjustment-layer
authoring path today, and adding one (a `"curves"` bridge arm, an Adjustments
panel entry, and a self-test exit code) is a separate UI change with its own
requirements. Leaving the app out keeps this change to the codec/render seam and
avoids a self-test file split that an app edit would otherwise force.

## Risks / Trade-offs

- **The composition order is an assumption.** Mitigated by marking it a
  `ponytail:` ceiling and keeping the composite curve semantics unchanged for
  `points`-only curves; a real Photoshop Curves fixture would settle it.
- **The `is_map` legacy variant is unsupported.** No fixture or oracle exercises
  it; an `is_map` block decodes to `None` and stays a no-op, which is the
  current behavior. Documented as a ceiling.
- **psd-tools does not name the per-channel rows.** Its `Curves` reads the v1
  prefix into `data` without channel labels, so it cannot validate the row
  mapping; ag-psd is the full-field oracle and the psd-tools check is partial.
- **The ag-psd oracle depends on `node` + `ag-psd`.** CI installs it; locally
  the test prints a skip and the suite still passes, so a missing oracle cannot
  mask a regression. The committed fixture is the durable artifact.
- **`CurvesParams` gains fields.** All struct literals must be updated
  mechanically; the compiler finds them, so this is a wide but safe diff.
- **No pixel parity claim.** The decoded parameters are handed to the existing
  op; how faithfully that op reproduces Photoshop's curve is owned by
  `pictura-adjust` and its tests, and the no-ImageMagick-equivalent caveat is
  unchanged.
- **GPU documents now decode further before being rejected.** A file with a
  `curv` layer previously returned `None` from `decode_adjustment` and was
  rejected by `gpu/mod.rs`; now it decodes and is still rejected because
  `adjustment_params` (`gpu/mod.rs:255`) has no Curves shader. The fallback
  outcome is identical.

## Migration Plan

None for documents: an existing file is read as before, and a `curv` layer that
previously no-op'd now renders. No golden fixture changes (the new `curves.psd`
is additive) and no rollback beyond reverting the commit. `CurvesParams` call
sites compile-fail until updated in the same change.

## Open Questions

- Whether Photoshop applies the composite curve before or after the per-channel
  curves. The kernel assumes per-channel then composite (D4); a CS6/CC capture
  of a Curves layer with both a composite and a per-channel curve would settle
  it.
- Whether Photoshop stores node coordinates in `0..=255` for all modes (the
  decoder assumes 8-bit light units) or 16-bit for 16-bpc documents. The codec
  reads 8-bit RGB only, so this is deferred with depth support.
- Whether real Photoshop writes the duplicate `Crv ` version-4 section for every
  Curves layer; the fixture carries it and the decoder ignores it either way.
