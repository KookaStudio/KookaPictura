## Context

`pictura-codec` preserves the `mixr` (Channel Mixer) block verbatim, and
`ADJUSTMENT_KEYS` (`crates/pictura-codec/src/common.rs:47`) already lists
`*b"mixr"`, so `is_adjustment_key` is true and the block is stored on
`Layer.adjustment` rather than in `extra_blocks`. Independently,
`pictura-render`'s `decode_adjustment`
(`crates/pictura-render/src/composite.rs:337`) has no `mixr` arm, so a Channel
Mixer adjustment layer composites as a no-op. `pictura-adjust` already
implements the operation: `ChannelMixerParams`
(`crates/pictura-adjust/src/types.rs:68`), `Adjustment::ChannelMixer`
(`types.rs:162`), the kernel (`crates/pictura-adjust/src/color.rs:268`), and
its tests (`src/tests.rs:604`, `:636`, `tests/oracle.rs:568`). The op validates
every source percentage and every constant to `abs <= 200.0` and models the
non-monochrome output as `(Σ w·rgb)/100 + constant/100·255`; the monochrome
branch uses `red` as the single source mix and `constant[0]` for all three
outputs.

The payload layout is **not** what psd-tools models. psd-tools
`ChannelMixer.read` (`psd_tools/psd/adjustments.py:135`) reads `2H` (version,
monochrome) plus `5h` (five shorts) and dumps everything else into `unknown`
with no field names, so psd-tools sees only the **red** row's three weights,
the two reserved bytes as one short value, and `constant`; it knows nothing of
the green/blue/gray rows and cannot author a faithful block. The independent
ground truth is **ag-psd**: `readMixrChannel`
(`src/additionalInfo.ts:4564`) and the `mixr` handler (`:4583`) plus
`ChannelMixerChannel` (`src/psd.ts:624`). Grounded from those and confirmed
empirically against `/tmp/agtest`:

```
u16 version       (must be 1)
u16 monochrome
if !monochrome: channel red, channel green, channel blue
always:         channel gray
channel = i16 red_mix, i16 green_mix, i16 blue_mix, 2 reserved bytes, i16 constant
```

Non-monochrome is `4 + 4×10 = 44` bytes (the gray row is present and ignored by
an RGB decode). Monochrome is `4 + 10 + 30` bytes in ag-psd's writer (the gray
channel at offset 4, then 30 zero bytes); the reader needs only the 10-byte gray
channel and skips the rest. Constants are Photoshop's `-200..=200` percent.

## Goals / Non-Goals

**Goals:**

- Decode a `mixr` payload into `Adjustment::ChannelMixer`, so the layer renders
  through the existing adjustment composite path.
- Provide `encode_channel_mixer` so the app can create a Channel Mixer layer,
  round-tripping against the decoder.
- Commit a fixture carrying known monochrome and non-monochrome `mixr` blocks
  and prove it with an independent ag-psd oracle (plus a cheap psd-tools
  partial check).
- Wire the kind `channel-mixer` through the app bridge and the Adjustments panel
  menu.

**Non-Goals:**

- A Channel Mixer GPU shader. Documents with one keep falling back to the CPU
  path, as today.
- Any `pictura-adjust` change. The op, its validation, and its tests already
  exist; the decoded parameters are handed to it unchanged.
- CMYK/non-RGB mixer semantics, mode gating, or preset names. The codec reads
  RGB/Gray only.
- Verified pixel parity against Photoshop. The decoding is structural; the
  render contract belongs to `image-adjustments`/`pictura-adjust`.
- The other still-deferred keys (`curv`, `selc`, `clrL`, version-3 `phfl`).

## Decisions

### D1. The `mixr` byte layout (ag-psd is the reference, psd-tools is wrong)

Decode follows ag-psd, not psd-tools:

```
u16 version        == 1
u16 monochrome     (0/1)
if !monochrome:    red, green, blue    (each 10 bytes)
always:            gray                (10 bytes)
i16 red_mix, i16 green_mix, i16 blue_mix, skip 2, i16 constant
```

`decode_channel_mixer` requires `version == 1`; reads the monochrome flag; when
clear reads the three 10-byte channels; always reads a 10-byte gray channel;
ignores every trailing byte. It requires at least the bytes it reads
(non-monochrome `44`, monochrome `14`), so a shorter buffer is `None`. The
per-channel field reads use the existing `be_i16` helper
(`crates/pictura-render/src/composite.rs:395`); the two reserved bytes are
skipped, not interpreted. No length equality check, so a future payload with
different padding still decodes.

**Why not psd-tools' `2H` + `5h`?** It is a truncation: it reads only the red
channel, names the reserved pair as a short, and cannot express the
green/blue/gray rows. Basing the decoder on it would render every non-red
output wrong and could never round-trip a real block. This is the same class of
ambiguity `layer-effects-legacy-lrfx` recorded for the `lrFX` blur width; here
the ambiguity is resolved by a second independent implementation (ag-psd)
rather than left open.

### D2. Monochrome mapping and the unused rows

The op's monochrome branch (`color.rs:284`) reads **only** `p.red` and
`p.constant[0]`. Decode therefore sets `monochrome = true`, `red = gray.rgb`,
`constant = [gray.constant, 0.0, 0.0]`, and fills the unused `green`/`blue`
rows with the identity defaults `[0, 100, 0]`/`[0, 0, 100]` (not the gray row),
because they are unobservable through the op and a Photoshop-default struct is
less misleading than fabricated data. Encode reads only `red`/`constant[0]` for
a monochrome block, so `decode → encode → decode` is exact.

### D3. Reject out-of-range weights in the decoder; clamp in the encoder

The decoder returns `None` when any source percentage or constant is outside
`-200..=200`, matching `pictura-adjust`'s `ChannelMixerParams` validation and
the sibling decoders' range checks. It does not clamp, so a corrupt file cannot
silently render a different adjustment. `encode_channel_mixer` clamps each of
the 12 values to `-200..=200` and rounds to the nearest `i16`, because it is
built from typed app input rather than untrusted bytes — the same split as
`encode_color_balance`/`encode_photo_filter`. Nothing here panics.

### D4. The encoder writes the full 44-byte block

`encode_channel_mixer(monochrome: bool, red: [f64; 3], green: [f64; 3],
blue: [f64; 3], constant: [f64; 3]) -> AdjustmentData` writes version 1, the
monochrome flag, then, when not monochrome, the red/green/blue rows followed by
a **gray** row whose source triple is `red` and whose constant is
`constant[0]` — exactly the fields the monochrome decode reads — and when
monochrome, that same gray row (`rgb = red`, `constant = constant[0]`) followed
by 30 zero bytes. Both forms are therefore `44` bytes,
matching ag-psd's writer; the gray row is ignored by an RGB decode but read by
ag-psd, so emitting it keeps the block self-consistent. `decode_adjustment` on
the encoder's output equals `Adjustment::ChannelMixer` with the clamped inputs.

### D5. New `channel_mixer` module, wired from `composite.rs`

Decode and encode live in a new `crates/pictura-render/src/channel_mixer.rs`
module, mirroring `color_balance.rs`, with the arm
`b"mixr" => crate::channel_mixer::decode_channel_mixer(&data.data)` added to
`decode_adjustment`. `composite.rs` is already `1008` LOC against a `1200` code
cap, and the sibling `blnc` codec set the module precedent; keeping the ~90 new
lines in a sibling keeps `composite.rs` inside budget and mirrors the shipped
shape. `encode_channel_mixer` is re-exported from `lib.rs` beside
`encode_color_balance`.

### D6. Fixture with hand-built raw bytes; ag-psd is the oracle

psd-tools' typed `ChannelMixer` writer is wrong, so the fixture is authored with
hand-built `mixr` bytes. `TaggedBlock.write` calls `write_bytes` when `data` is
a plain `bytes` (it only dispatches to `data.write` when `data` has a `write`
attribute), so passing raw bytes through the existing
`_adj_layer(psd, Tag.CHANNEL_MIXER, name, data)` helper stores them verbatim —
confirmed empirically: a PSD authored this way round-trips the 44-byte block
byte-for-byte and ag-psd reads it. `scripts/generate-fixtures.py` gains a
`_mixr_data(monochrome, red, green, blue, gray)` helper using `struct.pack`
(two `H`, then per channel `hhh` + `xx` + `h`) and a `channel_mixer()` builder
with a Base layer plus two adjustment layers:

- `Channel Mixer` (non-monochrome): `red = (30, -10, 50)`, `green = (10, 90,
  0)`, `blue = (0, 20, 110)`, `constant = (5, -20, 40)`, `gray = (100, 0, 0, 0)`
  (ignored).
- `Channel Mixer Mono` (monochrome): `gray = (20, 40, 60)`, `constant = -15`,
  written as the ag-psd 44-byte shape (gray + 30 zero bytes).

The oracle `crates/pictura-codec/tests/psd/agpsd_oracle.rs` runs
`node -e "<script>"` with the fixture path, where the script does
`require('ag-psd').readPsd(bytes, { skipLayerImageData: true,
skipCompositeImageData: true })` and prints one whitespace-separated line per
`channel mixer` child (`0` + the 16 RGB+gray numbers, or `1` + the 4 gray
numbers). Rust parses the tokens (no JSON, so no new dependency) and asserts
the authored fields. The test self-skips with a clear message when `node` or
`ag-psd` is unavailable, exactly like the psd-tools/magick oracles. CI installs
it with `npm i ag-psd` at the repo root (node resolves
`crates/pictura-codec`'s parents), or locally via
`NODE_PATH=/tmp/agtest/node_modules`.

Confirmed on the committed fixture: psd-tools re-serializes the raw bytes
verbatim for `Tag.CHANNEL_MIXER` (44 bytes; its five-short prefix is
`[30, -10, 50, 0, 5]`, the red row), and ag-psd reads the authored
non-monochrome and monochrome values.

### D7. A cheap psd-tools partial check always runs

`crates/pictura-render/tests/oracle/adjustment_oracle.rs` gains a second test that
feeds the fixture's non-monochrome `mixr` bytes to psd-tools'
`ChannelMixer.read` and asserts `version == 1`, `monochrome == 0`, and the first
five shorts equal the red row plus its constant. This is honest about
psd-tools' limits (it cannot see green/blue/gray) and runs whenever psd-tools is
present; ag-psd carries the full-field proof.

### D8. App authors the neutral identity default

`helpers.rs::adjustment_layer` gains a `"channel-mixer"` arm returning
`("Channel Mixer", encode_channel_mixer(false, [100.0, 0.0, 0.0], [0.0, 100.0,
0.0], [0.0, 0.0, 100.0], [0.0; 3]))`. Identity rows with zero constants are
Photoshop's default and a deliberate no-op in the engine, so a freshly added
layer does not change the composite (the same situation as the neutral Color
Balance default). `panel_group_menu.cpp` gains
`imp(QStringLiteral("Channel Mixer"),
QStringLiteral("adjustment:channel-mixer"))` after the Color Balance row. No new
bridge method: `add_adjustment` dispatches on the kind string and
`adjustment_layer` returns `None` only for an unknown kind. The command-tree
leaves already exist (`command_tree.cpp:215`, `:339`), so `command_tree.cpp` is
unchanged.

### D9. Self-test location and exit code (layout correction)

The next free self-test exit code is **294** (the highest in use is 293). The
brief asked for the check in `selftest_layers_controls.cpp`, but that file is
`1185` LOC against the `1200` **code** budget (it is a `*.cpp`, not a
`*_test.cpp`, so it does not get the test budget) and is not in
`scripts/file-size-allowlist.txt`. Another check cannot fit. Following rule 9
and the established sub-runner pattern (`runFileDropChecks`,
`runToolsSelectionChecks`), the adjustment checks (`lpr_photo_filter` 283,
`adjustments_photo_filter_menu` 284, `lpr_gradient_map` 285, `lpr_color_balance`
293) move, unchanged, into a new `selftest_layers_adjustments.{cpp,h}`
exposing `runLayersAdjustmentChecks(frame)`, registered in `CMakeLists.txt`
(new `.cpp`/`.h` must be listed explicitly) and called from
`selftest_layers_controls.cpp`. The relocated checks now run earlier in the
self-test sequence, at the sub-runner call site rather than inline, but each
check opens and closes its own document, so the move has no observable effect
on ordering or state. The new `lpr_channel_mixer` check (294) is added
there: it adds the `channel-mixer` kind, asserts the layer is reported as an
adjustment and that the neutral default leaves the composite unchanged, and
asserts the Adjustments panel menu contains `Channel Mixer`. Names carry no
milestone. The pure move keeps `selftest_layers_controls.cpp` well under its
cap without touching the allowlist.

## Risks / Trade-offs

- **psd-tools cannot fully validate the block.** Its `ChannelMixer` reads only
  the red row; using it as the primary oracle would silently bless a broken
  green/blue/gray layout. Mitigated by making ag-psd the independent oracle
  (D6) and documenting psd-tools' truncation (D1).
- **The ag-psd oracle depends on `node` + `ag-psd`.** CI's `oracles` job installs
  it; locally the test prints a skip and the suite still passes, so a missing
  oracle cannot mask a regression. The committed fixture is the durable artifact
  either way.
- **The neutral default makes the new layer invisible.** Photoshop's own
  default; the composite test uses non-neutral params to prove the decode path,
  and the app self-test asserts the neutral behavior (D8/D9).
- **The `selftest_layers_controls.cpp` split is extra scope.** It is forced by
  the code-size cap; the move is pure (no behavior change) and follows the
  existing sub-runner seam, so the diff is mechanical.
- **No pixel parity claim.** The decoded parameters are handed to the existing
  op unchanged; how faithfully that op reproduces Photoshop's mixer is owned by
  `pictura-adjust` and its tests, and the IM-based op caveat is unchanged.
- **GPU documents now decode further before being rejected.** A file with a
  `mixr` layer previously returned `None` from `decode_adjustment` and was
  rejected by `gpu/mod.rs`; now it decodes and is still rejected because
  `adjustment_params` (`gpu/mod.rs:255`) has no Channel Mixer shader. The
  fallback outcome is identical.

## Migration Plan

None for documents: an existing file is read as before, and a `mixr` layer that
previously no-op'd now renders. No golden fixture changes (the new
`channel_mixer.psd` is additive) and no rollback beyond reverting the commit.

## Open Questions

- Whether a real CS6 `mixr` monochrome block is 14 bytes (gray only) or 44
  bytes (gray + 30 zero bytes). ag-psd's writer emits 44 and its reader accepts
  either, so the decoder accepts `>= 14` for monochrome and the fixture exercises
  the 44-byte shape; a real CS6 Channel Mixer baseline would settle it.
- Whether Photoshop writes meaningful values in the non-monochrome gray row
  (ag-psd reads it; our decode ignores it). The fixture carries a known gray row
  so an ag-psd read can assert it, but the renderer does not consume it.
