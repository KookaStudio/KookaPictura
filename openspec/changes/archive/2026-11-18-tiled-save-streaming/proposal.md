# Proposal

## Why

Issue #1320, a follow-up of #1218. Before a native save starts, it reserves
`2 × document + min(document, 4 × canvas) + 160 MiB`. For M24 (the M12
fixture at 17000²: four dense layers and a white Background) that is about
11 GB, while the file it writes is 104 MB. Under `MemoryMax=2G` the save is
refused, whether the document was opened eagerly or lazily. Open, pan and fill
all fit (peak 1792 MiB). #1218's acceptance item "M12 under MemoryMax=2G:
open, pan, edit, save" stays open until the save fits.

The estimate is wrong in both directions:

- **Too high.** The save job's copy of the document shares every tile, so it
  costs only the tile tables, not a second document.
- **Unbounded.** The writer holds the whole encoded file in one `Vec<u8>`.
  Every encoded layer channel is a whole-plane `Vec`. The merged image is
  copied into a flat `3 × canvas` buffer (867 MB at 17000²). The session's
  encode cache then keeps every encoded stream after the save. The encoded
  size is unknown until the encode runs: a safe upper bound is the raw
  document, which would still refuse M24.

The raster Save As path is worse. It builds a flat composite, then a `Raster`
of flat RGBA, RGB and gray copies (about 15 bytes per pixel), then the encoded
file, and only the engine formats pass a memory gate at all.

Because the encoded size cannot be predicted, no estimate alone can be both
safe and admit M24. The save has to hold a bounded amount instead.

## What Changes

By owner decision (2026-11-17): stream the native and the row-based raster
saves, budget the encode cache, and give the formats that still need the
whole image an honest estimate.

- **Streamed native save.** The PSD/PSB writer writes into the save's temp
  file through a seekable sink. Section lengths, layer channel lengths and RLE
  scanline-count tables are written as placeholders and patched afterwards.
  Layer channels and the composite encode one band of rows at a time, with the
  band's rows encoded in parallel. ZIP and ZIP-with-prediction feed the
  deflater row by row. The bytes equal the in-memory encode, which stays the
  same code writing into a `Vec`.
- **A tiled merged image.** The save view composites the merged RGB straight
  into a tiled 3-channel plane; it no longer copies the composite into a flat
  buffer. The tiles can be packed or spilled while the encoder reads them.
- **A byte-budgeted encode cache.** The cache keeps encoded streams only while
  they fit a share of the memory governor's ceiling and the governor's room.
  A stream over the budget is written and not cached.
- **Streamed raster saves.** PNG, BMP (16/24/32-bit), TGA, Raw, Cineon and
  TIFF read rows from a tiled composite built band by band, and write them into
  the temp file. TIFF also writes its layer section and metadata in the file.
  GIF, JPEG, WebP, EPS, DCS, BMP at 8 bits or fewer, TIFF-JPEG, and the
  CMYK/Lab outputs keep a whole-image buffer under `ponytail:` comments, and
  are gated with an estimate of that buffer.
- **A save estimate of what a save holds.** The native save reserves the tile
  tables, one compositor band and the encoder's working rows, and admits the
  merged image's tiles against the room plus the scratch disks' free space.
  Raster saves are estimated per format, and every raster path is gated.

## Capabilities

- **ADDED** `codec/streamed-psd-save`
- **MODIFIED** `codec/psd-encode-cache` (parallel encode is per band of rows;
  the cache is byte-budgeted)
- **ADDED** `codec/streamed-raster-save`
- **ADDED** `document/save-memory-gate`

## Out of scope

- 16/32-bit, CMYK, Lab, Indexed, Bitmap and Duotone PSD outputs: they re-emit
  or convert the retained native stores, which are flat already. They keep
  their whole-plane work under `ponytail:` comments.
- PDF and Save for Web, which have their own writers.
- Copying an unchanged lazily backed layer's encoded bytes straight from the
  source file.
