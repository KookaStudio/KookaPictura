# Design: pdf-eps-import

## Context

`File > Open` on a `.pdf` calls `pdf_open`, which reopens only a PDF carrying
Pictura's embedded PSD (#626). There is no PostScript or PDF rasterizer in the
tree, and the Rust workspace has no PDF parser beyond that payload search.
The owner's screenshots on #627 (Photoshop's Import PDF dialog) fix the layout
and defaults: Crop To **Bounding Box**, Anti-aliased on, sizes in **Inches**,
Resolution **300 Pixels/Inch**, Mode **RGB Color**, **8 bit**, Thumbnail Size
**Small**, `Image Size: <n>` as the uncompressed byte size, `<k> of <n>
page(s) selected`.

## Decisions

### D1 Ghostscript as an external process

Rendering every PDF and EPS the way a print RIP does (fonts, shadings,
transparency groups, JBIG2/JPX, broken-file repair, encryption) is out of reach
for a hand-written renderer, and no pure-Rust crate reads PostScript. Ghostscript
does all of it, and its `bbox` device and per-page info answer Crop To. It is
run as a child process (`std::process::Command`), never linked, so its AGPL
does not bind the binary; arguments always include `-dSAFER -dBATCH -dNOPAUSE
-dNOPROMPT -q` and file reads are limited with `--permit-file-read=<input>`.

The executable is `gs` on `PATH`; `PICTURA_GHOSTSCRIPT` overrides it (the test
seam that simulates an absent Ghostscript, and the escape hatch for a non-PATH
install). A missing or failing binary is an error value, never a panic; the
app reports "Ghostscript is required to open PDF and EPS files" and opens
nothing.

Rejected: Qt PDF (PDFium) cannot read EPS, exposes no page boxes, Bounding
Box, CMYK or 16-bit output, and on Flatpak drags in QtWebEngine. `lopdf` adds
~40 crates (second copies of the RustCrypto crates, an RNG stack) for what D4
needs in a few hundred lines.

### D2 Page info from Ghostscript

One `gs -dNODISPLAY` run of a fixed PostScript program (`runpdfbegin`,
`pdfpagecount`, `pdfgetpage`) prints each page's `MediaBox`, `CropBox`,
`BleedBox`, `TrimBox`, `ArtBox`, and `Rotate`. Pictura parses that text
(`pdf::import::parse_page_info`) and applies the PDF defaults: a missing
CropBox is the MediaBox; missing Bleed/Trim/Art boxes are the CropBox; every
box is clipped to the MediaBox. Bounding Box comes from the `bbox` device
(`%%HiResBoundingBox` per page), queried only when Crop To is Bounding Box; an
empty page falls back to its CropBox.

### D3 Rasterization: render, then crop

A page renders with `pngalpha` (8-bit RGBA, transparent where nothing paints,
so the page lands on a transparent layer like CS6's `Layer 1`) at per-axis
resolution `rx = 72·W / box_w`, `ry = 72·H / box_h`, so a box maps exactly to
the requested W × H pixels even with Constrain Proportions off. Ghostscript
renders the MediaBox, rotated by `/Rotate`; Pictura maps the chosen box through
the same rotation and crops it out of the render, so all six Crop To choices
share one path. Anti-aliased sets `-dTextAlphaBits` and
`-dGraphicsAlphaBits` to 4, off to 1. The PNG is decoded by a minimal reader
for exactly what `pngalpha` emits (8-bit RGBA, non-interlaced, filters 0–4),
the counterpart of the minimal PNG writer already in `pictura-codec`.

EPS uses the same path with `-dEPSCrop`, which renders the `%%BoundingBox`.

### D4 Images mode: a minimal object reader

Images mode lists each image XObject a page draws (including those inside its
Form XObjects) and opens each at its own pixel size. `pdf::import::objects`
reads just enough PDF: the object syntax and object streams (Flate with PNG
predictors). It finds objects by scanning the file for `N G obj` headers in
file order, a later definition replacing an earlier one as incremental updates
append, so no cross-reference table or xref stream is parsed and a broken one
is no obstacle. Each image is not decoded by Pictura: its stream and every
object it reaches (`/ColorSpace`, `/SMask`, `/Mask`, `/Decode`, ICC streams) are
copied into a one-page PDF whose MediaBox is `[0 0 w h]` and whose content
draws it full-page; Ghostscript renders that at 72 ppi into exactly w × h
pixels, so every filter (DCT, JPX, JBIG2, CCITT) and colour space is
Ghostscript's. Duplicate references to one image object are listed once.
Inline images and images inside patterns are not listed.

ponytail: Images mode reads unencrypted PDFs only; an encrypted PDF disables
the Images radio (pages still open with the password).

### D5 Mode and depth after render

The render is RGB 8-bit. Grayscale, CMYK, Lab, and 16 bit are reached with
the existing `convert_mode` / `convert_bit_depth`, the same conversions Image >
Mode applies.

ponytail: CMYK and 16-bit are converted from an 8-bit RGB render, not rendered
natively (Ghostscript's `psdcmyk16` flattens onto white, losing the
transparent layer); native device colour would need a second alpha pass.

### D6 The document

Each opened page or image is a new untitled document holding one transparent
layer, `Layer 1`, at the dialog's resolution (`write_resolution`). Its display
name is the dialog's Name for a single selection; for several, the Name's stem
plus `-<page or image number>` and its extension (*inferred*: the Help text
says only "base name plus a number"). Like any raster import, Save asks for a
path, so the source PDF is never overwritten; Open Recent remembers the PDF.

### D7 Encrypted PDFs

Ghostscript reports `This file requires a password` (or `Password did not work`
on a wrong one). The window then asks for a password (`QInputDialog`, echo off)
and retries; Cancel opens nothing. The password reaches Ghostscript as
`-sPDFPassword=` inside an owner-only (0600) `@file` argument file in the
run's scratch directory, never in argv, where `ps` and `/proc` would show it.
That file format escapes only `"` and cannot spell a trailing backslash or a
line break. ponytail: such a password alone falls back to argv.

Scratch directories (renders, wrapped images, the argument file) are created
with `create_dir`, mode 0700, and an unguessable random suffix, so another
user cannot pre-create or symlink the path.

### D8 Suppress Warnings

Ghostscript's recoverable complaints (stderr lines starting `****` or
`WARNING`) and images Images mode could not list are collected; after the open
they are shown in one message box unless Suppress Warnings is checked.

### D9 Detection and Open As

Routing sniffs the first 1 KiB: `%PDF-` is PDF (a Pictura Photoshop PDF still
reopens with layers first), `%!PS-Adobe` or the DOS EPS magic `C5 D0 D3 C6` is
EPS; other files keep their existing path. `File > Open As…` shows the Open
file dialog with one filter per format (Photoshop, Large Document Format,
Generic PDF, Generic EPS, PNG, JPEG, TIFF, WebP, GIF, BMP); the chosen filter
names the decoder and skips sniffing. When that decoder rejects the file the
window says "Could not open <file>: format mismatch or damaged file."

### D10 Thumbnails

Page thumbnails come from one Ghostscript run over the pages at a resolution
that makes the largest page 256 px on its long side (written to a temporary
directory, one PNG per page); Images-mode thumbnails reuse each image's wrapped
render. Small shows them at 64 px, Large at 128 px, Fit Page at the preview's
width.
