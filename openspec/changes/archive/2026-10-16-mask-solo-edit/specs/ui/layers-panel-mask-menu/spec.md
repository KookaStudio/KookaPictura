# ui/layers-panel-mask-menu Specification (delta)

## ADDED Requirements

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

### Requirement: Mask context menu commands

The mask context menu SHALL list, in this order: **Enable Layer Mask** /
**Disable Layer Mask** (rendered per the mask's current enabled state), then
**Delete Layer Mask** and **Apply Layer Mask**, then **Add Mask To Selection**,
**Subtract Mask From Selection**, and **Intersect Mask With Selection**, then
**Select and Mask…**, then the disabled placeholder **Mask Options…** (Layer Mask
Display Options); a separator SHALL divide each of those groups.

- **Delete Layer Mask** SHALL remove the mask without applying it.
- **Apply Layer Mask** SHALL fold the mask's coverage into the layer's pixel
  alpha (multiplying the existing alpha) and then remove the mask. It SHALL be
  disabled on a smart-object layer, which cannot take a permanent mask.
- **Add/Subtract/Intersect Mask To/From/With Selection** SHALL load the mask's
  coverage as a pixel selection (white = selected) combined with `Add`,
  `Subtract`, or `Intersect` respectively.
- **Select and Mask…** SHALL be enabled and SHALL open the Refine Edge editor
  (the same dialog the selection tools' Select and Mask button opens).

#### Scenario: The menu lists the CS6 items in order [lmm_items]

- **WHEN** the user opens the mask context menu for a raster-mask row
- **THEN** it contains Enable/Disable Layer Mask, Delete Layer Mask, Apply Layer
  Mask, Add Mask To Selection, Subtract Mask From Selection, Intersect Mask With
  Selection, Select and Mask…, and Mask Options… in that order

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

#### Scenario: The selection commands combine the mask coverage [lmm_selection]

- **WHEN** the user picks Add Mask To Selection for a mask
- **THEN** the mask's white areas are added to the pixel selection

#### Scenario: Select and Mask opens Refine Edge [lmm_select_and_mask]

- **WHEN** the user picks Select and Mask… from the mask context menu
- **THEN** the Refine Edge editor opens

#### Scenario: Mask Options is a disabled placeholder [lmm_placeholders]

- **WHEN** the mask context menu is opened
- **THEN** Mask Options… is present but disabled with the `— not implemented yet`
  tooltip
