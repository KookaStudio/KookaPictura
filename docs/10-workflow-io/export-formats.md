# Export Formats

- **Spec ID:** `WF-004`
- **Status:** `Draft`
- **Parity tier:** `Core` (Export Layers To Files, Data Sets As Files, Zoomify, image formats); `Extended-only` (Render Video and video/image-sequence export, 3D export formats); `Non-goal (Linux)` (Device Central, QuickTime/Adobe Media Encoder integrations that have no direct Linux equivalent — documented rather than dropped)
- **New in CS6:** `Changed` — CS6's redesigned video engine expands export/import containers and adds DPX and other video export formats; audio/video support moves partly into Standard. The still-image export commands (`Export Layers To Files`, `Data Sets As Files`, `Zoomify`) and the Save As format dialogs are unchanged from CS5.
- **Depends on:** `ARCH-011` file-formats, `ARCH-008` document-model, `WF-003` save-and-save-as, `WF-005` web-export-and-slices, `09-automation/variables-and-data-driven-graphics.md`, `10-workflow-io/camera-raw-workflow.md`, `02-ui-ux/panels/timeline-panel.md`.

> All crate, module, widget, and type names below are **design proposals**. No code
> exists in this repository. CS6 behavior is taken from the fetched CS6 Help PDF
> unless marked *(inferred)*.

## CS6 behavior

CS6 exposes output through several distinct paths. "Export" is not one dialog; it
is a family of commands with separate option surfaces:

| Command | Menu path | Output |
|---|---|---|
| Export Layers To Files | `File > Scripts > Export Layers To Files` | One file per layer in PSD/BMP/JPEG/PDF/Targa/TIFF |
| Data Sets As Files | `File > Export > Data Sets As Files` | One PSD per data set (data-driven graphics) |
| Paths to Illustrator | `File > Export > Paths to Illustrator` | Paths written as an Illustrator-compatible file *(inferred; not documented in the CS6 Help PDF)* |
| Render Video | `File > Export > Render Video` | QuickTime movie or image sequence (Extended/CS6 video engine) |
| Video Preview / Send Video Preview To Device | `File > Export > Video Preview`, `File > Export > Send Video Preview To Device` | Device preview (Extended) |
| Zoomify | `File > Export > Zoomify` | JPEG tiles + HTML for pan/zoom web viewing |
| Save for Web & Devices | `File > Save For Web & Devices` | Optimized GIF/JPEG/PNG-8/PNG-24 + HTML/CSS (`WF-005`) |
| Save As / Photoshop PDF | `File > Save As` | Any Save As format, including PDF (`WF-003`) |
| Image Processor | `File > Scripts > Image Processor` | Folder of JPEG/TIFF/PSD copies (Batch workflow) |
| Web Photo Gallery | Bridge (`Tools > Photoshop > Web Photo Gallery`), or `File > Automate > Web Photo Gallery` with the optional plug-in | HTML gallery (`WF-005` / Bridge) |

### Export Layers To Files

`File > Scripts > Export Layers To Files` writes each layer as its own file:

1. **Destination** — Browse for the output folder (defaults to the source folder).
2. **File Name Prefix** — a common prefix for generated names.
3. **Visible Layers Only** — export only layers with visibility enabled.
4. **File Type** — **PSD, BMP, JPEG, PDF, Targa, TIFF**; set the format's options.
5. **Include ICC Profile** — embed the working-space profile.
6. **Run**.

Layers are named automatically; format options match the Save As dialogs
(`WF-003`).

### Data Sets As Files (data-driven graphics)

Requires variables and one or more data sets (`Image > Variables > Data Sets`):

`File > Export > Data Sets As Files` → enter a base name (a custom naming scheme is
allowed), **Select Folder** for the destination, choose which data sets to export,
OK. Output is **PSD files**, one per data set, in batch mode. The external
tab/comma-separated data-file syntax and its quoting rules are the input format
for bulk data sets (`09-automation/variables-and-data-driven-graphics.md`).

### Paths to Illustrator

`File > Export > Paths to Illustrator` writes the document's paths (work paths and
clipping paths) to a file that Illustrator can open, so vector outlines can be
reused. The CS6 Help PDF does not document this command in the fetched topics; it
is included here as *(inferred)* from the shipped File > Export menu and must be
verified (see `## Open questions`). Cross-reference `08-selection/paths-and-vector-selection.md`
and the EPS/vector path resources in `ARCH-011`.

### Render Video (CS6 video engine, Extended)

`File > Export > Render Video`:

1. Enter a name for the video or image sequence.
2. **Select Folder** for output; optional **Create New Subfolder**.
3. **File Options**: **QuickTime Export** or **Image Sequence**, then a file format
   from the pop-up menu.
4. Optional **Settings** for format-specific options.
5. For **Image Sequence**: **Starting** and **Digits** numbering, and a **Size**
   (pixel dimensions) option.
6. **Range**: **All Frames**, **In Frame**/**Out Frame**, or **Currently Selected
   Frames** (work-area bar).
7. Render options: **Alpha Channel** (None / Straight-Unmatted / Premultiplied;
   only formats that support alpha, e.g. PSD/TIFF) and **Frame Rate** (Document
   Frame Rate or a target standard such as NTSC→PAL).
8. **Render**.

QuickTime export file formats: **3G**, **FLC** (FLI), **Flash Video (FLV)**,
**QuickTime Movie** (required for audio), **AVI**, **DV Stream**, **Image
Sequence**, **MPEG-4**; third-party codecs (for example Avid AVR) require their
QuickTime codecs installed. CS6 video export formats include **DPX**, **MOV
(QuickTime)**, and **MP4**. Key-frame intervals affect size and seeking.

### Zoomify

`File > Export > Zoomify` posts a high-resolution image for pan/zoom viewing; the
base-size image downloads like an equivalent JPEG. Options: **Template** (viewer
background/navigation), **Output Location**, **Image Tile Options** (tile quality),
**Browser Options** (base image pixel width/height). Output is JPEG tiles plus an
HTML file to upload to a web server.

### Save As format dialogs (cross-reference)

The per-format option dialogs used by Save As, Save a Copy, and Export Layers To
Files are specified in `WF-003` (TIFF, JPEG, PNG, GIF, EPS, DCS, Photoshop Raw,
BMP, Cineon, Targa, PDF). This spec treats them as a shared option surface rather
than re-documenting each control.

### Web-oriented export (cross-reference)

`Save for Web & Devices`, slice export, HTML/CSS generation, and the GIF/JPEG/
PNG-8/PNG-24/WBMP optimization matrix are `WF-005`. Web Photo Gallery moved to
Bridge in CS5; the Photoshop `File > Automate > Web Photo Gallery` path requires
the optional plug-in and 32-bit mode on 64-bit macOS.

### Export As (CC-only, in scope)

CS6 has no `Export As` dialog; it exposes `File > Save For Web & Devices` for
web-oriented output and the Save As format dialogs for everything else. Photoshop
CC (post-CS6) added `File ▸ Export As…`, a single dialog that combines the output
settings and a live preview, and this project carries it as a deliberate
**CC-only extension**:

1. **Layer list** — the flattened document plus each top-level layer that has
   pixels (adjustment layers excluded), each with a checkbox; the checked rows
   are the export targets, and the document is checked by default. Selecting a
   row previews it.
2. **Preview** — the selected target after sizing and canvasing, with Fit / 1:1
   and zoom controls, and — for a small target — an estimated output size.
3. **File Settings** — **Format** is PNG, JPG, PNG-8, or GIF; **Transparency**
   is offered for PNG, PNG-8, and GIF, **Quality** for JPG. SVG export is a
   separate, deferred capability (issue #459), not offered here.
4. **Image Size** — Width, Height, Scale (percent), and a **Resample** menu
   (Bicubic, Bicubic Automatic, Bicubic Sharper, Bicubic Smoother, Bilinear,
   Nearest Neighbor, Preserve Details).
5. **Canvas Size** — Canvas Width and Height plus **Reset**; the output is
   canvas-sized with the resampled image placed top-left, padded with
   transparency or cropped, and Reset returns the canvas to the image size.
6. `Cancel` / `Export…`; one checked row writes a file, several write one file
   per row into a chosen folder (the document named from the document, a layer
   from the layer name).

PNG and JPG are written through the Qt image writers (`WF-003`/`WF-004`); PNG-8
and GIF through the engine web encoders (`WF-005`, `interop/raster-export`). A
layer target is the layer rendered alone on transparency, cropped to its bounds.
`Export As` never changes the document's path, modified state, or history.

### Mobile preview (legacy)

Adobe Device Central (CS5-era) connects from `File > New` (Device Central button)
or `File > Save For Web & Devices` (Device Central button) to preview output on
mobile devices. CS6 retains the entry points; the service is a CS5 artifact and is
`Non-goal (Linux)` pending an open substitute.

### 3D and special-purpose export (Extended)

CS6's support list includes 3D-related formats: **DAE (Collada)**, **U3D**,
**Wavefront|OBJ**, **KMZ (Google Earth 4)**, **Flash 3D** (export only), **JPS
(JPEG Stereo)**, and **MPO (Multi-Picture)** (export-only variants noted in the
support list). These belong to the Extended 3D pipeline
(`03-tools/3d-tools.md`, `05-layers/3d` scope) and are out of the still-image core.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Scripts > Export Layers To Files` | Dialog | — | Per-layer files, PSD/BMP/JPEG/PDF/Targa/TIFF |
| `File > Scripts > Image Processor` | Dialog | — | Folder → JPEG/TIFF/PSD copies |
| `File > Export > Data Sets As Files` | Dialog | — | One PSD per data set |
| `File > Export > Paths to Illustrator` | Command | — | *(inferred)* path/outline export |
| `File > Export > Render Video` | Dialog | — | QuickTime / Image Sequence (Extended) |
| `File > Export > Video Preview` | Dialog | — | Extended device/preview output |
| `File > Export > Send Video Preview To Device` | Command | — | Extended |
| `File > Export > Zoomify` | Dialog | — | JPEG tiles + HTML |
| `File > Save For Web & Devices` | Dialog | `Ctrl/Cmd+Shift+Alt+S` *(inferred)* | `WF-005` |
| `File > Export As` | Dialog | — | CC-only extension: layer list, preview, PNG/JPG/PNG-8/GIF, Image Size, Canvas Size |
| `File > Save As > Photoshop PDF` | Dialog | — | `WF-003` |
| `File > Automate > Web Photo Gallery` | Dialog | — | Optional CS5 plug-in / Bridge replacement |
| `File > New > Device Central` | Button | — | CS5-era mobile preview |
| Save for Web — **Device Central** button | Button | — | CS5-era mobile preview |

## Parameters & ranges

| Control | Command | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Destination | Export Layers To Files | folder | source folder | any writable path | — |
| File Name Prefix | Export Layers To Files | string | *(document name)* *(inferred)* | free text | — |
| Visible Layers Only | Export Layers To Files | bool | Off | on / off | Skip hidden layers |
| File Type | Export Layers To Files | enum | PSD *(inferred)* | PSD, BMP, JPEG, PDF, Targa, TIFF | Reuses Save As option dialogs |
| Include ICC Profile | Export Layers To Files | bool | Off *(inferred)* | on / off | Working-space profile |
| Base name | Data Sets As Files | string | *(document name)* *(inferred)* | custom scheme allowed | — |
| Data sets | Data Sets As Files | list | all *(inferred)* | subset selection | Output is PSD per set |
| File Options | Render Video | enum | QuickTime Export *(inferred)* | QuickTime Export / Image Sequence | Extended |
| Format | Render Video | enum | *(unresolved)* | 3G, FLC, FLV, QuickTime Movie, AVI, DV Stream, Image Sequence, MPEG-4 (QuickTime); DPX, MOV, MP4 (CS6 list) | Audio needs QuickTime Movie |
| Starting / Digits | Render Video (sequence) | int | 1 / 4 *(inferred)* | ≥1 | File numbering |
| Size | Render Video (sequence) | dimensions | document size | pixel dimensions | — |
| Range | Render Video | enum | All Frames | All Frames / In Frame & Out Frame / Currently Selected Frames | — |
| Alpha Channel | Render Video | enum | None | None / Straight-Unmatted / Premultiplied | Alpha-capable formats |
| Frame Rate | Render Video | enum/number | Document Frame Rate | document or target standard (NTSC/PAL) | — |
| Create New Subfolder | Render Video | bool | Off | on / off | — |
| Template | Zoomify | enum | *(default template)* | template list | Viewer chrome |
| Output Location | Zoomify | folder | *(unresolved)* | any writable path | — |
| Image Tile Options | Zoomify | enum/quality | *(unresolved)* | quality levels | JPEG tile quality |
| Browser Options | Zoomify | dimensions | *(unresolved)* | pixel width/height | Base image size in viewer |
| JPEG quality (tiles) | Zoomify | int | *(unresolved)* | compression scale | Tile quality |

## Algorithms & pipeline

### Export Layers To Files

1. Traverse the layer tree in document order; honor **Visible Layers Only**.
2. For each selected layer, derive a single-layer `Document` (the layer plus a
   transparency canvas sized to the document).
3. Serialize each derived document with the chosen format's serializer from
   `WF-003`, embedding the ICC profile when requested.
4. Generate names from the prefix plus the layer name (sanitized for the
   filesystem); collision handling is implementation-defined *(inferred)*.
5. Run as a cancellable batch with per-file progress (`09-automation/batch-processing.md`).

### Data Sets As Files

1. Read the defined variables/data sets (`Image > Variables`);
2. For each selected data set, apply it to a scratch copy of the base document
   (visibility, pixel replacement, text replacement);
3. Flatten/serialize as PSD to the destination with the base-name scheme;
4. Batch with progress; never mutate the open document (Help warns `Image > Apply
   Data Set` overwrites the base image, so export must work on a copy).

### Render Video / image sequence

1. Determine the frame range and frame rate.
2. For each output frame, render the composited canvas at the requested size
   (timeline/animation state per `02-ui-ux/panels/timeline-panel.md`).
3. Apply alpha handling (None / Straight-Unmatted / Premultiplied) and encode with
   the selected codec/container.
4. For Image Sequence, write numbered stills (Starting/Digits); for QuickTime/MP4/
   DPX, mux into the container via a codec backend.
5. On Linux there is no QuickTime/Adobe Media Encoder; the design substitutes an
   FFmpeg-style backend for the containers, and pure image-sequence output for the
   rest. This is a **behavioral-parity** substitution, not a codec-identical port.

### Zoomify

1. Downsample the composite into a tile pyramid at the base browser size.
2. Encode JPEG tiles at the chosen tile quality, grouped into Zoomify's tile
   folders.
3. Write the HTML page from a template plus browser-size parameters.

### Paths to Illustrator

1. Collect path resources (work paths and clipping paths) from the document.
2. Emit them in an Illustrator-readable vector form, preserving each path's
   subpaths, open/closed state, and coordinates. The exact target format
   (Illustrator legacy `.ai`/EPS-based) is unverified in this pass.

### Shared option surface

Export Layers To Files, Save As, and Save a Copy share the `SaveOptions` structs
and serializers from `WF-003`; the exporter builds a projection of the document and
calls the same `serialize` path. This avoids a second implementation of every
codec's option handling.

## Rust module mapping

Proposals, on top of `WF-003` and `ARCH-011`:

- `pictura_io::export::layers_to_files` — `LayersExportRequest { dest, prefix,
  visible_only, format, include_icc }`; derives per-layer documents and serializes.
- `pictura_io::export::data_sets` — `DataSetsExportRequest { dest, base_name,
  sets }`; applies data sets to a scratch document (reuse the Variables engine in
  `09-automation`).
- `pictura_io::export::paths` — writes path resources to an Illustrator-readable
  file (format TBD).
- `pictura_io::video::render` — `RenderVideoRequest { range, frame_rate, size,
  alpha, container, codec }`; a codec backend trait with an FFmpeg-style Linux
  implementation and a pure image-sequence writer.
- `pictura_io::zoomify` — `ZoomifyRequest { template, dest, tile_quality,
  browser_size }`; tile-pyramid builder + HTML template renderer.
- `pictura_shell::export::ExportController` — command routing, batch progress,
  cancellation.
- `pictura_io::batch::Progress` — shared progress/cancel channel reused by
  `09-automation/batch-processing.md`.

Types crossing the Qt boundary: the request structs, `Progress` events, and the
option enums reused from `WF-003`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ExportLayersDialog` | `QDialog` | Destination, prefix, visible-only, file type + format options, ICC |
| `DataSetsExportDialog` | `QDialog` | Base name, folder, data-set selection |
| `RenderVideoDialog` | `QDialog` | File options, format + settings, range, alpha, frame rate |
| `ZoomifyDialog` | `QDialog` | Template, output location, tile quality, browser size |
| `ExportProgressDialog` | `QDialog` | Batch progress + cancel (shared with automation) |
| `ExportMenu` | `QMenu` | Populates `File > Export` and `File > Scripts` export entries |
| `PathsExportDialog` | `QFileDialog` | Destination for the Illustrator path file |

Widgets over QML: dense modal dialogs and progress reporting
(`qt6-ui-design.md`). Long-running exports run off the UI thread with a progress
channel; the dialog stays responsive.

## Data-model impact

- Export operations are **non-mutating**: they build a projected/scratch document,
  serialize it, and discard it. The open document's history is untouched.
- Export settings (last destination, prefix, format, zoomify template, video
  settings) are stored as **application/command presets**, not in the document; the
  document itself gains nothing serializable.
- Data Sets export depends on the variables/data-sets model in
  `09-automation/variables-and-data-driven-graphics.md`.
- Video export depends on the timeline/video-layer model (Extended) and the frame
  renderer in the compositor.
- Paths export reads the path resources (`ARCH-008`), which are also written into
  PSD/EPS/TIFF.

## Edge cases

- **Hidden layers**: Export Layers To Files skips them only when Visible Layers
  Only is on; otherwise it exports them.
- **Empty/single-layer documents**: one output file.
- **Name collisions / illegal characters**: sanitize and de-duplicate; the exact
  CS6 scheme is unverified.
- **PSB-sized layers**: per-layer documents can be huge; stream rather than hold
  all outputs in memory.
- **Data sets with Pixel Replacement pointing at a missing file**: `Do Not
  Replace` leaves the layer as-is (Help note); export must not abort the whole
  batch.
- **Video without a codec/backend**: on Linux, unavailable containers degrade to
  image-sequence output or report a clear error; never crash.
- **Frame range beyond the timeline**: clamp and warn.
- **Alpha handling**: premultiplied/straight only for alpha-capable formats; PSD/
  TIFF examples from Help.
- **Zoomify very large images**: tile pyramid memory must be bounded; stream tiles.
- **ICC embedding**: some exporters (for example layered TIFF layers) have limited
  metadata carriage; surface what is dropped.
- **Device Central / Web Photo Gallery**: absent on Linux; the commands are
  Non-goal or replaced by external tooling; do not present a dead button.
- **3D formats (Extended)**: out of the still-image core; owned by the 3D spec.
- **GPU unavailable**: export falls back to CPU rendering and must still complete.

## Parity acceptance criteria

- Given a multi-layer document with some layers hidden and `Export Layers To
  Files` with Visible Layers Only on, the output folder contains one file per
  visible layer and none for hidden layers, each named with the prefix.
- Given `Export Layers To Files` in TIFF with Layer data unavailable (flattened
  formats), each output holds that single layer's pixels against transparency.
- Given data sets D1..Dn and `Data Sets As Files`, the destination contains n PSD
  files whose variable content matches each set, and the open document is unchanged.
- Given an image sequence export with Starting S, Digits N, and range R, the
  output files are numbered consistently and represent exactly frames R at the
  chosen size and frame rate.
- Given a QuickTime/MP4 export on a Linux build with the substitute backend, a
  playable file is produced with the chosen frame rate and duration within one
  frame; unsupported codecs report a clear error.
- Given `Zoomify` with a chosen browser size B and tile quality Q, the output
  contains a tile pyramid and an HTML file that displays the image panning/zooming
  at base size B.
- Given `Export > Paths to Illustrator`, each document path is emitted with its
  subpaths and coordinates preserved (once the target format is confirmed).
- Given a document with an ICC working space and Include ICC Profile on, exported
  TIFF/JPEG carry the profile; turning it off omits it.
- Given a format whose options exist in Save As, the Export Layers To Files option
  pane shows the same controls (shared surface).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the
  CS6 Help corpus, downloaded and text-extracted for this pass. Established:
  **Export layers to files** (Destination, File Name Prefix, Visible Layers Only,
  File Type PSD/BMP/JPEG/PDF/Targa/TIFF, Include ICC Profile, Run); **Generate
  graphics using data sets** (`File > Export > Data Sets As Files`, base name,
  Select Folder, choose data sets, PSD output) and the external data-file syntax;
  **Export video files or image sequences** (Render Video dialog: name, folder,
  subfolder, QuickTime Export vs. Image Sequence, Settings, Starting/Digits/Size,
  Range options, Alpha Channel None/Straight-Unmatted/Premultiplied, Frame Rate);
  the QuickTime export format list (3G, FLC, FLV, QuickTime Movie, AVI, DV Stream,
  Image Sequence, MPEG-4) and key-frame notes; `File > Export > Video Preview` and
  `File > Export > Send Video Preview To Device`; **Export to Zoomify** (Template,
  Output Location, Image Tile Options, Browser Options, JPEG tiles + HTML);
  the CS6 **Supported file formats** list (video export formats DPX/MOV/MP4; 3D
  formats 3DS, DAE, Flash 3D, JPS, KMZ, MPO, U3D, OBJ); the `File > Automate >
  Web Photo Gallery` optional plug-in note and Bridge replacement; Device Central
  (CS5) entry points; and the **Flatten frames into layers** note that frames can
  be exported as separate image files.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+New+Document+dialog+options+artboard+recent`
  — search-result page (snippet only); used only for the CS6 artboard context in
  `WF-002`, not for export behavior.

Internal cross-references (not external URLs): `docs/01-architecture/file-formats.md`
(per-format capabilities and the shared serializers), `docs/10-workflow-io/save-and-save-as.md`
(the option dialogs), `docs/10-workflow-io/web-export-and-slices.md` (Save for Web
and Web Photo Gallery).

**Export As (CC-only).** The CS6 Help corpus does not document an Export As
dialog (CS6 exposes Save For Web & Devices instead). The surface above is the
Photoshop CC (post-CS6) dialog, recorded from the shipped dialog (issue #457's
screenshots); everything not shown there is *(inferred)*. It is a CC-only scope
extension, not CS6 parity, and has no pixel oracle.

## Open questions

- **Paths to Illustrator.** The command is not documented in the fetched CS6 Help
  PDF topics. Confirm its exact menu label, output format, and whether it exports
  all paths or a selection. Resolve from a shipped CS6 build or the CS6 user guide.
- **Export Layers To Files naming and defaults.** The default prefix, collision
  scheme, and default file type are unstated. Resolve with a CS6 dialog pass.
- **Image Processor vs. Export Layers To Files.** CS6 Help documents the Image
  Processor under scripting; whether it is a separate export surface or purely a
  batch script is unresolved. Resolve against a CS6 install.
- **Linux video backend.** There is no decision recorded here on the codec backend
  (FFmpeg/GStreamer/pure-Rust). Resolve in `00-overview/feasibility-and-non-goals.md`
  and `01-architecture/plugin-and-scripting-abi.md`.
- **Data Set pixel-replacement error handling.** Whether CS6 aborts or skips a
  batch on a missing replacement file is unverified.
- **Zoomify tile geometry and template set.** Tile size, pyramid steps, and the
  exact template files are not documented in the Help text. Resolve from a Zoomify
  export or the Zoomify format documentation.
- **Device Central substitute.** No Linux equivalent is chosen; the command's
  parity status is `Non-goal (Linux)` pending a decision.
- **3D export formats.** DAE/U3D/OBJ/KMZ/Flash 3D/JPS/MPO option dialogs belong to
  the Extended 3D spec and are out of scope here.
- **Qt6 details.** Proposed dialog classes need verification against Qt6 current
  docs before implementation.
