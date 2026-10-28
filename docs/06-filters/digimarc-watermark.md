# Digimarc Watermark (Embed / Read)

- **Spec ID:** `FILT-105`
- **Status:** `Spec'd`
- **Parity tier:** `Approximate (in-tree scheme; not Digimarc-compatible)`.
- **New in CS6:** `No` — Embed/Read Watermark shipped in earlier versions and in CS6.
- **Depends on:** `06-filters/filters-overview.md`, `06-filters/extract-and-pattern-maker.md`
  (the non-goal precedent), `02-ui-ux/menus.md`, `02-ui-ux/application-frame.md`.

> This feature is **not** Adobe's Digimarc Barcode. Adobe's Embed Watermark called
> the licensed Digimarc service; the algorithm and the service are proprietary and
> unavailable, so exact parity is a non-goal. This document is a **decision record**
> for the in-tree approximation that ships instead, and stays the behavioural
> contract for it. Behavior is taken from the CS6 Help corpus unless marked
> *(inferred)*; the carrier scheme is the project's own.

## CS6 behavior

- `Filter ▸ Digimarc ▸ Embed Watermark` opens a dialog that writes a Digimarc
  watermark into the image. The dialog carries **Image Info** — a **Creator ID**
  (issued by Digimarc) and a watermark **type**, **Image ID** or **Transaction
  ID** — and **Watermark Durability** (Low / Medium / High). Higher durability is
  more resistant to editing but more visible. A `Personalize` button registered
  the creator with Digimarc online; a `Verify` button checked the mark.
- `Filter ▸ Digimarc ▸ Read Watermark` detects a mark and reports the creator and
  image information. A detected mark adds a copyright symbol to the document's
  title bar (`02-ui-ux/application-frame.md`).
- Both are listed in the CS6 Filter effects reference under the **Digimarc** group.

## Decision

Issue #350 asked for an explicit scope decision before any code (rule 8). Three
options were weighed: a documented non-goal (like Extract/Pattern Maker), a
refuse-with-message stub, or an in-tree approximation. The project chose the
**approximation**: a deterministic, in-tree watermark that never claims Digimarc
compatibility, so the menu entries do something honest and testable instead of
staying dead stubs. `Filter ▸ Digimarc ▸ Read Watermark` (#351) ships in the same
change so the payload round-trip is testable.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Digimarc > Embed Watermark` | Menu | — | Opens the Embed Watermark dialog (shared filter dialog) |
| Embed Watermark — Watermark Type | Choice | — | `Image ID` / `Transaction ID` |
| Embed Watermark — Creator ID | Number | — | `0..8 388 607` |
| Embed Watermark — Image ID | Number | — | `0..16 777 215` (the type's ID) |
| Embed Watermark — Watermark Durability | Radio | — | `Low` / `Medium` / `High` |
| `Filter > Digimarc > Read Watermark` | Menu | — | Detects and reports; sets the title indicator |
| Read Watermark — result dialog | Dialog | — | Type, Creator ID, ID, or "no watermark found" |
| Document title bar | Indicator | — | Copyright symbol when a mark was detected |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Watermark Type | choice | `Image ID` | Image ID / Transaction ID | 1 bit in the payload |
| Creator ID | number | `0` | `0..=8 388 607` | 23-bit field |
| Image ID | number | `0` | `0..=16 777 215` | 24-bit field |
| Watermark Durability | radio | `Medium` | Low / Medium / High | Maps to `(repetition, amplitude)` |

Durability: **Low** `(×1, ±4)`, **Medium** `(×2, ±6)`, **High** `(×4, ±8)`.

## Algorithms & pipeline

*(approximate; the project's own scheme, not Digimarc.)*

- **Payload.** A 66-bit codeword: an 8-bit magic byte (`0xA5`), a 2-bit
  durability field, a 1-bit type, a 23-bit creator ID, a 24-bit ID, and an 8-bit
  CRC-8 (polynomial `0x07`) over the preceding 58 bits.
- **Carrier (spread spectrum).** The image's colour planes are cut into 8×8
  cells in raster order. The codeword's bits are repeated by the durability's
  repetition factor to a message of `66 × repetition` bits; cell `c` carries
  message bit `c % message`. Every pixel is nudged by `amplitude × chip(x, y) ×
  sign(bit)`, where `chip` is a deterministic ±1 splitmix64 hash of the pixel's
  `(x, y)`. The chip is zero-mean within a cell, so the carrier preserves the
  local average and is a pseudo-random pattern spread across the plane. The
  transparency plane is never touched.
- **Reader.** For each cell, each colour plane's samples are mean-removed and
  correlated against the same chip; the per-message-position correlations are
  summed and each codeword bit's copies are added and signed. The reader tries
  each durability whose message fits the image, requires the decoded durability
  field to match, and accepts the first codeword whose magic and CRC-8 validate.
  The detection margin is the smallest per-bit absolute correlation.
- **Not Digimarc.** The carrier does not interoperate with Digimarc readers, and
  this build never claims it does. Durability raises redundancy and amplitude; it
  is not a robustness guarantee against crop, rotate, or heavy recompression.

## Rust module mapping

- `pictura_filters::digimarc` — `WatermarkKind`, `Durability`, `WatermarkPayload`,
  `embed`, `read`, `read_with_margin`.
- `Filter::EmbedWatermark { kind, creator_id, id, durability }` with the catalogue
  row `embed-watermark` (`Filter ▸ Digimarc ▸ Embed Watermark`).
- `read_watermark` bridge method (active pixel layer → decoded payload).

## Qt6 component mapping

- `FilterPreviewDialog` (shared) renders the Embed Watermark controls and its
  live preview.
- `DigimarcReadDialog` (new) reports a Read Watermark result.
- The document title indicator lives in `document_window.cpp::fullLabel` behind
  `DocEntry::watermarkDetected`.

## Data-model impact

- **None.** The watermark lives in pixels only; it is not stored in PSD metadata.
  Detection is recomputed by Read Watermark. `DocEntry::watermarkDetected` is
  transient session state.

## Edge cases

- **Small images** — a durability whose message does not fit the image's cell
  count is skipped by the reader; High needs more cells than Low.
- **Saturation** — pixels at 0 or 255 clip the carrier; the reader's mean removal
  and majority vote tolerate the lost samples on ordinary images.
- **Selection** — the filter is whole-layer, so a partial selection cannot starve
  the reader.
- **Grayscale** — a Grayscale layer carries the carrier on its single colour
  plane, like any value filter.
- **Preview vs commit** — a viewport preview anchors the carrier at the viewport
  origin, not the layer origin; the preview is invisible and non-committing.
- **Transparency** — the alpha plane is never modified.
- **Re-embedding** — a second Embed Watermark on the same pixels compounds the
  carriers, so the reader may fail or report the earlier payload. Read the mark
  before replacing it, or embed on a fresh copy of the pixels.

## Parity acceptance criteria

- Given a document with a pixel layer, `Filter ▸ Digimarc ▸ Embed Watermark` is
  enabled and opens a dialog with Watermark Type, Creator ID, Image ID, and
  Watermark Durability; OK records one history state and Cancel changes nothing.
- Given a payload embedded on the active layer, `Filter ▸ Digimarc ▸ Read
  Watermark` reports the same type, creator ID, and ID.
- Given a detected watermark, the document title shows a copyright symbol.
- Given an image with no carrier, Read Watermark reports that none was found.
- Given any Digimarc-marked file from Adobe, this build does not claim to read it;
  the mark is reported as absent unless it was written by this build's scheme.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6
  Help corpus: the Digimarc group in the Filter effects reference; the Embed
  Watermark dialog's Image Info (Creator ID, Image ID / Transaction ID) and
  Watermark Durability (Low/Medium/High, higher = more visible); Read Watermark
  detection; the title-bar copyright symbol on detection.

Marked inferred: the exact legacy dialog layout and the `Personalize`/`Verify`
buttons, which this approximation does not implement.

## Open questions

- **Durability semantics.** CS6's Low/Medium/High are Digimarc's, not ours; our
  mapping to `(repetition, amplitude)` is a project choice. Revisit if a real
  robustness target is set.
- **Composite vs layer.** Read inspects the active pixel layer. If whole-document
  detection is wanted, embed and read must agree on a document-origin anchor.
