# Bridge and Interop

- **Spec ID:** `WF-023`
- **Status:** `Draft`
- **Parity tier:** `Core` for file interop (PDF/EPS/AI import via Place, Smart Objects); **`Non-goal (Linux)`** for Adobe Bridge, Mini Bridge, Adobe Stock, and Flash-based integrations, with native replacements.
- **New in CS6:** `Changed` — Bridge is referenced as Bridge CS5 in the reused Help text; Mini Bridge is present but degraded (the removed Application bar took its launch button, see `AUTO-014`); CS6 adds `File > Open As Smart Object` / linked Smart Objects and restores PDF Presentation as an Automate option; Adobe Stock and Flash panels are **not** CS6 features.
- **Depends on:** `AUTO-014` mini-bridge, `AUTO-012` plugin-sdk, `AUTO-003` batch-processing, `05-layers/smart-objects.md`, `05-layers/linked-and-embedded-objects.md`, `01-architecture/file-formats.md`, `10-workflow-io/open-and-new.md`, `10-workflow-io/export-formats.md`, `00-overview/feasibility-and-non-goals.md`, `00-overview/licensing-and-provenance.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the
> behavioural contract for the feature, and `docs/dev/doc-code-parity-gaps.md`
> records what shipped and what did not. Facts confirmed by the fetched CS6
> Help reference are stated plainly; secondary/community facts are marked
> *(secondary)*; inferred design choices are marked *(inferred)*.

## CS6 behavior

### Adobe Bridge integration

**Adobe Bridge** is a separate Creative Suite application for browsing,
organizing, and batch-processing assets; Photoshop integrates with it rather
than embedding it. Documented touchpoints:

- `File > Browse in Bridge` opens Bridge; the keyboard shortcut
  `Shift+Ctrl+W` / `Shift+Command+W` is documented as "Close a file in Photoshop
  and open Bridge."
- Opening files **from Bridge** into Photoshop: select in Bridge and use its
  `File > Open With > Adobe Photoshop`, `File > Place > In Photoshop` (imports
  as a Smart Object into an open document), or `Tools > Photoshop > Merge To HDR
  Pro`.
- **Camera Raw from Bridge**: `Ctrl+R` / `Command+R` opens the selected images
  in the Camera Raw dialog; `Shift`+double-click bypasses it.
- **Get Photos From Camera** is a Bridge feature (`Use the Get Photos From
  Camera command in … Bridge … to download photos, and to organize, rename, and
  apply metadata`).
- Bridge carries other Suite workflows (watermarking, web galleries, batch
  processing) that are Bridge features, not Photoshop features.

Bridge is a Windows/macOS Creative Suite application with no Linux build; none
of its transport is required for Photoshop's documented file operations.

### Mini Bridge

A Photoshop **extension panel** (`Window > Extensions > Mini Bridge`) that
embeds a small Bridge browser. It is a Flash/CEP 4 panel depending on Bridge and
the proprietary SwitchBoard transport, was degraded in CS6, and was removed in
CC 2014. Documented and rationalized as a Linux non-goal in `AUTO-014`; the
native **Files panel** is the replacement.

### Illustrator interop

- **Export from Illustrator to PSD:** "You can retain (where possible) layers,
  masks, transparency, compound shapes, slices, image maps, and editable type
  when bringing your Illustrator art into Photoshop. In Illustrator, export the
  art in the Photoshop (PSD) file format. If your Illustrator art contains
  elements that Photoshop doesn't support, the appearance of the artwork is
  preserved, but the layers are merged and the artwork is rasterized."
- **Place:** `File > Place` imports PDF/AI as a **Smart Object** on a new layer
  (also `File > Open As Smart Object`). `File > Place` then move/scale/rotate.
- **Paste:** `Edit > Paste` from Illustrator offers **Paste As: Smart Object /
  Pixels / Paths / Shape Layer**. For the Paste dialog to appear, Illustrator's
  clipboard preferences must have **AICB** enabled.
- **Drag and drop:** dragging Illustrator vector objects into a Photoshop
  document creates a **vector Smart Object**; holding `Ctrl`/`Command` drops the
  vector as a **path**. Dragging a Photoshop layer into Illustrator works too.
- **Edit Contents:** a vector-PDF Smart Object opens in Illustrator for editing;
  saving updates all linked instances. `Export Contents` writes the original
  placed format (e.g. `AI`, `PDF`).
- **Paths out:** `File > Export > Paths to Illustrator` writes an Illustrator
  (`.ai`) file containing Photoshop paths.

### PDF interop

- **Open a PDF:** `File > Open` (or `File > Open With > Adobe Photoshop` from
  Bridge) shows the **Import PDF** dialog:
  - **Select:** Pages or Images; click/shift-click thumbnails; Thumbnail Size
    menu and *Fit Page*.
  - **Name:** base name for the new document(s); multiple pages open numbered.
  - **Crop To:** `Bounding Box`, `Media Box`, `Crop Box`, `Bleed Box`,
    `Trim Box`, `Art Box`.
  - **Image Size:** Width/Height with `Constrain Proportions`; multiple pages
    use the maximum width/height.
  - **Resolution, Mode, Bit Depth**; `Suppress Warning` for color profile
    warnings.
- **Place / Paste / drag:** PDF data can be placed on a separate layer as a
  Smart Object. Photoshop PDF is also a save format; `PDF Presentation` is an
  Automate option in CS6.
- Photoshop's Help calls PDF "the primary format for Adobe Illustrator and Adobe
  Acrobat."

### EPS interop

- `File > Open` an EPS: "When you open an EPS file containing vector art, it is
  rasterized." The open options are dimensions, resolution, mode, and an
  **Anti-aliased** toggle; EPS can also be brought in via Place/Paste/drag.

### Adobe Stock and Flash (non-goals)

- **Adobe Stock** is not present anywhere in the CS6 Help; the Stock service
  launched with Creative Cloud after CS6. Any Stock panel/browse integration is
  therefore out of CS6 parity scope.
- **Flash** appears in the CS6 file-format list only as **Flash 3D (export
  only)**; there is no Flash panel or Flash-based Photoshop integration in CS6
  beyond that and the (non-goal) Mini Bridge CEP panel. Flash itself is
  end-of-life and has no role on Linux.
- Other cloud/companion integrations in the CS6 What's New (Photoshop Touch,
  Kuler, Creative Cloud file transfer) are cloud services, excluded per
  `00-overview/feasibility-and-non-goals.md`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Browse in Bridge` | Menu | `Shift+Ctrl/Cmd+W` | CS6; non-goal, replaced by the Files panel / file manager. |
| `Window > Extensions > Mini Bridge` | Panel | — | CS6; non-goal (`AUTO-014`). |
| `File > Open As Smart Object` | Menu | — | Place a file as an embedded Smart Object. |
| `File > Place` | Menu | — | PDF/AI/PSD/TIFF/PSB as a Smart Object layer. |
| `Edit > Paste` (from Illustrator) | Dialog | `Ctrl/Cmd+V` | Paste As: Smart Object / Pixels / Paths / Shape Layer. |
| Drag and drop (Illustrator → Photoshop) | Interaction | `Ctrl`/`Cmd` = path | Vector Smart Object otherwise. |
| `Layer > Smart Objects > Edit Contents` | Menu | double-click layer | Opens Illustrator for vector PDF. |
| `Layer > Smart Objects > Replace Contents` | Menu | — | Keeps transforms/effects. |
| `Layer > Smart Objects > Export Contents` | Menu | — | Writes the original placed format. |
| Import PDF dialog | Dialog | — | Pages/Images, Crop To, Image Size, Resolution/Mode/Bit Depth. |
| Open EPS options | Dialog | — | Dimensions, resolution, mode, Anti-aliased. |
| `File > Export > Paths to Illustrator` | Menu | — | Writes a `.ai` file from paths. |
| `Window > Files` (proposed) | Panel | — | Native replacement for Bridge/Mini Bridge browsing (`AUTO-014`). |
| `File > Open With…` (proposed) | Menu | — | External application via desktop portal. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Bridge launch | action | — | `File > Browse in Bridge` | Non-goal on Linux. |
| Mini Bridge | panel | — | `Window > Extensions > Mini Bridge` | Non-goal (`AUTO-014`). |
| Place target | enum | Smart Object | Smart Object / Pixels / Paths / Shape Layer | Paste dialog (AIPDF source). |
| Smart Object link | enum | Embedded | Embedded / Linked | Linked Smart Objects (CS6). |
| Import PDF Select | enum | Pages | Pages / Images | Import PDF dialog. |
| Import PDF Crop To | enum | Bounding Box | Bounding Box / Media Box / Crop Box / Bleed Box / Trim Box / Art Box | Per PDF spec boxes. |
| Import PDF Constrain Proportions | bool | On | on / off | Multiple pages use max W/H. |
| Import PDF Resolution | ppi | source/file *(inferred)* | ≥1 | Rasterization resolution. |
| Import PDF Mode | enum | document default | Bitmap/Grayscale/RGB/CMYK/Lab | New-document color mode. |
| Import PDF Bit Depth | enum | 8 | 8 / 16 / 32 | New-document depth. |
| Import PDF Suppress Warning | bool | Off | on / off | Color-profile warnings. |
| Open EPS Anti-aliased | bool | On | on / off | Minimizes jagged vector edges. |
| Illustrator paste clipboard | requirement | — | AICB enabled | Needed for Paste As options. |
| External open handler | path/cmd | OS default | `xdg-open`/portal | Proposed Linux mechanism. |

## Algorithms & pipeline

### Placement and Smart Object linking

```text
place(path, doc):
    bytes = read(path)
    kind  = sniff(bytes)                 # PDF / AI / EPS / PSD / TIFF / raster
    node  = SmartObject {
        source:   Embedded(bytes) | Linked(path),
        format:   kind,
        transform: identity,
        filters:  [],
    }
    doc.add_layer(node)                  # one History state
```

- **Embedded** Smart Objects store the source bytes inside the PSD (PSB/PSD
  resource). **Linked** ones store a path + content hash and re-resolve on
  change; CS6 links instances so editing one updates all.
- **Edit Contents** for a *vector PDF* Smart Object requires an external editor
  in CS6 (Illustrator). On Linux the proposal is to edit via the user's chosen
  handler (see below) or to rasterize/edit internally when no handler exists.

### PDF rendering (proposal)

- Render PDF pages to raster for the `Open`/`Place`/`Import` paths. Qt6 provides
  `QPdfDocument`/`QPdfPageRenderer` (Qt PDF) for page rendering, search, and
  navigation; the `QImageIOHandler` plugin lets PDF be treated as a scalable
  image with `currentFrame` = page index.
- The Rust core consumes rendered page images and owns the Smart Object format
  mapping; the Qt layer owns the interactive picker (thumbnail grid).
- The PDF **box** concepts (Bounding/Media/Crop/Bleed/Trim/Art) are standard PDF
  page boxes; map directly onto the crop choice.

### EPS/AI rendering (proposal)

- EPS and legacy AI are PostScript-based; rasterization is the parity behavior
  (CS6 rasterizes EPS on open). **Ghostscript** is an interpreter for
  PostScript/PDF and can render/converts these formats; the proposal is an
  optional, sandboxed Ghostscript backend for EPS/AI rasterization and encap-
  sulated EPS preview handling, isolated behind a feature flag because of its
  license and attack surface.
- Modern Illustrator native `.ai` files are PDF-compatible; the PDF path handles
  them without Ghostscript.

### Bridge replacement (proposal)

- The native **Files panel** (`AUTO-014`) covers browse/preview/open/place,
  favorites, sorting/filtering, and drag-and-drop — the useful subset of Bridge
  and Mini Bridge. Batch rename, web galleries, and advanced metadata editing
  are Bridge features and are deferred to their own specs or declared non-goals
  (`00-overview/feasibility-and-non-goals.md`).
- `File > Browse in Bridge` becomes `File > Browse…` opening the OS/portal file
  dialog or the Files panel; opening/placing routes through the existing document
  commands.
- Opening in an external editor (the CS6 Illustrator round-trip) uses the
  freedesktop desktop-entry/portal association (`QDesktopServices::openUrl` or
  `xdg-open`), i.e. whatever the user has registered for `.ai`/`.pdf`/`.eps`.

## Rust module mapping

Proposals.

- `pictura-interop::place` — `PlaceRequest`, `sniff`, Smart Object creation
  (`05-layers/smart-objects.md`).
- `pictura-interop::smart_object` — embedded vs linked source resolution,
  content hashing, update propagation to linked instances.
- `pictura-interop::pdf` — page-box mapping, page selection, raster request to
  the Qt renderer; `ImportPdfOptions`.
- `pictura-interop::eps` — optional Ghostscript-backed rasterizer behind a
  feature flag; `EpsOpenOptions { resolution, mode, bit_depth, anti_alias }`.
- `pictura-interop::external` — resolve a handler for a MIME type
  (`application/pdf`, `application/postscript`, `application/illustrator`) via
  the desktop portal; launch/open with a document URI.
- `pictura-interop::paths_export` — write a `.ai` file from vector paths
  (`File > Export > Paths to Illustrator`).
- **No `crate::bridge`:** Bridge, SwitchBoard, Mini Bridge, Adobe Stock, and
  Flash are explicitly absent.

Crossing types: `PlaceRequest`, `SmartObjectSource::{Embedded,Linked}`,
`ImportPdfOptions`, `EpsOpenOptions`, `PageBox`, `ExternalHandler`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `FilesPanel` | `QDockWidget` | Native browse/preview/open/place (`AUTO-014`), the Bridge/Mini Bridge replacement. |
| `ImportPdfDialog` | `QDialog` | Pages/Images selection, thumbnail grid, Crop To, Image Size, Resolution/Mode/Bit Depth. |
| `PdfPageModel` | `QAbstractListModel` | Page thumbnails via Qt PDF (`QPdfDocument`). |
| `OpenEpsDialog` | `QDialog` | Dimensions, resolution, mode, Anti-aliased. |
| `PasteAsDialog` | `QDialog` | Smart Object / Pixels / Paths / Shape Layer. |
| `PlaceAsSmartObjectDialog` | `QDialog` | Embedded vs Linked; target document. |
| `ExternalOpenService` | `QObject` | `QDesktopServices::openUrl(…)` / portal launch; MIME→handler resolution. |
| `BridgeAction` | `QAction` | `File > Browse in Bridge` mapped to the Files panel / file dialog, marked non-goal in the menu. |

Widgets, not QML (`ARCH-003`). Qt PDF (`QPdfDocument`, `QPdfPageRenderer`)
handles PDF rasterization in the UI layer when available; the Rust core stays
free of Qt types. The external-open path uses the desktop portal, respecting the
Flatpak/sandbox policy (`11-cross-cutting/security-and-sandboxing.md`).

## Data-model impact

- **Smart Object nodes:** a `SmartObject` layer holds `source` (embedded bytes
  or a linked path + hash), `format`, `transform`, and non-destructive filters;
  linking is a document-model relationship (`05-layers/linked-and-embedded-objects.md`).
- **PSD/PSB keys:** embedded sources serialize into the existing Smart Object
  resources; linked paths serialize as PSD linked-file placeholders per the file
  format spec (`01-architecture/file-formats.md`).
- **XMP:** no new metadata is required beyond standard document metadata.
- **Undo:** placing a file, replacing contents, or committing an Illustrator
  edit is one History state in the parent document; a nested Smart Object
  document keeps its own history (`ARCH-009`).
- **No Bridge/Stock/Flash state** enters the document model.

## Edge cases

- **No handler for `.ai`/`.eps`:** offer rasterize-only placement or an explicit
  error; never stall.
- **Linked source moved/deleted:** mark the Smart Object as missing, keep the
  last-known raster, and offer relink; matching linked instances update together.
- **Ghostscript absent or disabled:** EPS/AI PostScript rasterization is
  unavailable; PDF-based `.ai` and placed PSD/TIFF still work.
- **Multi-page PDF:** page selection, numbering, and maximum width/height for
  constrained sizing; large PDFs stream page-by-page and stay cancellable.
- **Huge PDF/EPS pages:** rasterize at a preview resolution first, then at the
  requested settings; cancel leaves the document unchanged.
- **Color profile warnings:** honor `Suppress Warning` and route through the
  color-management layer (`ARCH-007`).
- **Sandboxed/portal mounts:** opening a file outside the sandbox requires a
  portal grant; surface the CS6-style permission failure, do not crash.
- **PDF across box types:** Bounding Box may still include a source background
  (CS6 documents this); do not attempt to strip it.
- **CMYK/spot/vector PDF:** preserve vector data when placed as a Smart Object;
  rasterize only at output.
- **Filename/path with non-ASCII or very long paths:** handle correctly; elide
  only in the UI.
- **Adobe Stock/Flash menu entries:** present only as documented non-goals with
  a pointer to the replacement, never a half-working stub (`AUTO-014`).
- **External app edits on a linked source:** detect mtime/hash change and offer
  to update or relink the instances.

## Parity acceptance criteria

1. Given `File > Place` on a PDF/AI/PSD/TIFF/PSB file, it appears as a Smart
   Object layer on a new layer, movable/scalable without rasterizing.
2. Given the Import PDF dialog, Pages/Images selection, Crop To (all six boxes),
   Constrain Proportions, Resolution, Mode, Bit Depth, and Suppress Warning
   behave as described.
3. Given an EPS file, opening it rasterizes with the dimensions/resolution/mode
   and Anti-aliased option.
4. Given Illustrator artwork pasted with AICB enabled, the Paste As options
   Smart Object / Pixels / Paths / Shape Layer are offered and produce the
   corresponding node types.
5. Given a vector Smart Object, `Edit Contents` opens it in the user's
   registered editor (or reports that none exists); `Export Contents` writes the
   original format (`AI`/`PDF`).
6. Given a linked Smart Object and an edit to its source, all linked instances
   update to match.
7. Given `File > Export > Paths to Illustrator`, the written `.ai` contains the
   document paths.
8. Given no Adobe Bridge / Mini Bridge / Adobe Stock / Flash integration is
   present, the `Window > Files` panel and `File > Browse…` provide browse,
   preview, open, and place, and the excluded entries are documented as
   non-goals.
9. Given a linked source that is missing, the Smart Object reports the broken
   link and offers relink without corrupting the document.
10. Given `PDF Presentation` (Automate), a multi-page PDF is produced from the
    selected documents, matching `AUTO-003` conventions.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help (downloaded with `curl`, text-extracted with
  `pdftotext`). Establishes: `File > Browse in Bridge` and the
  `Shift+Ctrl/Cmd+W` shortcut; opening from Bridge; `File > Place > In
  Photoshop`; `Tools > Photoshop > Merge To HDR Pro`; Camera Raw from Bridge
  (`Ctrl+R`, `Shift`+double-click); `Get Photos From Camera` in Bridge;
  Illustrator→PSD layer retention or rasterization; Smart Object creation by
  Open As Smart Object, Place, and Illustrator paste; drag-and-drop vector
  objects and `Ctrl`/`Command` path drop; Edit/Replace/Export Contents; the
  Paste As options and AICB requirement; the Import PDF dialog (Pages/Images,
  Crop To's six boxes, Image Size, Resolution, Mode, Bit Depth, Suppress
  Warning); PDF as Illustrator/Acrobat's primary format; EPS rasterization and
  the Anti-aliased option; `File > Export > Paths to Illustrator`; PDF
  Presentation; Flash 3D as export-only; Mini Bridge
  (`Window > Extensions > Mini Bridge`).
- `https://doc.qt.io/qt-6/qtpdf-index.html` — Qt PDF module: `QPdfDocument`,
  `QPdfPageRenderer`, `QPdfPageNavigator`, `QPdfSearchModel`, `QPdfView`, and
  the PDF-as-scalable-image `QImageIOHandler`; basis for the PDF rendering
  proposal.
- `https://www.ghostscript.com/` — Ghostscript as a PostScript/PDF interpreter
  and rasterizer (`Ghostscript consists of a PostScript interpreter layer and a
  graphics library`); basis for the optional EPS/AI rasterization backend.
- `https://doc.qt.io/qt-6/qsettings.html` — used only for the Linux storage
  context around external-handler preferences (shared with `WF-020`/`WF-021`).
- Cross-references: `docs/09-automation/mini-bridge.md` (`AUTO-014`, the native
  Files panel replacement and the Bridge/Flash non-goal rationale),
  `docs/05-layers/smart-objects.md`, `docs/05-layers/linked-and-embedded-objects.md`,
  `docs/01-architecture/file-formats.md`, `docs/00-overview/feasibility-and-non-goals.md`.

## Open questions

- **CS6 Illustrator live-editing.** Whether `Edit Contents` always requires a
  matching Illustrator version, and what happens when none is installed, is not
  fully documented. *Resolves with:* a CS6 test with/without Illustrator.
- **Import PDF defaults.** The default Resolution/Mode/Bit Depth of the Import
  PDF dialog are not stated. *Resolves with:* a CS6 dialog capture.
- **AI format split.** Which `.ai` files are PDF-compatible vs legacy
  PostScript, and how Photoshop decides, affects whether Ghostscript is needed.
  *Resolves with:* a corpus test.
- **EPS/AI rasterization fidelity.** Ghostscript's output vs CS6's PostScript
  rasterizer differs (anti-aliasing, color management, overprint); how close
  parity must be is a product decision. *Resolves with:* a paired-render
  comparison.
- **Bridge replacement scope.** Whether batch rename, web gallery, and full
  metadata editing belong in the Files panel or a separate spec. *Resolves
  with:* `AUTO-014`'s relationship note and
  `10-workflow-io/file-info-and-metadata.md`.
- **PDF Presentation parity.** Its exact options (page order, transitions) and
  relationship to `AUTO-003` Batch are unspecified. *Resolves with:* the CS6
  Help PDF Presentation section and a CS6 run.
- **External-open mechanism under Flatpak.** Whether to use the desktop portal
  only or allow a configurable command. *Resolves with:*
  `11-cross-cutting/security-and-sandboxing.md`.
- **Smart Object source fidelity.** Whether Kooka Pictura embeds the original
  bytes verbatim (CS6 behavior) or normalizes formats, and the implications for
  PSD round-trip. *Resolves with:* `01-architecture/file-formats.md`.
