# Crop Tool

- **Spec ID:** `TOOL-011`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Crop tool was redesigned: an interactive preview with a crop box and handles is placed automatically, aspect-ratio and straighten controls moved to the options bar, cropping is non-destructive by default-available, and a Classic mode restores pre-CS6 behavior.
- **Depends on:** `TOOL-012` (Perspective Crop), `01-architecture/document-model.md` (`ARCH-002`), `01-architecture/undo-history.md`, `02-ui-ux/toolbox-and-options-bar.md`.

## CS6 behavior

Cropping removes portions of an image to strengthen composition. In CS6 the Crop
tool is a redesigned, interactive tool separate from the older CS5 workflow:

- Selecting the Crop tool places a crop box with eight handles (four L-shaped
  corner brackets and four thin edge bars, all drawn inside the box) on the
  image immediately; the user can also drag a new box. The
  area outside the box is shown through a **crop shield** (a tinted overlay), and
  the crop box carries an overlay guide (Rule of Thirds by default).
- The image moves inside the box when dragged; the box stays fixed (the
  opposite of CS5, where the box moved over a fixed image). Source: Photoshop
  Essentials CS6.
- **Size and proportions** come from the Aspect Ratio menu (presets, custom
  W/H, `Front Image`, `W x H x Resolution`). The double-arrow control swaps
  width and height (keyboard `X`), replacing CS5's "Rotate Crop Box" button.
- **Straighten**: either drag in the rotate band just outside the box to rotate
  the image behind the box (beyond the band the cursor is the default arrow and
  a click applies the crop), or use the **Straighten** control and draw a
  reference line along a feature that should be horizontal or vertical. The
  canvas is automatically resized to accommodate the rotated pixels. Source: CS6
  reference, "Straighten an image | CS6".
- **View** selects the overlay guide; `O` cycles overlays, `Shift+O` cycles
  orientation for triangular/spiral overlays.
- **Delete cropped pixels**: enabled by default, permanently discards pixels
  outside the box. When disabled, the crop is **non-destructive** — pixels are
  retained and hidden, and the crop can be re-edited later. Right-clicking the
  crop box exposes common crop options.
- **Use Classic mode** (Settings menu) makes the tool behave like earlier
  Photoshop versions.
- Committing is `Enter`/`Return`, a double-click inside the box, a click away from
  the box beyond the rotate band (on the canvas or the workspace past its edge),
  or the commit button; `Esc` cancels. A far click while only the preview is
  shown does nothing. Resetting the box is `Backspace`/`Delete`.
- The same tool slot also holds the CS6 **Perspective Crop** tool
  (`TOOL-012`) and, lower in the toolbox, the Slice tools.
- `Image > Crop` (selection-based) and `Image > Trim` remain as non-interactive
  alternatives.

### Non-destructive crop

The CS6 reference states: "The Crop tools in CS6 are non-destructive and you can
choose to retain the cropped pixels to optimize the crop boundaries later." With
**Delete Cropped Pixels** off, the retained pixels can be revealed by
`Image > Reveal All` or by dragging the Crop tool beyond the image edge, and the
crop can be re-opened by re-selecting the Crop tool. The Hide/keep behavior is
unavailable for images that contain only a Background layer; the Background must
first be converted to a regular layer.

### Smart objects

Widely reported (community, not the official CS6 reference): a crop applied to a
document containing a Smart Object does not change the Smart Object's embedded
source pixels; re-editing the Smart Object reveals its full, uncropped content.
The Photoshop community also reports that **Delete Cropped Pixels** is
unavailable/disabled for Smart Object and locked Background layers. Both
behaviors are marked inferred and tracked under `## Open questions`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox | Tool (`C` group) | `C` | Cycles Crop / Perspective Crop / Slice / Slice Select |
| Options bar | Aspect Ratio menu | — | Presets, custom W/H, `Front Image`, `W x H x Resolution` |
| Options bar | Width / Height fields | — | Values cleared by Clear |
| Options bar | Resolution field | — | Shown only for `W x H x Resolution` |
| Options bar | Swap Width/Height | `X` | Replaces CS5 Rotate Crop Box |
| Options bar | Straighten toggle | `Ctrl`/`Cmd` invoke | Then draw reference line |
| Options bar | View (overlay) menu | `O` | Rule of Thirds, Grid, Golden Ratio, … |
| Options bar | Settings menu | — | Classic mode, Auto-center preview, Show Cropped Area, crop shield |
| Options bar | Delete Cropped Pixels | — | Checkbox, on by default |
| Options bar | Clear / Reset / Cancel / Commit | `Backspace`/`Delete`, `Esc`, `Enter` | |
| Context menu | Right-click crop box | — | Common crop options |
| Menu | `Image > Crop` | — | Selection-based crop |
| Menu | `Image > Trim` | — | Transparent/edge-color trim |
| Menu | `Image > Reveal All` | — | Restores non-destructively cropped pixels |
| Toolbox slot | Perspective Crop | `Shift+C` | See `TOOL-012` |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Aspect Ratio | Enum + numeric | `Unconstrained` | Unconstrained, Original Ratio, Square, common print ratios, custom W x H, `W x H x Resolution`, `Front Image` | Exact preset list version-dependent; see Open questions |
| Width (`W`) | Number | Empty | > 0 | Aspect-ratio value or px with resolution |
| Height (`H`) | Number | Empty | > 0 | |
| Resolution | Number | Empty | > 0 ppi | Appears for `W x H x Resolution` |
| Swap W/H | Action | — | — | Shortcut `X` |
| Straighten | Toggle + reference line | Off | Two points | Canvas auto-resized |
| Crop rotation | Angle | 0° | Continuous; `Shift` constrains to 15° | `Ctrl` prevents box shrink |
| Overlay | Enum | Rule of Thirds | Rule of Thirds, Grid, Golden Ratio, Diagonal, Triangle, Golden Spiral | Exact catalog version-dependent |
| Overlay show mode | Enum | Auto | Always / Auto / Never | |
| Use Classic mode | Bool | Off | — | Pre-CS6 behavior; key `P` |
| Auto-center preview | Bool | On | — | Centers preview on canvas (Modern mode) |
| Show Cropped Area | Bool | On | — | Shows retained outside area (Modern mode); key `H` |
| Enable crop shield | Bool | On | — | Tint over cropped region |
| Shield color / opacity | Color / percent | — | 0–100% | |
| Auto Adjust Opacity | Bool | — | — | Reduces opacity while editing |
| Delete Cropped Pixels | Bool | On | — | Off = non-destructive |
| Resampling interpolation | Enum | General-preference default (Bicubic) | Per General preferences | Only when W/H/resolution set |

## Algorithms & pipeline

- **Crop geometry** is a rectangle (plus optional rotation) in document space.
  Commit either (a) copies/crops the retained region into a new canvas, or
  (b) records a non-destructive crop region and hides the remainder.
- **Non-destructive crop** is behavioral parity only: retain the full layer
  raster, store the active crop rectangle (and rotation) as layer/canvas
  metadata, and clip rendering to it. Re-editing reuses the stored region;
  `Reveal All` clears it. Exact CS6 storage location/keys are not public — see
  Open questions.
- **Straighten** computes the angle of the reference line from the horizontal or
  vertical, rotates the image by its negative about the image center, and grows
  the canvas to the bounding box of the rotated image, then crops to the box.
  Standard rotation + resampling; matches the documented behavior.
- **Resampling** when W/H/resolution are specified uses the interpolation
  method selected in General preferences (Bicubic by default). This is
  documented.
- **Classic mode** reuses the legacy Crop tool state machine (box drag/rotate,
  fixed image).
- Behavioral parity only, algorithm TBD where Adobe's internals are closed
  (overlay catalog, shield compositing, exact resampling kernel).

## Rust module mapping

Proposed (names provisional), following `ARCH-002`:

- `pictura-core::tools::crop::CropState` — active box (`CropRect`), rotation,
  perspective quad (delegates to `TOOL-012`), aspect-ratio constraint, overlay
  enum, shield settings, `delete_cropped: bool`.
- `pictura-core::tools::crop::commit(doc, state) -> Command` — produces either a
  destructive crop command or a non-destructive `SetCropRegion` command.
- `pictura-core::document::CropRegion` — optional non-destructive crop
  rectangle/rotation stored on a layer or document.
- `pictura-core::geometry::transform` — rotation about a point; reused by
  `Ruler` straighten (`TOOL-017`) and Perspective Crop.
- Boundary types: `CropRect { x, y, w, h: f64 }`, `AngleDeg`, `Quad` (4 points),
  `CropOverlay`.

## Qt6 component mapping

- `CropTool` (`QObject`/`QGraphicsItem`-style overlay) — draws box, handles,
  shield, and overlay grid; emits `rectChanged`, `angleChanged`, `commitRequested`.
- `CropOptionsBar` (`QWidget`) — aspect-ratio combo, W/H/resolution spin boxes,
  swap/clear/straighten/reset/cancel/commit buttons, Delete Cropped Pixels
  checkbox, View and Settings menus.
- `CropSettingsMenu` (`QMenu`) — Classic mode, Auto-center preview, Show Cropped
  Area, crop-shield color/opacity.
- `CropOverlayRenderer` — Rule of Thirds / Grid / Golden Ratio / spiral drawing;
  cached on GPU where possible, CPU fallback.
- Widgets over QML for the options bar (native toolbars, key handling); the
  on-canvas overlay is a `QGraphicsScene` item so it composes with the existing
  canvas scene.

## Data-model impact

- New optional per-layer/document field: `crop_region` (rect + rotation +
  optional perspective quad) with a flag for destructive vs non-destructive.
- Non-destructive crop keeps the full raster; a destructive crop shrinks layer
  bounds and discards masked-out tiles.
- Serialization: PSD must round-trip non-destructive crop state and slice-like
  metadata; exact key names TBD (Open questions). XMP may mirror crop
  dimensions.
- Undo: one history state per commit. Record shape: before/after
  `crop_region` + whether tiles were deleted; destructive commit also records
  the discarded region for potential redo (memory tradeoff).

## Edge cases

- **Background-only documents**: Hide/non-destructive crop unavailable until the
  Background is converted to a regular layer.
- **Bitmap mode**: crop-box rotation is not allowed (CS5 behavior documented;
  assumed to carry into CS6 — inferred).
- **Smart objects / locked Background**: Delete Cropped Pixels reported
  unavailable (community; inferred).
- **16/32-bit and CMYK/Lab**: geometry is color-model independent; resampled
  commit must operate in the document's working space.
- **1-pixel / empty documents**: box clamping; minimum 1×1.
- **PSB/huge documents**: crop regions stored as 64-bit-capable coordinates.
- **GPU-unavailable**: overlay/shield and preview render on CPU.
- **Undo/redo**: re-editing a non-destructive crop must not accumulate hidden
  state; `Reveal All` clears it as one reversible step.
- **Aspect-ratio + rotation**: rotation is applied to the image, not the locked
  aspect box; verify behavior against CS6.

## Parity acceptance criteria

1. Given a new RGB 8-bit document, selecting the Crop tool places a crop box
   with handles and a Rule of Thirds overlay, matching the CS6 default.
2. Given Delete Cropped Pixels **off**, cropping then `Image > Reveal All`
   restores the original full canvas with pixel values unchanged.
3. Given Delete Cropped Pixels **on**, after commit the layer bounds equal the
   crop box and the discarded pixels are absent from the PSD.
4. Given an aspect ratio of 2:3, dragging any handle keeps W:H = 2:3 within one
   pixel.
5. Given `X` (swap), the box's W and H exchange and the on-screen orientation
   flips.
6. Given a reference line at angle θ, the committed image is rotated by −θ
   within 0.1°, with no uncovered canvas corners.
7. Given Classic mode on, dragging the image moves the crop box over a fixed
   image (CS5 behavior) rather than moving the image.
8. Given commit, exactly one history state is added; undo restores the prior
   state identically.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help reference. "Crop and straighten photos | CS6"
  (interactive preview, Size/View/Crop options, Classic mode, Delete Cropped
  Pixels, non-destructive crop, straighten); "Crop tool changes and
  enhancements"; "Crop images" (CS5 legacy); "Nondestructive editing"
  (cropping non-destructively, Hide option, Reveal All); key shortcuts for Crop
  tool; tool shortcut groups.
- `https://www.photoshopessentials.com/photo-editing/crop-image-cs6` —
  Steve Patterson, "Cropping Images In Photoshop CS6": automatic crop box and
  handles, aspect-ratio menu, rotate/portrait-landscape `X`, image moves inside
  the box, overlays and `O`, rotate/straighten, Reset/Cancel, Delete Cropped
  Pixels and non-destructive re-editing.
- `https://www.photoshopessentials.com/basics/perspective-crop-tool-photoshop` —
  Perspective Crop tool (used for `TOOL-012`); confirms CS6 introduction and
  the Show Grid option.
- `https://community.adobe.com/questions-712/how-do-i-crop-the-contents-of-a-smart-object-1121429`
  — community: Smart Object contents are not cropped by the Crop tool
  (community; surfaced via search, not directly fetched).
- `https://www.adobe.com/learn/photoshop/web/quick-tip-crop-image-in-photoshop`
  — Delete Cropped Pixels is enabled by default (surfaced via search snippet,
  not directly fetched).

## Open questions

- **Exact CS6 aspect-ratio preset list** (ratios, order, labels). *Resolves
  with:* a direct CS6 screenshot/UI capture or an archived CS6 options-bar
  reference.
- **Exact overlay catalog and orientation behavior** (Triangle, Golden Spiral,
  Diagonal). *Resolves with:* CS6 UI observation; PDF names only Rule of
  Thirds, Grid, Golden Ratio while shortcuts mention Triangle and Golden Spiral.
- **Where non-destructive crop state is stored** (PSD layer key vs. document
  descriptor) and how it serializes. *Resolves with:* PSD file-format spec
  plus a CS6-saved PSD sample.
- **Smart Object crop semantics**: which flag gates Delete Cropped Pixels, and
  whether hiding is per-layer or per-canvas. *Resolves with:* testing on CS6 or
  the community thread above.
- **Bitmap-mode rotation restriction** in CS6. *Resolves with:* CS6 behavior
  test.
- **Classic mode scope**: exactly which CS6 controls are disabled in Classic
  mode. *Resolves with:* CS6 observation.
