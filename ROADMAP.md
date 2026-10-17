# Kooka Pictura Roadmap

Kooka Pictura is a photo editor for Linux with Photoshop-format (PSD/PSB) files
at its core. This page lists what works today and what is planned.

Legend: `[x]` shipped, `[ ]` planned. A partly finished item stays unchecked
with an `(in progress)` note.

Deeper technical status for PSD/PSB lives in
[`docs/dev/psd-support-roadmap.md`](docs/dev/psd-support-roadmap.md).

## At a glance

| Phase | Focus | Status |
|---|---|---|
| Current | Layered editing, adjustments, filters, color, PSD/PSB | Shipped |
| Phase 1 | Common formats, core tools, finished layer management | In progress |
| Phase 2 | Complete PSD/PSB, Pictura Raw, remaining tools | Planned |
| Later | Automation, scripting, printing | Not planned yet |

## What works today

- Open and save PSD/PSB across Gray, RGB, CMYK, Lab, Indexed, and Bitmap at
  8/16/32-bit, with raw, RLE, and ZIP compression.
- Layers: groups and nesting, raster and vector masks, 27 blend modes, layer
  styles and effects, fill and adjustment layers, smart objects, locks, labels,
  and filtering.
- Adjustments and the full classic filter families, accelerated on the GPU.
- Selections: marquee, ellipse, lasso, magic wand, quick selection, saved
  selections, and selection-masked edits.
- Color: ICC profiles, Assign/Convert Profile, and color settings.
- Metadata, File Info, and Smart Objects.
- Pictura Raw, the built-in raw processor, with its Basic controls.

## Phase 1 — Everyday editing

### Workspace

- [x] Dockable panels, dark theme, remembered layout
- [x] Layers, History, Color, Swatches, Info, and Histogram panels
- [x] Toolbox, options bar, and keyboard shortcuts
- [x] Preferences (General, Interface)
- [ ] Remaining Preferences pages
- [ ] Recent files

### Files

- [x] Open and save PSD/PSB
- [x] Open PNG, JPEG, GIF, BMP, TIFF, and WebP
- [ ] Save As / Export to PNG, JPEG, TIFF, WebP, and BMP
- [ ] Save for Web
- [x] Installers: AppImage, `.deb`/`.rpm`, snap, Flatpak, Windows
      portable/installer, and macOS DMG

### Tools

- [x] Move, rectangular and elliptical marquee, lasso, polygonal lasso, magic
      wand, quick selection, crop, eyedropper, hand, zoom, brush, and pencil
- [ ] Eraser, background eraser, and magic eraser
- [ ] Gradient and paint bucket
- [ ] Clone stamp and pattern stamp
- [ ] Dodge, burn, and sponge
- [ ] Blur, sharpen, and smudge
- [ ] Type tool

### Layers

- [x] Groups, masks, blend modes, layer effects, and fill layers
- [x] Merge layers, merge visible, and flatten image
- [ ] Clipping masks and Merge Clipping Mask compositing
- [ ] Align and distribute
- [ ] Layer comps

### Image

- [x] Image size, canvas size, rotate/flip, and crop
- [x] Image modes and bit depth (Gray, RGB, CMYK, Lab, Indexed, Bitmap)
- [ ] Remaining HDR tone methods

## Phase 2 — Full PSD, Pictura Raw, and the rest of the tools

- [ ] Complete PSD/PSB text: editable type layers and text warp
- [ ] Multichannel and Duotone rendering
- [ ] Clipping and nested-knockout compositing
- [ ] Pictura Raw: the remaining tabs — HSL, Split Toning, Tone Curve, Detail,
      Lens, Effects, and Calibration
- [ ] Open real camera raw files (demosaic and import)
- [ ] Pen and path tools
- [ ] Shape tools
- [ ] Healing tools (spot healing, healing brush, patch, red eye, content-aware
      move)
- [ ] Mixer brush and color replacement
- [ ] Magnetic lasso
- [ ] Rotate view
- [ ] Slice tools
- [ ] Color sampler, ruler, note, and count
- [ ] Perspective crop

Pictura Raw is the built-in raw processor, filling the same role as Adobe Camera
Raw (which ships as an add-on in Photoshop). Today it exposes the 11 Basic
controls; Phase 2 adds the remaining tabs and the ability to open raw files
directly.

## Later

- [ ] Actions and batch processing
- [ ] Scripting and an automation API
- [ ] Printing

## Not planned

Kooka Pictura targets photo editing. The following are out of scope rather than
merely unscheduled:

- 3D tools
- Video and the timeline
- Measurement, counting, and DICOM (Extended-only)
- Print fidelity matching Adobe's engine
- Adobe cloud services (Creative Cloud, Cloud Documents, Behance, Stock)
- Adobe Camera Raw parity (Pictura Raw is an independent approximation)
- Adobe plug-in (`.8bf`) binary compatibility
- ExtendScript / JSX compatibility (a substitute API is planned instead)
- Bundled Adobe assets (brushes, patterns, gradients, profiles, fonts)
- Byte-exact PSD output (preserve-and-round-trip instead)

See
[`docs/00-overview/feasibility-and-non-goals.md`](docs/00-overview/feasibility-and-non-goals.md)
for the rationale and revisit conditions behind each.
