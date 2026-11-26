## Context

`read_psd` parses the header and gates on `depth == 8` (or `depth == 1` for
Bitmap) before it reads anything (`crates/pictura-codec/src/read.rs:47-52`).
`PixelBuffer` is `Vec<u8>` (`crates/pictura-core/src/lib.rs:52-57`) and the
whole engine — compositor, colour, ops, and `write_psd`'s
`doc.depth != BitDepth::Eight` guard (`crates/pictura-codec/src/write.rs:562`) —
assumes one byte per sample. The color-mode slice already normalizes non-RGB
modes on read and records `Document.source_mode`, warning the user through
`mode_notice` (`crates/pictura-app/src/cxxqt_object/impl_core.rs:232`); depth is
the same shape of gap and is the other half of roadmap P4.

The depth-1 Bitmap path (shipped with `color-mode-read`) already made the row
stride depth-aware for the 1-bit case; this change generalizes that to 16 and 32.

**Grounding.** Both reference decoders are available locally and were read for
this design:

- **psd-tools 1.19** (`/usr/lib/python3.14/site-packages/psd_tools`):
  `compression/__init__.py:115` `_row_size = (width * depth + 7) // 8` (2·width
  at 16, 4·width at 32); `:409` `decode_rle` reads per-row byte counts and
  decodes each row of `row_size`; `:444` `decode_prediction` is byte-wise at 8,
  per-`u16` at 16, and shuffle-then-byte-wise at 32; `psd/image_data.py:109`
  `set_data`/`:61` `get_data` thread the header depth through. Most importantly
  `api/pil_io.py:410` `_create_image` defines the reference narrowing:
  `I;16B` scaled by `1/256` (depth 16) and `F;32BF` scaled by `256` and clipped
  (depth 32).
- **ag-psd** (`node_modules/ag-psd/dist/psdReader.js`): `:234` accepts depths
  `[1, 8, 16, 32]`; `:1020` `bytesToArray` documents "PSD files store 16-bit
  channel data in big-endian byte order" and returns a `Uint16Array`, and reads
  32-bit as a big-endian `Float32Array`; `:1081` `decodePredicted` is per-`u16`
  (`0x10000`) at 16 and byte-wise (`0x100`, width `*4`) with a 4-plane unshuffle
  at 32; `:1063` `readDataRaw` byte-reverses each `f32` group.

The two agree on every layout fact this change depends on. The 8-bit narrowing
conventions were then pinned empirically against psd-tools' own composite (see
D3–D4) so the oracle compares exactly, not within a guessed tolerance.

## Goals / Non-Goals

**Goals:**

- Open 16- and 32-bit PSD/PSB files, producing a renderable 8-bit RGB/Grayscale
  `Document`.
- Narrow samples with a documented, reproducible conversion that matches the
  independent `psd-tools` decoder exactly.
- Keep the data honest: the file's original depth is recorded and reported, and
  the save-as-8-bit is documented rather than silent.
- Reuse the existing decode paths (raw, RLE, ZIP, ZIP-with-prediction) by making
  the row stride and the prediction step depth-aware, rather than adding a
  parallel reader.

**Non-Goals:**

- A true `u16`/`f32` sample model threaded through core/render/colour/app, and
  preserving depth on save (design D1 option B; deferred).
- HDR processing or Photoshop's 32-bit tone-mapped display of a float document.
  The read is a display-referred narrowing (D4).
- Photoshop pixel parity for the 16→8 rounding. The conversion matches
  `psd-tools`, which is stated as the reference, not Adobe.
- Editing/authoring 16/32-bit channels, and depth-1/16/32 Bitmap or Indexed.

## Decisions

### D1. Normalize-on-load to 8-bit, not a 16/32-bit sample model (chosen)

Of the two options in the brief:

- **(A) Normalize-on-load** — decode the 16/32-bit samples, narrow to 8-bit for
  the existing model, record `source_depth`, render normally, and write 8-bit.
- **(B) True `u16`/`f32` model** — a second buffer type (or a generic sample)
  threaded through `pictura-core`, `pictura-render`, `pictura-color`, and
  `pictura-app`, preserving depth on save.

We take **(A)**, for three reasons:

1. **The model is 8-bit by construction.** `PixelBuffer.data: Vec<u8>`, the
   compositor, `pictura-color`, the ops crates, and `write_psd` all assume one
   byte per sample. (B) is not one change: it is a cross-crate type change with
   no consumer in this slice, because nothing downstream can *use* a 16-bit
   sample yet.
2. **The reference decoder already normalizes.** psd-tools' PIL path narrows
   every channel to 8-bit in `_create_image` before it hands pixels to any
   consumer; an oracle for (A) therefore exists and is exact (D3, D4). This is
   also the pattern the shipped `color-mode-read` slice established.
3. **(B)'s honest smallest version is still too large for one change, and the
   only thing it buys now is a lossless save.** If preserving depth on save
   becomes a requirement, (B) is the follow-up; the `source_depth` field (and,
   for the palette, `source_mode`) is the hook it will read. The roadmap's
   "needs a 16-bit sample representation" note is consistent with this being a
   later, separately-scoped change.

The cost of (A) is explicit: **opening a 16/32-bit PSD and saving writes an
8-bit file.** That is a real, documented loss, so the reader records
`source_depth` and the app warns (D7), exactly as the colour-mode slice does for
mode.

### D2. What is accepted, narrowed, and rejected

| Header depth | Bitmap (0) | Grayscale (1) | Indexed (2) | RGB (3) | CMYK (4) | Lab (9) |
|---|---|---|---|---|---|---|
| 1 | accept (bit-expand, shipped) | Unsupported | Unsupported | Unsupported | Unsupported | Unsupported |
| 8 | accept (as gray, shipped) | accept | accept | accept | accept | accept |
| 16 | Unsupported | accept, narrow | Unsupported | accept, narrow | accept, narrow | accept, narrow |
| 32 | Unsupported | accept, narrow | Unsupported | accept, narrow | accept, narrow | accept, narrow |

Any other depth is `PsdError::Unsupported`, as today. Bitmap is a 1-bit format
and Indexed's indices are 8-bit, so neither is meaningful at 16/32; both stay
rejected rather than miss-decoded. The mode gate and the header channel-count
check (`header_channels >= mode.color_channels()`) are unchanged. Multichannel
and Duotone remain `Unsupported` modes regardless of depth.

### D3. Depth 16: big-endian `u16`, row stride `2 * width`, narrow by `v >> 8`

A depth-16 sample is an unsigned big-endian 16-bit integer. The raw row byte
count is `2 * width`; a depth-16 RLE scanline's byte count refers to the
*compressed* bytes and its decode writes exactly `2 * width` bytes.
ZIP-with-prediction at depth 16 is a per-`u16` running sum: for each row and
`x` in `1..width`, `v[x] = (v[x] + v[x-1]) mod 2^16`, then the samples are read
big-endian. This is psd-tools `decode_prediction` depth 16
(`compression/__init__.py:448`) and ag-psd `decodePredicted(array, width,
height, 0x10000)` (`psdReader.js:1098`).

The 8-bit narrowing is

```
s8 = v >> 8        // v = big-endian u16, 0..=65535
```

Grounded and verified exactly: psd-tools' PIL path reads the channel as `I;16B`,
multiplies by `1/256`, and converts to `L` (`api/pil_io.py:413-415`), which is
`v >> 8`. A hand-built 8-pixel row with `v = [0, 1, 255, 256, 257, 32768, 65534,
65535]` decodes through psd-tools' `.composite()` to `[0, 0, 0, 1, 1, 128, 255,
255]` — identical to `v >> 8`. The oracle therefore compares with **tolerance
0**.

`ponytail:` this is `psd-tools`' convention, not verified as Photoshop's own
16→8 mode conversion (which rounds); the recorded ceiling is a possible ≤1
per-channel difference from Photoshop and no parity is claimed.

### D4. Depth 32: big-endian `f32`, row stride `4 * width`, narrow by `clamp(f * 256)`

A depth-32 sample is a big-endian IEEE-754 `f32`. The raw row byte count is
`4 * width`. ZIP-with-prediction at depth 32 stores the data *shuffled* (the
four byte planes of each row are packed together, `123412341234` → `111222333444`)
and delta'd byte-wise; on read the byte-wise delta is inverted over the `4 *
width`-byte row and the four planes are un-shuffled back into big-endian floats.
This is psd-tools `decode_prediction` depth 32 (`compression/__init__.py:451`,
`_shuffled_order`/`_restore_byte_order`) and ag-psd `readDataZip` depth 32
(`psdReader.js:1103`). Raw and RLE payloads are **not** shuffled — the shuffle
exists only inside the prediction codec.

The 8-bit narrowing is

```
s8 = clamp(trunc(f * 256), 0, 255)     // f = big-endian f32
```

Grounded and verified exactly: psd-tools' PIL path reads `F;32BF`, multiplies by
`256`, and converts to `L` with clipping (`api/pil_io.py:416-419`). A hand-built
row with `f = [0.0, 0.001, 0.5, 1.0, 1.5, -0.5, 0.99609375, 255.0]` decodes
through psd-tools' `.composite()` to `[0, 0, 128, 255, 255, 0, 255, 255]` —
identical to the formula. Tolerance 0 again.

`ponytail:` this is a **display-referred raw narrowing**, not Photoshop's 32-bit
HDR display. Values at or above 1.0 clip to white, values at or below 0.0 clip
to black, and no sRGB transfer is applied even if the file stores linear floats.
It matches the `psd-tools` reference and is offered so an HDR file opens at all;
true HDR tone mapping is part of the deferred (B) model.

### D5. One depth-aware decode path; narrow after decode

Rather than a second reader, the shipped decode paths are generalized:

- `row_bytes(width, depth) = (width * depth).div_ceil(8)` replaces the ad-hoc
  `if bits == 1 { width.div_ceil(8) } else { width }` at the composite and the
  `width.div_ceil(8)` inside `decode_bitmap_channel`.
- `decode_prediction` gains the depth argument (byte-wise 8, per-`u16` 16,
  shuffle+byte-wise 32). Depth 1 still rejects ZIP.
- The composite's decoded `data` is narrowed in place from 16/32 to 8-bit
  *before* `split_planes`, so `split_planes`, `normalize`, and every colour-mode
  conversion run on the 8-bit layout they already assume.
- `read_channel_data` (layers/masks) takes `depth` instead of the `bits` flag,
  decodes at the document depth, and narrows the plane to 8-bit before
  returning — the layer path then matches the composite, exactly as the depth-1
  Bitmap expansion already does. Non-color channels (`-1`, `-2`, spot) are
  narrowed too, because the model stores every plane as 8-bit.
- The Patterns resource keeps its own 8-bit `decode_channel_data` call
  (`crates/pictura-codec/src/patterns.rs:214`); patterns are always 8-bit and
  the 16-bit pattern fixture's skip behaviour is unchanged.

### D6. Record the source depth; keep the write 8-bit

`Document` gains `source_depth: Option<BitDepth>` (`None` default). On a
successful read it is `Some(BitDepth::Sixteen)` / `Some(BitDepth::ThirtyTwo)`
when the header depth was 16/32, and `None` otherwise. Depth-1 Bitmap keeps its
shipped `source_mode = Some(Bitmap)` and does **not** set `source_depth`, so the
1-bit path's behaviour, tests, and user-visible notice are byte-identical to
what shipped; the reason is documented on the field. `write_psd` is untouched
and still refuses `depth != BitDepth::Eight` — a document read from a 16/32-bit
file is `depth == Eight` after normalization and writes as an 8-bit RGB/Gray
file. `source_depth` is deliberately **not** re-emitted; re-encoding to the
source depth is the deferred (B) work this field will feed.

### D7. App notice and self-test

`PictureView` exposes `depth_notice() -> QString` returning `"Converted from
16-bit"` / `"Converted from 32-bit"` from `doc.source_depth` (empty otherwise),
mirroring `mode_notice` (`crates/pictura-app/src/cxxqt_object/impl_core.rs:232`)
and declared in the `qobject` trait next to it
(`crates/pictura-app/src/cxxqt_object.rs:112`). `frame.cpp::openPath` shows the
mode and depth notices together after a successful open
(`crates/pictura-app/cpp/frame.cpp:358`). One C++ self-test check takes the next
free exit code **298** (295 `present_cache_edge`, 296 `lpr_selective_color`, 297
`color_mode_open`; verified no `298`), writes a minimal flat 16-bit RGB PSD to a
`QTemporaryDir`, opens it, and asserts the open succeeds, `depth_notice()` names
16-bit, the working mode is RGB, and a composite pixel equals the expected
`v >> 8`. To avoid a `CMakeLists.txt` edit it is added to the existing
`selftest_layers_adjustments.cpp` sub-runner, as `color_mode_open` was.

**Concurrent-session risk.** `frame.cpp`, `impl_core.rs`, `cxxqt_object.rs`, and
the `selftest_layers_adjustments.cpp` sub-runner are the app files a socket
concurrent session is most likely to touch. The apply session must rebase those
edits onto whatever is present rather than overwrite, and should prefer the
single status-bar call site when merging.

### D8. Fixtures and oracles

| Fixture | Author | Proves |
|---|---|---|
| `rgb16.psd` | psd-tools, flat RGB, hand-built big-endian `u16` planes | the `v >> 8` narrowing and the depth-16 read |
| `rgb32.psd` | psd-tools, flat RGB, hand-built big-endian `f32` planes | the `clamp(f * 256)` narrowing and the depth-32 read |

Both are **flat** (no layers): psd-tools 1.19 cannot author a consistent
16/32-bit layered file — `create_pixel_layer` writes 8-bit channel bytes while
the header claims 16/32, and the merged composite is left 8-bit-sized, so
`PSDImage.composite()` then raises a length mismatch. Layer narrowing is
therefore proven by a hand-built Rust file in the `layered_psd_depth` style
(the shipped depth-1 precedent), not by psd-tools.

The oracle (`tests/psd/depth_oracle.rs`, mirroring `color_mode_oracle.rs`) re-reads
each fixture with psd-tools' `.convert("RGB")` composite and compares exactly
(tolerance 0); a Rust unit suite (`tests/depth.rs`) covers the RLE row stride,
the ZIP/ZIP-with-prediction layouts (streams built from the psd-tools
algorithm), the 16-bit layered narrowing, a 16-bit CMYK file narrowed-then-
converted, and the malformed cases (truncated 16-bit raw, 16-bit Bitmap, 16-bit
Indexed) as typed errors, never panics. A round-trip test reads each fixture,
asserts `depth == Eight` and `source_depth == Some(..)`, writes it, re-reads,
asserts `source_depth == None` with stable pixels, and confirms psd-tools opens
the output as 8-bit RGB. The depth fixtures are additive; no existing fixture or
golden changes. The one existing test to edit is
`unsupported_depths_and_color_modes_are_rejected`, which currently asserts
depths 16/32 are `Unsupported`: those two cases move to the new accepted set and
the test keeps a genuinely invalid depth (e.g. 4) plus the modes 7/8.

## Risks / Trade-offs

- **Save is lossy in depth.** Mitigated by `source_depth` + the app notice; a
  future (B) re-encode slice can consume the field. Also lossy in mode for a
  non-RGB source, exactly as the colour-mode slice documents.
- **The 16→8 rounding may differ from Photoshop by ≤1.** Mitigated by stating
  `psd-tools` as the reference and comparing exactly to it; no Adobe parity
  claimed.
- **32-bit float is display-referred, not HDR.** Highlights clip and no transfer
  is applied. Mitigated by the notice and the explicit `ponytail:` ceiling; (B)
  is the fix.
- **Per-layer narrowing before compositing is not Photoshop's pipeline.**
  Photoshop composites at the source depth. Same bounded approximation as the
  colour-mode slice.
- **`source_depth` struct-literal churn.** One `Option` field, defaulted by the
  existing constructors; the compiler enumerates the literals that list every
  field.
- **A concurrent app session.** See D7; rebase rather than overwrite.
- **psd-tools cannot author 16/32 layers.** Layer narrowing is covered by a
  hand-built Rust file; the psd-tools oracle covers the composite only.

## Migration Plan

None for documents: a previously refused file now opens; an 8-bit or depth-1
Bitmap file is read exactly as before (the Bitmap path is untouched). No
existing golden fixture changes. Rolling back is reverting the commit; the
`Document` field's default is `None`, so old constructed documents are
unchanged.

## Open Questions

- Whether the 16→8 conversion should match Photoshop's rounding rather than
  psd-tools' `v >> 8`; unresolved without a Photoshop-authored oracle, so the
  documented reference stands.
- Whether Photoshop stores 32-bit float linearly (which would make a future
  reader apply the sRGB transfer rather than read raw). The engine has no HDR
  pipeline, so this is deferred with (B).
- Whether the app should show the depth notice as a transient status message or
  a document property; the design assumes the status bar, like the mode notice.
