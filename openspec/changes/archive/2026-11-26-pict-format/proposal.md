# Proposal

## Why

`P-FMT-17` records that Photoshop CS6 reads and writes Apple QuickDraw PICT
files and that Kooka Pictura supports neither direction. PICT is a legacy
interchange format still found in archived Mac artwork and page-layout jobs.
Photoshop's own PICT support is bitmap-only: it opens PICTs that hold a bitmap
or pixmap and refuses pictures built only from QuickDraw drawing objects. The
format is publicly specified in *Inside Macintosh: Imaging With QuickDraw*
(Appendix A) and *Inside Macintosh: QuickTime* for the `$8200` JPEG record, so
an in-house, clean implementation is possible without a new dependency.

The parity tracker lists PICT beside IFF, PCX and SGI as the remaining legacy
formats. This change ships PICT only; the other three stay tracked by #1504,
#1505 and #1506.

## What Changes

- **Engine reader.** Add a Qt-free `pictura_codec` PICT decoder for version 1
  and extended version 2 files (with the 512-byte file header or a headerless
  picture). It reads `BitsRect`/`BitsRgn`, `PackBitsRect`/`PackBitsRgn`,
  `DirectBitsRect`/`DirectBitsRgn` and `CompressedQuickTime` (`$8200`) records:
  1-bit bitmaps, 1/2/4/8-bit indexed pixmaps with a colour table, 16-bit
  RGB555 direct pixels, and 32-bit direct pixels with or without alpha. Packing
  types 1 (unpacked), 2 (drop pad byte), 3 (pixel RLE) and 4 (component RLE)
  and per-scanline PackBits are decoded. Multiple image records composite in
  order, and an embedded QuickTime JPEG composites at its rectangle.
- **Safe refusals.** The decoder is bounds-checked throughout, charges the
  declared canvas and every record to `ImageBudget` before allocating, and
  returns a typed `PictError` rather than panicking. A picture with no bitmap
  record (vector-only), an unsupported pixel format or packing type, an
  uncompressed QuickTime record (`$8201`), a malformed record, a malformed
  embedded JPEG, or a size over budget is refused with a named reason.
- **Engine writer.** Add a `pictura_codec` PICT writer for extended version 2
  with a `DirectBitsRect`. Resolution 32 writes 32-bit direct pixels (RGB, or
  RGBA when the composite has transparency) with component PackBits;
  resolution 16 writes unpacked 16-bit RGB555. Compression None writes the
  PackBits form; JPEG Low/Medium/High/Maximum writes a `$8200` QuickTime JPEG
  and flattens alpha over white. Output is deterministic and goes through the
  existing atomic Save As path.
- **File workflows.** `File ▸ Open` sniffs PICT by its version opcode and
  routes it to the engine decoder (Qt has no PICT reader); `File ▸ Open As`
  offers a PICT filter; `File ▸ Save As` offers `PICT File (*.PCT *.PIC
  *.PICT)` and opens a PICT Options dialog for Resolution (16/32 bits per
  pixel) and Compression (None, JPEG Low/Medium/High/Maximum), remembering the
  last settings. The Save As filter, suffix aliases, feature-loss warning and
  dispatch use the same engine catalog as the other raster formats.
- **Documentation.** Record the supported bitmap-only scope, the ignored
  vector records, the 8191-pixel 32-bit and 16383-pixel 16-bit width ceilings,
  and the JPEG quality mapping in the roadmap, the parity row and the change
  design.
- **Out of scope.** QuickDraw vector rasterisation (lines, rectangles, ovals,
  arcs, polygons, regions, text and patterns are skipped, not drawn), OLE2
  "PICT" containers, `$8201` uncompressed QuickTime, PICT resources embedded in
  a resource fork, and IFF/PCX/SGI (#1504-#1506).

## Capabilities

### New Capabilities

- `codec/pict-format`: the PICT reader (supported records, packing, colour,
  compositing and safe refusals) and the PICT writer (resolution, compression,
  alpha, determinism and limits).

### Modified Capabilities

- `interop/image-import`: the raster probe recognises PICT, and the decode edge
  routes PICT to the engine decoder instead of Qt.
- `interop/open-as`: the Open As format list gains a PICT filter.
- `interop/save-format-options`: Save As gains the PICT filter, PICT Options
  dialog, remembered settings and dispatch.

## Impact

- **Engine (`pictura-codec`):** new `pict/` reader and `save/pict.rs` writer,
  shared PackBits helpers, `SaveKind::Pict`/`SaveFormat::Pict`, two `SaveOptions`
  fields, and `ImageFormat::Pict` in the probe. No new dependency: the existing
  `image` crate (JPEG feature) encodes and decodes the embedded JPEG.
- **App (Rust):** `decode_import` routes PICT to the engine; the typed Save As
  option parser reads the `pict` object; `save_engine` includes PICT in the
  sRGB conversion set.
- **App (C++):** `PictOptionsDialog`, `SaveFormatKind::Pict`, the Save As/Open
  filter rows, the Open As table, and the save dialog dispatch.
- **Tests:** codec unit tests with synthetic builders, an ImageMagick oracle for
  both directions, a committed decode fixture, Qt dialog/save cases, and a
  test-plan row for every scenario. A Photoshop CS6 round trip remains manual.
- **Docs:** `ROADMAP.md`, `PARITY.md` (`P-FMT-17`), and `docs/dev/STATE.md`.
- **Related issues:** closes #1503. Leaves #1504, #1505 and #1506 open.
