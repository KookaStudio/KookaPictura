# Smart Objects

- **Spec ID:** `LAY-020`
- **Status:** `Draft`
- **Parity tier:** `Core` (embedded smart objects, transforms, contents editing); `Extended-only` (image-stack Stack Modes)
- **New in CS6:** `Changed` — the Smart Object command set is unchanged from CS5, but CS6 adds new smart-filter-capable filters (**Blur Gallery**, **Oil Paint**) and the JDI note that Smart Filter settings are shared across layer comps. Help also states that **Liquify** became Smart-Object-compatible in a later Creative Cloud update, not in the shipped CS6 build. See `## Open questions`.
- **Depends on:** `ARCH-008` document-model, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `TOOL-001` move-and-transform, `LAY-021` smart-filters, `LAY-022` layer-comps, `LAY-024` linked-and-embedded-objects, `10-workflow-io/open-and-new.md`, `01-architecture/file-formats.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*.

## CS6 behavior

A **Smart Object** is a layer that contains source image data from a raster or vector image (a Photoshop, Illustrator, PDF, TIFF, JPEG, or EPS file), preserving the source content with all its original characteristics so the layer can be edited non-destructively. A Smart Object is recognisable in the Layers panel by the badge icon in the lower-right corner of its thumbnail.

### Creating Smart Objects

| Method | Result |
|---|---|
| `File > Open As Smart Object`, pick a file, `Open` | New document whose sole layer is a Smart Object containing that file |
| `File > Place` in an open document | File is imported as a Smart Object; it is transformed interactively before commit |
| `Layer > Smart Objects > Convert to Smart Object` (also written `Layer > Smart Object > Convert to Smart Object`) | Selected layer(s) are bundled into one Smart Object |
| `Edit > Preferences > General > Place Or Drag Raster Images As Smart Objects` | When on (default), a file dragged onto an open document or an image copied between documents is placed as a Smart Object; when off it becomes a standard raster layer |
| Bridge `File > Place > In Photoshop` | File is imported as a Smart Object |
| Drag PDF/Illustrator layers/objects into the document | Placed as a Smart Object |
| Paste from Illustrator, choose `Smart Object` in the Paste dialog | Vector artwork stays vector instead of rasterising; requires Illustrator PDF + AICB (No Transparency Support) clipboard preferences |

Help recommends placing **PSD, TIFF, or PSB** rather than JPEG, because those can be re-edited and re-saved without loss; re-saving a modified JPEG forces a flatten and a lossy re-compress. A multilayer source placed into a document appears as a **flattened** version on the new layer; to copy separate layers, duplicate them in the source image instead.

### Duplicating

| Command | Behaviour |
|---|---|
| `Layer > New > Layer Via Copy`, or drag the layer to the New Layer icon | Duplicate is **linked** to the original: edits to either affect the other |
| `Layer > Smart Objects > New Smart Object Via Copy` | Duplicate is **independent**: edits to the original do not affect the copy |

The new layer keeps the original name with a `copy` suffix.

### Editing, replacing, exporting, rasterising

- **Edit contents** — `Layer > Smart Objects > Edit Contents`, or double-click the layer thumbnail. Raster and camera-raw content opens in Photoshop; vector PDF content opens in Illustrator. When the source is saved, all linked instances in the Photoshop document update.
- **Replace contents** — `Layer > Smart Objects > Replace Contents`, navigate to the replacement, `Place`, `OK`. Scaling, warping, and effects on the existing Smart Object are preserved. Multiple linked instances, if any, update too. This is the documented route for swapping a low-resolution placeholder for a final version.
- **Export contents** — `Layer > Smart Objects > Export Contents`, choose a folder, `Save`. The content is written in its original placed format (JPEG, AI, TIF, PDF, …). A Smart Object originally created from layers is exported as **PSB**.
- **Rasterise** — `Layer > Rasterize > Smart Object` (also `Layer > Smart Objects > Rasterize` in the image-stack topic). This flattens the content at its **current size**; transforms, warps, and filters are no longer editable afterward and are not retained on a re-created Smart Object.

### Non-destructive transform

A Smart Object can be scaled, rotated, skewed, distorted, perspective-transformed, and warped without altering the original data. The Free Transform **Interpolation** pop-up in the options bar does not apply to Smart Objects — they use the `Edit > Preferences > General` default (see `TOOL-001`). While a Smart Object carrying Smart Filters is being transformed, Photoshop **turns off the filter effects**, then re-applies them after the transform completes.

### Stack Modes (Photoshop Extended)

An **image stack** is a multi-layer document combined into one Smart Object and then rendered with a **stack mode**. Stack rendering is non-destructive and non-cumulative: each mode operates on the original image data and replaces the previous render.

- Setup: combine the images as layers (manually or `File > Scripts > Load Files into Stack`), `Select > All Layers`, `Edit > Auto-Align Layers` (usually `Auto` or `Reposition`), `Layer > Smart Objects > Convert to Smart Object`, then `Layer > Smart Objects > Stack Mode`.
- A stack must contain **at least two layers**.
- Modes (Help's own table): `Entropy`, `Kurtosis`, `Maximum`, `Mean`, `Median`, `Minimum`, `Range`, `Skewness`, `Standard Deviation`, `Summation`, `Variance`, and `None`. `Mean`/`Median` are the documented choices for noise reduction; `Median` also removes unwanted content.
- Stack modes operate **per channel** and only on **non-transparent** pixels.
- `Layer > Smart Objects > Stack Mode > None` removes rendering and reverts to a regular Smart Object.
- `File > Scripts > Statistics` automates the same pipeline (choose a stack mode, optionally auto-align).

### Layer-comps interaction

Layer comps record layer visibility, position, and appearance, but **Smart Filter settings cannot differ between layer comps**: once a Smart Filter is applied to a layer it appears in every comp of the image (see `LAY-022`). Smart Objects themselves otherwise behave as ordinary layers for comps (visibility/position/appearance are captured per layer).

### Placed vector formats and PSD/PSB/TIFF carriage

- `File > Place` and `Open As Smart Object` accept raster and vector sources. A **PDF or AI** file placed as a Smart Object keeps its vector data, so it stays resolution-independent and can be opened in Illustrator via Edit Contents. `File > Open` on a generic PDF, by contrast, opens a rasterised document and offers an Import PDF dialog (Pages/Images, Crop To box, Width/Height, Resolution, Mode, Bit Depth).
- **EPS** placed/opened is PostScript and is rasterised (with an Anti-aliased option); it does not remain editable vector in Photoshop.
- A placed **PSD/PSB/TIFF** is embedded whole and can be re-edited without flattening the host document; a placed multilayer file appears flattened on the layer unless the source layers are duplicated in.
- In PSD, the Smart Object content is carried as a **Placed Layer Data** block (`SoLd`, CS3+, replacing the older `plLd`); the object's transform/warp and page selection live in the accompanying descriptor. The embedded source file bytes are held in the same placed-layer structure. See `## Data-model impact` and `LAY-024`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Open As Smart Object` | Menu | — | Opens a file as a one-layer Smart Object document |
| `File > Place` | Menu | — | Imports a file as a Smart Object into the current document |
| `File > Scripts > Load Files into Stack` | Menu | — | One image per layer for an image stack |
| `File > Scripts > Statistics` | Menu | — | Automated image stack + stack mode |
| `Layer > Smart Objects > Convert to Smart Object` | Menu | — | Bundles one or more layers |
| `Layer > Smart Objects > New Smart Object Via Copy` | Menu | — | Independent duplicate |
| `Layer > Smart Objects > Edit Contents` | Menu | — | Opens the source (Photoshop or Illustrator) |
| `Layer > Smart Objects > Replace Contents` | Menu | — | Swaps source, keeps transform/effects |
| `Layer > Smart Objects > Export Contents` | Menu | — | Writes source in its original format |
| `Layer > Smart Objects > Stack Mode` | Submenu | — | Extended-only: None + 11 render modes |
| `Layer > Rasterize > Smart Object` | Menu | — | Flattens at current size |
| Layers panel — thumbnail double-click | Gesture | `Double-click` | Edit contents |
| Layers panel — badge icon | Indicator | — | Lower-right of thumbnail marks a Smart Object |
| Layers panel — visibility column | Toggle | `Click` | Hide/show the Smart Object as a whole |
| `Edit > Preferences > General` | Preference | — | `Place Or Drag Raster Images As Smart Objects`; `Image Interpolation` default |
| Layers panel context menu | Context menu | `Right-click` | Same Smart Object commands (CC-era also adds Convert to/Embed Linked — not CS6) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| `Place Or Drag Raster Images As Smart Objects` | bool | on | on / off | General preference; governs drag/paste behaviour |
| `Image Interpolation` | enum | Bicubic | Nearest Neighbor / Bilinear / Bicubic / Bicubic Smoother / Bicubic Sharper | Preference default; Smart Object transforms ignore the options bar (cf. `TOOL-001`) |
| Stack Mode | enum | None | None, Entropy, Kurtosis, Maximum, Mean, Median, Minimum, Range, Skewness, Standard Deviation, Summation, Variance | Extended-only; per-channel, non-transparent pixels |
| Minimum layers for a stack | int | 2 | ≥ 2 | Help states the requirement |
| Placed layer type | enum | — | 0 unknown, 1 vector, 2 raster, 3 image stack | From the PSD `plLd` record, *(sourced)* |
| Page number / total pages | int / int | 1 / 1 | ≥ 1 | Multi-page PDF/AI placement (`plLd`) |
| Anti-alias policy | int | — | — | `plLd` field, exact encoding not sourced |
| Warp version | int | 0 | 0 | `plLd` field |
| Transform | 8 × double | identity | free | `plLd` x/y for four transform points |
| Smart-filter Blend Mode | enum | Normal | 27 CS6 layer blend modes | Per Smart Filter; see `LAY-021` |
| Smart-filter Opacity | int percent | 100 | 0–100 | Per Smart Filter |

## Algorithms & pipeline

### Object model

A Smart Object is a layer whose pixel contribution is produced by rendering an **embedded source document** through a stored affine/projective transform and the Smart Filter stack. The host document holds:

```text
SmartObject {
  source: EmbeddedDocument { psd_or_bytes, original_format, page, total_pages }
        | LinkedDocument   // post-CS6; see LAY-024
  transform: Matrix,          // 4-point / matrix form in PSD
  warp: Option<WarpDescriptor>,
  placed_layer_type: Vector | Raster | ImageStack,
  stack_mode: Option<StackMode>,
  filters: SmartFilterStack,  // LAY-021
}
```

The render is **resolution-independent up to the source**: the object is resampled from the source at the current document resolution and transform, and Photoshop caches a raster proxy for interactive compositing. Rasterising bakes that proxy into a pixel layer at the current size.

*(inferred)* A lazy implementation renders the embedded document once into a cache keyed by `(source_revision, transform, doc_resolution, bit_depth)`; edits to the source or test-only changes to the transform invalidate the cache. Smart-filter parameters that are below the filter entry in the stack are part of the key only through the rendered source, because filters are applied after the object render.

### PSD placed-layer serialization

The file-format specification documents two related keys:

- **`plLd` — Placed Layer** (pre-CS3; key retained for reading). Fields: type `plcL`; version `3`; unique ID (Pascal string); page number; total pages; anti-alias policy; placed layer type (`0` unknown, `1` vector, `2` raster, `3` image stack); `4 × 8` doubles for the x/y of four transform points; warp version (`0`); warp descriptor version (`16`); variable warp descriptor.
- **`SoLd` — Placed Layer Data** (Photoshop CS3). Fields: identifier `soLD`; version `4`; descriptor version `16`; a variable **descriptor of placed-layer information**. `SoLd` replaces `plLd` on write.
- **`SoLE` — Smart Object Layer Data** is documented as **Photoshop CC 2015**, not CS6 (version 4 or 5, descriptor). It must not be relied on for CS6 round-trips.
- The Smart Filter mask and filter data are separate blocks (`FMsk`, `FXid`/`FEid`); see `LAY-021`.
- In PSB, `lnk2`, `FMsk`, `FEid`, `FXid`, `SoLd`-adjacent keys use 8-byte lengths (the spec's PSB list); see `ARCH-008`.

*(inferred)* Unknown descriptor fields inside `SoLd` must be preserved verbatim so a CS6-authored file round-trips even where the engine does not understand a placed-layer property.

### Stack-mode math

Stack modes are per-channel reductions over the non-transparent pixel set `S` for each pixel location, with `v` the channel value:

| Mode | Formula / definition |
|---|---|
| Maximum | `max(v)` over `S` |
| Minimum | `min(v)` over `S` |
| Mean | `sum(v) / |S|` |
| Median | middle value of `v` over `S` |
| Summation | `sum(v)` over `S` |
| Range | `max(v) - min(v)` |
| Variance | `sum((v − mean)²) / (|S| − 1)` |
| Standard Deviation | `sqrt(variance)` |
| Skewness | `sum((v − mean)³) / ((|S| − 1) · stddev³)` |
| Kurtosis | `sum((v − mean)⁴) / ((|S| − 1) · stddev⁴)` |
| Entropy | `− Σ p(v)·log₂ p(v)`, with `p(v) = occurrences(v) / |S|` |

These formulas are given verbatim in the CS6 Help "Stack modes" table. None of the modes are cumulative; each re-renders from the original data.

## Rust module mapping

Proposals, layered on the document model in `ARCH-008`:

- `pictura_core::smart::SmartObject` — `{ source: SmartSource, transform, warp, placed_type, stack_mode, filters: SmartFilterStack }`.
- `pictura_core::smart::SmartSource` — `Embedded { bytes, format, page, total_pages } | Linked { path, ... }` (linked variant is post-CS6, `LAY-024`).
- `pictura_core::smart::StackMode` — the 11 modes + `None`; `reduce_stack(pixels: &[ChannelPlane], mode) -> ChannelPlane`.
- `pictura_core::smart::render` — render the embedded document at a `(resolution, transform, depth)` key into a cached `TileStore`.
- `pictura_transform::Matrix` — reuse the affine/homography types from `TOOL-001`.
- `pictura_filter::smart` — Smart Filter stack evaluation (`LAY-021`).
- `pictura_io::psd::placed` — parse/write `plLd`/`SoLd` records and preserve the descriptor bytes.
- `pictura_io::vector` — rasterise placed PDF/AI/EPS/SVG at a target resolution (candidate crates: PDF via a renderer adapter, see `ARCH-011`).

Crossing types: `NodeId(u64)`, `SmartSourceId`, `Matrix`/`WarpDescriptor`, `StackMode`, and a `RenderedProxy { bounds, tiles, revision }` handle that Qt reads only as a thumbnail/overlay, not as per-pixel data.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SmartObjectPropertiesPage` | `QWidget` | Source summary (format, page, embedded/linked), "Edit/Replace/Export Contents", Rasterize |
| `StackModeSubMenu` | `QMenu` | Populates the 11 stack modes with the current one checked (Extended only) |
| `PlacementDialog` | `QDialog` | `File > Place` transform session: bounding box overlay, commit/cancel |
| `SourceRevisionWatcher` | `QFileSystemWatcher` | (post-CS6) link change detection for linked objects |
| `LayersModel` extension | `QAbstractItemModel` | Smart Object badge role + Smart Filter child rows (shared with `LAY-021`) |
| `SmartObjectThumbnailDelegate` | `QStyledItemDelegate` | Renders the lower-right badge and source-type glyph |

Widgets over QML for the panel/dialog controls (dense, docked, keyboard-centric) per `ARCH-003`; only the on-canvas transform overlay lives in the `QGraphicsView` vector layer.

## Data-model impact

- `NodeKind::SmartObject` (already sketched in `ARCH-008`) must carry the embedded source, transform matrix, warp descriptor, placed-layer type, optional warp descriptor, stack mode, and the Smart Filter stack.
- The embedded source is a **nested `Document`**; its history is separate from the host document's history. Opening it for editing opens a second document window (Help: edits to the source propagate to all linked instances).
- Undo: a `Place`/`Convert`/`Replace Contents`/`Rasterize`/stack-mode change is one command; `Rasterize` is destructive and needs a pixel backup (or keep the Smart Object data for lossless undo, which is a non-parity enhancement). Transform edits are non-destructive and undo by restoring the previous matrix/descriptor.
- Serialization: `plLd` (read), `SoLd` (write), warp descriptor, and — for linked objects — `lnk2`/`lnkD`/`lnk3` (`LAY-024`). Unknown placed-layer descriptor fields are preserved as opaque bytes.
- XMP: no dedicated Smart Object metadata; the source bytes inside the PSD carry their own metadata.
- Layer comps: visibility/position/appearance are captured, but the Smart Filter stack is not comp-varied (`LAY-022`).

## Edge cases

- **Painting filters**: painting, dodging, burning, cloning etc. cannot be applied directly to a Smart Object; they require editing the contents, adding a layer above, or rasterising.
- **Transform + Smart Filters**: filter effects are suppressed during transform and re-applied on commit; a GPU/compositor cache must invalidate correctly.
- **Rasterise size**: rasterising bakes at the current size; downscaling before rasterise permanently loses detail — warn as CS6 does not.
- **Multilayer source**: only the flattened appearance is shown; layer duplication is a separate workflow.
- **Vector PDF/AI**: keep editable; **EPS** is rasterised; **SVG** support is unconfirmed in CS6 and treated as *(inferred)*/open.
- **Image stack**: fewer than two layers is invalid; content should be co-registered; modes operate per channel on non-transparent pixels, so partially transparent stacks change the per-pixel set.
- **Bit depth / mode**: 8/16/32 and CMYK/Lab apply. A Smart Filter that does not support the image's mode/depth shows a warning icon (see `LAY-021`).
- **PSB / huge docs**: the embedded source can itself be PSB-sized; embedding must not force a full in-memory copy — stream/refcount the source bytes.
- **Background layer**: cannot be converted directly; must first be turned into a regular layer.
- **Video / 3D**: video layers can be Smart Objects and carry smart filters (Help's video notes); 3D is Extended-only and out of core scope.
- **Linked layer mask**: a layer mask may be linked or unlinked to the Smart Object independently (Help's mask note).
- **GPU unavailable**: rendering falls back to CPU; thumbnails and overlays must still work.

## Parity acceptance criteria

- Given a layer and `Layer > Smart Objects > Convert to Smart Object`, the layer becomes a Smart Object, its appearance is unchanged within tolerance, and `Rasterize > Smart Object` restores a pixel layer at the current size.
- Given a Smart Object, `Edit Contents` opens a source document; saving it updates every linked instance in the host document.
- Given two linked duplicates (`Layer Via Copy`) and one independent duplicate (`New Smart Object Via Copy`), editing the source changes the two linked copies but not the independent one.
- Given `Replace Contents`, the object's transform, warp, and filter stack are preserved against the previous object.
- Given `Export Contents`, a Smart Object created from a file exports in its original format; one created from layers exports as PSB.
- Given an image stack of ≥2 aligned layers and `Stack Mode > Median`, each output channel equals the per-pixel median of the non-transparent input values, channel by channel.
- Given `Stack Mode > Mean` then `Stack Mode > Maximum`, the second render is computed from the original stack, not from the Mean result (non-cumulative).
- Given a Smart Object with Smart Filters, starting a transform suppresses the filter render during the gesture and re-applies it on commit.
- Given a placed vector PDF/AI, scaling the Smart Object up does not show a hard raster resolution limit at the source's native resolution.
- Given a PSD authored by CS6 containing a Smart Object, Kooka Pictura opens and re-saves it without losing the placed-layer descriptor or unknown descriptor fields.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus. Established: definition of Smart Objects; the create/duplicate/edit/replace/export/rasterise workflows and their exact menu labels; `File > Place` and `Open As Smart Object` behaviour; JS: flattened multilayer placement; `New Smart Object Via Copy` independence vs `Layer Via Copy` linkage; Export Contents formats (PSB when created from layers); transform non-destructiveness and filter suppression during transform; layer-mask link/unlink; the `Place Or Drag Raster Images As Smart Objects` preference; the full **Stack Modes** table and formulas, the ≥2-layer rule, per-channel/non-transparent scope, `Statistics` script; the Layer Comps note that Smart Filter settings cannot vary across comps; the `Liquify`/Blur Gallery smart-filter notes (with the "Creative Cloud update" wording); `File > Open` PDF/EPS rasterising behaviour.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification. Established: `plLd` Placed Layer record fields (type `plcL`, version 3, page/total pages, anti-alias policy, placed layer type 0/1/2/3, 4×2 transform doubles, warp version, descriptor v16); `SoLd` Placed Layer Data (CS3, id `soLD`, version 4, descriptor v16); `SoLE` Smart Object Layer Data labelled **Photoshop CC 2015**; `FMsk` Filter Mask (CS3); PSB 8-byte key list including `lnk2`, `FMsk`, `FEid`, `FXid`.
- `https://bjango.com/articles/photoshopcc2014smartobjects/` — established that linked Smart Objects arrived in Photoshop **CC 14.2** (2014), that CC 2014 added **Convert to Linked**/**Embed Linked**, and that **Layer Comps inside Smart Objects** are a CC 2014 feature (both post-CS6).

Consulted as search-result snippets only (not individually fetched; community/Adobe-summary):

- `https://community.adobe.com/questions-712/how-to-put-artboards-in-photoshop-cs6-1086264` and `https://www.reddit.com/r/photoshop/comments/6tcw9b/artboards_in_cs6/` — artboards are not a CS6 feature (relevant to `LAY-023`).

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern Help pages were inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **Liquify Smart-Object support timing.** The fetched Help text says Liquify became Smart-Object-compatible in a later "Creative Cloud update", which contradicts the CS6 "excluded filters" list (Extract, Liquify, Pattern Maker, Vanishing Point). Resolve by testing a shipped CS6 build or a CS6-era Help snapshot: is Liquify smart-compatible in CS6.0 or only in CC?
- **`SoLd` descriptor field catalogue.** The exact inner descriptor keys (source ID, non-affine transform, warp, page, comp selection) are not enumerated in the fetched spec text. Resolve by parsing CS6-authored PSDs or the `psd-tools`/`PhotoshopAPI` structures.
- **`SoLE` in CS6.** The spec labels `SoLE` "Photoshop CC 2015"; confirm no CS6 file writes it and whether CS6 ignores it safely on read.
- **Rasterisation resolution and cache policy.** Whether Photoshop rasterises at source resolution, document resolution, or the transformed bounding box, and how the cache is invalidated, is not documented. Resolve with a resolution/sharpness comparison against CS6.
- **SVG placement.** Whether CS6 `File > Place` accepts SVG as a vector Smart Object is unconfirmed. Resolve from the CS6 support list or a CS6 test.
- **Image-stack alignment.** The exact `Auto-Align` algorithm used before stacking is a separate tool spec (`TOOL-001`/auto-align); confirm which modes produce CS6-identical stacks.
- **Layer comps + Smart Objects.** Help only states Smart Filter settings cannot vary across comps. Whether any other Smart Object property (e.g. Stack Mode, Replace Contents) is comp-scoped in CS6 is unverified.
