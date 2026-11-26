# Design

## Scope and sources

PICT is Apple's QuickDraw metafile. Photoshop CS6 reads and writes the format
as a bitmap interchange, not as a QuickDraw renderer: it opens pictures that
carry a bitmap or pixmap and does not rasterise drawing objects. This change
follows that scope. The format facts come from *Inside Macintosh: Imaging With
QuickDraw* (Appendix A, picture opcodes) and *Inside Macintosh: QuickTime* (the
QuickTime image description used by `CompressedQuickTime`). No Adobe source or
binary is consulted.

The reader accepts:

| Record | Opcodes | Forms |
|---|---|---|
| Bitmap | `$0090`, `$0091`, `$0098`, `$0099` | 1-bit, `rowBytes` high bit clear |
| Indexed pixmap | `$0090`, `$0091`, `$0098`, `$0099` | 1/2/4/8 bits, `rowBytes` high bit set, colour table |
| Direct pixmap | `$009A`, `$009B` | 16-bit RGB555, 32-bit RGB/RGBA |
| QuickTime JPEG | `$8200` | baseline/progressive JPEG in the QuickTime wrapper |

Version 1 and extended version 2 are read. A file may carry the standard
512-byte application header or be a headerless picture (resource/clipboard
form); the version opcode at offset 522 or 10 selects the form. Version 2 uses
two-byte opcodes and word alignment; version 1 uses one-byte opcodes.

The writer emits extended version 2 only. That is the form Photoshop and
ImageMagick write and read, and it carries the resolution header the options
dialog controls.

## Reader

### Header

- `picSize` is read and ignored in version 2 (Apple says it is not meaningful
  there); the frame follows it.
- The frame (`top`, `left`, `bottom`, `right`, signed 16-bit) is the canvas. A
  degenerate frame is malformed.
- The version opcode is `$0011` in version 2 followed by `$02FF` (extended).
  Version 1 is the byte pair `$11 $01`. A headerless picture is recognised when
  the same bytes appear at offset 10.
- The canvas is charged to `ImageBudget` (`max_dimension`, `max_alloc_bytes`)
  before it is allocated. A record's own bounds are charged the same way.

### Opcode loop

The loop reads an opcode, skips the records it does not draw, and decodes the
bitmap records. It stops at `$00FF` (version 2) or `$FF` (version 1) and
requires at least one decoded bitmap record.

Skipping follows the published sizes:

- fixed-size opcodes use their table size;
- `$0100..=$7FFF` skip `2 * (opcode >> 8)` bytes (this covers `HeaderOp` at
  `$0C00`);
- `$8100..=$FFFF` skip a 32-bit length and that many bytes;
- `$8000..=$80FF` skip nothing;
- region and polygon records carry a 16-bit size;
- comments carry a kind and a 16-bit length.

The parser does not attempt to draw lines, rectangles, ovals, arcs, polygons,
regions, text or patterns. It also refuses the pattern opcodes (`$0012`-
`$0014`) because their nested pixmap has no fixed size to skip safely. A picture
with no bitmap record is refused as `Unsupported("PICT has no bitmap image
records")`.

### Bitmap records

`BitsRect`/`BitsRgn` and `PackBitsRect`/`PackBitsRgn` begin with `rowBytes`
(two bytes) and a bounds rectangle. When the high bit of `rowBytes` is set the
record is a `PixMap` and the remaining 36 bytes of the structure follow;
otherwise it is a `BitMap` and the data is 1-bit. A pixmap then carries a
colour table (seed, flags, count, entries), then `srcRect`, `dstRect`, `mode`
and the pixel data. `DirectBitsRect`/`DirectBitsRgn` begin with the base
address, `rowBytes`, bounds and the rest of the `PixMap`, and carry no colour
table.

`BitsRect`/`BitsRgn` are unpacked. `PackBitsRect`/`PackBitsRgn` use a
per-scanline PackBits stream. `DirectBits*` use the `packType` in the `PixMap`:

- 0 selects the default (16-bit → 3, 32-bit → 4);
- 1 is unpacked;
- 2 drops the pad byte of a 32-bit pixel (three bytes per pixel);
- 3 is PackBits over the 16-bit pixel bytes;
- 4 is PackBits over the component planes (red first, alpha first when four
  components).

`rowBytes < 8` forces the unpacked form, as the specification requires. A
scanline length of zero is treated as an empty (all-zero) row so a writer that
uses it for blank rows still decodes.

### Colour

- A 1-bit bitmap maps 0 to the background colour and 1 to the foreground
  colour. The default is white/black; `FgColor`, `BkColor`, `RGBFgCol` and
  `RGBBkCol` update the two colours.
- An indexed pixmap maps 1/2/4/8-bit samples through the colour table. The
  table's high `ctFlags` bit means the entries are in index order; otherwise the
  entry's value is its index (modulo the table size). An index outside the
  table is black.
- 16-bit direct pixels are big-endian `0RRRRRGGGGGBBBBB`; the 5-bit channels
  scale to 8 bits with `v << 3 | v >> 2`.
- 32-bit direct pixels are RGB or RGBA. Component-RLE rows hold the planes in
  red, green, blue order, or alpha, red, green, blue when `cmpCount` is four.
  Unpacked rows hold `[pad, R, G, B]` or `[A, R, G, B]`; drop-pad rows hold
  `[R, G, B]`.

### Compositing

Each image record is drawn into the canvas at its `dstRect` with a nearest
neighbour map when the destination size differs from the source bounds. The
destination is replaced (QuickDraw `srcCopy`), so a source with alpha writes
that alpha. The canvas starts opaque white. A JPEG record is decoded by the
`image` crate (already present with its JPEG feature) and placed at the
rectangle in its QuickTime wrapper; a degenerate rectangle places it at 0,0.

The reader keeps one decoded record at a time, so peak memory is the canvas plus
one record. A record counter caps a hostile file at 4096 image records.

### Errors

`PictError` names the refusal:

- `Malformed(&'static str)` for truncated or inconsistent bytes;
- `Unsupported(&'static str)` for a known-but-unimplemented record or pixel
  format, a vector-only picture, an uncompressed QuickTime record, or too many
  records;
- `DimensionLimit` / `AllocationLimit` for the budget;
- `Jpeg` for an embedded JPEG the decoder rejects.

`decode_pict` returns the error; `decode_pict_rgba` is the `Option` form the
application's decode edge uses, mirroring `decode_webp`.

## Writer

`save/pict.rs` writes extended version 2:

1. 512 zero bytes of application header.
2. `picSize` = 512 (ignored by version 2 readers), frame `(0, 0, h, w)`.
3. Version opcode `$0011 $02FF`, then `HeaderOp` `$0C00` with the extended
   marker `$FFFE`, 72 dpi in 16.16 fixed point, the frame, and a reserved zero.
4. A clip region equal to the frame.
5. One image record:
   - **32-bit, no compression:** `DirectBitsRect` with base address
     `$000000FF`, `rowBytes = 4w | $8000`, `packType 4`, `pixelType 16`,
     `pixelSize 32`, `cmpCount` 3 or 4, `cmpSize 8`, `srcCopy` mode. Each row
     is the concatenated component planes, PackBits encoded, with a one- or
     two-byte length prefix (`rowBytes > 250` selects two bytes).
   - **16-bit, no compression:** `DirectBitsRect` with `rowBytes = 2w | $8000`,
     `packType 1`, `pixelSize 16`, `cmpCount 3`, `cmpSize 5`, and unpacked
     big-endian RGB555 rows. Alpha is flattened over white.
   - **JPEG Low/Medium/High/Maximum:** `$8200`, a 154-byte QuickTime image
     description, then the JPEG stream and a pad byte when its length is odd.
     The JPEG is RGB; alpha is flattened over white. The app-supplied JPEG
     encoder is used when present, otherwise the `image` crate's encoder.
6. `EndOfPicture` `$00FF`, with the pixel data padded to an even length.

Quality mapping: None = no JPEG; Low/Medium/High/Maximum = JPEG quality 3, 6,
9 and 12 on the CS6 0-12 scale (the existing `jpeg_quality_percent` table maps
that to the encoder's 0-100 scale).

Limits: the frame is signed 16-bit, so either dimension must be at most 32767.
A 32-bit `rowBytes` must fit 15 bits, so the width is at most 8191; a 16-bit
row is at most 16383 pixels wide. Wider images are refused rather than split
into tiles. The writer is deterministic and the existing `save_document` path
writes it through a temporary file and rename.

## Application wiring

- **Probe.** `ImageFormat::Pict` parses the frame at offset 514 (file form) or 2
  (headerless) and reports the width/height. `sniff` checks the version bytes
  at 522/10 before the generic fallback.
- **Decode.** `decode_import` routes a probed PICT to `decode_pict_rgba` and
  falls back to Qt only as it does for WebP. Qt has no PICT reader, so the
  engine is authoritative for this format.
- **Save As.** `SaveFormat::Pict` (`id` 14, wire code 11) has aliases `pct`,
  `pic` and `pict`, canonical `.pct`, `SaveRoute::Engine(SaveKind::Pict)` and
  `FLAG_RASTER`. The C++ catalog adds the `PICT File (*.PCT *.PIC *.PICT)` row,
  the options kind `Pict = 11`, and the `pict` JSON object
  (`resolution`, `compression`). `PictOptionsDialog` offers the two resolutions
  and five compression choices; the accepted values are remembered per session.
- **Open.** The Open filter lists add PICT, and Open As gains a PICT row. The
  Open As signature list is empty for PICT so the engine decoder decides,
  because the version bytes are not in the first 16 bytes.
