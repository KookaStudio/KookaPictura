## Context

`pictura-codec` preserves the `selc` (Selective Color) block verbatim, and
`ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:49`) already lists
`*b"selc"`, so `is_adjustment_key` is true and the block is stored on
`Layer.adjustment` rather than in `extra_blocks`. Independently,
`pictura-render`'s `decode_adjustment` (`crates/pictura-render/src/composite.rs:338`)
has no `selc` arm, so a Selective Color adjustment layer composites as a no-op;
`deferred_keys_still_none` (`crates/pictura-render/src/tests/adjustment.rs:882`)
asserts exactly that. `pictura-adjust` implements sixteen destructive
adjustments but has no Selective Color op; `docs/04-image-ops/adjustments/selective-color.md`
(`ADJ-023`) specifies the operation and its Relative/Absolute semantics.

The block layout was previously ambiguous and is now confirmed three ways:

- **libpsd** (`/tmp/opencode/libpsd-master/src/selective_color.c`): `u16 version
  == 1`; `u16 correction_method` (`0` = relative, else absolute); 10 plates × 4
  `i16` (cyan, magenta, yellow, black). The first plate is ignored/reserved and
  "should be set to all zeroes"; the remaining nine are reds, yellows, greens,
  cyans, blues, magentas, whites, neutrals, blacks. Each correction is asserted
  in `-100..=100`.
- **ag-psd** (`/tmp/agpsd/src/additionalInfo.ts:4890`): reads `u16` version
  (must be 1), maps the `u16` method (nonzero → `absolute`), `skipBytes(8)` for
  the reserved plate, then reads the nine named ranges (`reds` … `blacks`), each
  as four `i16` (`c`, `m`, `y`, `k`). Semantically correct.
- **psd-tools** 1.19.0 (`psd_tools.psd.adjustments.SelectiveColor`): reads
  `"2H"` then ten `"4h"` plates, and writes the same; it names no plates but its
  framing is correct.

The algorithm is libpsd's `psd_selective_color_proc`, itself built from
`psd_rgb_to_intcmyk`, `psd_rgb_to_inthsb`, and `psd_intcmyk_to_rgb`.

## Goals / Non-Goals

**Goals:**

- Add `Adjustment::SelectiveColor` and a kernel implementing libpsd's integer
  pipeline so a decoded `selc` layer renders through the existing adjustment
  composite path.
- Decode a `selc` payload into the new params, and provide
  `encode_selective_color` so the app can author the neutral default.
- Commit a fixture carrying relative and absolute `selc` blocks and prove it
  with an independent ag-psd oracle (plus a psd-tools partial check).
- Drop `selc` from the deferred set in the doc comment and the test.
- Ship one app check for the new kind/menu.

**Non-Goals:**

- The reserved plate 0. It is skipped on read and written as zeroes.
- CMYK/Lab/Grayscale document modes and any profile-based CMYK conversion.
- Photoshop's family-membership curves and verified pixel parity.
- A Selective Color GPU shader or parameter-editing UI.
- Photo Filter version 3 and Color Lookup; they stay deferred.

## Decisions

### D1. The `selc` byte layout (three-way agreement)

```
u16  version         // must be 1
u16  correction      // 0 = relative, nonzero = absolute
10 x plate:          // each 4 x i16 { cyan, magenta, yellow, black } in -100..=100
    plate 0          // reserved, ignored
    plates 1..9      // reds, yellows, greens, cyans, blues, magentas,
                     // whites, neutrals, blacks
```

The header is 4 bytes and each plate is 8, so the block is `4 + 80 = 84` bytes.
Decode reads the version at offset 0, the method at offset 2, skips plate 0
(offsets 4..12) without validating it, and reads plates 1..9 (offsets 12..84)
into `ranges[0..9]`. The method maps `0` to Relative and any other value to
Absolute, matching both libpsd (`== 0` test) and ag-psd (`nonzero` test).

A payload shorter than 84 bytes, a version other than 1, or a range correction
outside `-100..=100` is `None`, never a panic. Every byte after offset 84 is
ignored. The `-100..=100` rejection matches the sibling `blnc`/`mixr` decoders
and libpsd's asserts; plate 0 is not validated because it is not modelled.

**Why relative over absolute?** Photoshop's UI default is Relative (per
`ADJ-023`); the reserved plate is where Photoshop has historically stored stale
values, so skipping it is both safer and what ag-psd does.

### D2. The engine params

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectiveColorMethod { Relative, Absolute }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectiveRange {
    pub c: i16,
    pub m: i16,
    pub y: i16,
    pub k: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectiveColorParams {
    pub method: SelectiveColorMethod,
    /// Nine ranges in Photoshop's order: reds, yellows, greens, cyans, blues,
    /// magentas, whites, neutrals, blacks.
    pub ranges: [SelectiveRange; 9],
}
```

One struct and one array keep the model small. `SelectiveRange: Default` gives
the neutral all-zero range and `SelectiveColorParams` gets a `Default` (relative
method, `[SelectiveRange::default(); 9]`) so the app authors the Photoshop
default. This follows the task's `ranges: [SelectiveRange; 9]` shape rather than
`ADJ-023`'s proposed `[CmykDelta; 9]`/`CorrectionMethod` names; the field layout
is the same.

### D3. The kernel: libpsd's integer pipeline

Validation: every `c`/`m`/`y`/`k` in every range must be in `-100..=100`, else
`AdjustError::InvalidParams`. If all nine ranges are zero, return `Ok(())`
without touching the buffer (see D4).

Per composite pixel `(r, g, b)`:

1. `(sc, sm, sy, sk) = rgb_to_intcmyk(r, g, b)` where
   `dc = 255 - r`, `dm = 255 - g`, `dy = 255 - b`, `k = min(dc, dm, dy)`, and
   when `k < 255` each of `c/m/y` is `(d - k) * 255 / (255 - k)`, else zero.
   All `/` truncate toward zero (the operands are non-negative).
2. `hue = rgb_to_int_hue(r, g, b)` in `0..=359`:
   `cmax = max`, `cmin = min`, and if `cmax == cmin` then `hue = 0`; else
   `d = cmax - cmin`, `h = (g - b) * 60 / d` when `r == cmax`,
   `h = 120 + (b - r) * 60 / d` when `g == cmax`, else
   `h = 240 + (r - g) * 60 / d`, and `hue = (h + 360) % 360`.
3. Copy `dst = (sc, sm, sy, sk)`.
4. For `i in 1..=6` (model index `i - 1`, families reds…magentas): compute
   `r0 = -105 + i * 60`, `r1 = r0 + 30`, `r2 = r1 + 30`, `r3 = r2 + 30`. When
   `r0 <= hue < r3`, the opacity is `255` inside `[r1, r2)`,
   `(hue - r0) * 255 / 30` below `r1`, and `(r3 - hue) * 255 / 30` above `r2`.
   Then, for each ink component with a nonzero correction:
   Relative `dst += src * corr * opacity / 25500`; Absolute
   `dst += 255 * corr * opacity / 25500`.
5. For `i in 7..=9` (model indices 6..8, whites/neutrals/blacks), select by the
   source black ink: `i == 7` when `sk == 0`, `i == 8` when `0 < sk < 255`,
   `i == 9` when `sk == 255`. With no hue opacity, for each nonzero correction:
   Relative `dst += src * corr / 100`; Absolute `dst += 255 * corr / 100`.
6. Clamp every destination ink to `0..=255`.
7. `(r, g, b) = intcmyk_to_rgb(dst)` where each output channel is
   `(65535 - (ink * (255 - k) + (k << 8))) >> 8` (the shift is on a
   provably non-negative value, so it is a plain floor shift).

Every division is truncating integer division, so the Rust kernel must use `i32`
(not floats) to match C exactly. A hue may fall in two adjacent families'
windows (they overlap by 30), and both plates' corrections are added; the
feather ramps keep the sum continuous.

**Why the integer pipeline rather than `ADJ-023`'s CMYK profile path?** The spec
explicitly leaves the RGB→CMYK path undocumented and profile-dependent, and the
codec reads only 8-bit RGB. libpsd is a concrete, testable grounding for an
8-bit RGB approximation; the `ponytail:` ceiling below records the gap.

### D4. All-zero is the exact identity (a deliberate deviation)

libpsd always runs the RGB→CMYK→RGB round-trip, even with zero corrections.
That round-trip is lossy: measured over all 2^24 sRGB triples, only 256 triples
are exact and the worst channel error is 2 LSB. Photoshop's zero-slider
adjustment is a no-op, and the app's neutral default must not tint the canvas, so
the kernel returns early when all nine ranges are zero. This is the only
intentional deviation from libpsd and is marked with a `ponytail:` comment.

### D5. New `selective_color` module, wired from `composite.rs`

Decode and encode live in a new `crates/pictura-render/src/selective_color.rs`,
mirroring `curves.rs` and `color_balance.rs`, with the arm
`b"selc" => crate::selective_color::decode_selective_color(&data.data)` added to
`decode_adjustment`. `composite.rs` is 1011 LOC (budget 1200), so a sibling
module keeps it inside budget. `encode_selective_color` is re-exported from
`lib.rs` beside `encode_curves`.

The decode/reject/round-trip tests live in the new module's `#[cfg(test)] mod
tests` because `crates/pictura-render/src/tests/adjustment.rs` is at 1398/1400
LOC; the fixture-decode check goes in `tests/oracle/adjustment_oracle.rs`, which is at
233 LOC.

### D6. The encoder

`encode_selective_color(method: SelectiveColorMethod, ranges: &[SelectiveRange; 9])
-> AdjustmentData` writes the `u16` version 1, the `u16` method (0/1), an 8-byte
zero reserved plate, and the nine plates in order, each `{c, m, y, k}` as
big-endian `i16`. It clamps every correction to `-100..=100`, so its output
always decodes. The neutral default is `encode_selective_color(Relative,
&[SelectiveRange::default(); 9])`, an 84-byte block (version 1, relative method, and eighty zero bytes).

### D7. Fixture with a relative and an absolute block; ag-psd is the oracle

`scripts/generate-fixtures.py` gains a `selective_color()` builder with a Base
layer plus:

- `Selective Color` (relative, method 0): plate 0 zero; reds `(10, -20, 30, 0)`,
  yellows `(0, 0, 0, 5)`, greens `(-10, 0, 0, 0)`, cyans `(0, 15, 0, 0)`, blues
  `(0, 0, -25, 0)`, magentas `(5, 0, 0, 0)`, whites `(0, 0, 0, 0)`, neutrals
  `(20, -10, 0, 0)`, blacks `(0, 0, 0, -40)`.
- `Selective Color Abs` (absolute, method 1): plate 0 zero; reds `(1, 2, 3, 4)`
  through blacks `(33, 34, 35, 36)`.

psd-tools' `SelectiveColor` reads and writes the 10-plate form, so passing a
`SelectiveColor(version=1, method=…, data=[…10 plates…])` as the `TaggedBlock`
data stores it verbatim (confirmed empirically: reopening the probe yields
`version=1`, the method, and all ten plates). Register
`"selective_color.psd": selective_color` in `FIXTURES`; regeneration is
byte-stable and additive, so no existing golden changes.

The new test in `crates/pictura-codec/tests/psd/agpsd_oracle.rs` runs
`node -e "<script>" <fixture>` with
`require('ag-psd').readPsd(bytes, { skipLayerImageData: true,
skipCompositeImageData: true })`, prints one line per `selective color`
adjustment (mode plus the nine named ranges in order), parses the tokens in Rust
(no JSON, no new dependency), and asserts the authored values. It self-skips
with a clear message when `node` or `ag-psd` is unavailable, exactly like the
`mixr`/`curv` oracles. `.github/workflows/ci.yml`'s `oracles` job already
installs `ag-psd`.

### D8. A psd-tools partial check and a pure-Rust fixture decode

`crates/pictura-render/tests/oracle/adjustment_oracle.rs` gains:

- `psd_tools_reads_fixture_selective_color_prefix`: feeds the fixture's relative
  `selc` bytes to `psd_tools.psd.adjustments.SelectiveColor.read` and asserts
  `version == 1`, `method == 0`, and `data[1..]` equals the nine authored plates
  (psd-tools names no plates, so it is a field check, not a labelled one). Self-
  skips without psd-tools.
- `fixture_selective_color_decodes`: reads both fixture layers and asserts
  `decode_adjustment` yields the authored relative/absolute params. Pure Rust.

### D9. The app kind, panel entry, and self-test check

`helpers.rs::adjustment_layer` gains `"selective-color" => ("Selective Color",
encode_selective_color(SelectiveColorMethod::Relative, &[SelectiveRange::default();
9]))`. `panel_group_menu.cpp` gains
`imp(QStringLiteral("Selective Color"), QStringLiteral("adjustment:selective-color"))`
in the `adjustmentsPanel` block; dispatch is generic through the `adjustment:`
prefix (`layers_panel_menu.cpp:199`), so no other wiring is needed. The
`Selective Color` command-tree leaves already exist.

`selftest_layers_adjustments.cpp` (156 LOC, ample budget) gains one
`lpr_selective_color` check using **exit code 296**. Code 295 is already taken by
`present_cache_edge` in `selftest_canvas.cpp` (committed in `7ee4805`); 296 is
the next free code. The check creates a 4x4 RGB document, samples before, adds
`selective-color`, asserts the new layer's kind is `adjustment` and the composite
is unchanged (the all-zero default is the exact identity, D4), and asserts the
Adjustments panel menu contains `Selective Color`.

## Risks / Trade-offs

- **The kernel is not Photoshop's pipeline.** Mitigated by grounding on libpsd,
  marking the profile-free conversion a `ponytail:` ceiling, and claiming no
  pixel parity.
- **The RGB→CMYK round-trip is lossy.** Mitigated by the exact-identity early
  return for all-zero ranges; a non-zero range's output is within libpsd's own
  semantics, not Photoshop's.
- **libpsd's hue/tonal windows are an approximation.** `ADJ-023` calls the
  membership functions undocumented; the windows here are libpsd's, recorded as
  the ceiling. A CS6 calibration sweep would refine them.
- **The reserved plate is ignored without validation.** A malformed file with a
  corrupt reserved plate still decodes, which matches Photoshop and ag-psd.
- **psd-tools does not name the plates.** ag-psd is the full-field oracle; the
  psd-tools check is framing plus values.
- **The ag-psd oracle depends on `node` + `ag-psd`.** CI installs it; locally the
  test skips and the suite still passes; the committed fixture is durable.
- **A GPU document now decodes further before being rejected.** A `selc` layer
  previously returned `None`; now it decodes and is still rejected by
  `adjustment_params` (no Selective Color shader), so the CPU fallback outcome is
  identical.

## Migration Plan

None for documents: an existing file is read as before, and a `selc` layer that
previously no-op'd now renders. No golden fixture changes (the new
`selective_color.psd` is additive) and no rollback beyond reverting the commit.
The `Adjustment` enum gains a variant, so exhaustive `match` sites compile-fail
until updated in the same change.

## Open Questions

- Whether Photoshop's RGB→CMYK round-trip matches libpsd's integer form or uses
  the document CMYK working space (`ADJ-023` leaves this open). A CS6 fixture
  with a known Selective Color layer would settle it.
- Whether libpsd's 30/15 hue window and tonal selection reproduce Photoshop's
  family weighting closely enough for a parity claim. A CS6 calibration sweep
  across hue and luminance would settle it.
- Whether Photoshop ever writes a non-zero reserved plate; the fixture writes
  zeroes and the decoder ignores any value either way.
