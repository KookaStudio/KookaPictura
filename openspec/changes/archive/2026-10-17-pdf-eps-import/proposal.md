# Proposal: pdf-eps-import

## Why

Issue #627 (parity register row WF-3, from the #486 audit): `File > Open` reads
only Pictura's own Photoshop PDFs (the embedded PSD, #626); any other PDF is
refused, EPS is not read at all, and `File > Open As…` is an inert menu leaf
(`command_tree.cpp`). `docs/10-workflow-io/open-and-new.md` *Open PDF* and
*Open EPS* specify the **Import PDF** dialog (Pages / Images, thumbnails, Name,
Crop To, Anti-aliased, Image Size, Mode, Bit Depth, Suppress Warnings) and the
EPS rasterize options; *Format detection and dispatch* specifies signature
sniffing and `Open As` overriding it.

## What Changes

- **Renderer**: Ghostscript (`gs`) run as a child process with `-dSAFER`, the
  external renderer adapter `docs/01-architecture/file-formats.md` proposes. It
  rasterizes PDF, EPS, and PostScript, renders every page box, reports the
  Bounding Box (`bbox` device), and toggles anti-aliasing. It is not linked
  (its AGPL stays out of the binary); when it is absent, opening a PDF or EPS
  reports a clear error and nothing crashes.
- **PDF structure** `pictura_codec::pdf::import`: page count, the five page
  boxes with their defaults and `/Rotate`, and the Bounding Box, all reported
  by Ghostscript; for Images mode, a minimal object reader (no new
  dependency) lists the image XObjects of each page and wraps each in a
  one-page PDF so Ghostscript decodes every filter and colour space.
- **EPS** `pictura_codec::eps`: the DOS binary EPS header and the
  `%%BoundingBox` / `%%HiResBoundingBox` comments.
- **App**: the **Import PDF** dialog matching the CS6 layout (owner
  screenshots on #627), a password prompt for encrypted PDFs, the **Rasterize
  Generic EPS Format** dialog, `File > Open As…` with a format list that
  overrides detection, and signature-based routing of `.pdf` / `.ai` / `.eps`
  in Open, Open Recent, and drag-and-drop.
- CI and Flatpak ship Ghostscript.

## Capabilities

### New Capabilities

- `interop/pdf-eps-import`: generic PDF and EPS import through the Import PDF
  and Rasterize Generic EPS Format dialogs.
- `interop/open-as`: `File > Open As…` picks the decoder.

### Modified Capabilities

None. `interop/pdf-export`'s reopen of Pictura's own Photoshop PDFs is kept
and now sits in front of the Import PDF dialog.

## Impact

- No new crate. Rendering needs `gs` at run time; the Flatpak bundles it.
- Not in scope: placing a PDF or EPS as a smart object (`File > Place`), the 3D
  option (no 3D engine; the radio is shown disabled), legacy PostScript `.ai`
  files (PDF-compatible `.ai` files open through Import PDF).
