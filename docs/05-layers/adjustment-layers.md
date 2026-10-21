# Adjustment Layers

- **Spec ID:** `LAY-012`
- **Status:** `Draft`
- **Parity tier:** `Core` (the adjustment types themselves; **32-bit adjustment layers are Extended-only**)
- **New in CS6:** `Changed` — CS6 adds the **Color Lookup** adjustment (3DLUT/Abstract/Device Link LUTs), moves adjustment presets from the Adjustments panel into the **Properties** panel, keeps the adjustment icons always visible, and adds **Invert** and **Threshold** mask adjustments in 32-bit documents. CS6 is the last version before the Properties-panel workflow was reworked again in CC.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/color-management.md` (`ARCH-007`), `05-layers/blend-modes.md` (`LAY-010`), `05-layers/fill-layers.md` (`LAY-013`), `05-layers/layer-masks.md`, `05-layers/vector-masks-and-clipping-masks.md`, `05-layers/smart-objects.md`, `05-layers/smart-filters.md`, `04-image-ops/adjustments-overview.md`, `04-image-ops/32-bit-hdr.md`.

> Module/crate/widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Per-adjustment algorithms are out of scope here (see the
> `04-image-ops/adjustments/` specs); this document covers the *layer* semantics.
> PSD keys are from the Adobe File Formats Specification (via `ARCH-008`).

## CS6 behavior

An **adjustment layer** stores color/tonal adjustment parameters rather than
pixels. It applies its adjustment to the accumulated composite of the layers
**below it** in the stack (or only to its **clipping base** when clipped), and it
can be re-edited or discarded at any time without destroying the underlying pixel
values. Adjustment layers have the same opacity, blend mode, visibility, reorder,
group, duplicate, and delete behavior as image layers. Because they hold
parameters, they add far less file size than pixel layers.

A **fill layer** (Solid Color, Gradient, Pattern) is the sibling concept: it adds
color content with a mask rather than transforming what is below. See `LAY-013`.

Adjustment types available in CS6 (Adjustments panel):

| Adjustment | PSD key | Notes / key parameters |
|---|---|---|
| Brightness/Contrast | `brit` | Brightness, Contrast; **Use Legacy** toggle |
| Levels | `levl` | Input/Output levels, gamma, per-channel, Auto, eyedropper targets |
| Curves | `curv` | up to 14 control points, per-channel, presets, targeted adjustment tool |
| Exposure | `expA` | Exposure, Offset, Gamma; designed for HDR |
| Vibrance | `vibA` | Vibrance, Saturation |
| Hue/Saturation | `hue ` / `hue2` | Hue, Saturation, Lightness; color ranges; Colorize |
| Color Balance | `blnc` | Shadows/Midtones/Highlights CMYK-RGB sliders; Preserve Luminosity |
| Black & White | `blwh` | per-color mixers; Tint |
| Photo Filter | `phfl` | Filter color, Density, Preserve Luminosity |
| Channel Mixer | `mixr` | per-output-channel source percentages; Monochrome |
| Color Lookup | `clrL` | 3DLUT File, Abstract, Device Link LUTs (CS6-new) |
| Invert | `nvrt` | no parameters |
| Posterize | `post` | Levels 2–255 (bit-depth-limited) |
| Threshold | `thrs` | Levels 1–255 |
| Gradient Map | `grdm` | gradient, Reverse, Dither |
| Selective Color | `selc` | per-color CMYK sliders; Relative/Absolute |
| Solid Color / Gradient / Pattern (fill) | `SoCo` / `GdFl` / `PtFl` | fill layers, see `LAY-013` |

The CS6 Help PDF's Adjustments-panel text does not name Color Lookup, but the
feature is CS6-new (community sources) and the PSD spec carries the `clrL`
adjustment key; the Help omission is recorded in `## Open questions`.

### Layer masks

An adjustment layer carries a **layer mask** by default (paint on the mask to
limit the adjustment to part of the image; gray values vary strength). New
adjustment layers get a mask automatically unless **Add Mask by Default** is
deselected in the Adjustments/Properties panel menu. If a **pixel selection** is
active when the layer is created, the mask is built from it (unselected areas
masked out). If a closed **path** is selected, a **vector mask** is created
instead. A mask can also be built from **Color Range** in the Masks section of the
Properties panel (`Fuzziness`, `Range`, `Localized Color Clusters`). Mask
**Density** and **Feather** can be adjusted non-destructively; CS6 enables
**Invert** and **Threshold** adjustments in 32-bit/channel documents.

### Clipping and confinement

- **Clip to Layer** (`Layer > Create Clipping Mask`, `Ctrl/Cmd+Alt+G`, or the
  Clip-to-Layer button in the Properties panel) makes the adjustment affect only
  the layer immediately below (the clipping base). Clicking again releases it.
- Alternatively, put the adjustment at the top of a **group** whose mode is
  anything other than `Pass Through` (`Layer > New > Group From Layers`), which
  confines the adjustment to the group's layers.
- Clipped layers are assigned the opacity and mode attributes of the base layer,
  unless `Blend Clipped Layers As Group` is deselected (`LAY-010`).

### Opacity, blend mode, and fill

Adjustment and fill layers expose **Opacity** and **Blend Mode**; a fill layer
also exposes **Fill** (adjustment layers have no fill-opacity concept in the layer
panel, although their blend mode and opacity reduce the effect). Lowering opacity
reduces the strength of an adjustment.

### Editing, merging, rasterizing

- Double-click the adjustment/fill thumbnail (or `Layer > Layer Content Options`)
  to re-edit parameters in the Properties panel. **Inverted adjustment layers have
  no editable settings** (the Help's note applies to the Invert adjustment).
- Merging an adjustment layer with the layer below **rasterizes** and permanently
  applies the adjustment. An adjustment or fill layer **cannot be the target** of
  a merge (it must be the source). Fill layers can be rasterized without merging.
- Adjustment/fill layers whose masks are all white add little file size, so
  merging them is not necessary to save space.
- Adjustment presets (CS5: Adjustments panel; CS6: Presets menu in the Properties
  panel) exist for Levels, Curves, Exposure, Hue/Saturation, Black & White,
  Channel Mixer, and Selective Color; user presets are added to the list.

### Smart filters and adjustment layers

Adjustment layers contain no pixel data, so **filters — including smart filters —
cannot be applied to an adjustment layer**. A filter can only become a **Smart
Filter** when applied to a **Smart Object**. The CS6 Help's Smart Filter rules:
any filter can be a Smart Filter except Extract, Liquify, Pattern Maker, and
Vanishing Point; Shadow/Highlight and Variations can also be applied as Smart
Filters. To filter an adjustment's result, rasterize/merge it into a pixel layer,
or convert a layer to a Smart Object and apply the smart filter there. Blur
Gallery effects in CS6 support Smart Objects and can be applied as smart filters
on smart objects, not on adjustment layers.

### Edition / bit-depth differences

- **32-bit adjustment layers are available in Photoshop Extended only** (CS6
  Help, Exposure section). Standard CS6 does not offer adjustment layers at 32
  bpc.
- Adjustments work on layers in **16-bpc** images (with fewer adjustments
  available than 8-bit for some operations).
- HDR (32-bpc) support list for adjustment layers: Levels, Vibrance,
  Hue/Saturation, Channel Mixer, Photo Filter, and Exposure (Help, "Features that
  support 32-bpc HDR images").

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Adjustments panel | Dock | `Window > Adjustments` | Icons always visible in CS6; clicking creates an adjustment layer |
| Properties panel | Dock | n/a | CS6 editing surface + Presets menu; mask controls (Density/Feather/Invert/Color Range) |
| Layers panel | Drop-down | n/a | Blend mode, Opacity (`Fill` for fill layers) |
| Layers panel > New Adjustment Layer button | Button/menu | n/a | Creates a layer by type |
| Layer > New Adjustment Layer | Menu | n/a | Submenu of all types; New Layer dialog (name, color, mode, opacity) |
| Layer > New Fill Layer | Menu | n/a | Solid Color / Gradient / Pattern (`LAY-013`) |
| Layer > Layer Content Options | Menu | n/a | Re-edit selected adjustment/fill layer |
| Layer > Create Clipping Mask | Menu | `Ctrl/Cmd+Alt+G` | Clip adjustment to the layer below |
| Properties panel > Clip to Layer button | Button | n/a | Toggles the clipping mask |
| Layer > Layer Mask | Menu | n/a | Add/reveal/delete/hide; vector mask variants |
| Layer > Rasterize > Layer Style / Layer / Fill Content | Menu | n/a | Destructive |
| Properties > Masks > Color Range | Dialog | n/a | Build the layer mask from sampled colors |

Adjustment-panel channel shortcuts (CS6 Help): `Alt/Option+2` composite channel;
`Alt/Option+3/4/5` red/green/blue (or legacy 1/2/3 with "Use Legacy Channel
Shortcuts"); `Alt/Option`-click `Auto` to define Auto options; `Delete`/
`Backspace` deletes the adjustment layer.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Adjustment type | enum | none | 16 types (table above) + 3 fill types | Fixed at creation; cannot be changed |
| Layer name | string | "<Adjustment> N" | — | New Layer dialog |
| Blend Mode | enum | Normal | 27 modes (`LAY-010`) | |
| Opacity | percent | 100 | 0–100 | Reduces the adjustment strength |
| Fill | percent | 100 | 0–100 | Fill layers only |
| Layer mask | raster | auto white | 0–255 per pixel | Created from selection/path if present |
| Mask Density | percent | 100 | 0–100 | Non-destructive mask strength (`ARCH-008`) |
| Mask Feather | px | 0 | ≥ 0 | 8-byte double in PSD |
| Mask Invert | bool | off | on/off | CS6 also in 32-bit |
| Clipping | bool | off | base / non-base | PSD clipping byte |
| Adjustment preset | enum | none | per adjustment | Levels/Curves/Exposure/Hue/Sat/B&W/Channel Mixer/Selective Color |
| Add Mask by Default | bool | on | on/off | Panel-menu preference |
| Color Lookup source | enum | 3DLUT File | 3DLUT File, Abstract, Device Link | Mac CS6 has extra Abstract profiles (community) |

Per-adjustment parameter ranges are owned by the `04-image-ops/adjustments/*`
specs (e.g. `levels.md`, `curves.md`, `hue-saturation.md`, `selective-color.md`).

## Algorithms & pipeline

This is the layer-level behavior; the adjustment math itself is delegated.

1. **Composite the backdrop.** Accumulate all visible layers below the adjustment
   (respecting groups and `Pass Through`). When the adjustment is clipped, the
   backdrop is limited to the clipping base's rendered result (and the base's
   opacity/mode apply to the clipped group unless `Blend Clipped Layers As Group`
   is off).
2. **Apply the adjustment function** `A(backdrop)` per pixel in the document color
   space. `A` is the per-type transfer defined in `04-image-ops/`; adjustments that
   operate in linear light (Exposure) or on separate channels (Levels, Curves)
   state their own space rules. 32-bit adjustments operate on unbounded floats.
3. **Mask.** Multiply the adjusted result's delta into the backdrop using the layer
   mask (and/or vector mask) so masked-out pixels keep the original backdrop.
   Density scales the mask; Feather blurs it.
4. **Blend.** Composite the adjustment output with the backdrop using the layer's
   blend mode and overall opacity. This is why an adjustment can be used as a
   creative overlay (e.g. set a Curves layer to `Soft Light`).
5. **Composite** the result upward into the document.

Key semantics:

- **Selection/path at creation** becomes a mask (raster) or vector mask; later
  edits to the mask change which pixels are adjusted.
- **Group isolation.** A non-`Pass Through` group composites its children
  (including adjustments) first, so an adjustment inside such a group does not
  affect layers outside it; inside a `Pass Through` group it does.
- **Fill layer counterpart.** A fill layer follows steps 1/3/4/5 but step 2
  *replaces* the backdrop with generated content (solid/gradient/pattern) instead
  of transforming it, and a fill layer's content is not confined to the layers
  below — it is an independent source.
- **Bit depth.** 8/16-bit adjustments clamp to the channel range; 32-bit
  adjustments preserve float headroom. Standard CS6 lacks 32-bit adjustment
  layers (Extended-only); the engine must gate them by edition.

## Rust module mapping

Design proposal — adjustment layers reuse the existing adjustment kernels and the
compositor's pass machinery; they add no new pixel kernels.

- `pictura_core::node::NodeKind::Adjustment { params: AdjustmentParams }` where
  `AdjustmentParams` is an enum with one variant per adjustment (`ARCH-008`).
- `pictura_adjust::Adjustment` trait — `fn apply(&self, src: TileView, dst:
  TileMut, ctx: &AdjustContext)`; one impl per adjustment. Owned by
  `04-image-ops`; `pictura-core` composes them.
- `pictura_core::composite` — handles the adjustment pass: build backdrop, call
  `Adjustment::apply` on the dirty tiles, apply mask, blend, composite.
- `pictura_adjust::params` — typed parameter structs (e.g. `LevelsParams`,
  `CurvesParams { points: Vec<[f32;2]>, channel: Channel }`, `ColorLookupParams {
  source: LutKind, id: LutId }`).
- `pictura_adjust::preset` — adjustment presets for the supported seven types,
  loadable/saveable.
- `pictura_adjust::lut` — `.3dl`/`.cube`/`.look` LUT loaders and a 3-D LUT
  sampler for Color Lookup.
- `pictura_core::edition` — `Edition::Extended` gate for 32-bit adjustments.
- `pictura_core::mask` — density/feather/invert application shared with layer
  masks.

Crossing types: `AdjustmentParams`, `TileView`/`TileMut`, `Mask`, `BlendMode`,
`ColorMode`, `BitDepth`, `LutId`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

- `AdjustmentsPanel` (`QDockWidget`) — grid of adjustment icons; clicking creates
  a layer via the command layer.
- `PropertiesPanel` (`QDockWidget`) — swaps to the editor for the selected node;
  hosts the Presets menu and the Masks section.
- `AdjustmentEditor` subclasses — `LevelsEditor`, `CurvesEditor`,
  `HueSaturationEditor`, `SelectiveColorEditor`, `ColorLookupEditor` (LUT combo
  with 3DLUT/Abstract/Device Link groups), etc., mostly reused by `04-image-ops`.
- `MaskPropertiesWidget` (`QWidget`) — Density, Feather, Invert, Color Range
  button, "Add Mask by Default" hook.
- `ClipToLayerButton` (`QToolButton`) — checkable clipping toggle.
- `NewAdjustmentLayerDialog` (`QDialog`) — name, color, mode, opacity, and
  `Use Previous Layer to Create Clipping Mask`.
- `LayerContentMenu` — `Layer > Layer Content Options` wiring.

## Data-model impact

- `Node.kind = Adjustment { params }`; `Node.mask` and `Node.vector_mask` reuse the
  existing mask model; `Node.blend`, `Node.opacity`, `Node.clipping` are shared
  with pixel layers (`ARCH-008`). Adjustment layers have no fill opacity.
- **PSD adjustment keys** (from the File Formats Specification): `SoCo` (solid
  color), `GdFl` (gradient), `PtFl` (pattern), `brit`, `levl`, `curv`, `expA`,
  `vibA`, `hue `/`hue2`, `blnc`, `blwh`, `phfl`, `mixr`, `clrL` (Color Lookup),
  `nvrt`, `post`, `thrs`, `grdm`, `selc`. The stored payload for each key is the
  same data as the corresponding adjustment's load file; unknown payload fields
  are preserved verbatim.
- The adjustment layer is itself an additional-layer-information block; the layer
  record's channel data is empty (adjustments have no channel pixels) and the
  mask uses channel ID `-2`/`-3` as usual.
- Undo: creating/deleting/reordering an adjustment layer and editing any of its
  parameters are commands with scalar diffs (`ARCH-009`). Mask painting records
  mask tiles; merging/rasterizing records structural changes plus pixel backups.
- Presets and Color Lookup LUT references are external assets, not document data,
  except the chosen LUT identity is stored in `clrL`.
- 32-bit adjustment layers are gated by `Edition::Extended`; files created in
  Standard CS6 cannot contain them.

## Edge cases

- **No layers below / adjustment at bottom.** The backdrop is empty/transparent;
  the adjustment has no visible effect (or reveals black, per adjustment).
- **Clipping without a base.** A clipped adjustment on the bottom layer behaves
  as unclipped.
- **Pass-through groups.** An adjustment inside a passthrough group affects parent
  layers below the group; inside an isolated group it does not (`LAY-010`).
- **All-white mask.** Adds negligible file size; merging is optional (Help).
- **All-black mask.** No visible effect; the layer still exists and must round-trip.
- **Bitmap / Indexed modes.** Adjustment layers are not available (Photoshop
  restricts layers in Indexed mode, and Bitmap has no per-pixel color).
- **CMYK/Lab.** Adjustments run in the document space with mode-specific controls
  (e.g. Selective Color CMYK sliders, Lab Curves).
- **32-bit Standard build.** Adjustment layers are unavailable; the UI must gate
  them off and files containing them require Extended.
- **Invert adjustment** has no editable settings; the Properties panel shows a
  read-only state or no controls (Help: "Inverted adjustment layers do not have
  editable settings").
- **Merging an adjustment as target** is refused; only source-side merge is legal.
- **Empty / 1-px documents.** Adjustment on a zero-area layer is a no-op; mask
  math must handle empty tiles.
- **PSB / huge docs.** The adjustment pass streams per tile; Curves/Levels LUT
  building is O(256)/O(65536), not per-pixel branching.
- **Color Lookup asset missing.** A referenced LUT that is not installed must
  degrade gracefully (identity or last-known) and report, not crash.
- **GPU unavailable.** Adjustments must run on the CPU fallback; Color Lookup
  3-D LUT sampling has a CPU reference.
- **Undo of a mask paint on an adjustment.** Must not accidentally re-run or
  reorder the adjustment's own parameters.

## Parity acceptance criteria

1. Given a Level/Curves/Hue-Saturation adjustment layer over a pixel layer, the
   composite matches the destructive `Image > Adjustments` equivalent within the
   tolerance `T` from `11-cross-cutting/testing-strategy.md`, for 8- and 16-bit.
2. Given a selection when creating an adjustment layer, the mask equals the
   selection (white inside, black outside); the adjustment affects only unmasked
   pixels.
3. Given a gray-painted mask, the adjustment strength varies with the gray value;
   `Density` and `Feather` change the mask as specified without editing pixels.
4. Given a clipped adjustment, only the base layer below changes; releasing the
   clip applies the adjustment to all layers below.
5. Given a non-passthrough group containing an adjustment, layers outside the
   group are unchanged; with `Pass Through`, they are affected.
6. Given Opacity 50 % on an adjustment layer, the result is a 50 % blend between
   the unadjusted and fully adjusted backdrop (per the layer blend mode).
7. Given an Invert adjustment layer, toggling visibility inverts and restores the
   composite; the layer has no editable parameters.
8. Given a Color Lookup adjustment with a known 3-D LUT, output matches a
   reference trilinear/tetrahedral sample of the LUT within tolerance (and the
   chosen LUT identity survives a PSD round trip via `clrL`).
9. Given a PSD with each adjustment type, opening and re-saving preserves every
   adjustment key and its payload byte-for-byte, including unknown fields.
10. Given a Standard-edition build, adjustment layers at 32 bpc are unavailable;
    given Extended, the six HDR-supported adjustments are available.
11. Given `Rasterize`/merge of an adjustment layer, the underlying pixels change
    to the applied result and the node becomes a pixel layer.
12. Given `Add Mask by Default` off, a newly created adjustment layer has no mask.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
  (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used:
  "Adjustment and fill layers" (pp. 288–290): about adjustment/fill layers,
  create, confine to areas, selection/path/Color-Range masks, edit/merge,
  all-white-mask file-size note, the "Inverted adjustment layers do not have
  editable settings" note; "Adjustments panel overview" (p. 245): CS6 Properties
  panel presets for Levels/Curves/Exposure/Hue-Saturation/Black&White/Channel
  Mixer/Selective Color, adjustment icons always visible, Clip to Layer;
  "Apply a correction to only the layer below"; "Color adjustment commands"
  (p. 246); "Features that support 32-bpc HDR images" (pp. 9, 146): the HDR
  adjustment-layer list and "Adjustment layers for 32-bit images are available in
  Photoshop Extended only"; "Applying Smart Filters" (pp. 187–188): smart filters
  require a Smart Object and the filter exclusion list; "What's new in CS6 >
  Masks" (p. 10): Invert/Threshold mask adjustments in 32-bit; 16-bpc adjustment
  support note (p. 112).
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification. Established the adjustment-layer
  keys (`SoCo`, `GdFl`, `PtFl`, `brit`, `levl`, `curv`, `expA`, `vibA`, `hue `/
  `hue2`, `blnc`, `blwh`, `phfl`, `mixr`, `clrL`, `nvrt`, `post`, `thrs`, `grdm`,
  `selc`) and that each stores the same data as the adjustment's load file. Also
  used by `ARCH-008`.
- `https://www.photoshopessentials.com/photo-editing/color-lookup-cs6` —
  established that Color Lookup is **new in CS6**, its Adjustments-panel icon,
  the three LUT categories (3DLUT File, Abstract, Device Link), and that the Mac
  build ships six extra Abstract profiles. Community tutorial source.
- `https://search.brave.com/search?q=Photoshop+CS6+new+Color+Lookup+adjustment+introduced`
  — search results page only (not fetched); corroborated Color Lookup as a CS6
  feature alongside the fetched tutorial.

Not fetched / not used as a primary source: `helpx.adobe.com` (documented HTTP 403
in `README.md`).

## Open questions

- **CS6 Help omission of Color Lookup.** The fetched CS6 Help PDF does not name
  Color Lookup in its Adjustments-panel prose, yet the feature is CS6-new and the
  PSD spec has the `clrL` key. *Resolves with:* a CS6 adjustment-panel screenshot
  or an Adobe release-notes page (helpx is 403; use search snippets).
- **Full adjustment-type list in CS6.** Whether the panel also exposes any
  additional entries (e.g. HDR Toning is a command, not a layer) is not
  enumerated in the Help. *Resolves with:* a CS6 Adjustments-panel capture.
- **`Use Legacy` Brightness/Contrast semantics** and the exact legacy vs modern
  curve are not sourced here. *Resolves with:* `04-image-ops/adjustments/
  brightness-contrast.md`.
- **32-bit adjustment algorithm differences** (Exposure's linear-light math,
  Levels/Curves float behavior) are out of scope for this layer spec. *Resolves
  with:* the per-adjustment specs and `04-image-ops/32-bit-hdr.md`.
- **Color Lookup interpolation** (trilinear vs tetrahedral) and whether Adobe's
  LUT sampling clamps or extrapolates is undocumented. *Resolves with:* CS6
  pixel-diff tests.
- **Mask density/feather serialization** for adjustment layers specifically
  (vs pixel-layer masks) needs confirmation against CS6-made PSDs. *Resolves
  with:* controlled test files (also open in `ARCH-008`).
- **Which merge direction rasterizes what** when an adjustment sits over a group
  or another adjustment is not fully specified. *Resolves with:* CS6 merge tests.
- **Edition gating source of truth.** Whether 32-bit adjustments are gated purely
  by edition or also by document mode/bit depth needs confirmation. *Resolves
  with:* CS6 Extended vs Standard comparison.
