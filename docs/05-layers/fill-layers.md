# Fill Layers

- **Spec ID:** `LAY-013`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` (Solid Color, Gradient, and Pattern fill layers predate CS6). CS6 adds a **Dither** option to the Gradient Overlay/style family, and CS6 permits layer styles on groups; the fill-layer controls themselves are unchanged from CS5.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/color-management.md` (`ARCH-007`), `05-layers/blend-modes.md` (`LAY-010`), `05-layers/layer-styles.md` (`LAY-011`), `05-layers/adjustment-layers.md` (`LAY-012`), `05-layers/layer-masks.md`, `07-color-painting/gradient-presets.md`, `07-color-painting/pattern-presets.md`, `03-tools/gradient-and-paint-bucket.md`.

> Module/crate/widget names are **design proposals**. The repository contains
> a working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and
> what did not. Gradient/pattern generation algorithms are owned by the
> `07-color-painting/` specs; this document covers fill layers as *layers* and
> their difference from layer styles. PSD keys are from the Adobe File Formats
> Specification (via `ARCH-008`).

## CS6 behavior

A **fill layer** is a non-destructive layer whose content is generated from one of
three sources — a **solid color**, a **gradient**, or a **pattern** — rather than
stored as pixels. It is created with a fill type, gets a **layer mask** by
default, and exposes the same blend mode, opacity, visibility, reorder, group,
duplicate, and delete behavior as image layers. Fill layers "do not affect the
layers underneath them" in the Help's wording: they *add* generated content to the
composite, whereas adjustment layers *transform* what is below. They are the
sibling concept to adjustment layers and appear together under
`Layer > New Fill Layer` and under the New Adjustment Layer button menu.

Creation (CS6 Help, "Create a fill layer"):

- `Layer > New Fill Layer > Solid Color / Gradient / Pattern` opens a New Layer
  dialog (name, color, mode, opacity), then the type's options.
- The **New Adjustment Layer button** at the bottom of the Layers panel offers the
  same three fill types.
- **Solid Color** fills with the current **foreground color** by default; the color
  picker selects a different fill color.
- **Gradient** shows the Gradient Editor / gradient picker and options: `Style`
  (shape of the gradient), `Angle`, `Scale`, `Reverse`, `Dither`, `Align With
  Layer`; dragging in the image window moves the gradient center.
- **Pattern** shows a pattern picker, `Scale`, `Snap To Origin`, and `Link With
  Layer`; with `Link With Layer` selected the pattern can be dragged to position
  while the dialog is open.

### Confinement, editing, and merging

- Fill layers automatically receive a **layer mask**. A **pixel selection** at
  creation becomes the mask; a closed **path** becomes a **vector mask**; a mask
  can also be built from **Color Range** in the Properties panel's Masks section
  (`LAY-012`).
- Double-click the fill-layer thumbnail (or `Layer > Layer Content Options`) to
  re-edit the fill in the Properties panel.
- A fill layer can be **rasterized** (`Layer > Rasterize > Fill Content`) without
  merging; merging it downward permanently applies its content. Like adjustment
  layers, a fill layer **cannot be the target** of a merge (only the source).

### Difference from layer styles

Both fill layers and the layer-style **Color/Gradient/Pattern Overlay** effects
generate color procedurally, but they are different mechanisms:

| Aspect | Fill layer | Layer-style Overlay |
|---|---|---|
| What it is | An independent layer node with its own content, mask, blend mode, opacity, and Fill | An effect attached to another layer's transparency |
| Coverage | The whole layer rectangle, clipped by the layer mask/vector mask | The host layer's opaque pixels only (modulated by its alpha) |
| Existence without content | Produces its color/gradient/pattern even with no host pixels | Does nothing if the host layer is empty |
| Editable color/gradient/pattern | Yes, via the Properties panel; can be rasterized to pixels | Yes, via the Layer Style dialog; never a real layer until `Create Layers`/`Rasterize Layer Style` |
| Other effects | Can carry its own blend mode; can be grouped, clipped, and (if rasterized) filtered | Belongs to the host layer's style; effects share the host's Advanced Blending |
| Typical use | Backdrop colors, gradient skies, tiled pattern fills, gradient maps over the document | Colored tint on type/shapes, gradient sheen, pattern texture on an object |
| Serialization | `SoCo` / `GdFl` / `PtFl` adjustment-style layer records | `lrFX`/extended style descriptor blocks (`LAY-011`) |

A fill layer *is* a real layer, so it participates in clipping (`Ctrl/Cmd+Alt+G`),
groups, layer comps, and can be converted to a Smart Object and receive Smart
Filters. An Overlay effect is not a layer and is edited only through the Layer
Style dialog.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layer > New Fill Layer | Menu | n/a | Solid Color / Gradient / Pattern; New Layer dialog first |
| Layers panel > New Adjustment Layer button | Button/menu | n/a | Offers the three fill types |
| Properties panel | Dock | n/a | Edits the selected fill layer's color/gradient/pattern + mask controls |
| Layer > Layer Content Options | Menu | n/a | Re-open the fill settings |
| Layers panel | Panel | n/a | Blend Mode, Opacity, Fill, mask thumbnail; mask/vector-mask shortcuts as for any layer |
| Layer > Layer Mask | Menu | n/a | Add/reveal/delete/hide masks on the fill layer |
| Layer > Rasterize > Fill Content | Menu | n/a | Converts the fill to pixels |
| Gradient Editor / Pattern picker | Dialog/panel | n/a | Reused from the Gradient and pattern presets |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Fill type | enum | Solid Color | Solid Color, Gradient, Pattern | Fixed at creation |
| Name | string | "<Fill type> N" | — | New Layer dialog |
| Blend Mode | enum | Normal | 27 modes (`LAY-010`) | |
| Opacity | percent | 100 | 0–100 | Scales content + mask compositing |
| Fill | percent | 100 | 0–100 | Fill layers expose Fill as well as Opacity |
| Mask | raster | auto white | 0–255 per pixel | Selection/path/Color-Range derivation |
| Mask Density / Feather / Invert | percent / px / bool | 100 / 0 / off | 0–100 / ≥ 0 / on–off | Non-destructive mask controls (`ARCH-008`) |
| **Solid color** | color | foreground | document color space | Picker |
| **Gradient** | gradient | last-used | preset list / custom | Gradient Editor |
| Gradient Style | enum | Linear | Linear, Radial, Angled, Reflected, Diamond | |
| Gradient Angle | degrees | 0 | 0–360 | |
| Gradient Scale | percent | 100 | 1–1000 `(inferred)` | |
| Gradient Reverse | bool | off | on/off | Flips orientation |
| Gradient Dither | bool | off | on/off | Reduces banding (also CS6 Gradient Overlay) |
| Gradient Align With Layer | bool | on | on/off | Uses the layer bounding box |
| **Pattern** | pattern | last-used | pattern list | |
| Pattern Scale | percent | 100 | 1–1000 `(inferred)` | |
| Pattern Link With Layer | bool | on | on/off | Pattern moves with the layer |
| Pattern Snap To Origin | button | — | — | Align pattern origin to the document/layer origin |

Ranges not stated by the CS6 Help are marked `(inferred)` and repeated in
`## Open questions`. Gradient and pattern parameter details are owned by
`07-color-painting/gradient-presets.md` and `pattern-presets.md`.

## Algorithms & pipeline

Fill layers are generated content; the compositor can evaluate them lazily or
rasterize into tiles on demand. *(The following is a design proposal/inferred;
the Help documents only the controls and behavior.)*

1. **Generate content.** Produce an RGBA tile for the fill layer's bounding box:
   - **Solid:** every pixel = the fill color at full alpha.
   - **Gradient:** evaluate the gradient along the chosen geometry (Linear/Radial/
     Angled/Reflected/Diamond), using `Align With Layer` to size the ramp to the
     layer's bounding box, `Angle` to rotate, `Scale` to stretch, `Reverse` to
     flip. `Dither` adds ordered/error-diffusion noise before write-back to hide
     8-bit banding (`03-tools/gradient-and-paint-bucket.md`).
   - **Pattern:** tile the pattern, transformed by `Scale`, positioned by
     `Link With Layer`/`Snap To Origin`/drag; the pattern defines RGBA (patterns
     can carry transparency).
2. **Apply masks.** Multiply generated alpha by the layer mask and vector mask
   (with Density/Feather/Invert).
3. **Blend.** Composite the masked content into the backdrop using the layer's
   blend mode and `Opacity`; `Fill` scales the generated content only.
4. **Composite** upward. Unlike an adjustment, no backdrop read is required for
   generation (except that the blend mode may read the backdrop for B(Cb,Cs)).

Because generation is deterministic from parameters, the fill is non-destructive
until rasterized; re-editing a gradient or pattern re-renders it. A fill layer
whose mask is all white adds little file size (Help).

### 8 / 16 / 32-bit and color modes

- Fill colors/gradients/patterns are evaluated in the document color space at the
  document bit depth; 32-bit usage follows the same float rules as other layers.
- Indexed and Bitmap documents restrict layer creation; fill layers are not a
  normal Indexed-mode feature.

### Relationship to paint tools

The Gradient and Paint Bucket tools paint a raster equivalent of a gradient/pattern
onto a layer; a fill layer is the non-destructive counterpart. The Gradient
Editor, pattern presets, dither, and style/scale semantics are shared
(`03-tools/gradient-and-paint-bucket.md`, `07-color-painting/gradient-presets.md`).

## Rust module mapping

Design proposal — fill layers are a `NodeKind` whose content is generated by the
existing gradient/pattern engines and then flows through the normal composite
path.

- `pictura_core::node::NodeKind::Fill { source: FillSource }` with
  `FillSource = Solid(Color) | Gradient(GradientFill) | Pattern(PatternFill)`.
- `pictura_core::fill::GradientFill { gradient_id, style, angle_deg, scale,
  reverse, dither, align_with_layer }`.
- `pictura_core::fill::PatternFill { pattern_id, scale, link_with_layer,
  origin }`.
- `pictura_paint::gradient::evaluate(fill, rect, transform, depth) -> TileMut` —
  shared with the Gradient tool.
- `pictura_paint::pattern::tile(pattern_id, scale, origin, rect, depth) ->
  TileMut` — shared with the Paint Bucket/Pattern Stamp.
- `pictura_core::composite` — treats a fill node like any source: generate, mask,
  blend, composite.
- `pictura_core::command::RasterizeFill { node }` — replaces the fill node with a
  pixel node holding the generated tiles.
- `pictura_core::style` — applies layer styles to a fill node's mask-defined
  alpha, if CS6 permits (see `## Open questions`).

Crossing types: `FillSource`, `GradientId`, `PatternId`, `Color`, `Mask`,
`BlendMode`, `BitDepth`, `TileMut`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

- `NewFillLayerDialog` (`QDialog`) — name, color, mode, opacity; followed by the
  type-specific editor.
- `FillPropertiesWidget` (`QWidget`) — host that swaps between:
  - `SolidColorEditor` — color button/picker.
  - `GradientFillEditor` — gradient strip/picker, Style, Angle, Scale, Reverse,
    Dither, Align With Layer, canvas drag-to-move.
  - `PatternFillEditor` — pattern grid picker, Scale, Link With Layer, Snap To
    Origin, drag-to-position.
- `LayerContentMenu` — `Layer > Layer Content Options` wiring.
- `FillThumbnailDelegate` (`QStyledItemDelegate`) — renders the solid/gradient/
  pattern preview in the Layers tree (with mask overlay when `Alt`-clicked).

## Data-model impact

- `Node.kind = Fill { source }`; shares `mask`, `vector_mask`, `blend`,
  `opacity`, `fill_opacity`, `clipping`, `visible`, and `styles` with other layer
  kinds (`ARCH-008`). The mask is the mechanism that makes a fill layer selective.
- **PSD keys** (Adobe File Formats Specification): `SoCo` (Solid Color),
  `GdFl` (Gradient Fill), `PtFl` (Pattern Fill) — the same keys used for the
  corresponding adjustment-style records. Each stores the fill descriptor:
  solid color (4-byte color space + values), gradient (gradient descriptor with
  color/opacity stops, style, angle, scale, reverse, dither, align), or pattern
  (pattern id/name, scale, link, origin). Unknown descriptor fields are preserved
  verbatim for round-tripping.
- Fill layers have **no channel image data** of their own beyond the mask
  (`-2`/`-3` channel IDs), and the layer record's content rectangle is the layer
  bounds.
- Undo: creation/deletion/reorder, fill-source parameter edits, mask painting, and
  `Rasterize Fill Content` are commands (`ARCH-009`). Rasterization is destructive
  and needs a structural + pixel backup record.
- Fill layers participate in layer comps (visibility/position/style) and clipping;
  no special serialization beyond the keys above.

## Edge cases

- **Solid layer over the whole canvas.** No mask needed; a fill layer without a
  mask behaves as a full-canvas color sheet that still blends with layers below
  per its mode.
- **Empty mask / no mask.** An all-black mask hides the fill; no mask shows it
  everywhere. CS6 creates a white mask by default (all visible).
- **Indexed / Bitmap.** Fill layers are not available; the UI must gate them.
- **32-bit HDR.** Fill colors/gradients evaluate in float; `Dither` should not
  quantize away HDR range.
- **Gradient dither.** `Dither` operates before write-back; at 16/32-bit it is
  usually a no-op but must not crash or band.
- **Pattern not installed.** A referenced pattern missing on open must fall back
  (last-known or placeholder) and warn, never crash.
- **Pattern with transparency.** Pattern alpha must multiply the layer mask, not
  replace it.
- **Merge target.** A fill layer cannot be the merge target; merging it downward
  applies its content.
- **Rasterize then filter.** After `Rasterize Fill Content`, the result is a pixel
  layer and can take filters; before that it cannot (no pixels to filter).
- **Layer style on a fill layer.** If CS6 allows it, effects use the mask-defined
  alpha; if not, the command must be refused. See `## Open questions`.
- **Clipping.** A clipped fill layer is confined to the base's opaque area and
  takes the base's opacity/mode unless `Blend Clipped Layers As Group` is off
  (`LAY-010`).
- **PSB / huge docs.** Gradient/pattern generation for a 300,000 px canvas must
  stream by tile, not allocate a full-canvas buffer.
- **GPU unavailable.** Gradient/pattern evaluation needs a CPU fallback matching
  the GPU result within tolerance.
- **Color management.** Solid/gradient colors are document-space values; an
  embedded profile change does not alter the stored color numbers, only the
  displayed result (`ARCH-007`).

## Parity acceptance criteria

1. Given a Solid Color fill layer with color `C`, blend Normal, opacity 100 %,
   the composite equals a pixel layer flood-filled with `C`.
2. Given a Gradient fill layer with Style Linear, Angle `θ`, Scale `s`, Reverse
   on/off, the rendered ramp matches the Gradient tool's equivalent within the
   tolerance `T` from `11-cross-cutting/testing-strategy.md`, including `Dither`
   behavior for 8-bit.
3. Given a Pattern fill layer with Scale `s` and Link With Layer on, the pattern
   tiles the layer's bounding box and moves with the layer; off + Snap To Origin,
   the origin is the document origin.
4. Given a selection when creating a fill layer, the layer mask equals the
   selection and the fill is confined to it.
5. Given a fill layer with a layer mask painted gray, the fill's alpha modulates
   with the gray value; Density/Feather/Invert change it non-destructively.
6. Given `Layer > Rasterize > Fill Content`, the node becomes a pixel layer whose
   pixels equal the generated content and the fill parameters are discarded.
7. Given a fill layer clipped to a base, only the base's opaque area shows the
   fill; releasing the clip reveals it over the layers below.
8. Given a fill layer set to a non-Normal blend mode, the result matches the
   `LAY-010` formula between the generated content and the backdrop.
9. Given a PSD with `SoCo`, `GdFl`, and `PtFl` layers, opening and re-saving
   preserves each fill descriptor and unknown descriptor fields byte-for-byte.
10. Given a fill layer used instead of an Overlay style, the two differ exactly as
    the *Difference from layer styles* table specifies (coverage by the layer mask
    vs. by the host layer's alpha).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
  (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used:
  "Adjustment and fill layers" (pp. 288–290): fill layers fill with a solid
  color, gradient, or pattern and "do not affect the layers underneath"; create a
  fill layer; Solid Color uses the current foreground color; Gradient options
  (Gradient Editor, Style, Angle, Scale, Reverse, Dither, Align With Layer,
  drag-to-move center); Pattern options (pattern picker, Scale, Snap To Origin,
  Link With Layer, drag-to-position); mask confinement via selection/path/Color
  Range; editing via Layer Content Options; merging/rasterizing and the merge-target
  restriction; the all-white-mask file-size note. "Gradient Overlay and Gradient
  Stroke dither added in CS6" and "Dither reduces banding" (p. 10, p. 272).
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification. Established the `SoCo`, `GdFl`,
  and `PtFl` fill-layer keys and their descriptor payloads, and the layer/mask
  channel layout. Also used by `ARCH-008`.

Not fetched / not used as a primary source: `helpx.adobe.com` (documented HTTP 403
in `README.md`).

## Open questions

- **Gradient Scale and Pattern Scale ranges.** The CS6 Help does not state
  numeric clamps; `1–1000 %` is inferred from the UI. *Resolves with:* a CS6
  dialog capture.
- **Whether layer styles can be applied to fill (and adjustment) layers.** The
  Help excludes only background/locked/group layers. *Resolves with:* a CS6 test
  applying Drop Shadow / Color Overlay to a Solid Color fill layer.
- **Default fill color.** Help says Solid Color uses the current foreground color;
  whether CS6 stores/creates it from the foreground at creation and whether the
  default gradient/pattern is the last-used preset are not fully specified.
  *Resolves with:* a CS6 capture.
- **Gradient dither algorithm** (ordered vs error diffusion) and its toggleable
  effect at 8-bit are not documented; it is shared with the Gradient tool.
  *Resolves with:* `07-color-painting/gradient-presets.md` and CS6 test renders.
- **Descriptor field layout** for `GdFl`/`PtFl` (stop encoding, units, pattern
  reference form) is only partly in the fetched file-format excerpt. *Resolves
  with:* the full additional-layer-information section or CS6-made PSDs.
- **Fill-layer rasterization bit depth and curve** — whether rasterizing at
  8/16/32-bit is bit-exact to the live render — is unverified. *Resolves with:*
  a CS6 rasterize-and-diff test.
- **Clipping + fill layer opacity interaction** (does the base's mode/opacity
  override the fill layer's, per `Blend Clipped Layers As Group`) needs a
  controlled CS6 reference, as with `LAY-010`.
- **Indexed-mode behavior.** Whether any fill-layer functionality exists in
  Indexed mode is not documented in the fetched Help. *Resolves with:* a CS6
  Indexed-mode test.
