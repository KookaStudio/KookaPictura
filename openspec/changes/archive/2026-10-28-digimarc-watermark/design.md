# Design

## Context

See `proposal.md` for motivation and `specs/` for the behavioral contract.

Filters are pure functions over a planar 8-bit `PixelBuffer`
(`crates/pictura-filters`), reached from the app through the filter catalogue:
a `Filter` variant, a catalogue row (params + menu data + a slot builder), and
the generic `FilterPreviewDialog` / commit path. The app applies a filter to the
active pixel layer, cropped to a clip `PixelBuffer` with no absolute origin
(`pictura-render/src/filter.rs`), so a kernel must be a function of the clip's
own coordinates. A commit crops the whole layer rect (origin at the layer's
top-left); a preview crops the viewport plus an apron.

`docs/02-ui-ux/application-frame.md` already specifies that a Digimarc-detected
watermark adds a copyright symbol to the document title bar, and
`docs/02-ui-ux/menus.md` already lists both commands as enabled.

## Goals / Non-Goals

**Goals:**

- A deterministic, in-tree watermark that survives its own round-trip: embed a
  payload, read the same payload back.
- A CS6-shaped Embed dialog (Image Info + Watermark Durability) reusing the
  generic filter dialog, its live preview, undo, and Last Filter.
- A Read command that reports the payload and drives the title indicator.
- Honest labeling: the scheme is our own and is never presented as Digimarc.

**Non-Goals:**

- Digimarc Barcode interoperability or any claim of Digimarc compatibility.
- Surviving arbitrary attacks (crop, rotate, heavy recompression, resampling).
  Durability only raises the redundancy and amplitude; the contract is a clean
  round-trip plus a monotone detection margin, not a robustness guarantee.
- Persisting the payload in PSD metadata. The watermark lives in pixels only;
  detection is recomputed by Read Watermark, matching CS6's pixel watermark.
- Watermarking the document composite directly; the filter targets the active
  pixel layer, like every other destructive filter.

## Decisions

### 1. `pictura_filters::digimarc` owns the payload and the carrier

The module exposes:

- `WatermarkKind { ImageId, TransactionId }` and
  `Durability { Low, Medium, High }`.
- `WatermarkPayload { kind, creator_id, id }`.
- `embed(buf: &mut PixelBuffer, payload: &WatermarkPayload, durability) ->
  Result<(), FilterError>`.
- `read(buf: &PixelBuffer) -> Option<WatermarkPayload>` and
  `read_with_margin(buf) -> Option<(WatermarkPayload, i64)>`.

The codeword is 66 bits:

| field | bits | notes |
|---|---|---|
| magic | 8 | `0xA5`, rejects most false positives |
| durability | 2 | `0` Low, `1` Medium, `2` High |
| type | 1 | `0` Image ID, `1` Transaction ID |
| creator ID | 23 | `0..=8_388_607` |
| ID | 24 | `0..=16_777_215` |
| CRC-8 | 8 | polynomial `0x07` over the first 58 bits |

Each codeword bit is repeated `R` times (the durability's repetition) to form a
message of `M = 66 * R` bits. The image is cut into 8×8 cells in raster order;
cell `c` carries message bit `c % M`. Every pixel in a cell is nudged by
`amplitude * chip(x, y) * sign(bit)`, where `chip` is a deterministic ±1
splitmix64 hash of the pixel's `(x, y)`, so each cell is DC-neutral and the
carrier is a zero-mean pseudo-random pattern (spread spectrum). Durability maps
to `(R, amplitude)` = Low `(1, 4)`, Medium `(2, 6)`, High `(4, 8)`; higher
durability is more redundant and slightly more visible, matching CS6's note that
a more durable watermark is more visible.

Reading computes, per cell, the correlation of each colour plane against the
chip after removing the cell mean, accumulates the correlations per message
position, sums each codeword bit's `R` copies, and takes the sign. It tries each
durability whose message fits the image, requires the decoded durability field to
match the trial, and accepts the first codeword whose magic and CRC-8 validate.
`read_with_margin` returns the smallest per-bit absolute correlation, the
detection margin the durability test compares.

### 2. Embed is a catalogue filter, Read is a bespoke command

Embed changes pixels and wants the standard filter affordances, so it is a
`Filter::EmbedWatermark` variant with a `menu: true` catalogue row whose path
`["Filter", "Digimarc", "Embed Watermark"]` derives the existing command id. Its
four controls are a Choice (Watermark Type), two Numbers (Creator ID, Image ID),
and a Radio (Watermark Durability), all supported by the shared dialog. The row
sets `whole_layer: true` so a selection cannot embed only part of the layer and
starve the reader, `alpha: Never` (transparency is untouched), and
`gpu: CpuPreferred` (the carrier is cheap and byte-exact on the CPU).

Read does not change pixels, so it is not a filter: a `read_watermark` bridge
method returns the decoded payload from the active pixel layer's buffer
(`helpers_composite::layer_buffer`), and a `wireFilterMenu` sibling registers the
Read handler, which reports the result and sets the title indicator. Reading the
active layer keeps embed and read anchored to the same buffer origin; the
composite would be anchored to the document origin instead.

### 3. The title indicator is per document

`DocEntry` gains `bool watermarkDetected`; `fullLabel` appends `" ©"` when it is
set, so the tab, float window, and tile labels all carry it through the existing
`updateTabTitle` refresh. Read Watermark sets the flag on success and leaves it
unset on a miss; it is transient session state, recomputed by re-reading.

## Risks / Trade-offs

- **False positives.** An 8-bit magic plus CRC-8 over 66 bits keeps a random
  image's false-positive probability near `2^-16`; the durability field and
  per-durability trial remove the ambiguity between repetition factors.
- **Small images.** A durability whose message does not fit the cell count is
  skipped, so High needs more cells than Low. The reader documents the minimum.
- **Preview vs commit phase.** A viewport preview anchors the carrier at the
  viewport clip origin, not the layer origin, so a previewed carrier is not
  byte-identical to the commit's. The carrier is invisible and non-committing, so
  this is cosmetic; the commit and the reader share the layer origin.
- **Clipping.** Pixels near 0/255 saturate; the reader's per-cell mean removal and
  majority vote tolerate the lost samples on ordinary images.
