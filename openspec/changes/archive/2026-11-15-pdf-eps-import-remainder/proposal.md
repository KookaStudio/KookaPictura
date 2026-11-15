# Proposal

## Why

Issue #627's first half shipped (#727): the Import PDF and Rasterize Generic
EPS Format dialogs, page rendering, Images mode, and Open As. The reopened
remainder still has four holes. Mode and bit depth are labels over an 8-bit
RGBA render (CMYK and 16 bit are reached by converting that raster). Images
mode is disabled on every encrypted PDF. Inline images never appear in the
Images grid. The 3D radio is shown disabled and cannot be implemented: this
project has no 3D engine, and ROADMAP lists 3D tools and layers as not planned.

The first three are gaps in the shipped import path and close the issue. The
fourth is scope the project has already declined; leaving it attached to #627
would keep the issue open forever for a feature the repository does not target.

## What Changes

- **Native mode and depth.** Pages, embedded images, and EPS opened as CMYK
  (8- and 16-bit) or 16-bit RGB SHALL render through Ghostscript's native PSD
  devices instead of converting from an 8-bit RGB render; 16-bit Grayscale
  SHALL derive from the native RGB 16-bit render at 16-bit precision. A second
  `pngalpha` pass recovers the page's coverage; the white-paper PSD render is
  un-composited through that coverage so unpainted areas stay transparent, and
  the RGB display is derived from the un-composited native store. Anti-aliasing
  and cropping keep their current behavior.
- **EPS Bit Depth.** The Rasterize Generic EPS Format dialog SHALL gain
  **Bit Depth** (8 bit, 16 bit; default 8 bit), matching the Import PDF dialog,
  so the 16-bit native EPS path is reachable.
- **Encrypted Images mode.** The password already collected for pages SHALL be
  passed to the Images object reader. The reader SHALL decrypt with the
  existing standard security handler and list/open the images when the password
  unlocks them; a wrong or unsupported password SHALL return nothing, so Images
  never enables on a file the password does not unlock.
- **Inline images.** Images mode SHALL list inline images (`BI` / `ID` / `EI`)
  from page content streams after a page's image XObjects, and SHALL open one
  at its native size through the same wrapper Ghostscript already renders.
- **3D is out of scope.** The remaining 3D bullet SHALL be removed from #627;
  the radio stays shown disabled, as the spec already says, and the existing
  ROADMAP not-planned entry for 3D stays the tracking statement.
- **Roadmap and parity text.** The #627 row in `ROADMAP.md` and row WF-3 in
  `docs/dev/doc-code-parity-gaps.md` SHALL describe only the shipped work plus
  this change (no 3D).

No new dependency. Ghostscript 9.55+ is already required; native imports run
Ghostscript twice per page or image. The public `ImportTarget` shape is
unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `interop/pdf-eps-import`: the opening and Images-mode requirements gain
  native CMYK/16-bit rendering, Images mode on encrypted PDFs, and inline
  images; the EPS dialog requirement gains Bit Depth. The 3D radio remains
  disabled.

## Impact

- `crates/pictura-codec/src/ghostscript.rs`: native PSD device args and a
  bounded PSD read.
- `crates/pictura-codec/src/pdf/import/`: native render/crop/un-composite,
  encrypted image listing, content-stream inline-image scan; `objects.rs`
  exposes decryption and stream access already present.
- `crates/pictura-app/src/cxxqt_object/pdf_export/import*`: the
  `Imported { Rgba(Rendered), Native(NativeRender) }` bridge result and the
  password plumbing for image thumbnails and opens.
- `PdfImages::read` gains a password and `PdfImage.object` becomes
  `Option<u32>` for inline occurrences; every call site in the app bridges,
  `open_jobs`, and the codec test fixtures is updated.
- `crates/pictura-app/src/cxxqt_object/open_jobs/imp.rs`: native import
  finalization (crop, layer, alpha, source stores), replacing the
  convert-after-render path for the native modes.
- `crates/pictura-app/cpp/import_save_import.cpp`,
  `import_pdf_dialog.cpp`, `rasterize_eps_dialog.cpp`: pass the password to
  Images work and add the EPS Bit Depth control; no 3D change.
- Tests: Ghostscript-backed Rust codec tests and Qt import tests; no new
  fixture dependency.
- Docs: `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (rides
  `TASK-ALLOWS-DOCS` in the docs commit).
