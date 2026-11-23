# Feature parity tracker

Assessed: 2026-11-21
Pictura commit: 4736b57e
CS6 baseline: Adobe Photoshop CS6 13.0 (2012), Standard and Extended
CC baseline: Adobe Photoshop 28.0 (November 2026 release; LTS 27.11.1)

This tracker measures feature coverage against a frozen Photoshop baseline. It does not claim behavioural or pixel parity. The canonical gap register remains [docs/dev/doc-code-parity-gaps.md](docs/dev/doc-code-parity-gaps.md); this tracker aggregates it rather than replacing it.

## Headline

| Baseline | Complete | Partial | Missing | Unknown | Denominator | Coverage | Excluded non-goals |
|---|---|---|---|---|---|---|---|
| CS6 Standard | 208 | 79 | 58 | 4 | 349 | 59.6% | 8 |
| CS6 Standard, in scope | 208 | 79 | 52 | 2 | 341 | 61.0% | 8 |
| CS6 Extended | 209 | 79 | 74 | 4 | 366 | 57.1% | 23 |
| CS6 Extended, in scope | 209 | 79 | 53 | 2 | 343 | 60.9% | 23 |
| CC 28.0 | 210 | 81 | 79 | 4 | 374 | 56.1% | 20 |
| CC 28.0, in scope | 210 | 81 | 61 | 2 | 354 | 59.3% | 20 |

## Baselines

- **CS6 Standard:** Photoshop CS6 13.0 (2012) without the Extended-only 3D, measurement, and DICOM groups. Video and the Timeline remain in Standard; CS6 moved them there from CS5 Extended.
- **CS6 Extended:** Photoshop CS6 13.0 with 3D, measurement, and DICOM.
- **CC 28.0:** Adobe Photoshop 28.0, the November 2026 release; the LTS is 27.11.1. The project owner chose 28.0 as the current CC target.
- **CC derivation:** CC = CS6 inventory minus Adobe-retired features plus Adobe-added capabilities. Legacy 3D is the retirement example; Artboards is a CC-only example.
- **Sources:** the CS6 contract and Adobe CS6 Help (src:1, src:2); Adobe 28.0 release notes and help (src:4, src:5); the 3D discontinuation FAQ (src:3).

## How the numbers are counted

One row is one named capability from the category source list. Dialog controls and individual options are notes on their parent row. Filter, blend-mode, and layer-style families are one row with a `members:<n>/<n>` note. Menu-command and toolbox-present counts are diagnostics and are not added to the denominator.

For each baseline, coverage = 100 × Complete / (Complete + Partial + Missing + Unknown). An `N/A` applicability leaves the denominator. Partial, Missing, and Unknown earn no completion credit. The full-product score includes intentional non-goals; the in-scope score excludes only `Scope: non-goal` rows and shows its own denominator. Percentages are one decimal place, rounded half up, and always show the integer numerator and denominator. A zero denominator displays `—`.

Coverage is not behavioural or pixel parity with Photoshop. Every Complete row names implementation evidence; verification is a separate `Check` value and may be `none`. Approximate or modelled behaviour is named in `Notes`. A disputed baseline presence is `Y` + `Unknown` + a `conflict:<src>` note and is never `N/A`.

## Category summary

| Category | CS6 Standard | CS6 Extended | CC 28.0 |
|---|---|---|---|
| Files & interchange | 27/40 (67.5%) | 27/42 (64.3%) | 28/46 (60.9%) |
| Layers & compositing | 31/38 (81.6%) | 31/39 (79.5%) | 31/39 (79.5%) |
| Tools & painting | 41/70 (58.6%) | 42/81 (51.9%) | 42/71 (59.2%) |
| Selections & masking | 18/22 (81.8%) | 18/22 (81.8%) | 18/22 (81.8%) |
| Adjustments & filters | 30/55 (54.5%) | 30/55 (54.5%) | 30/64 (46.9%) |
| Type & vector | 17/17 (100.0%) | 17/17 (100.0%) | 17/19 (89.5%) |
| Color & bit depth | 14/29 (48.3%) | 14/29 (48.3%) | 14/29 (48.3%) |
| Automation & extensibility | 3/18 (16.7%) | 3/18 (16.7%) | 3/22 (13.6%) |
| Shell, panels & preferences | 27/60 (45.0%) | 27/63 (42.9%) | 27/62 (43.5%) |

## Feature rows

### Files & interchange

Source list: CS6 and CC supported-format lists plus named File-menu, metadata, print, and web-export capabilities.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-FMT-01 | PSD/PSB read and write | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src/read.rs; crates/pictura-codec/src/write.rs | manual:codec oracle tests | Preserves unknown blocks; not byte-exact | — |
| P-FMT-02 | PNG read and write | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-03 | JPEG read and write | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-04 | TIFF read and write | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-05 | WebP read and write | N/A | N/A | Y | Complete | in-scope | crates/pictura-codec/src/webp.rs | none | post-CS6:src:1; src:6 | — |
| P-FMT-06 | GIF read and write through Indexed | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-07 | BMP read and write | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-08 | PDF import through Ghostscript | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-09 | EPS import through Ghostscript | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-10 | Photoshop PDF export | Y | Y | Y | Partial | in-scope | crates/pictura-codec/src | none | JPEG 2000, spot colours, proof setup ship; Preserve Photoshop Editing is partial | #1513 |
| P-FMT-11 | EPS export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-12 | SVG export | N/A | N/A | Y | Partial | in-scope | crates/pictura-codec/src/save/mod.rs | none | post-CS6:src:1; src:6; write only | #1511 |
| P-FMT-13 | TGA export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-14 | Cineon export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-15 | DCS export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | Process-ink-only ceiling | — |
| P-FMT-16 | Photoshop Raw export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | 8-bit ceiling | — |
| P-FMT-17 | PICT read and write | Y | Y | Y | Missing | in-scope | — | none | Legacy Adobe format | #1503 |
| P-FMT-18 | IFF read and write | Y | Y | Y | Missing | in-scope | — | none | Legacy Adobe format | #1504 |
| P-FMT-19 | PCX read and write | Y | Y | Y | Missing | in-scope | — | none | Legacy Adobe format | #1505 |
| P-FMT-20 | SGI read and write | Y | Y | Y | Missing | in-scope | — | none | Legacy Adobe format | #1506 |
| P-FMT-21 | OpenEXR read and write | Y | Y | Y | Missing | in-scope | — | none | No EXR codec | #1507 |
| P-FMT-22 | JPEG 2000 file read and write | Y | Y | Y | Missing | in-scope | — | none | JPEG 2000 exists only inside the PDF writer | #1508 |
| P-FMT-23 | HEIC/HEIF read and write | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:1; src:6 | #1509 |
| P-FMT-24 | AVIF read and write | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:1; src:6 | #1510 |
| P-FMT-25 | DICOM read and analysis | N/A | Y | Y | Missing | non-goal | — | none | edition:Extended-only; DICOM is an intentional non-goal | non-goal:feasibility:74 |
| P-FMT-26 | Camera raw open and demosaic | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src/dng/develop.rs; crates/pictura-app/src/cxxqt_object/open_jobs/raw_open.rs | manual:tst_raw_open covers the raw open route | DNG/CR2/NEF/ARW, uncompressed ORF and RW2 File > Open route; approximate parity; ORF colour comes from the embedded matrix (#1471) and compressed ORF remains (#1535); RW2 decodes in every observed coding, the compressed ones ported from rawspeed (#1472, #1541), and 10 Panasonic bodies open with colour, nine from camera-JPEG fits (#1542; the rest #1558); per-camera matrices for 14 Canon, 11 Nikon (plus 3 proxies) and 11 Sony bodies ship (#1465, #1469, #1470); smart-object open remains (#1466) | — |
| P-FMT-27 | DNG output | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src/dng/write/ | none | Mosaic and linear DNG output with `crs:` settings; JPEG, TIFF and PSD from Save Options remain (#633) | — |
| P-FMT-28 | Print | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Print colour management, proofing, marks, and persisted defaults remain | #634 |
| P-FMT-29 | EXIF, IPTC, and XMP metadata | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-30 | File Info tabs | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-31 | Save for Web | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Slice names, per-slice optimisation, Web Snap, and saved output settings remain | #1514 |
| P-FMT-32 | Export As | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-FMT-33 | Zoomify export | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-34 | Paths to Illustrator | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-FMT-35 | Data Sets As Files | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-FMT-36 | Render Video | Y | Y | Y | Missing | non-goal | — | none | Video is an intentional non-goal | non-goal:feasibility:73 |
| P-FMT-37 | Place and Open As Smart Object | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-FMT-38 | Load Files into Stack | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-FMT-39 | Open As | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/frame_import.cpp | none | Format-specific Open As through Ghostscript and the raster decoders | — |
| P-FMT-40 | Background Save and crash journal | Y | Y | Y | Complete | in-scope | crates/pictura-app/src | none | — | — |
| P-FMT-41 | Maximize PSD/PSB Compatibility preference | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-42 | Document format version policy | Y | Y | Y | Complete | in-scope | crates/pictura-codec/src | none | — | — |
| P-FMT-43 | 3D file import (U3D, 3DS, OBJ, KMZ, Collada) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-FMT-44 | Video frame import | Y | Y | Y | Missing | non-goal | — | none | Video is an intentional non-goal; src:1; src:7 | non-goal:feasibility:73 |
| P-FMT-45 | Content Credentials | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4 | #1512 |
| P-FMT-46 | Per-camera colour matrices for CR2 and vendor raw | Y | Y | Y | Partial | in-scope | crates/pictura-codec/src/dng/camera_matrices.rs | none | 14 Canon CR2, 11 Nikon NEF and 11 Sony ARW bodies (plus 3 Sony same-sensor aliases) fit from spectral sensitivities, 3 Nikon bodies by same-sensor proxy; other cameras are refused; the EOS 600D CFA phase (#1476), unseeded Nikon (#1498) and Sony (#1525) bodies and the other vendor containers (#399) remain | follow-up: #1476, #1498, #1525 |
| P-FMT-47 | Open a raw file as a Smart Object | Y | Y | Y | Missing | in-scope | — | none | Shift+Open Image raw open is not implemented | #1466 |

### Layers & compositing

Source list: named Layer-menu and Layers-panel capabilities.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-LAY-01 | Layer stack, groups, and nesting | Y | Y | Y | Complete | in-scope | crates/pictura-core/src/layer_stack.rs | none | — | — |
| P-LAY-02 | Background layer semantics | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-03 | Layer masks | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-04 | Vector masks (rendering) | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-05 | Vector-mask path authoring | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/menu_layer_ops.cpp | none | Current Path creates the vector mask from the selected path; the Paths panel row and mask thumbnail ship | #261 |
| P-LAY-06 | Clipping masks | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-07 | Blend modes and Pass Through | Y | Y | Y | Complete | in-scope | crates/pictura-core/src | none | members:27/27 | — |
| P-LAY-08 | Layer styles | Y | Y | Y | Partial | in-scope | crates/pictura-render/src | none | members:10/10; Bevel Contour and gradient editor ship, Bevel Texture remains | #606 |
| P-LAY-09 | Advanced Blending engine | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-10 | Advanced Blending dialog controls | Y | Y | Y | Missing | in-scope | — | none | Engine ships; dialog controls are absent | #695 |
| P-LAY-11 | Adjustment layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-12 | Fill layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-13 | Shape layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-14 | Type layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-15 | Embedded smart objects | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-16 | Smart filters | Y | Y | Y | Partial | in-scope | crates/pictura-render/src | none | Several smart-filter variants and masks remain | LAY-15 |
| P-LAY-17 | Layer comps | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Panel, capture, apply, update, and PSD persistence ship | #265 |
| P-LAY-18 | Layer locks | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-LAY-19 | Colour labels | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-LAY-20 | Layer filtering and search | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-LAY-21 | Align and distribute | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-22 | Auto-Align Layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | Behavioural parity only | #582 |
| P-LAY-23 | Auto-Blend Layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #583 |
| P-LAY-24 | Merge, flatten, and Merge Clipping Mask | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-25 | Merge Down appearance | Y | Y | Y | Unknown | in-scope | crates/pictura-render/src | none | Register records the result as UNRESOLVED when the target has non-default blend or opacity | #813 |
| P-LAY-26 | Ctrl+J duplicate-group semantics | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #590 |
| P-LAY-27 | Smart-object stack modes | Y | Y | Y | Complete | in-scope | crates/pictura-core/src | none | — | #591 |
| P-LAY-28 | Pattern fill layers | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #262 |
| P-LAY-29 | Linked smart-object relink and convert | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels/layers_row_actions.cpp | none | Relink to File repairs supported external links; broader relink and convert-to-linked remain | #113 |
| P-LAY-30 | Blend-If editor | Y | Y | Y | Missing | in-scope | — | none | No Blend-If UI | #264 |
| P-LAY-31 | Export Layers To Files | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Shared Save As options and Targa remain | LAY-9 |
| P-LAY-32 | Layer-style Global Light, Create Layers, Rasterize | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #580 |
| P-LAY-33 | Bevel contour, style gradient editor, channel include | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #581 |
| P-LAY-34 | Layer thumbnail size | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #586 |
| P-LAY-35 | Shape layer naming | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #588 |
| P-LAY-36 | Knockout compositing | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-37 | Layer matting | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-38 | Layer source cache | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-LAY-39 | Artboards | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Artboards are a post-CS6 feature | non-goal:ROADMAP.md:704 |
| P-LAY-40 | 3D layers and the 3D workspace | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |

### Tools & painting

Source list: CS6 toolbox slots (71 catalogue slots plus the two missing single row/column tools) plus named brush-engine, brush-dynamics, and preset capabilities.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-TOOL-01 | Move | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/tool_move.cpp | none | — | — |
| P-TOOL-02 | Rectangular Marquee | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-03 | Elliptical Marquee | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-04 | Lasso | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-05 | Polygonal Lasso | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-06 | Magnetic Lasso | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-07 | Magic Wand | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-08 | Quick Selection | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-09 | Crop | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-10 | Perspective Crop | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-11 | Slice | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Slice style and modifiers, Divide/Combine/Duplicate, and the Slice Options dialog remain | TOOL-19 |
| P-TOOL-12 | Slice Select | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Slice style and modifiers, Divide/Combine/Duplicate, and the Slice Options dialog remain | TOOL-19 |
| P-TOOL-13 | Eyedropper | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Modifiers and Shift-click sampler remain | TOOL-12 |
| P-TOOL-14 | Color Sampler | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Modifiers and Shift-click sampler remain | TOOL-12 |
| P-TOOL-15 | Ruler | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Protractor and Straighten from the ruler remain | TOOL-11 |
| P-TOOL-16 | Note | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-17 | Count (Extended) | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-18 | Spot Healing Brush | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Healing Mode, Source, Sample All Layers, and depth/mode handling remain | TOOL-14 |
| P-TOOL-19 | Healing Brush | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Healing Mode, Source, Sample All Layers, and depth/mode handling remain | TOOL-14 |
| P-TOOL-20 | Patch | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-21 | Content-Aware Move | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-22 | Red Eye | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-23 | Brush | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-24 | Pencil | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-25 | Color Replacement | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/tool_colorreplacement.cpp; crates/pictura-app/cpp/tool_catalog.cpp | none | The full CS6 mode list, 32-bit subset, and Bitmap/Indexed/Multichannel availability gates ship | #547 |
| P-TOOL-26 | Mixer Brush | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Load Solid Colors Only and keyboard shortcuts remain | TOOL-16 |
| P-TOOL-27 | Clone Stamp | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-28 | Pattern Stamp | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Cross-document colour-mode rule and Impressionist remain | TOOL-28 |
| P-TOOL-29 | History Brush | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Merged/composite source and Edit > Fill > History remain | TOOL-15 |
| P-TOOL-30 | Art History Brush | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Merged/composite source and Edit > Fill > History remain | TOOL-15 |
| P-TOOL-31 | Eraser | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-32 | Background Eraser | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Background conversion, edge halo, and pen pressure remain | TOOL-25 |
| P-TOOL-33 | Magic Eraser | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-34 | Gradient | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Availability gates and Gradient Fill remain | TOOL-26 |
| P-TOOL-35 | Paint Bucket | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Availability gates remain | TOOL-26 |
| P-TOOL-36 | Blur | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | 16/32-bit operation remains | TOOL-27 |
| P-TOOL-37 | Sharpen | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | 16/32-bit operation remains | TOOL-27 |
| P-TOOL-38 | Smudge | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | 16/32-bit operation remains | TOOL-27 |
| P-TOOL-39 | Dodge | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-40 | Burn | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-41 | Sponge | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-42 | Pen | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-43 | Freeform Pen | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-44 | Add Anchor Point | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-45 | Delete Anchor Point | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-46 | Convert Point | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-47 | Horizontal Type | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-48 | Vertical Type | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-49 | Horizontal Type Mask | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-50 | Vertical Type Mask | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-51 | Path Selection | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Path operations, align/arrange, constrained dragging, and multi-select remain | TOOL-18 |
| P-TOOL-52 | Direct Selection | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Path operations, align/arrange, constrained dragging, and multi-select remain | TOOL-18 |
| P-TOOL-53 | Rectangle | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Gradient/pattern fill and stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, and proportional geometry | TOOL-22 |
| P-TOOL-54 | Rounded Rectangle | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Gradient/pattern fill and stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, and proportional geometry | TOOL-22 |
| P-TOOL-55 | Ellipse | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Gradient/pattern fill and stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, and proportional geometry | TOOL-22 |
| P-TOOL-56 | Polygon | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Gradient/pattern fill and stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, and proportional geometry | TOOL-22 |
| P-TOOL-57 | Line | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Gradient/pattern fill and stroke, dashed stroke, path ops, fill-pixels options, snap-to-pixels, and proportional geometry | TOOL-22 |
| P-TOOL-58 | Custom Shape | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | .CSH libraries, Define Custom Shape, and geometry modes remain | TOOL-21 |
| P-TOOL-59 | Object Rotate (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-60 | Object Roll (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-61 | Object Pan (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-62 | Object Slide (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-63 | Object Scale (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-64 | Camera Rotate (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-65 | Camera Roll (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-66 | Camera Pan (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-67 | Camera Walk (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-68 | Camera Zoom (Extended) | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-TOOL-69 | Hand | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Scrubby zoom, animated zoom, Bird's Eye View, and Flick Panning remain | TOOL-9 |
| P-TOOL-70 | Rotate View | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TOOL-71 | Zoom | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Scrubby zoom, animated zoom, Bird's Eye View, and Flick Panning remain | TOOL-10 |
| P-TOOL-72 | Single Row Marquee | Y | Y | Y | Missing | in-scope | — | none | Tool absent from the catalogue | TOOL-1 |
| P-TOOL-73 | Single Column Marquee | Y | Y | Y | Missing | in-scope | — | none | Tool absent from the catalogue | TOOL-1 |
| P-TOOL-74 | Brush engine | Y | Y | Y | Complete | in-scope | crates/pictura-paint/src | none | — | — |
| P-TOOL-75 | Brush dynamics | Y | Y | Y | Partial | in-scope | crates/pictura-paint/src | none | Texture and Dual Brush remain; the control model, Shape Dynamics, Scattering, Colour Dynamics, Transfer, Pose, and padlocks ship | SYS-4 |
| P-TOOL-76 | Sampled, erodible, and airbrush tips | Y | Y | Y | Partial | in-scope | crates/pictura-paint/src | none | Round/elliptical and bristle tips ship; sampled, erodible, and airbrush tips plus Define Brush Preset remain | SYS-5 |
| P-TOOL-77 | Bristle brushes | Y | Y | Y | Partial | in-scope | crates/pictura-paint/src | none | Engine and presets ship; OpenGL preview, .abr bristle keys, and tilt-driven projection remain | #1520 |
| P-TOOL-78 | Mixer brush engine | Y | Y | Y | Partial | in-scope | crates/pictura-paint/src | none | Load Solid Colors Only and shortcuts remain | #540 |
| P-TOOL-79 | Brush presets and .abr libraries | Y | Y | Y | Missing | in-scope | — | none | No Brush Presets panel, .abr libraries, or ABR v10 reading | CLR-11 |
| P-TOOL-80 | Airbrush held dabs | Y | Y | Y | Complete | in-scope | crates/pictura-paint/src | none | 20 dabs/s approximation | #1120 |
| P-TOOL-81 | Measurement scale, presets, and exporters | N/A | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | edition:Extended-only; shipped #635 | — |

### Selections & masking

Source list: named Select-menu capabilities and the selection-mask engine.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-SEL-01 | Select All | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/menu_registrar.cpp | none | — | — |
| P-SEL-02 | Deselect | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-03 | Reselect | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-04 | Inverse | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-05 | All Layers | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-06 | Deselect Layers | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-07 | Similar Layers | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SEL-08 | Colour Range | Y | Y | Y | Partial | in-scope | crates/pictura-select/src | none | Skin tones, localised clusters, out-of-gamut, previews, and save/load ship; Detect Faces, .axt import, and per-session persistence remain | SEL-5 |
| P-SEL-09 | Refine Edge | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | Interactive Refine Radius and Erase brushes ship; the CC workspace, Onion Skin, mask Brush, and Refine Hair remain | — |
| P-SEL-10 | Modify: Border, Smooth, Expand, Contract, Feather | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | — | — |
| P-SEL-11 | Grow and Similar | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | — | — |
| P-SEL-12 | Transform Selection | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #526 |
| P-SEL-13 | Save Selection | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Selected Areas polarity ships; 16/32-bit channels remain (#1331) | #1331 |
| P-SEL-14 | Load Selection | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Selected Areas polarity ships; 16/32-bit channels remain (#1331) | #1331 |
| P-SEL-15 | Quick Mask | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | — | #172 |
| P-SEL-16 | Apply Image | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #609 |
| P-SEL-17 | Calculations | Y | Y | Y | Complete | in-scope | crates/pictura-render/src/calculations.rs | none | Add, Subtract, Multiply, Darken, and Lighten with channel and mask sources | — |
| P-SEL-18 | Channel Options (Masked and Selected Areas) | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/frame_selection_dialogs.cpp | none | Masked and Selected Areas polarity, persisted as DisplayInfo 1077 | — |
| P-SEL-19 | Selection channels and alpha channels | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Combine, layer-mask target, channel names, and polarity ship; 16/32-bit depth remains | #1331 |
| P-SEL-20 | Anti-aliased selection rasterisation | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | Control-server selection ops remain binary | #614 |
| P-SEL-21 | Selection combine operations | Y | Y | Y | Complete | in-scope | crates/pictura-select/src | none | — | #615 |
| P-SEL-22 | Cross-image channel duplication and outline drag | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #1332 |

### Adjustments & filters

Source list: the Image > Adjustments list (23 rows; 25 commands when Auto Tone/Contrast/Color are counted separately) plus the Filter menu submenus and stand-alone filters.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-IMG-01 | Brightness/Contrast | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src/tonal.rs | none | Auto ships | #578 |
| P-IMG-02 | Levels | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Eyedroppers and Alt-drag clipping preview remain | #560 |
| P-IMG-03 | Curves | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | On-image, pencil, smooth, sliders, display options, CMYK/Lab graphs remain | #561 |
| P-IMG-04 | Exposure | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-05 | Vibrance | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Auto and presets remain | #564 |
| P-IMG-06 | Hue/Saturation | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Colorize, range-band editing, eyedroppers, and on-image tool remain | #573 |
| P-IMG-07 | Colour Balance | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | #572 |
| P-IMG-08 | Black & White | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-09 | Photo Filter | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-10 | Channel Mixer | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | #571 |
| P-IMG-11 | Colour Lookup | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-12 | Invert | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-13 | Posterize | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-14 | Threshold | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Histogram under the slider remains | #563 |
| P-IMG-15 | Gradient Map | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Dither and CS6 print-toning presets remain | #574 |
| P-IMG-16 | Selective Colour | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Composite-channel-only gate and preset whitelist remain | #577 |
| P-IMG-17 | Shadows/Highlights | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Local-surround operator and eight controls remain | #562 |
| P-IMG-18 | HDR Toning | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | Local Adaptation Toning Curve and Corner remain | #557 |
| P-IMG-19 | Desaturate | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-20 | Match Colour | Y | Y | Y | Partial | in-scope | crates/pictura-adjust/src | none | On-canvas eyedropper and live Info feedback remain | #575 |
| P-IMG-21 | Replace Colour | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-22 | Equalize | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | — |
| P-IMG-23 | Auto Tone, Auto Contrast, and Auto Colour | Y | Y | Y | Complete | in-scope | crates/pictura-adjust/src | none | — | #578 |
| P-FLT-01 | Blur family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:8/8 | #594 |
| P-FLT-02 | Blur Gallery | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:3/3 | — |
| P-FLT-03 | Brush Strokes family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:8/8 | — |
| P-FLT-04 | Distort family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:12/12 | — |
| P-FLT-05 | Noise family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:5/5 | #593 |
| P-FLT-06 | Pixelate family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:7/7 | — |
| P-FLT-07 | Render family (excluding Lighting Effects) | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:4/4 | — |
| P-FLT-08 | Sharpen family | Y | Y | Y | Partial | in-scope | crates/pictura-filters/src | none | members:5/5; Smart Sharpen Basic/Advanced split and 32-bpc disable remain | FILT-4 |
| P-FLT-09 | Sketch family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:14/14 | — |
| P-FLT-10 | Stylize family | Y | Y | Y | Partial | in-scope | crates/pictura-filters/src | none | members:9/9; eight Stylize filters are absent from the Filter Gallery category | FILT-5 |
| P-FLT-11 | Texture family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:6/6 | — |
| P-FLT-12 | Video family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:2/2 | #344 |
| P-FLT-13 | Other family | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | members:5/5 | #364 |
| P-FLT-14 | Artistic family | Y | Y | Y | Partial | in-scope | crates/pictura-filters/src | none | members:15/15; Colored Pencil document-background paper remains | FILT-6 |
| P-FLT-15 | Filter Gallery | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Canvas and Shear previews over an existing Smart Filter chain remain | FILT-11 |
| P-FLT-16 | Liquify | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | — | — |
| P-FLT-17 | Oil Paint | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | — | — |
| P-FLT-18 | Adaptive Wide Angle | Y | Y | Y | Partial | in-scope | crates/pictura-filters/src | none | Auto, lens profiles, Panorama, Smart Filter re-edit, and rotation handles remain | #891 |
| P-FLT-19 | Lens Correction | Y | Y | Y | Partial | in-scope | crates/pictura-filters/src | none | Save/Load Settings, Set Lens Default, and Alt-drag fringe isolation remain | #933 |
| P-FLT-20 | Vanishing Point | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | — | #349 |
| P-FLT-21 | Camera Raw Filter | N/A | N/A | Y | Partial | in-scope | crates/pictura-app/cpp/pictura_raw_dialog.cpp | none | post-CS6:src:1; src:4; Pictura Raw is an independent approximation; Presets is a stub | #633 |
| P-FLT-22 | Lighting Effects | Y | Y | Y | Unknown | in-scope | crates/pictura-filters/src/render/lighting.rs | none | conflict:11; src:3; the general filter reference predates CC 28.0 and the former 3D FAQ now redirects; availability in CC 28.0 is unverified | follow-up:verify the menu and filter in Photoshop CC 28.0 |
| P-FLT-23 | Extract and Pattern Maker | Y | Y | Y | Missing | non-goal | — | none | Optional CS6 plug-ins; not a parity target | non-goal:ROADMAP.md:705 |
| P-FLT-24 | Digimarc watermark | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | Approximate spread-spectrum carrier; not Digimarc-compatible | #350 |
| P-FLT-25 | Fade | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #491 |
| P-FLT-26 | Smart Filter chain | Y | Y | Y | Partial | in-scope | crates/pictura-render/src | none | Shadow/Highlight and Variations, per-filter blend/opacity, white masks, and some Smart Filter variants remain | LAY-15 |
| P-FLT-27 | Per-Smart-Filter blend and opacity | Y | Y | Y | Missing | in-scope | — | none | No per-filter blend or opacity | #998 |
| P-FLT-28 | Shadow/Highlight and Variations as Smart Filters | Y | Y | Y | Missing | in-scope | — | none | Not available as Smart Filters | #996 |
| P-FLT-29 | White filter mask and Add Filter Mask | Y | Y | Y | Missing | in-scope | — | none | No white mask or Add Filter Mask | #1000 |
| P-FLT-30 | Lens Blur, Displace, and Adaptive Wide Angle as Smart Filters | Y | Y | Y | Missing | in-scope | — | none | Not available as Smart Filters | #1001 |
| P-FLT-31 | Photoshop validation of authored filter masks | Y | Y | Y | Missing | in-scope | — | none | No Photoshop round-trip validation | #999 |
| P-FLT-32 | Smart Filter chain off the UI thread | Y | Y | Y | Missing | in-scope | — | none | Chain renders on the UI thread | #1002 |
| P-FLT-33 | Fade disabled on a Smart Object | Y | Y | Y | Complete | in-scope | crates/pictura-filters/src | none | — | #1004 |
| P-FLT-34 | Generative Fill and Generative Expand | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud/AI service; local-only editor | non-goal:feasibility:76 |
| P-FLT-35 | Generate Image and Prompt to Edit | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud/AI service | non-goal:feasibility:76 |
| P-FLT-36 | Neural Filters and Sky Replacement | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud/AI service | non-goal:feasibility:76 |
| P-FLT-37 | Generative Upscale, Relight, and Generative Restyle | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud/AI service | non-goal:feasibility:76 |
| P-FLT-38 | Quick Compare | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4; no local equivalent | #1515 |
| P-FLT-39 | Enhance Edge and AI Assisted Editor | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud/AI service | non-goal:feasibility:76 |
| P-FLT-40 | Clarity and Dehaze adjustment layer | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4 | #1516 |
| P-FLT-41 | Light adjustment layer | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4 | #1517 |

### Type & vector

Source list: Type menu and Character/Paragraph capabilities plus type on path, warp text, type styles, and font handling.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-TYP-01 | Point type | Y | Y | Y | Complete | in-scope | crates/pictura-render/src/type_layer.rs | none | — | — |
| P-TYP-02 | Area type | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #683 |
| P-TYP-03 | Type on path | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #683 |
| P-TYP-04 | Warp text | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #684 |
| P-TYP-05 | Character panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #517 |
| P-TYP-06 | Paragraph panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #518 |
| P-TYP-07 | Character Styles panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #519 |
| P-TYP-08 | Paragraph Styles panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-TYP-09 | Glyphs panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-TYP-10 | Missing fonts and missing glyph protection | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | #685 |
| P-TYP-11 | Type colour picker | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TYP-12 | RTL paragraph direction and justification | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #518 |
| P-TYP-13 | Faux bold, italic, underline, fractional widths, and tsume | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #517 |
| P-TYP-14 | Variable fonts | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4 | #1518 |
| P-TYP-15 | OpenType feature UI | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels/character_panel.cpp; crates/pictura-app/cpp/menu_type.cpp; crates/pictura-render/src/type_style.rs | none | The Character panel exposes eight shared toggles with per-font gating; the Type menu adds Oldstyle and Ornaments and gates on the editable target. The engine shapes all ten. | — |
| P-TYP-16 | Dynamic text on shapes and paths | N/A | N/A | Y | Missing | in-scope | — | none | post-CS6:src:4 | #1519 |
| P-TYP-17 | Type masks | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-TYP-18 | Font family picker | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-TYP-19 | Type rasterisation | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |

### Color & bit depth

Source list: Image > Mode, Color Settings, picker, and swatch capabilities.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-CLR-01 | RGB colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src/lib.rs | none | — | — |
| P-CLR-02 | Grayscale colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-03 | CMYK colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-04 | Lab colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-05 | Indexed colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-06 | Bitmap colour mode | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-07 | Duotone colour mode | Y | Y | Y | Missing | in-scope | — | none | No duotone conversion or Options dialog | #553 |
| P-CLR-08 | Multichannel colour mode | Y | Y | Y | Missing | in-scope | — | none | No multichannel conversion | #554 |
| P-CLR-09 | 8-bit per channel | Y | Y | Y | Complete | in-scope | crates/pictura-core/src | none | — | — |
| P-CLR-10 | 16-bit per channel | Y | Y | Y | Complete | in-scope | crates/pictura-core/src | none | — | — |
| P-CLR-11 | 32-bit per channel | Y | Y | Y | Partial | in-scope | crates/pictura-core/src | none | 32-bit preview options and the Info readout remain | #570 |
| P-CLR-12 | Image mode conversion | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-CLR-13 | Colour Table, Indexed transparency, and Forced palette | Y | Y | Y | Missing | in-scope | — | none | No colour table UI or forced palette | #555 |
| P-CLR-14 | Bitmap Halftone Screen and Custom Pattern | Y | Y | Y | Missing | in-scope | — | none | No halftone screen or custom pattern | #556 |
| P-CLR-15 | Assign Profile | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-16 | Convert to Profile | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | #568 |
| P-CLR-17 | Colour Settings | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | .csf save/load, Gray/Spot working spaces, and installed-profile discovery remain | #569 |
| P-CLR-18 | Colour picker and gamut alerts | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #600 |
| P-CLR-19 | Colour Libraries (PANTONE) | Y | Y | Y | Missing | in-scope | — | none | Disabled in the picker | #600 |
| P-CLR-20 | HUD colour picker | Y | Y | Y | Missing | in-scope | — | none | No HUD picker | #601 |
| P-CLR-21 | HDR colour picker | Y | Y | Y | Missing | in-scope | — | none | Picker is 8-bit only | #602 |
| P-CLR-22 | 16/32-bit picker fields | Y | Y | Y | Missing | in-scope | — | none | No depth-aware fields | #604 |
| P-CLR-23 | ACO and ASE swatch libraries | Y | Y | Y | Partial | in-scope | crates/pictura-codec/src | none | Preset Manager imports and exports; the Swatches panel libraries, rename, and display modes remain | #605 |
| P-CLR-24 | Gradient Editor | Y | Y | Y | Complete | in-scope | crates/pictura-paint/src | none | — | #598 |
| P-CLR-25 | Define Pattern and .pat library | Y | Y | Y | Missing | in-scope | — | none | No Define Pattern or pattern library | #599 |
| P-CLR-26 | Pattern Overlay Snap To Origin and Texture sub-options | Y | Y | Y | Partial | in-scope | crates/pictura-render/src | none | Snap To Origin and Texture remain | #606 |
| P-CLR-27 | ICC colour management | Y | Y | Y | Complete | in-scope | crates/pictura-color/src | none | — | — |
| P-CLR-28 | 32-bit preview options | Y | Y | Y | Missing | in-scope | — | none | View menu stub | #570 |
| P-CLR-29 | Proof Setup and Gamut Warning | Y | Y | Y | Missing | in-scope | — | none | View menu stubs | #503 |

### Automation & extensibility

Source list: File > Automate, Scripts, Actions, and plug-in capabilities.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-AUT-01 | Actions panel and engine | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels/actions_panel.cpp | none | — | #301 |
| P-AUT-02 | Action recording and playback | Y | Y | Y | Complete | in-scope | crates/pictura-script/src | none | App-native events; not Adobe-interoperable | #301 |
| P-AUT-03 | File > Automate > Batch | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/batch_runner.cpp | none | Batch Import and Bridge sources remain | #1406 |
| P-AUT-04 | Image Processor | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-05 | Conditional Mode Change | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-06 | PDF Presentation | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-07 | Contact Sheet II | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-08 | Picture Package | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-09 | Photomerge | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-10 | Web Photo Gallery | Y | Y | Y | Missing | in-scope | — | none | Not implemented | #619 |
| P-AUT-11 | Create Droplet | Y | Y | Y | Missing | in-scope | — | none | No droplet format or launcher | #617 |
| P-AUT-12 | Scripting host and DOM | Y | Y | Y | Missing | in-scope | — | none | No QuickJS host or DOM | #620 |
| P-AUT-13 | Script console, sandbox, and security prompt | Y | Y | Y | Missing | in-scope | — | none | No script console | #621 |
| P-AUT-14 | ExtendScript and JSX compatibility | Y | Y | Y | Missing | non-goal | — | none | Adobe-specific object model | non-goal:feasibility:79 |
| P-AUT-15 | Variables and data-driven graphics | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #622 |
| P-AUT-16 | Rust C-ABI plug-in loader | Y | Y | Y | Missing | in-scope | — | none | No plug-in ABI | #623 |
| P-AUT-17 | Adobe .8bf plug-in compatibility | Y | Y | Y | Missing | non-goal | — | none | Adobe host ABI is proprietary | non-goal:feasibility:78 |
| P-AUT-18 | Mini Bridge and the Files panel | Y | Y | Y | Unknown | non-goal | — | none | Cloud/Bridge service; CC presence unverified; src:1; src:7 | non-goal:feasibility:76 |
| P-AUT-19 | Libraries | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud service | non-goal:feasibility:76 |
| P-AUT-20 | Cloud Documents | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud service | non-goal:feasibility:76 |
| P-AUT-21 | Share for Review | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud service | non-goal:feasibility:76 |
| P-AUT-22 | UXP plug-ins | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe ecosystem | non-goal:feasibility:76 |

### Shell, panels & preferences

Source list: Window-menu panels, Preferences panes, workspaces, shortcuts, and accessibility.

| ID | Feature | S | E | CC | Status | Scope | Evidence | Check | Notes | Tracking |
|---|---|---|---|---|---|---|---|---|---|---|
| P-PAN-01 | Layers panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels/layers_panel.cpp | none | — | — |
| P-PAN-02 | History panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | History Options, Clear History, purge, and New Document from state remain | #514 |
| P-PAN-03 | Channels panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Spot channels, Split/Merge, and Panel Options remain | #513 |
| P-PAN-04 | Paths panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Make Work Path, clipping-path designation, and thumbnail size remain | #521 |
| P-PAN-05 | Colour panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-PAN-06 | Swatches panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Swatch libraries, rename, and display modes remain | #605 |
| P-PAN-07 | Info panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Sampler persistence, sample size, marquee/transform readouts, before/after, and gamut warning remain | #516 |
| P-PAN-08 | Histogram panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | View modes, statistics, source menu, cache/refresh, and 16/32-bit bins remain | #515 |
| P-PAN-09 | Navigator panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Proxy colour, Ctrl-drag positioning, zoom buttons, and typed zoom remain | #520 |
| P-PAN-10 | Notes panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Author/colour options and annotation round-trip remain | #522 |
| P-PAN-11 | Properties panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-PAN-12 | Adjustments panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Mode/depth gating, fill-layer presets (#1521), and icon artwork remain | #1521 |
| P-PAN-13 | Brush panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-PAN-14 | Clone Source panel | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/panels | none | Overlay controls remain | #537 |
| P-PAN-15 | Character panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #517 |
| P-PAN-16 | Paragraph panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #518 |
| P-PAN-17 | Glyphs panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-PAN-18 | Paragraph Styles panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | — |
| P-PAN-19 | Character Styles panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #519 |
| P-PAN-20 | Actions panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #301 |
| P-PAN-21 | Layer Comps panel | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/panels | none | — | #265 |
| P-PAN-22 | Tool Presets panel | Y | Y | Y | Missing | in-scope | — | none | No Tool Presets panel or .tpl libraries | #511 |
| P-PAN-23 | Styles panel | Y | Y | Y | Missing | in-scope | — | none | No Styles panel or .asl libraries | #512 |
| P-PAN-24 | Measurement Log panel | N/A | Y | Y | Missing | in-scope | — | none | edition:Extended-only; Record Measurements and export workflow are absent | #510 |
| P-PAN-25 | Timeline panel | Y | Y | Y | Missing | non-goal | — | none | Video/Timeline is an intentional non-goal; src:1; src:7 | non-goal:feasibility:73 |
| P-PAN-26 | 3D panel | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-PAN-27 | Mini Bridge panel | Y | Y | Y | Unknown | non-goal | — | none | Cloud/Bridge service; CC presence unverified; src:1; src:7 | non-goal:feasibility:76 |
| P-PAN-28 | Libraries panel | N/A | N/A | Y | Missing | non-goal | — | none | post-CS6:src:4; Adobe cloud service | non-goal:feasibility:76 |
| P-PREF-01 | General preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Sparse pane | #506 |
| P-PREF-02 | Interface preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Sparse pane | #506 |
| P-PREF-03 | File Handling preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Sparse pane | #506 |
| P-PREF-04 | Performance preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Memory, history, cache, tile, scratch, and GPU controls remain | #636 |
| P-PREF-05 | Units & Rulers preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Sparse pane | #506 |
| P-PREF-06 | Guides, Grid & Slices preferences | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Sparse pane | #506 |
| P-PREF-07 | Cursors preferences | Y | Y | Y | Missing | in-scope | — | none | Pane absent | UI-9 |
| P-PREF-08 | Transparency & Gamut preferences | Y | Y | Y | Missing | in-scope | — | none | Pane absent | UI-9 |
| P-PREF-09 | Plug-ins preferences | Y | Y | Y | Missing | in-scope | — | none | Pane absent | UI-9 |
| P-PREF-10 | Type preferences | Y | Y | Y | Missing | in-scope | — | none | Pane absent | UI-9 |
| P-PREF-11 | 3D preferences | N/A | Y | N/A | Missing | non-goal | — | none | edition:Extended-only; retired:src:3 | non-goal:feasibility:72 |
| P-SHELL-01 | Application frame | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/frame.cpp | none | — | — |
| P-SHELL-02 | Menu customization and enablement matrix | Y | Y | Y | Missing | in-scope | — | none | Edit > Menus, Show All Menu Items, and the enablement matrix remain | UI-5 |
| P-SHELL-03 | Type menu | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/menu_type.cpp | none | Work path, convert to shape, find/replace, and the option rows ship; Google Fonts (#1442) and Check Spelling (#1443) stay open | #502 |
| P-SHELL-04 | View menu stubs | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/frame_view.cpp | none | Show/Extras, grid, pixel grid, layer edges, target path, lock/clear slices, 200%, and print size ship; Gamut Warning, Pixel Aspect Ratio, Snap To (#305), and 32-bit Preview (#570) remain | #503 |
| P-SHELL-05 | File menu stubs | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp/frame_commands.cpp | none | Open As, File > Browse, and Statistics ship; the remaining Automate and Scripts leaves stay with #558, #617, #619, #620, and #630 | #504 |
| P-SHELL-06 | Status bar readouts and options | Y | Y | Y | Missing | in-scope | — | none | Status bar readouts and options popup remain | UI-1 |
| P-SHELL-07 | Active layer name in the title bar | Y | Y | Y | Missing | in-scope | — | none | Not shown | UI-2 |
| P-SHELL-08 | Window Arrange: Float, Tile, Cascade, Match, New Window | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #500 |
| P-SHELL-09 | Document tabs and tear-out/floating windows | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | #317 |
| P-SHELL-10 | Open-document list in the Window menu | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp/menu_window_arrange.cpp | none | The open-document list at the bottom of the Window menu | #923 |
| P-SHELL-11 | Workspaces | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | Essentials, Painting, Photography, and Typography | — |
| P-SHELL-12 | Keyboard shortcuts editor | Y | Y | Y | Partial | in-scope | crates/pictura-app/cpp | none | Shortcut sets, save/load, workspace assignment, Summarize HTML, and legacy channel shortcuts remain | UI-12 |
| P-SHELL-13 | Accessibility | Y | Y | Y | Missing | in-scope | — | none | AT-SPI2, QAccessible, high contrast, colour-blind, and reduced-motion support remain | UI-11 |
| P-SHELL-14 | Edition gating (Standard and Extended) | Y | Y | Y | Complete | in-scope | crates/pictura-core/src | none | — | #650 |
| P-SHELL-15 | Dark UI and icon set | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-16 | Panel docking and remembered layout | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-17 | Toolbox and options bar | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-18 | Quick fill with Alt+Backspace and Ctrl+Backspace | Y | Y | Y | Missing | in-scope | — | none | Not implemented | UI-13 |
| P-SHELL-19 | Localization and RTL UI | Y | Y | Y | Missing | in-scope | — | none | QTranslator, .qm, RTL, and locale number formatting remain | XC-2 |
| P-SHELL-20 | Screen modes | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-21 | Rulers and guides | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-22 | Smart guides | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-23 | Isolation mode | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |
| P-SHELL-24 | Canvas rotation | Y | Y | Y | Complete | in-scope | crates/pictura-render/src | none | — | — |
| P-SHELL-25 | Panel column and rail | Y | Y | Y | Complete | in-scope | crates/pictura-app/cpp | none | — | — |

## Worked examples

| Kind | Row | Source |
|---|---|---|
| complete workflow | P-FMT-01 | src:1 |
| partial implementation | P-FMT-10 | WF-2 |
| unknown assessment | P-LAY-25 | #813 |
| intentional non-goal | P-PAN-25 | non-goal:feasibility:73 |
| CC-only capability | P-LAY-39 | src:4 |
| retired capability | P-LAY-40 | src:3 |

## Largest remaining gaps

- **Cross-cutting depth (SYS-2, #490):** bit-depth and colour-mode availability gating is unenforced across filters and adjustments; P-IMG rows and P-FLT rows carry the individual ceilings.
- **Painting depth (SYS-4, #492; SYS-5, #493):** brush Texture and Dual Brush, sampled/erodible tips, and Define Brush Preset remain; P-TOOL-75 and P-TOOL-76.
- **Automation (AUTO-4, #619; AUTO-5, #620):** Image Processor, most Automate commands, droplets, and the scripting host remain; P-AUT-04 through P-AUT-13.
- **CC-only generative and cloud (non-goal:feasibility:76):** Generative Fill/Expand, Generate Image, Neural Filters, Relight, Libraries, Cloud Documents, and UXP remain outside the local product; P-FLT-34 through P-FLT-37, P-FLT-39, and P-AUT-19 through P-AUT-22.
- **Extended-only (non-goal:feasibility:72):** 3D layers, the 3D panel, 3D file import, and the ten 3D tools are missing; P-LAY-40, P-PAN-26, and the P-TOOL 3D rows.

## Baseline history

| Date | Pictura commit | CS6 Standard | CS6 Extended | CC version | CC |
|---|---|---|---|---|---|
| 2026-11-21 | 4736b57e | 208/349 (59.6%) | 209/366 (57.1%) | 28.0 | 210/374 (56.1%) |
| 2026-11-21 | 39ea0380 | 202/349 (57.9%) | 203/366 (55.5%) | 28.0 | 204/374 (54.5%) |
| 2026-11-21 | 7ab28fc5 | 192/349 (55.0%) | 193/366 (52.7%) | 28.0 | 194/374 (51.9%) |
| 2026-11-20 | 89c8ba95 | 191/349 (54.7%) | 192/366 (52.5%) | 28.0 | 193/374 (51.6%) |

## Update procedure

- A feature ship or removal updates the affected row, the headline, the category row, and the history in the same pull request that updates `ROADMAP.md`.
- Row IDs are stable. A new capability receives a new ID; existing rows are never renumbered.
- Preserve each assessment in the history, including distinct assessed commits on the same date. A new Photoshop major adds a new frozen CC baseline.
- The baseline is re-frozen at each assessment; the date and Pictura commit above are part of the record.

## Sources

1. Adobe, Photoshop CS6 Help (13.0, 2012): menu, tool, panel, and feature reference; distilled in docs/02-ui-ux/menus.md, docs/03-tools/, and docs/02-ui-ux/panels/.
2. Adobe, Photoshop CS6 Help, "What's new in CS6" (2012).
3. Adobe, "Photoshop 3D | Common questions on discontinued 3D features" (2024), https://helpx.adobe.com/photoshop/kb/3d-faq.html.
4. Adobe, "What's new in Adobe Photoshop on desktop" (November 2026, version 28.0), https://helpx.adobe.com/photoshop/desktop/whats-new/whats-new-in-adobe-photoshop-on-desktop.html.
5. Adobe, "Adobe Photoshop on desktop release notes" (November 2026, version 28.0; LTS 27.11.1), https://helpx.adobe.com/photoshop/desktop/whats-new/photoshop-on-desktop-release-notes.html.
6. Adobe, "Photoshop supported file formats" (2026), https://helpx.adobe.com/photoshop/using/file-formats.html.
7. Adobe, Photoshop desktop help: tools, panels, filters, adjustments, and preferences (2026).
8. Adobe, "Measurement features in Photoshop" (2024), https://helpx.adobe.com/photoshop/using/measurement.html.
9. Adobe, "DICOM files in Photoshop" (2024), https://helpx.adobe.com/photoshop/using/dicom-files.html.
10. Adobe, "Create panoramic images with Photomerge" (2024), https://helpx.adobe.com/photoshop/using/create-panoramic-images-photomerge.html.
11. Adobe, "Filter effects reference" (2026), https://helpx.adobe.com/photoshop/using/filter-effects-reference.html.
12. Adobe, Photoshop 2026 and MAX 2026 feature announcements (2026), https://blog.adobe.com/.
