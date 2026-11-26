## Context

`pictura-codec` preserves the `blnc` (Color Balance) block verbatim, but
`ADJUSTMENT_KEYS` in `crates/pictura-codec/src/common.rs:46` does not list it, so
`is_adjustment_key` (`common.rs:52`) is false and the block lands in
`extra_blocks` rather than `Layer.adjustment`. Independently,
`pictura-render`'s `decode_adjustment`
(`crates/pictura-render/src/composite.rs:329`) has no `blnc` arm, so even a
classified block would composite as a no-op. `pictura-adjust` already implements
the operation: `ColorBalanceParams` (`crates/pictura-adjust/src/types.rs:85`),
`Adjustment::ColorBalance` (`types.rs:149`), the kernel
(`crates/pictura-adjust/src/color.rs:102`), and its tests
(`crates/pictura-adjust/src/tests.rs:679`, `tests.rs:703`).

The payload is a fixed struct, not a descriptor. psd-tools 1.19
(`psd_tools/psd/adjustments.py:76-107`, `ColorBalance`) reads shadows, midtones,
and highlights as `read_fmt("3h")` and luminosity as `read_fmt("B")`, then
`write_padding(..., 4)`; `Tag.COLOR_BALANCE = b"blnc"` is
`psd_tools/constants.py:348`. The binary layout therefore has an independent
reference and a working writer to diff the encoder against.

## Goals / Non-Goals

**Goals:**

- Classify `blnc` as an adjustment in the codec, so the block becomes
  `AdjustmentData` instead of an opaque extra block.
- Decode a `blnc` payload into `Adjustment::ColorBalance`, so the layer renders
  through the existing adjustment composite path.
- Provide `encode_color_balance` so the app can create a Color Balance layer,
  round-tripping against the decoder and matching psd-tools' own writer.
- Wire the kind `color-balance` through the app bridge and the Adjustments panel
  menu.

**Non-Goals:**

- A Color Balance GPU shader. Documents with one keep falling back to the CPU
  path, as today.
- Any `pictura-adjust` change. The op, its validation, and its tests already
  exist; the decoded parameters are handed to it unchanged.
- Verified pixel parity against Photoshop. The decoding is structural; the
  render contract belongs to `image-adjustments`/`pictura-adjust`.
- The other still-deferred keys (`curv`, `mixr`, `selc`, `clrL`, real `SoCo`,
  version-3 `phfl`).

## Decisions

### D1. The `blnc` byte layout

psd-tools `ColorBalance.read`/`write` is the reference:

```
3h  shadows      (cyan-red, magenta-green, yellow-blue)
3h  midtones
3h  highlights
B   luminosity   (0/1)
    pad to a 4-byte boundary
```

That is nine big-endian `i16` shifts in `-100..=100`, one luminosity byte, and
one pad byte (19 → 20 bytes). `decode_color_balance` reads the nine values with
the existing `be_i16` helper (`crates/pictura-render/src/composite.rs:385`) at
offsets 0..18 and the luminosity byte at 18, and ignores everything after byte
19. It requires at least 19 bytes so the padded 20-byte block decodes; a shorter
buffer is `None`. No length equality check, so a future payload with different
padding still decodes.

### D2. Reject out-of-range shifts in the decoder

The decoder returns `None` when any of the nine shifts is outside
`-100..=100`, matching `pictura-adjust`'s `ColorBalanceParams` validation and
the existing decoders' range checks. It does not clamp, so a corrupt file cannot
silently render a different adjustment; the layer stays a no-op. `luminosity` is
interpreted as a boolean (`!= 0`), mirroring `decode_photo_filter`'s handling of
the same flag. Nothing here panics.

`preserve_luminosity` is the only field the fixed struct carries beyond the nine
shifts; Photoshop's per-band "preserve luminosity" tri-state is not representable
and is collapsed to the single boolean the engine already models.

### D3. The encoder clamps, then matches psd-tools

`encode_color_balance(shadows: [f64; 3], midtones: [f64; 3], highlights:
[f64; 3], preserve_luminosity: bool) -> AdjustmentData` clamps each shift to
`-100.0..=100.0` and rounds to the nearest `i16`, then writes the nine
big-endian values, the luminosity byte, and one pad byte (20 total). Clamping
lives only in the encoder, because it is built from typed app input rather than
untrusted bytes — the same split as `encode_photo_filter`
(`composite.rs:734`). The output always decodes. Because it reproduces the
psd-tools `ColorBalance` layout exactly, `ColorBalance.read(BytesIO(blnc.data))`
reads back the same four fields; this is the parity check (D6).

### D4. The app authors the neutral Photoshop default

`helpers.rs::adjustment_layer` gains a `"color-balance"` arm returning
`("Color Balance", encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true))`.
All-zero shifts are the Photoshop default and a deliberate no-op in the engine
(`color.rs:113` early-returns when every band is zero), so unlike the sibling
`photo-filter`/`gradient-map` kinds a freshly added layer does not change the
composite. The panel entry is what makes the kind discoverable; the user then
edits the bands. `panel_group_menu.cpp` gains
`imp(QStringLiteral("Color Balance"),
QStringLiteral("adjustment:color-balance"))` after the Gradient Map row. No new
bridge method is needed: `add_adjustment` already dispatches on the kind string
(`crates/pictura-app/src/cxxqt_object/impl_filters.rs:9`), and `adjustment_layer`
returns `None` only for an unknown kind (`helpers.rs:44`).

### D5. Codec classification is the root fix

`blnc` is added to `ADJUSTMENT_KEYS` (`common.rs:46`, 20 → 21 entries). This is
the root-cause fix: the decode arm alone would never be reached for a real file,
because the block would not be stored on `Layer.adjustment`. The codec
round-trip test's key/bytes cases (`crates/pictura-codec/src/tests.rs:422`) gain
a `blnc` payload so a regression in the whitelist fails the read-back equality,
not just a decode unit test.

### D6. psd-tools parity: encode in Rust, read in psd-tools

Unlike the fixture-based siblings (`photo-filter` extended the psd-tools-authored
`adjustment.psd`; `gradient-map` added a psd-tools-authored fixture), this change
does not touch a golden fixture. Instead a new integration test
`crates/pictura-render/tests/oracle/adjustment_oracle.rs` calls `encode_color_balance`,
passes the raw bytes to a `python3` script that does
`ColorBalance.read(BytesIO(data))`, and asserts the printed
`shadows`/`midtones`/`highlights`/`luminosity` equal the inputs. This validates
the encoder against an independent writer rather than only against our own
decoder, and it leaves `crates/pictura-codec/tests/fixtures/` untouched. The test
self-skips with a message when `python3` or `psd_tools` is absent, matching
`crates/pictura-render/tests/document/document_oracle.rs:290`.

### D7. Composite test with non-neutral parameters

The `photo-filter`/`gradient-map` composite tests live in
`crates/pictura-render/src/tests/adjustment.rs` and use the app default to make
the layer visibly change the backdrop. Because the Color Balance app default is
neutral, the composite test builds a non-neutral `ColorBalanceParams` directly
(e.g. a positive midtone red shift) and asserts the composited result differs
from the backdrop-only composite. That is the "no longer a no-op" proof; the app
self-test (D4) asserts registration and menu wiring instead of a pixel delta.

### D8. Self-test exit code

The next free self-test exit code is 293 (the highest in use is 292). One check
`lpr_color_balance` adds the `color-balance` kind, asserts the new layer is
reported as an adjustment, and asserts the Adjustments panel menu contains
`Color Balance`. It does not assert a pixel change, because the authored layer is
the neutral default. Names carry no milestone. The check lives in
`crates/pictura-app/cpp/selftest_layers_controls.cpp` next to the sibling checks
and must keep the file inside its `scripts/file-size-allowlist.txt` ceiling.

## Risks / Trade-offs

- **The neutral default makes the new layer invisible.** A user who adds Color
  Balance sees no change until they edit a band. This is Photoshop's own default
  and the honest behavior for a fixed struct; D4 records it, and the composite
  test uses non-neutral params to prove the decode path.
- **Collapsing per-band luminosity preservation to one boolean.** The engine
  models a single `preserve_luminosity`; Photoshop's tri-state is not
  representable. This is an existing `pictura-adjust` limitation, marked with a
  `ponytail:` note in the decoder, not introduced here.
- **The parity oracle depends on psd-tools.** CI's `oracles` job installs it;
  locally the test prints a skip and the suite still passes, so a missing oracle
  cannot mask a regression.
- **GPU documents now decode further before being rejected.** A file with a
  `blnc` layer previously returned `None` from `decode_adjustment` and was
  rejected by `gpu/mod.rs`; now it decodes and is still rejected because
  `adjustment_params` (`gpu/mod.rs:250`) has no Color Balance shader. The fallback
  outcome is identical.
- **No pixel parity claim.** The decoded parameters are handed to the existing
  op unchanged; how faithfully that op reproduces Photoshop's band weighting is
  owned by `pictura-adjust` and its tests.

## Migration Plan

None for documents: an existing file is read as before, and a `blnc` layer that
previously no-op'd because it was not even classified as an adjustment now
renders. No golden fixture changes and no rollback beyond reverting the commit.

## Open Questions

- Whether a real CS6 `blnc` stores the nine shifts on the same `-100..=100`
  scale. psd-tools' struct and writer confirm the field order and width; a real
  CS6 Color Balance baseline would confirm the scale and the exact band windows
  the engine approximates.
- Whether Photoshop writes any meaningful trailing bytes beyond the 4-byte pad.
  The decoder ignores everything past byte 19 either way.
