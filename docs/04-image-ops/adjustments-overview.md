# Adjustments — Overview

- **Spec ID:** `ADJ-000`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the adjustment *engine* is unchanged, but the host UI moves: CS5 kept controls and presets inside the Adjustments panel, CS6 adds the new **Properties panel** (controls + a Preset menu), leaving the Adjustments panel as icon buttons only. CS6 also adds the **Color Lookup** adjustment and improves the Levels/Curves/Brightness-Contrast **Auto** button.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/color-management.md` (`ARCH-007`), `04-image-ops/image-modes.md` (`IMG-004`), `04-image-ops/bit-depth-and-conversion.md` (`IMG-005`), `05-layers/adjustment-layers.md`, `05-layers/blend-modes.md`, `05-layers/layer-masks.md`, `05-layers/vector-masks-and-clipping-masks.md`, `02-ui-ux/panels/adjustments-panel.md`, `02-ui-ux/panels/properties-panel.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts taken from a fetched source are attributed in `##
> Sources`; anything else is marked *(inferred)*.

## CS6 behavior

Photoshop CS6 exposes color and tonal correction in two forms with identical
math but different lifetimes:

1. **Destructive commands** under `Image > Adjustments > …`. They rewrite the
   pixels of the **active layer** in place and **discard image information**
   (CS6 Help, repeated in every adjustment section). No mask, no re-edit.
2. **Adjustment layers** created from the **Adjustments panel** or
   `Layer > New Adjustment Layer > …`. They store the adjustment parameters, not
   pixels, and affect all the layers below them; they carry a layer mask
   by default and expose the same opacity and blending-mode options as image
   layers. They can be clipped to the layer directly below, grouped, hidden,
   reordered, duplicated, and copied between documents. This is the recommended,
   non-destructive path.

The CS6 Help states the general model: all Photoshop color adjustment tools work
essentially the same way, mapping an existing range of pixel values to a new
range, and differ only in how much control they provide. Accessing a color/tone
command in the Adjustments panel automatically creates an adjustment layer.

### The adjustment taxonomy (CS6)

Two command families feed the same panel:

- **Tonal adjustments** — Levels, Curves, Brightness/Contrast, Exposure,
  Shadow/Highlight (`Image > Adjustments` only), HDR Toning, Auto Tone/Contrast/
  Color, Equalize.
- **Color adjustments** — Hue/Saturation, Vibrance, Color Balance, Black & White,
  Photo Filter, Channel Mixer, Selective Color, Color Lookup (CS6), Gradient Map,
  Invert, Posterize, Threshold, Desaturate, Match Color, Replace Color.

Availability split (CS6):

| Command | Adjustment layer | Destructive `Image > Adjustments` | Notes |
|---|---|---|---|
| Levels | Yes | Yes | PSD key `levl` |
| Curves | Yes | Yes | PSD key `curv` |
| Brightness/Contrast | Yes | Yes | PSD key `brit` |
| Exposure | Yes (8/16; 32-bit Extended only) | Yes | PSD key `expA` |
| Vibrance | Yes | Yes | PSD key `vibA` |
| Hue/Saturation | Yes | Yes | PSD keys `hue ` / `hue2` |
| Color Balance | Yes | Yes | PSD key `blnc` |
| Black & White | Yes | Yes | PSD key `blwh` |
| Photo Filter | Yes | Yes | PSD key `phfl` |
| Channel Mixer | Yes | Yes | PSD key `mixr` |
| Color Lookup (CS6) | Yes | Yes | new in CS6 |
| Selective Color | Yes | Yes | PSD key `selc` |
| Gradient Map | Yes | Yes | PSD key `grdm` |
| Invert | Yes | Yes | PSD key `nvrt` |
| Posterize | Yes | Yes | PSD key `post` |
| Threshold | Yes | Yes | PSD key `thrs` |
| Shadow/Highlight | **No** | Yes | never an adjustment layer (community sources plus the Help's note that it applies adjustments directly and discards image information) |
| HDR Toning | **No** | Yes | requires flattened layers |
| Desaturate | **No** | Yes | command only |
| Auto Tone / Auto Contrast / Auto Color | (via Levels/Curves Auto) | Yes | also drive the Auto button |
| Equalize | **No** | Yes | command only |
| Match Color | **No** | Yes | command only |
| Replace Color | **No** | Yes | command only |
| Fill layers (Solid Color / Gradient / Pattern) | Yes (fill layer) | — | unlike adjustment layers, fill layers do not affect the layers underneath |

The PSD adjustment keys above are the canonical serialization of adjustment-layer
parameters (`ARCH-008`); each stores the same data as that adjustment's load
file.

### Adjustment layer mechanics

- **Mask** — every adjustment/fill layer gets a layer mask by default unless
  `Add Mask by Default` is deselected in the panel menu. A pixel selection at
  creation masks the unselected area black; a closed path at creation produces a
  **vector mask** instead. Painting gray on the mask varies the effect. The Help
  notes that painting black areas on the mask with the Brush tool blocks the
  adjustment from affecting the image there.
- **Clipping** — the `Clip to Layer` button confines the adjustment to the layer
  immediately below; clicking it again makes the adjustment apply to all layers
  below. In the Layers panel this is the
  clipping-mask relationship (base/non-base byte in the PSD layer record).
- **Blend mode / opacity / fill opacity** — adjustment layers carry the full
  layer property set; the result is composited with the document's 27 blend
  modes. Blend math is behavioral parity per the PDF blend specification
  (`05-layers/blend-modes.md`).
- **Grouping** — to confine an adjustment to a fixed set of layers, put those
  layers in a group and change the group's mode from `Pass Through` to any other
  mode, then place the adjustment on top of the group.
- **Merge / rasterize** — merging an adjustment layer into the layer below
  rasterizes the adjustment and applies it permanently within the merged layer.
  Adjustment layers with all-white masks add negligible file size.
- **Properties** — `Layer > Layer Content Options`, or double-clicking the layer
  thumbnail, reopens the settings; the Help notes that inverted adjustment layers
  have no editable settings (i.e. an inverted/mask-only layer).

### CS6 changes (the "adjustment presets" change)

CS5 vs CS6 panel behavior:

- In CS5 the Adjustments panel held a list of adjustment presets for common
  corrections; in CS6 those presets move to a Presets menu in the Properties
  panel.
- In CS6 the Adjustments panel shows its adjustment icons at all times.
- Presets can be **saved** for **Levels, Curves, Exposure, Hue/Saturation, Black &
  White, Channel Mixer, and Selective Color**; a saved preset is appended to the
  list. `Load Preset` also appears in the adjustment dialogs.
- The What's New section lists improved Auto options for the Levels, Curves, and
  Brightness/Contrast adjustments; Alt/Option-clicking the Auto button opens the
  Auto Color Correction Options dialog.
- The CS6 JDI (productivity) list adds sample-size options to the context menu of
  various Eyedropper tools (black point and white point in Levels, and so forth).
- CS6 adds the **Color Lookup** adjustment layer (community sources) and the
  **HDR Toning** command.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Layer > New Adjustment Layer` | Menu | — | one submenu entry per adjustment |
| `Image > Adjustments` | Menu | — | destructive; one submenu entry per adjustment |
| Adjustments panel | Dock | — | CS6: adjustment **icons only**; clicking one creates a layer |
| Properties panel | Dock | — | CS6: holds the controls and the **Preset menu**; also the Masks controls |
| Layers panel "New Adjustment Layer" button | Button | — | half-filled-circle icon; choose type |
| Layers panel | Dock | `F7` | blend mode, opacity, fill, mask, clipping, visibility |
| Layer > Layer Content Options | Menu | — | reopen adjustment settings |
| Layer > New Fill Layer | Menu | — | Solid Color / Gradient / Pattern fill layers |
| `Edit > Undo` / `Step Backward` | Menu | `Ctrl+Z` / `Ctrl+Alt+Z` | one state per committed edit |
| Panel menu (`Adjustments` / `Properties`) | Menu | — | `Save Preset`, `Load Preset`, `Auto Options`, `Add Mask by Default`, `Auto-Select Parameter`, `Auto-Select Targeted Adjustment Tool` |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Adjustment type | Enum | — | the taxonomy above | chosen at creation; fixed thereafter for a layer |
| Layer mask | Raster mask | white (all shown) | 0–255 per pixel | created by default (`Add Mask by Default`) |
| Vector mask | Path | none | any closed path | created when a closed path is active |
| Clip to layer below | Bool | off | on / off | `Clip to Layer` button |
| Blend mode | Enum | Normal | 27 CS6 modes | full layer set |
| Opacity | Percent | 100 | 0–100 | layer composite opacity |
| Fill opacity | Percent | 100 | 0–100 | affects layer pixels only (n/a for adjustment data) |
| Visibility | Bool | on | on / off | toggle without deleting |
| Preset | Enum | — | saved presets per adjustment | Levels, Curves, Exposure, Hue/Sat, B&W, Channel Mixer, Selective Color |
| Mask density | Percent | 100 | 0–100 | CS6 Properties/Masks controls |
| Mask feather | Number (px) | 0 | 0–1000 (community) | CS6 masks controls |

## Algorithms & pipeline

### The shared point-operation model

Every tonal/color adjustment in this family is a **pointwise map** on channel
samples (optionally after a color-space transform):

```text
out(x,y,c) = f_c( in(x,y,c) )        for a per-channel map (Levels, Curves)
out(x,y)   = g( in(x,y,·) )          for a vector map (Vibrance, Hue/Sat)
```

Two implementations are equivalent within the tolerance:

1. **Direct evaluation** per pixel (keeps full 32-bit float precision).
2. **Lookup table (LUT)** — build a 1-D LUT of N entries for per-channel maps
   (N = 256 / 4096 / 65536 by bit depth) or a 3-D LUT for vector maps, then
   sample. The Adobe pipeline is LUT-based in practice; a LUT makes the
   GPU path trivial and is exact when N ≥ the sample range. 8/16-bit documents
   are exact; 32-bit float documents must use direct evaluation or a large LUT
   with interpolation (LUT interpolation is an approximation — mark it).

The document bit depth sets the scalar type: `u8` (8 bpc), `u16` (16 bpc),
`f32` (32 bpc HDR). All normalized math below uses `v ∈ [0,1]` for integer
depths and unbounded/`[0,∞)` linear values for 32-bit (see `IMG-005`).

### Where the adjustment sits in the pipeline

An adjustment layer is a node in the layer stack. Compositing walks bottom-to-top
(`ARCH-008`):

1. The backdrop (all layers below the adjustment) is composited.
2. If clipped, the backdrop is the clip base only.
3. The adjustment transforms its **input** — for a normal adjustment layer, the
   accumulated composite below; for a smart filter-like behaviour or a
   destructive command, the target layer's own pixels.
4. The transformed result is masked (`mask × vector_mask`), scaled by opacity,
   and blended into the backdrop with the layer's blend mode.

Adjustment layers therefore always read the **composited** colors, not their own
pixel buffer (they have none). Destructive commands read and write the target
layer's pixels directly.

### Color mode and per-channel semantics

- **RGB** — three channels + composite ("RGB") channel. Per-channel edits index
  R/G/B; the composite edit applies the same map to all three.
- **CMYK** — four channels + composite; Curves shows **ink percentages** rather
  than light values and the graph is oriented so 0% ink is at the lower-left.
- **Lab** — L/a/b; Curves shows **light values**.
- **Grayscale** — single channel; composite only (per-channel menus collapse).
- **Alpha / spot channels** — in Levels the Help says spot and alpha channels are
  edited individually; composite multi-channel selection is available in
  the destructive command (Shift-select channels in the Channels panel) but not
  in a Levels adjustment layer.
- **Bitmap / Indexed** — per-pixel color adjustments are not meaningful; the
  commands are unavailable (verify per command).

### Auto corrections (shared by Levels and Curves)

The `Auto` button and the Auto Tone/Contrast/Color commands share the **Auto
Color Correction Options** model (see `ADJ-001` for the full control list). The
three documented algorithms:

- **Enhance Monochromatic Contrast** — clips all channels identically and so
  preserves the overall color relationship; used by `Auto Contrast`.
- **Enhance Per Channel Contrast** — maximizes the tonal range in each channel;
  may add or remove color casts; used by `Auto Tone`.
- **Find Dark & Light Colors** — finds average lightest/darkest pixels to
  maximize contrast with minimal clipping; used by `Auto Color`; combined with
  **Snap Neutral Midtones** to neutralize a cast.

Exact black/white-point statistics and the midtone-neutralization math are closed
by Adobe; parity is behavioral.

### Bit-depth availability (CS6 documented)

The CS6 32-bpc feature list names the adjustments available at 32 bpc:
**Levels, Exposure, Hue/Saturation, Channel Mixer, Photo Filter** (and, in the
Layers-command list, also **Vibrance**). **Curves** and **Brightness/Contrast**
are absent from the list and are therefore unavailable at 32 bpc *(documented by
absence; confirm — see Open questions)*. Adjustment layers for 32-bit images are
available "in Photoshop Extended only."

## Rust module mapping

Design proposal.

- `pictura-core::adjust` — `Adjustment` enum (`Levels | Curves | BrightnessContrast
  | Exposure | Vibrance | …`), parameter structs, and `AdjustmentLayer { params,
  blend, opacity, mask }`. Serialization keys mirror the PSD keys (`levl`, `curv`,
  `brit`, `expA`, `vibA`, …).
- `pictura-image::adjust::lut` — `ToneLut { entries: usize, scalar: Scalar }` with
  `build_u8`, `build_u16`, `build_f32`, and `sample_f32`. Shared by every
  per-channel adjustment.
- `pictura-image::adjust::ops` — `apply_lut`, `apply_vector`, `apply_log_lut`
  (RGB/CMYK/Lab/Gray dispatch), tile-parallel with rayon.
- `pictura-render::adjust` — wgpu compute / fragment variants that sample a 1-D
  LUT texture or a 3-D LUT; CPU path is the reference.
- `pictura-core::adjust::auto` — `AutoCorrection { algorithm, clip_shadows,
  clip_highlights, target_shadow, target_midtone, target_highlight,
  snap_neutral_midtones }` and the statistics gatherer.
- Boundary types: `Scalar`, `ChannelSet`, `Rect`, `TileRef`, `MaskRef`,
  `BlendMode`. Pixel buffers are planar per channel, matching `ARCH-008`.

## Qt6 component mapping

Design proposal (Widgets; QML only for the curve editor if a canvas is easier).

- `AdjustmentsPanel` (`QWidget`) — the icon grid; each click emits
  `createAdjustmentLayer(kind)`.
- `PropertiesPanel` (`QStackedWidget`) — hosts one properties widget per
  adjustment plus the Masks controls (density/feather, Color Range, Invert).
- `AdjustmentPresetMenu` (`QMenu`) — lists built-ins + saved presets; backed by a
  `PresetModel` persisted to disk.
- `AutoCorrectionDialog` (`QDialog`) — the shared Auto Color Correction Options.
- `AdjustmentLayerModel` / `LayersModel` (`QAbstractItemModel`) — exposes kind,
  params, mask, blend, opacity, clipping.
- Reusable widgets: `ToneCurveEditor` (Curves, and the Levels output/gamma mini
  controls), `LevelsHistogramWidget`, `ScrubSpinBox` for numeric fields.

The panel split (icons in Adjustments, controls in Properties) mirrors CS6; a
single combined panel is a deliberate divergence and must be a settings option,
not the default, to preserve CS6 muscle memory.

## Data-model impact

- A new node kind `NodeKind::Adjustment { params, mask }` exists in the document
  arena (`ARCH-008`); its parameters are typed, not opaque.
- PSD serialization: adjustment parameters go in the documented adjustment keys
  (`levl`, `curv`, `brit`, `expA`, `vibA`, …); unknown keys are preserved
  verbatim. Mask uses channel IDs `-2` (user) / `-3` (real user when both masks
  exist). Clipping and blend keys use the layer-record fields.
- Undo: editing an adjustment parameter is a **scalar command** with a small
  record (before/after parameter blob); committing a destructive `Image >
  Adjustments` edit is a **pixel command** retaining the pre-edit tiles. Live
  drag inside the Properties panel coalesces into a single history state on
  release/commit, matching Photoshop.
- Adjustment layers with all-white masks add negligible file size; do not force
  a rasterization on save.
- 32-bit: the parameter structs must be depth-agnostic (store normalized floats),
  so the same adjustment round-trips at 8/16/32.

## Edge cases

- **No layer / background layer** — destructive commands apply to the background
  and permanently alter it; adjustment layers always need at least one layer
  below (an adjustment on the bottom is a no-op).
- **Bitmap / Indexed / Multichannel** — many adjustments are unavailable; the
  command must be dimmed rather than silently converting the mode.
- **Grayscale** — per-channel menus collapse; Vibrance/Saturation have no
  meaningful chroma and should be disabled or inert (mark).
- **32-bit** — Curves/Brightness-Contrast unavailable; Exposure native; integer
  LUTs invalid on float data.
- **Empty / 1-px documents** — LUT still builds; apply is a no-op over zero
  pixels; no panic.
- **Huge (PSB) documents** — build the LUT once and apply per tile; never
  materialize a full-canvas float copy.
- **GPU unavailable** — CPU reference produces the same result within the
  tolerance; the LUT makes CPU↔GPU equality attainable for 8/16-bit.
- **Clipping mask with no base** — clipped adjustment whose base is hidden yields
  no effect; document explicitly.
- **Undo mid-drag** — interactive edits are coalesced; released edits are atomic.
- **Adjustment above a group** — respects the group's `Pass Through` vs isolated
  mode (`ARCH-008`).
- **Inverted / empty mask** — in CS6 inverted adjustment layers have no editable
  settings; represent as mask-only.
- **Mode conversion later** — converting RGB→CMYK reinterprets adjustment
  parameters (Curves becomes ink percentages); parameter migration is a
  documented open question.
- **Memory** — an adjustment layer costs parameters + mask only, not a pixel
  buffer.

## Parity acceptance criteria

1. Given an RGB document, `Layer > New Adjustment Layer > Levels` creates a layer
   that changes the composite of the layers below but leaves their pixels
   unchanged; deleting the layer restores the original appearance exactly.
2. Given an active selection, creating an adjustment layer produces a mask that
   is black in the unselected area and white in the selection.
3. Given `Clip to Layer`, the adjustment affects only the immediate base layer;
   toggling the clip button restores effect on all layers below.
4. Given a saved preset for each of Levels/Curves/Exposure/Hue-Sat/B&W/Channel
   Mixer/Selective Color, the preset persists across restart and re-applies the
   same parameter values.
5. Given an `Image > Adjustments > Levels` edit, exactly one history state is
   added and `Ctrl+Z` restores the pre-edit pixels bit-exactly.
6. Given a 32-bpc document, the Curves and Brightness/Contrast entries are
   unavailable, while Levels and Exposure are available.
7. Given a Bitmap document, per-pixel color adjustments are dimmed.
8. Given a 1×1 document and any adjustment, the operation completes without
   panic and produces a deterministic result.
9. Given a group set to `Pass Through` containing an adjustment, moving the
   adjustment above/below the group changes the composite per the documented
   pass-through semantics.
10. Given an adjustment layer with a mask painted gray, the effect is
    proportional to the mask value at each pixel within the tolerance of the
    testing strategy.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference, downloaded and text-extracted. Established:
  Adjustments-panel overview and the CS5/CS6 preset-location change; the general
  model that all color adjustment tools map an existing pixel-value range to a
  new one; the color-adjustment command list; adjustment layers affect all
  layers below them, carry opacity and blending-mode options, are created
  automatically when using the panel, and discard image information when applied
  destructively; `Clip to Layer` behavior; default mask creation and the
  selection/path mask variants; `Add Mask by Default`; merge/rasterize behavior;
  that inverted adjustment layers have no editable settings; the save-preset list
  (Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer,
  Selective Color); that fill layers do not affect layers underneath; the Auto
  algorithm names and clip defaults; the 32-bpc adjustment list and the
  "Photoshop Extended only" note for 32-bit adjustment layers; the JDI
  eyedropper sample-size change and the improved-Auto statement.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/propertiespanel.html`
  — CS6 Properties panel: new in CS6, replaces the Adjustments controls mode and
  the Masks panel; Preset popup for pre-supplied adjustment settings; Save Preset;
  mask density/feather (to 1000 px) and Color Range in the Masks mode.
- `https://www.apogeephoto.com/photoshop-cs6-cc-the-adjustments-panel-and-properties-panel`
  — CS6 Adjustments panel (icons create an adjustment layer whose controls appear
  in the Properties panel) and the Properties panel's two views,
  Toggle Visibility / Reset / Delete / Clip to Layer / Previous State.

Secondary / community (not fetched this pass; verify against CS6): the PSD
adjustment-key list is taken from `01-architecture/document-model.md`, itself
sourced from the Adobe File Formats Specification; the Color Lookup-as-CS6 claim
rests on community tutorials surfaced by search (e.g.
`https://www.photoshopessentials.com/photo-editing/color-lookup-cs6`) and is
marked *(inferred)* here.

## Open questions

- **Which exact adjustments CS6 exposes as adjustment layers vs command-only**
  (especially Color Lookup, Shadow/Highlight, Desaturate, Replace Color, Match
  Color, Equalize, Auto Tone/Contrast/Color). *Resolves with:* a CS6 Layers >
  New Adjustment Layer capture and the Image > Adjustments submenu.
- **The complete CS6 Adjustments-panel icon set and order.** *Resolves with:* a
  clean CS6 panel screenshot.
- **Exact availability matrix by color mode** (e.g. Vibrance in CMYK/Lab,
  Brightness/Contrast in Lab). *Resolves with:* a CS6 mode-by-mode observation.
- **Whether `Add Mask by Default` affects fill layers as well as adjustment
  layers.** *Resolves with:* CS6 observation.
- **The precise list of built-in adjustment presets** shipped with CS6 (the
  Properties Preset menu) per adjustment. *Resolves with:* a CS6 install capture
  (see `ADJ-002` for the Curves list surfaced so far).
- **Auto button memory semantics** — whether CS6's improved Auto re-applies the
  last-used Auto Color Correction settings and how that interacts with `Save as
  Defaults`. *Resolves with:* a CS6 experiment plus the Help's Auto section.
- **Undo granularity for a long drag** in the Properties panel — one state per
  drag or per parameter change. *Resolves with:* a CS6 History-panel trace.
- **Behavior of adjustment parameters on a mode conversion** (RGB↔CMYK/Lab) and
  on merge/rasterize. *Resolves with:* CS6 experiments with saved test files.
