# ui/layers-panel-mask-menu Specification

## Purpose
The mask-thumbnail context menu: a right-click on a raster layer mask thumbnail
opens CS6's mask commands (enable/disable, delete, apply, load as selection) in
place of the generic row menu, acting on the clicked mask.

## Requirements

### Requirement: Layer mask thumbnail context menu

Right-clicking a raster layer mask thumbnail SHALL open a **mask context menu**
instead of the generic row context menu, and SHALL make that mask the view's mask
edit target (selecting its row) so the menu's commands act on it. When the row
carries no raster mask, the mask thumbnail has no menu and the generic row menu
SHALL be shown. Every command that changes the document SHALL record exactly one
undoable state. Commands the system does not yet implement SHALL appear disabled
with a `— not implemented yet` tooltip rather than being hidden.

#### Scenario: Right-clicking a mask thumbnail opens the mask menu [lmm_open]

- **WHEN** the user right-clicks the mask thumbnail of a raster-mask row
- **THEN** the mask context menu is shown (not the generic row menu) and the mask
  becomes the edit target with its focus border

#### Scenario: A row without a mask gets the generic menu [lmm_no_mask]

- **WHEN** the user right-clicks where a mask thumbnail would be on a row that
  carries no raster mask
- **THEN** no mask menu is shown

### Requirement: CS6 mask context menu commands

The mask context menu SHALL list, in this order: **Enable Layer Mask** /
**Disable Layer Mask** (rendered per the mask's current enabled state), then
**Delete Layer Mask** and **Apply Layer Mask**, then **Add Mask To Selection**,
**Subtract Mask From Selection**, and **Intersect Mask With Selection**, then
**Refine Mask…**, then **Mask Options…** (Layer Mask Display Options); a
separator SHALL divide each of those groups.

- **Delete Layer Mask** SHALL remove the mask without applying it.
- **Apply Layer Mask** SHALL fold the mask's coverage into the layer's pixel
  alpha (multiplying the existing alpha) and then remove the mask. It SHALL be
  disabled, and refused without changing the document, on a smart-object layer,
  which cannot take a permanent mask, and on a group or an adjustment/fill
  layer, which renders from its children or content rather than its own alpha.
- **Add/Subtract/Intersect Mask To/From/With Selection** SHALL load the mask's
  coverage as a pixel selection (white = selected) combined with `Add`,
  `Subtract`, or `Intersect` respectively.
- **Refine Mask…** SHALL be enabled and SHALL open Refine Mask on the clicked
  layer's mask (`ui/layers-mask-commands`).
- **Mask Options…** SHALL be enabled and SHALL open Layer Mask Display Options.

#### Scenario: The menu lists the CS6 items in order [lmm_items]

- **WHEN** the user opens the mask context menu for a raster-mask row
- **THEN** it contains Enable/Disable Layer Mask, Delete Layer Mask, Apply Layer
  Mask, Add Mask To Selection, Subtract Mask From Selection, Intersect Mask With
  Selection, Refine Mask…, and Mask Options… in that order

#### Scenario: The enable/disable label follows the state [lmm_label]

- **WHEN** the mask context menu is opened for an enabled mask, and again for a
  disabled mask
- **THEN** the first shows `Disable Layer Mask` and the second shows
  `Enable Layer Mask`

#### Scenario: Delete removes the mask without applying [lmm_delete]

- **WHEN** the user picks Delete Layer Mask for a masked layer
- **THEN** the mask is removed, the layer's pixels are unchanged, and one undo
  state is recorded

#### Scenario: Apply folds the mask into alpha [lmm_apply]

- **WHEN** the user picks Apply Layer Mask for a raster-masked, non-smart layer
- **THEN** the layer's alpha becomes its previous alpha multiplied by the mask
  coverage, the mask is removed, and one undo state is recorded

#### Scenario: Apply is disabled on a smart object [lmm_apply_smart]

- **WHEN** the mask context menu is opened for a smart-object layer
- **THEN** Apply Layer Mask is disabled

#### Scenario: Apply needs a layer that renders from its pixels [lmm_apply_pixel_only]

- **WHEN** the mask context menu is opened for a fill layer, and Apply Layer Mask is run on a group or a fill layer anyway
- **THEN** Apply Layer Mask is disabled, the run is refused, the mask stays, and no undo state is recorded

#### Scenario: The selection commands combine the mask coverage [lmm_selection]

- **WHEN** the user picks Add Mask To Selection for a mask
- **THEN** the mask's white areas are added to the pixel selection

#### Scenario: Refine Mask opens on the clicked mask [lmm_refine_mask]

- **WHEN** the user picks Refine Mask… from the mask context menu
- **THEN** the Refine Edge dialog opens titled "Refine Mask" on that layer's mask

#### Scenario: Mask Options opens the display options [lmm_mask_options]

- **WHEN** the user picks Mask Options… from the mask context menu
- **THEN** Layer Mask Display Options opens with the current rubylith colour and opacity
