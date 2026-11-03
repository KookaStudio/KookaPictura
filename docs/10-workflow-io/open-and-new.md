# Open and New

- **Spec ID:** `WF-002`
- **Status:** `Draft`
- **Parity tier:** `Core` (New, Open, Open As, Open Recent, Open as Smart Object, PDF/EPS import); `Extended-only` (video/image-sequence open and video layers)
- **New in CS6:** `Changed` — CS6 adds a redesigned video import engine and broadens audio/video import to both Standard and Extended editions. Formats newly marked with an asterisk in the CS6 support list include audio (AAC, AIFF, M2A, M4A, MP2, MP3) and video containers (`.264`/AVC, 3GP/3GPP, F4V, FLV, MPE, MTS, MXF, R3D, TS, VOB). The still-image New/Open dialogs are unchanged from CS5.
- **Depends on:** `ARCH-011` file-formats, `ARCH-008` document-model, `ARCH-007` color-management, `WF-001` document-lifecycle, `WF-003` save-and-save-as, `LAY-020` smart-objects, `10-workflow-io/camera-raw-workflow.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 behavior is taken from the fetched CS6 Help PDF
> unless marked *(inferred)*.

## CS6 behavior

### New document

`File > New` opens the **New** dialog:

1. A name for the image.
2. Optional **Preset** choice (document size presets; a device-oriented path via
   the **Device Central** button).
3. Width and height via the **Size** menu or text boxes. Choosing a filename from
   the bottom of the **Preset** menu matches the new image's width, height,
   resolution, color mode, and bit depth to **any open image**.
4. **Resolution**, **Color Mode**, and **bit depth**.
5. Canvas color: **White**, **Background Color**, or **Transparent**. Transparent
   creates a single transparent layer as the document contents.
6. Optional **Advanced** options: a color profile or **Don't Color Manage This
   Document**, and **Pixel Aspect Ratio** (choose **Square** unless the image is
   for video, in which case non-square pixel ratios are available).
7. Optionally **Save Preset**, or OK to open the new file.

If a selection was copied to the clipboard, the New dialog's dimensions and
resolution are **automatically based on the clipboard image data**.

The CS6 New dialog is **artboard-less**: the CS6 Help PDF documents no artboard
control in `File > New` (the only "Artboard" mentions are Illustrator-only Save
for Web options), and community sources place Photoshop artboards in CC 2015,
not CS6. See `## Open questions` — this contradicts the repository `GLOSSARY.md`.

### Duplicate

`Image > Duplicate` copies the whole image (layers, layer masks, channels) into
memory without saving to disk; optional **Duplicate Merged Layers Only**. See
`WF-001`.

### Open

- `File > Open` (`Ctrl/Cmd+O`): select one or more files (Shift-click a range,
  Ctrl/Cmd-click individual files) and click Open. Each selected file opens as
  its own document, following the **Open Documents As Tabs** preference; a file
  that fails to open does not stop the others. Some formats (camera raw, PDF)
  then show a format-specific options dialog. If a color-profile warning
  appears, choose whether to use the embedded profile as the working space,
  convert the document color to the working space, or discard the embedded
  profile (`ARCH-007`).
- `File > Open Recent`: submenu of recently used files. The count is the **Recent
  File List Contains** option in File Handling (`WF-001`).
- Plug-in modules supply many formats; a format missing from the Open dialog or
  the `File > Import` submenu may need its plug-in installed.
- Photoshop may fail to determine the correct format (for example after a
  cross-OS transfer that mislabeled the extension); the user then specifies the
  format explicitly.

### Open As (force the format)

- **Windows**: `File > Open As`, pick the file, choose the format from the **Open
  As** pop-up menu, click Open.
- **macOS**: `File > Open`, choose **All Documents** in the **Show** menu, pick
  the file, choose the format from the **Format** menu, click Open.
- If it still fails, the chosen format may not match the true format, or the file
  may be damaged.

This is the CS6 escape hatch for extension/content mismatch. Automated signature
sniffing (`ARCH-011`) should normally make Open As unnecessary, but the command
must exist for parity.

### Open as Smart Object

`File > Open As Smart Object` opens a file as a document whose sole layer is a
Smart Object containing that file. Help recommends placing PSD, TIFF, or PSB over
JPEG because those support lossless re-edit; a multilayer source appears
**flattened** on the new layer. Editing the Smart Object opens the source in
Photoshop (raster/camera raw) or Illustrator (vector PDF). See `LAY-020`.

### Open PDF

For generic PDF (not a Photoshop PDF), `File > Open` shows the **Import PDF**
dialog:

- **Select**: **Pages** or **Images**; click thumbnails (Shift-click for many);
  **Thumbnail Size** menu; the selected-item count appears under the preview.
- **Name** for the new document; multiple pages/images open as several documents
  with the base name plus a number.
- **Crop To**: **Bounding Box**, **Media Box**, **Crop Box**, **Bleed Box**,
  **Trim Box**, **Art Box** (definitions per the CS6 Help text).
- **Image Size**: **Width**/**Height** with **Constrain Proportions** (on by
  default); multi-page selections show the max width/height and scale
  proportionally; **Resolution**, **Mode**, **Bit Depth**.
- **Suppress Warning** for color-profile warnings.

Opening a **Photoshop PDF** (saved with Preserve Photoshop Editing Capabilities)
does not show the Import PDF dialog. PDF can also arrive via `Place`, Paste, or
drag-and-drop as a Smart Object (`LAY-020`).

### Open EPS

`File > Open` on an EPS rasterizes vector artwork to pixels. The user sets desired
dimensions, resolution, and mode, with **Constrain Proportions** to keep the
aspect ratio, and **Anti-aliased** to minimize jagged edges.

### Open camera raw

- `File > Open` browsing to a camera raw file routes through the **Camera Raw**
  plug-in dialog before the document opens.
- `Shift`-clicking Open (and `Shift`-double-clicking a Bridge thumbnail) bypasses
  the Camera Raw dialog and opens with current/default settings.
- In the Camera Raw dialog, `Shift`+clicking **Open Image** opens the raw file in
  Photoshop as a **Smart Object**, so Camera Raw settings can be revisited later.
- Camera Raw also edits JPEG/TIFF files. The JPEG/TIFF Handling preference decides
  whether they always open in Camera Raw, never, or only when they already carry
  Camera Raw settings.
- Camera Raw supports images up to 65,000 px in either dimension and up to 512
  megapixels, and converts CMYK to RGB on open.
- Camera Raw opens a copy as 8- or 16-bit depending on **Workflow Options**; it
  never overwrites the original raw file. Full develop controls live in
  `10-workflow-io/camera-raw-workflow.md`.

### Open With / external applications

From Bridge, `File > Open With > Adobe Photoshop` opens the selected file; from
the OS file manager, "Open with Photoshop" associates the file type. Photoshop's
own `File > Open` remains the canonical path.

### Import (video frames / layers, and scanners)

- **Video (Extended)**: `File > Open` on a video file, or
  `Layer > Video Layers > New Video Layer From File` to add it to an open
  document. Frames are referenced in a **video layer**; the layer references the
  original file, so edits do not alter the source.
- **Image sequence (Extended)**: open one file with the **Image Sequence**
  option, or import via the video-layer command; choose one file (selecting more
  than one disables the option), specify a **frame rate**, and each image becomes
  a frame in a video layer. Files should share pixel dimensions and be named in
  numeric/alphabetical order.
- **Place video (Extended)**: `File > Place` with a video/image sequence wraps the
  footage in a Smart Object (navigable via the Animation panel, Smart-Filter
  capable).
- **Scanners / WIA (Windows)**: `File > Import > WIA Support`, or `File > Import`
  and select the device; see the Help scan topic.
- **Annotations**: `File > Import > Annotations` imports PDF annotations
  *(inferred from the Import submenu; Help details in the annotations topic)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > New` | Dialog | `Ctrl/Cmd+N` *(inferred)* | Presets, size, mode, depth, background, Advanced, Save Preset, Device Central |
| `Image > Duplicate` | Dialog | — | In-memory copy |
| `File > Open` | Dialog | `Ctrl/Cmd+O` | Multi-file selection; format options for raw/PDF; profile warning |
| `File > Open Recent` | Submenu | — | Count from File Handling |
| `File > Open As` (Windows) / `File > Open`+Format (macOS) | Dialog | — | Force type on extension mismatch |
| `File > Open As Smart Object` | Dialog | — | Single smart-object document |
| `File > Place` | Dialog | — | Smart Object import into open doc (`LAY-020`) |
| `File > Open with > Adobe Photoshop` (Bridge) | Command | — | External open path |
| `Layer > Video Layers > New Video Layer From File` | Command | — | Extended: add video/sequence to open doc |
| `File > Import` | Submenu | — | WIA/scan, annotations; device-dependent |
| `Import PDF` dialog | Dialog | — | Pages/Images, Crop To, Image Size |
| `EPS` open options | Dialog | — | Dimensions, resolution, mode, anti-alias |
| Camera Raw dialog | Dialog | `Ctrl/Cmd+R` (Bridge) | Shift+Open bypasses; Shift+Open Image = Smart Object |
| `Edit/PS > Preferences > Camera Raw` | Preference | `Ctrl/Cmd+K` | JPEG/TIFF Handling |
| `Edit/PS > Preferences > File Handling` | Preference | — | Recent File List Contains |
| `File > Device Central` | Command | — | CS5-era mobile preview (legacy) |

## Parameters & ranges

| Control | Dialog | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Name | New | string | `Untitled-N` *(inferred)* | free text | Used for window title |
| Preset | New | enum | Custom *(inferred)* | size presets + open-document entries | Bottom entries match an open image |
| Width / Height | New | number + unit | 1000 px *(inferred)* | >0; units px/in/cm/mm/pt/pica/columns | — |
| Resolution | New | number | 72 ppi *(inferred)* | >0 ppi | — |
| Color Mode | New | enum | RGB *(inferred)* | Bitmap, Grayscale, RGB, CMYK, Lab | Indexed/Duotone not creatable directly |
| Bit Depth | New | enum | 8 (RGB), 16/32 | 1/8/16/32 per mode | 32-bit float HDR |
| Background Contents | New | enum | White | White / Background Color / Transparent | Transparent ⇒ single transparent layer |
| Color Profile | New (Advanced) | enum | sRGB *(inferred)* | working-space list / Don't Color Manage | — |
| Pixel Aspect Ratio | New (Advanced) | enum | Square | Square + non-square video ratios | — |
| Save Preset | New | action | — | names a preset | — |
| Select | Import PDF | enum | Pages | Pages / Images | — |
| Crop To | Import PDF | enum | Bounding Box *(inferred)* | Bounding/Media/Crop/Bleed/Trim/Art Box | — |
| Constrain Proportions | Import PDF / EPS | bool | On | on / off | — |
| Width / Height | Import PDF | number | page size | >0 | Max of selected pages |
| Resolution | Import PDF | number | 72 ppi *(inferred)* | >0 | — |
| Mode | Import PDF | enum | RGB *(inferred)* | document modes | — |
| Bit Depth | Import PDF | enum | 8 | 1/8/16/32 | — |
| Suppress Warning | Import PDF | bool | Off | on / off | Profile warnings |
| Anti-aliased | EPS open | bool | On *(inferred)* | on / off | Smooth vector edges |
| Image Sequence | Open | bool | Off | on / off | Extended; one file only |
| Frame Rate | Open (sequence) | number | 30 fps *(inferred)* | >0 fps | Extended |
| JPEG/TIFF Handling | Camera Raw prefs | enum | *(unresolved)* | Automatically open / Disable / Open only with settings | Applies to JPEG/TIFF |
| Recent File List Contains | File Handling | int | *(unresolved)* | positive int | Open Recent length |

## Algorithms & pipeline

### New-document construction

1. Build a `Document` with the requested `width × height × resolution`, color
   mode, and bit depth (`ARCH-008`).
2. Seed content: `White` and `Background Color` create a locked **Background**
   layer flattened to the canvas; `Transparent` creates one regular transparent
   layer. Background/layer conversion rules are in `05-layers/layers-overview.md`.
3. Attach the chosen working-space profile, or mark the document
   un-color-managed, and record the pixel aspect ratio.
4. The document is untitled and clean until the first edit.

### Format detection and dispatch

1. Read the file signature (magic bytes), not the extension (`ARCH-011`).
2. If the signature is camera raw, hand off to the Camera Raw adapter.
3. If PDF and not a Photoshop PDF, present Import PDF, then rasterize the chosen
   pages via the external renderer adapter (`ARCH-011`).
4. If EPS/PostScript, rasterize with the requested dimensions/resolution and
   anti-aliasing.
5. Otherwise dispatch to the matching codec and build a document in a compatible
   mode/depth, populating metadata, ICC profile, alpha, paths, and frames where
   the format carries them.
6. `Open As` overrides step 1 with the user-chosen codec; failure to parse
   reports "format mismatch or damaged file".

### Color-profile-on-open decision

Per `ARCH-007`: embedded-profile mismatch offers **Use the embedded profile (as
working space)**, **Convert document colors to working space**, or **Discard the
embedded profile**. The choice must be recorded so it can be replayed/undone.

### Video/open pipeline (Extended)

A video layer stores a reference to the source plus a frame index, not decoded
frames in the document (Help: "A video layer references the original file").
Image sequences become one frame per file. Placed footage becomes a Smart Object.
Rendering/export is `WF-004`.

### Smart-object open

`Open As Smart Object` embeds the source bytes and creates a single
`NodeKind::SmartObject` layer; the source document is rendered through the placed
transform (`LAY-020`). Placement (`File > Place`) differs only in that it targets
an existing document and runs an interactive transform before commit.

## Rust module mapping

Proposals, on top of `ARCH-011` (`pictura_io::detect`, `pictura_io::raster`,
`pictura_io::pdf`, `pictura_io::eps`, `pictura_io::raw`):

- `pictura_shell::open::OpenController` — orchestrates Open/Open As/Open Recent;
  owns the recent-file list and format overrides.
- `pictura_io::detect::sniff(path) -> FormatId` — signature detection (already
  proposed in `ARCH-011`); the source of truth for format dispatch.
- `pictura_core::newdoc::NewRequest` — `{ name, size, resolution, mode, depth,
  background: White | Color | Transparent, profile, pixel_aspect }`; builds the
  initial `Document`.
- `pictura_io::pdf::ImportOptions` — `{ select: Pages | Images, pages: Vec<u32>,
  crop_to: CropBox, width, height, constrain, resolution, mode, depth, suppress_warning }`.
- `pictura_io::pdf::rasterize` — external renderer adapter (Ghostscript/Poppler
  candidate per `ARCH-011`).
- `pictura_io::eps::ImportOptions` — `{ width, height, resolution, mode,
  constrain, antialias }`.
- `pictura_io::raw::CameraRawOpen` — `{ file, workflow: OpenAs8Or16, as_smart_object: bool }`.
- `pictura_io::video::VideoLayerRef` — reference to source + frame range; image
  sequence expansion.
- `pictura_io::presets::DocumentPresets` — New-document presets and the
  "match open document" entries.
- `pictura_shell::recent::RecentFiles` — persisted list, count from preferences.

Types crossing the Qt boundary: `NewRequest`, `ImportOptions` (PDF/EPS),
`RecentFiles` entries, and a render/parse `Progress` channel for large imports.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `NewDocumentDialog` | `QDialog` | Name, preset, size, mode/depth, background, Advanced, Save Preset |
| `OpenFileDialog` | `QFileDialog` | Format filter from `ARCH-011`; routes raw to Camera Raw |
| `ImportPdfDialog` | `QDialog` | Pages/Images thumbnail grid, Crop To, Image Size |
| `EpsImportDialog` | `QDialog` | Dimensions, resolution, mode, anti-alias |
| `CameraRawDialog` | `QDialog` | ACR-style develop editor before open (`camera-raw-workflow.md`) |
| `RecentFilesMenu` | `QMenu` | Shared with `WF-001` |
| `FormatOverrideDialog` | `QDialog` | Open As: force a codec |
| `OpenProgressDialog` | `QDialog` | Progress/cancel for PDF/EPS/video/large imports |
| `DocumentPresetModel` | `QAbstractListModel` | New-dialog presets + open-document entries |

Widgets over QML: dialogs are dense, modal, keyboard-driven (`qt6-ui-design.md`).
The Import PDF thumbnail grid is a `QListView` + `QStyledItemDelegate`; the New
dialog is a plain form. Camera Raw is a complex docked-style dialog treated as its
own spec.

## Data-model impact

- `Document` gains `pixel_aspect_ratio`, `profile: Option<IccProfile>` or an
  explicit un-color-managed flag, and `source: Created | Opened(path, format) |
  Duplicated(parent)` provenance used by Save and Revert.
- Video layers add `NodeKind::Video { source_ref, frame_index, frame_rate }`
  (Extended); image sequences expand to a video layer backed by N files.
- Import populates metadata (`ARCH-008`), alpha channels, paths (TIFF clipping
  path), and animation frames where the source carries them.
- Undo: creating, opening, or duplicating a document is not undoable in the normal
  document history (it is window-level). The profile-on-open decision is recorded
  so it can be surfaced/replayed, but it happens before the first history state.
- Serialization: opening does not modify the source file. Re-saving a PSD/PSB
  preserves unknown blocks (`ARCH-011`).

## Edge cases

- **Extension ≠ content**: signature detection wins; `Open As` is the manual
  fallback. A mislabeled file must not be trusted.
- **No profile / profile mismatch**: honor `ARCH-007` warnings; "Don't Color
  Manage" creates an unmanaged document.
- **PDF with no vector/text**: opens as a raster image; renderer absent ⇒ clear
  error, no crash (`ARCH-011`).
- **Photoshop PDF vs generic PDF**: only generic PDF shows Import PDF.
- **EPS without a PostScript renderer**: fail gracefully.
- **Multiple PDF pages/images**: open several documents with base-name numbering.
- **Camera raw unsupported camera**: surface the camera model; offer generic TIFF
  if applicable.
- **`Shift`-open** bypasses Camera Raw; `Shift`+Open Image yields a Smart Object.
- **Clipboard-driven New**: dimensions/resolution preloaded from the clipboard.
- **Transparent New**: no Background layer; the bottommost layer is a regular
  layer (no stacking/opacity restrictions of a Background).
- **Indexed/Duotone**: not directly creatable in New; convert from a base mode.
- **Video/sequence**: mismatched pixel dimensions degrade the animation; naming
  must sort correctly; selecting multiple files disables Image Sequence.
- **Extended vs Standard**: 3D/video open is Extended; CS6 adds some audio/video
  import to Standard but the video-layer model remains Extended-driven.
- **Huge files**: stream/tile; do not load a multi-GB source into one allocation.
- **GPU unavailable**: no open-path impact.

## Parity acceptance criteria

- Given `File > New` with Width/Height/Resolution/Mode/depth set, the opened
  document reports exactly those dimensions, mode, depth, and resolution, and the
  named canvas color is honored (Transparent ⇒ one transparent layer).
- Given a copied selection on the clipboard, `File > New` preloads the clipboard
  image's dimensions and resolution.
- Given the bottom of the **Preset** menu, choosing an open document matches its
  width, height, resolution, mode, and bit depth exactly.
- Given the CS6 New dialog, there is no artboard creation control (artboard-less),
  consistent with the artboard open question.
- Given a `.gif` file whose content is actually a PSD, `File > Open` fails the
  extension but signature detection opens it correctly; with detection disabled,
  `Open As > Photoshop` opens it.
- Given a generic multi-page PDF, `File > Open` shows Import PDF; selecting Pages
  2–3 with Crop To = Media Box and resolution R produces two documents with the
  correct dimensions and R ppi.
- Given a Photoshop PDF (Preserve Photoshop Editing Capabilities on), `File > Open`
  does not show Import PDF.
- Given an EPS with vector art, opening at dimensions D, resolution R, mode M with
  Anti-aliased on rasterizes to D at R ppi in mode M with smoothed edges.
- Given a camera raw file, `File > Open` routes through Camera Raw; `Shift`-open
  bypasses it; `Shift`+Open Image yields a Smart Object that reopens Camera Raw on
  double-click.
- Given `File > Open As Smart Object` on a JPEG, the document has a single Smart
  Object layer whose Edit Contents reopens the source.
- Given a folder of sequentially named images, opening one with Image Sequence and
  a frame rate F creates a video layer with one frame per file at F (Extended).
- Given a web or unsupported format whose plug-in is absent, the Open dialog/Import
  submenu omits it and Open reports a clear error.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the
  CS6 Help corpus, downloaded and text-extracted for this pass. Established: the
  full **Create an image** New-dialog procedure (name, Preset, Size, width/height,
  Resolution, Color Mode, bit depth, White/Background Color/Transparent, Advanced
  color profile and Don't Color Manage, Pixel Aspect Ratio, Save Preset, Device
  Central, clipboard-driven dimensions, match-open-document preset entry); the
  **Duplicate an image** procedure; **Open files** including the profile warning
  choices, Open Recent and the Recent File List Contains preference, the plug-in
  note, and the cross-OS mislabeling case; **Open As** (Windows) and
  Open + All Documents + Format (macOS); the **Open PDF files** Import PDF dialog
  in full (Select Pages/Images, thumbnails, Name, Crop To's six boxes, Image Size
  Width/Height/Constrain Proportions, Resolution, Mode, Bit Depth, Suppress
  Warning) and the Photoshop-PDF exemption; **Open an EPS file** (dimensions,
  resolution, mode, Constrain Proportions, Anti-aliased); `File > Open As Smart
  Object` and `File > Place` behavior; the CS6 **Supported file formats** list with
  the asterisked CS6-new audio (AAC, AIFF, M2A, M4A, MP2, MP3) and video
  (`.264`/AVC, 3GP/3GPP, F4V, FLV, MPE, MTS, MXF, R3D, TS, VOB) import formats and
  video export formats (DPX, MOV, MP4); the Camera Raw open/save entry points
  (`File > Open`, Shift-click bypass, Shift+Open Image as Smart Object, JPEG/TIFF
  Handling, 65,000 px / 512 MP limit, CMYK→RGB), and Basic vs Extended edition
  scoping; **Import video files and image sequences** (File > Open or
  Layer > Video Layers > New Video Layer From File, the Image Sequence option and
  single-file rule, frame-rate prompt, sequential-naming guidance) and
  **Place a video or image sequence** (Smart Object wrapping); the `File > Import`
  scanner/WIA mention.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+New+Document+dialog+options+artboard+recent`
  and
  `https://html.duckduckgo.com/html/?q=Photoshop+CS6+artboards+New+document+dialog+introduced`
  — search-result pages (results seen as snippets, not individually fetched).
  They surfaced `https://community.adobe.com/t5/photoshop-ecosystem-discussions/how-to-put-artboards-in-photoshop-cs6/m-p/10833212`
  and `https://edex.adobe.com/teaching-resources/artboards-photoshop-cc-2015-s-quot-killer-quot-new-feature`,
  both placing Photoshop artboards in CC 2015 and not in CS6; the CS6 Help PDF
  itself mentions "Artboard" only as Illustrator-only Save for Web controls. Used
  for the artboard-less verification.

Internal cross-references (not external URLs): `docs/01-architecture/file-formats.md`
(the read/write matrix and detection strategy), `docs/05-layers/smart-objects.md`
(`LAY-020`), `docs/GLOSSARY.md`.

## Open questions

- **Artboards vs. the repository glossary.** `docs/GLOSSARY.md` describes
  "Artboard (CS6)" as introduced in CS6, but the CS6 Help PDF does not document
  Photoshop artboards and community sources place them in CC 2015. Resolve before
  any artboard-dependent behavior; this affects the "artboard-less New dialog"
  claim and `05-layers/artboards.md`.
- **New dialog "recent" list.** The task brief asked to verify a "recent" list in
  the CS6 New dialog. The CS6 Help procedure lists a Preset menu but no Recent
  section; the Recent tab in the New Document dialog is a later (CC 2015+)
  redesign. Confirm against a shipped CS6 build. Resolve by screenshot/behavior
  test.
- **New-dialog defaults.** Default preset, resolution (72 ppi assumed), color mode,
  and background color are not fully stated in the Help text. Resolve from a
  default CS6 preferences/profile.
- **`Open As` shortcut and menu availability.** Whether CS6 exposes a dedicated
  shortcut and how it appears on macOS beyond All Documents + Format is
  unconfirmed.
- **`File > Import` exact submenu.** The Help text references the Import submenu
  and WIA but does not enumerate every CS6 entry (for example annotations). Resolve
  from a shipped menu tree.
- **Camera Raw JPEG/TIFF Handling default.** The default of the Automatic/Disable/
  With-settings choice is unstated. Resolve from Camera Raw preferences.
- **Video-layer data model.** CS6 Help says a video layer references the original
  file but does not give the exact embedding/relink semantics; coordinate with
  `02-ui-ux/panels/timeline-panel.md` and `LAY-020`.
- **Open As Smart Object source carriage.** Whether the source is embedded whole
  (SoLd/`lnk2`-style) for every input or re-rendered is `LAY-020`'s open question.
- **Qt6 details.** Proposed dialog/model classes are sketches; verify against Qt6
  current docs before implementation.
