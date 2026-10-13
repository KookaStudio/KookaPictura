# Layer Masks

- **Spec ID:** `LAY-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 moves mask editing from the CS5 **Masks panel** into the **Properties panel**. Clicking the mask thumbnail in the Layers panel activates the mask (CS5 used the Masks-panel Pixel Mask button), and CS6 enables **Invert** and **Threshold** adjustments for masks in 32-bit/channel images. Mask add/delete, paint semantics, density, feather, refine, link/unlink, and rubylith display are CS5-era.
- **Depends on:** `LAY-001`, `LAY-002`, `LAY-003`, `LAY-005`, `01-architecture/document-model.md` (`ARCH-008`), `08-selection/selection-model.md`, `08-selection/refine-edge.md`, `05-layers/layer-styles.md`.

> Module and type names are design proposals. Behavior from the CS6 Help
> reference (cited); inferred items are marked.

## CS6 behavior

A **layer mask** is a resolution-dependent bitmap image, edited with the
painting or selection tools. Masks hide portions of a layer and reveal layers
below; they are **non-destructive** (re-editable without losing hidden pixels)
and are stored as alpha channels. In the Layers panel a layer mask appears as
an additional thumbnail to the right of the layer thumbnail, representing the
grayscale channel created when the mask is added.
A layer can carry both a layer mask and a vector mask (see `LAY-005`).

**Background caveat:** to create a layer or vector mask on the Background layer,
first convert it with `Layer > New > Layer From Background`.

### Grayscale semantics (paint to add/subtract)

A layer mask is a grayscale image. The editing contract:

- **Black** hides the layer (the layers below show).
- **White** reveals the layer.
- **Gray** makes the layer partially visible; darker gray is more transparent,
  lighter gray more opaque.

From the "Editing layer masks" procedure (CS6): with the mask active, the
foreground/background colors default to grayscale; painting with
**white subtracts from the mask and reveals the layer**, painting with **black
adds to the mask and hides the layer or group**, and gray gives partial
transparency.

### Add a mask: reveal vs hide, selection-based, transparency-based

- **Reveal/hide the entire layer:** with nothing selected (`Select > Deselect`),
  click **Add Layer Mask** or `Layer > Layer Mask > Reveal All` to reveal all;
  Alt/Option-click Add Layer Mask or `Layer > Layer Mask > Hide All` to hide all.
- **Selection-based:** with a selection active, click the **New Layer Mask**
  button to create a mask that **reveals the selection**; Alt/Option-click Add
  Layer Mask to create a mask that **hides the selection**; or use `Layer > Layer
  Mask > Reveal Selection` / `Hide Selection`.
- **From transparency:** `Layer > Layer Mask > From Transparency` converts layer
  transparency into a mask (the transparency becomes an opaque color hidden by
  the new mask; the exact color depends on prior processing). Useful for video
  and 3D workflows.
- **Defaults:** adjustment and fill layers automatically receive a layer mask
  (mask icon to the right of the layer thumbnail); `Add Mask by Default` in the
  Adjustments/Properties panel menu controls this.
- **Move/copy a mask between layers:** drag the mask to another layer; Alt/Option-
  drag duplicates it.

### Mask activation, display, and rubylith

- **Edit the mask:** in CS6, click the **Mask thumbnail** in the Layers panel
  (CS5: the Pixel Mask button); a border appears around the active thumbnail.
  Click the layer thumbnail to edit the layer instead.
- **View the mask alone:** Alt/Option-click the mask thumbnail to show only the
  grayscale mask; click again to restore. (Alternatively the eye icon in the
  Properties/Masks panel.)
- **View as rubylith overlay:** Alt+Shift/Option+Shift-click the mask thumbnail
  to overlay the mask in the rubylith masking color; repeat to turn it off.
- **Change rubylith color/opacity:** double-click the mask channel (or, in CS5,
  the mask thumbnail) to open **Layer Mask Display Options**; pick a color and an
  opacity 0–100%. **These settings affect only the display, not how underlying
  areas are protected.**
- **Paste a copied selection into a mask:** Alt/Option-click the mask thumbnail,
  `Edit > Paste`, then `Select > Deselect`; the selection is converted to
  grayscale and added to the mask.

### Disable, link, apply, delete

- **Disable/enable:** click Disable/Enable Mask in the Properties panel (CS6) or
  Masks panel (CS5); Shift-click the mask thumbnail; or `Layer > Layer Mask >
  Disable` / `Enable`. A **red X** appears over the thumbnail and the layer
  content shows unmasked.
- **Link/unlink:** by default the layer/group is linked to its mask (link icon
  between the thumbnails), so they move together with the Move tool. Click the
  link icon to unlink and move them independently; click between the thumbnails
  to re-link.
- **Apply:** permanently delete the hidden portions. Layer masks are stored as
  alpha channels, so applying/deleting can reduce file size. Via `Apply Mask` in
  the Masks/Properties panel, or the `Layer` menu. (You cannot apply a layer mask
  permanently to a Smart Object layer when deleting the mask.)
- **Delete without applying:** `Delete` in the panel then confirm, or the Layer
  menu.
- **Load mask as a selection:** Ctrl/Cmd-click the mask thumbnail selects the
  unmasked areas; Shift adds, Alt/Option subtracts, Ctrl+Alt+Shift/Option+Shift
  intersects. (Ctrl/Cmd-clicking the layer thumbnail selects the layer's
  non-transparent areas instead.)

### Density, feather, invert, Mask Edge, Color Range

The Properties panel (CS6) or Masks panel (CS5) provides mask controls:

- **Density** slider — mask opacity. At **100%** the mask is completely opaque
  and blocks the underlying area; lowering density reveals more of the area under
  the mask.
- **Feather** slider — blurs mask edges for a softer transition, applied from the
  mask edges outward within the selected pixel range.
- **Invert** — reverses masked and unmasked areas (layer masks only, not vector
  masks).
- **Mask Edge** — opens the **Refine Mask** dialog (the same option set as
  Refine Edge; see `08-selection/refine-edge.md`): View Mode, Refine/Erase Radius
  brushes, Smart Radius, **Radius**, **Smooth**, **Feather**, **Contrast**,
  **Shift Edge**, **Decontaminate Colors** (with **Amount**), and **Output To**.
  Note: **Decontaminate Colors changes pixel color** and therefore outputs to a
  new layer/document.
- **Color Range** — confines the mask by color, via the Color Range dialog
  (White = unmasked pixels, black = masked; adjust **Fuzziness**). See
  `08-selection/color-range.md`.
- In 32-bit/channel images, CS6 enables **Invert** and **Threshold** for masks.

### Filter masks (distinct from layer masks)

Smart Filters use a **filter mask** on a Smart Object layer. It shares the paint
semantics (black hides the filter, white shows, gray partial), supports density,
feathering, and invert, but **Mask Edge is not available for filter masks**, and
filter masks are moved/copied between smart-filter effects. See
`05-layers/smart-filters.md`; this spec covers layer masks proper.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel | Dock | `F7` | Mask thumbnail right of layer thumbnail; active-thumbnail border; red X when disabled; indentation for clipped layers. |
| Add Layer Mask button | Panel button | n/a | Click = Reveal All / reveal selection; Alt/Option-click = Hide All / hide selection; Ctrl/Cmd-click = vector mask reveal all/path; Ctrl+Alt/Cmd+Option-click = vector mask hide all. |
| `Layer > Layer Mask` | Menu | n/a | Reveal All, Hide All, Reveal Selection, Hide Selection, From Transparency, Delete, Apply, Enable/Disable, Link/Unlink. |
| `Layer > Vector Mask` | Menu | n/a | Vector-mask counterparts (see `LAY-005`). |
| Properties panel (CS6) | Dock | n/a | Mask buttons, Density, Feather, Invert, Mask Edge, Color Range, Apply/Disable/Delete, eye. |
| Masks panel (CS5) | Dock | n/a | Superseded by Properties panel in CS6. |
| Layer Mask Display Options | Dialog | double-click mask thumbnail / mask channel | Rubylith color + opacity (display only). |
| Refine Mask dialog | Dialog | via Mask Edge | Refine Edge option set. |
| `Select > Refine Edge` | Menu | `Ctrl+Alt+R` | Shared refinement engine. |
| Channels panel | Dock | n/a | Layer masks are stored as alpha channels. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Mask kind | enum | none | Layer (pixel) mask | Vector mask is `LAY-005`. |
| Initial state | enum | Reveal All | Reveal All / Hide All / Reveal Selection / Hide Selection / From Transparency | Add Layer Mask modifier keys. |
| Mask value | 8/16/32-bit gray | white | 0 (black) … 255/65535/float (white) | 0 hides, max reveals; mid = partial. |
| Density | percent | 100 | 0–100 | Mask opacity. |
| Feather | double px | 0 | ≥ 0 (not stated) | Softens mask edges inward/outward. |
| Invert | bool | off | on/off | Layer masks only. |
| Mask Edge | dialog | n/a | Refine Edge set | Not available for filter masks. |
| Radius / Smart Radius | double px / bool | 0 / off | ≥ 0 | Refine Mask. |
| Smooth | int | 0 | not stated | Refine Mask. |
| Contrast | percent | 0 | not stated | Refine Mask. |
| Shift Edge | percent | 0 | negative = inward, positive = outward | Refine Mask. |
| Decontaminate Colors / Amount | bool / percent | off / 0 | 0–100 (not stated) | Changes pixel color; forces output to new layer/doc. |
| Rubylith color/opacity | color / percent | red / 50% (not stated) | 0–100% | Display only. |
| Link to layer | bool | on | on/off | Move independently when off. |
| Enabled | bool | on | on/off | Shift-click thumbnail. |

## Algorithms & pipeline

1. **Storage.** A layer mask is a single-channel grayscale buffer at the document
   bit depth, stored as an alpha channel. In PSD it occupies the layer record's
   user-layer-mask channel (id `-2`, or `-3` when both user and vector masks
   exist), per `ARCH-008`.
2. **Composite multiply.** At composite time the mask value multiplies the
   layer's contribution (and, where the layer style record enables it, restricts
   effects via **Layer Mask Hides Effects**). `final_alpha = layer_alpha ×
   mask_value × density`, before layer opacity/fill and blend. *(formula
   inferred; observable contract: black hides, white reveals, gray partial.)*
3. **Density** scales the mask toward fully-opaque-blocks-area at 100%; lower
   density interpolates the mask toward all-white (revealing more). *(inferred)*
4. **Feather** applies an edge blur within the selected radius from the mask
   edges outward; **Mask Edge** runs the Refine Edge pipeline (radius,
   smart-radius edge-detection, smooth, contrast, shift, decontaminate).
5. **From Transparency** reads the layer's alpha, converts fully-transparent
   pixels to hidden mask pixels and opaque pixels to revealed; the color left
   behind is implementation-dependent *(matching the Help's warning)*.
6. **Apply** bakes `layer_alpha ×= mask` and removes the mask channel; **Delete**
   discards it with no pixel change.
7. **Selection round-trip.** Ctrl/Cmd-clicking the mask thumbnail loads
   `mask > 0` (unmasked areas) as a selection; the inverse is masked areas.
   *(inferred from documented behavior.)*
8. **Rubylith display** is a viewer overlay only; it must never feed back into
   compositing.

## Rust module mapping

- `pictura_core::mask` — `RasterMask { buffer: MaskBuffer, density: u8, feather: f32, enabled: bool, linked: bool }`; `MaskBuffer` is single-channel at `BitDepth`.
- `pictura_core::mask::ops` — `add_reveal_all`, `add_hide_all`, `from_selection`, `from_transparency`, `invert`, `apply_to_layer`, `delete`.
- `pictura_core::mask::refine` — `RefineMaskParams` (radius, smart_radius, smooth, feather, contrast, shift_edge, decontaminate+amount, output) and the refinement kernel shared with `08-selection/refine-edge.md`.
- `pictura_core::mask::display` — `MaskDisplayOptions { rubylith_color, opacity }` (view-only).
- `pictura_core::composite::mask` — multiply into the layer contribution; gated by the `Layer Mask Hides Effects` blending flag.
- `pictura_core::selection` — bridge to/from `Selection` for Ctrl/Cmd-click load.
- Crossing types: `NodeId`, `MaskRef`, `BitDepth`, `Rect`, `Selection`, `Rgb8`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MaskPropertiesPage` | `QWidget` | Properties-panel page: mask chips/buttons, Density, Feather, Invert, Mask Edge, Color Range, apply/disable/delete. |
| `MaskThumbnailDelegate` | `QStyledItemDelegate` | Renders layer/vector mask thumbnails, active border, red X when disabled, link icon. |
| `LayerMaskDisplayOptionsDialog` | `QDialog` | Rubylith color + opacity (view-only). |
| `RefineMaskDialog` | `QDialog` | Reuses the Refine Edge dialog (View Mode, radius brushes, sliders, output). |
| `ColorRangeDialog` | `QDialog` | Mask confinement by color. |
| `LayersPanel` | (from `LAY-002`) | Hosts Add Layer Mask button and mask activation. |

## Data-model impact

- `Node.mask: Option<MaskRef>` (raster and/or vector, per `ARCH-008`).
- Persisted fields map to the PSD layer-mask record: default color (0/255), mask
  flags (enabled, invert, link), **density** and **feather** (present when the
  mask-parameters flag bit is set). The `-3` "real user mask" channel appears
  only when both a user mask and a vector mask exist.
- Mask buffers are pixel data → undo records use `pixel_backups` (pre-edit tiles)
  rather than copying the whole canvas (`ARCH-009`).
- Display options (rubylith color/opacity) are document/user state, not a
  pixel-level effect; store if CS6 round-trips them, otherwise as preferences.
- Applying/deleting a mask is a destructive command with pixel backups.

## Edge cases

- **Background layer.** Must be converted to a regular layer before adding a
  mask.
- **Locked layer / Background.** Mask add may be permitted on partial locks; full
  lock and background must refuse (documented for opacity; mask follows the same
  "protected content" principle) *(inferred)*.
- **Smart Object.** A layer mask can be applied to a Smart Object, but cannot be
  **applied permanently** when deleting it; smart filters also carry separate
  **filter masks** (no Mask Edge).
- **Both masks present.** The layer record's `-3` channel is used; both masks
  affect compositing.
- **1-bit Bitmap / Indexed.** Mask/color operations are restricted by the image
  mode.
- **32-bit.** Invert/Threshold are enabled for masks in CS6; mask buffers are
  float.
- **CMYK/Lab.** Masks are mode-independent grayscale; compositing in the working
  space.
- **Empty / 1-px documents.** Zero-length mask channels are legal.
- **PSB.** Mask channel lengths and offsets use PSB widths; tiling required.
- **Undo/redo.** Paint-on-mask and Refine Mask are single commands; apply/delete
  are reversible via pixel backups.
- **GPU unavailable.** Mask multiply and refinement run on the CPU reference.

## Parity acceptance criteria

1. Given a new mask via Add Layer Mask with no selection, the mask is Reveal All
   (white) and the layer is unchanged; Alt/Option-click yields Hide All (black).
2. Given a selection, Reveal Selection makes selected areas visible and Hide
   Selection makes them hidden.
3. Given painting on an active mask, black hides the layer (reveals layers
   below), white reveals, gray gives partial transparency.
4. Given Density 100%, masked areas are fully blocked; lowering Density reveals
   progressively more of the area under the mask.
5. Given a Feather value, the mask edge transitions over the specified pixel
   range.
6. Given Mask Edge → Refine Mask, the radius/smooth/contrast/shift/decontaminate
   options apply and Decontaminate forces output to a new layer/document.
7. Given Shift-click on the mask thumbnail, the mask toggles disabled (red X) and
   the layer renders unmasked.
8. Given Ctrl/Cmd-click on the mask thumbnail, the unmasked areas load as a
   selection; Ctrl/Cmd-click on the layer thumbnail instead loads non-transparent
   areas.
9. Given Apply Mask, hidden pixels are permanently removed and the mask is gone;
   Delete Mask removes it with no pixel change; both are undoable.
10. Given `Layer > Layer Mask > From Transparency`, transparent pixels become
    hidden by the new mask.
11. Given a PSD with a user mask plus a vector mask, both masks affect the
    composite and round-trip with density/feather.
12. Given a filter mask on a Smart Filter, black/white/gray paint semantics match
    but Mask Edge is unavailable.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Photoshop CS6 Help (fetched with `curl`, extracted with `pdftotext -layout`).
  Sections used: "About layers / Layers for non-destructive editing"
  (pp. 156–157); "Load selections from a layer or layer mask's boundaries"
  (p. 169); "Editing layer masks" (p. 170); "Masking layers" — About layer and
  vector masks, Add layer masks, Unlinking layers and masks, Disable/enable,
  Apply/delete, Select/display the mask channel, rubylith color/opacity,
  Adjusting mask opacity and edges (Density, Feather, Mask Edge) (pp. 176–179);
  "Refine selection edges" (pp. 212–213); "Mask Smart Filter effects"
  (pp. 187–188); "Confine adjustment and fill layers to specific areas"
  (pp. 288–289); "Keys for the Layers panel" (pp. 89–90); "What's new in CS6"
  (32-bit mask Invert/Threshold, p. 8).

## Open questions

- **Feather/density numeric ranges.** The Help states the effect but not the
  slider maxima for Feather or the exact density interpolation curve. *Resolves
  with:* a CS6 UI capture or pixel calibration.
- **Refine Mask slider ranges** (Radius, Smooth, Contrast, Shift Edge, Amount) are
  not itemized in the Help. *Resolves with:* CS6 dialog capture.
- **From Transparency color.** The opaque color produced is admitted to vary;
  whether it is deterministic is unknown. *Resolves with:* CS6 test PSDs.
- **Mask-parameters PSD round-trip.** The block layout is sourced from psd-tools
  (see `ARCH-008` open questions). Whether CS6 always writes density/feather
  records (flag bit 4) or only when non-default is not confirmed. *Resolves with:*
  a CS6 mask PSD byte inspection.
- **Rubylith defaults** (color and opacity on a fresh install) are not stated.
  *Resolves with:* a preference dump.
- **Applying a mask on a Smart Object** is prohibited when deleting the mask;
  the exact allowed/invalid operation matrix needs a controlled test.
