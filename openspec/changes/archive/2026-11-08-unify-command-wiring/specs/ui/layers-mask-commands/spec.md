# Spec Delta

## MODIFIED Requirements

### Requirement: Row menu mask commands follow the mask state

The Layers panel row menu SHALL expose the mask creation commands as an **Add
Layer Mask** submenu containing Reveal All, Hide All, Reveal Selection, Hide
Selection, and From Transparency. The submenu SHALL be enabled only when the
clicked layer has no layer mask, and Reveal Selection and Hide Selection SHALL
be enabled only while a selection exists. It SHALL enable Delete Layer Mask only
when the layer has a mask. It SHALL enable Enable Layer Mask only when that mask
is disabled, and Disable Layer Mask only when it is enabled. The enabled state
SHALL come from the registry command's enable provider.

#### Scenario: A layer with an enabled mask [lmc_row_menu_enabled_mask]

- **WHEN** the row menu opens on a layer whose layer mask is enabled
- **THEN** Delete Layer Mask and Disable Layer Mask are enabled, and the Add Layer Mask submenu and Enable Layer Mask are greyed out

#### Scenario: A layer without a mask, and with a disabled one [lmc_row_menu_states]

- **WHEN** the row menu opens on a layer without a mask, and then on one whose mask is disabled
- **THEN** only the Add Layer Mask submenu is enabled for the first, and Delete and Enable Layer Mask are enabled for the second

#### Scenario: The Add Layer Mask submenu follows the selection [lmc_row_menu_submenu]

- **WHEN** the Add Layer Mask submenu is opened with a selection active and again with none
- **THEN** Reveal Selection and Hide Selection are enabled in the first and disabled in the second

## ADDED Requirements

### Requirement: Layer menu mask commands share the mask surfaces' availability

The Layer menu's mask commands SHALL use the same availability as the
mask-thumbnail menu and the Properties panel. Apply Layer Mask SHALL be
disabled, and refused without changing the document, on a smart-object layer and
on a group or an adjustment/fill layer, matching the mask-thumbnail menu. The
menu bar, the mask-thumbnail menu, and the Properties panel SHALL agree on a
mask command's enabled state.

#### Scenario: Apply is disabled in the Layer menu on a smart object [lmc_layer_menu_apply_smart]

- **WHEN** a smart-object layer is active and the Layer menu's mask submenu is opened
- **THEN** Apply Layer Mask is disabled

#### Scenario: The mask entry points agree [lmc_mask_entry_agreement]

- **WHEN** a group or an adjustment/fill layer is active
- **THEN** Apply Layer Mask is disabled in the Layer menu, the mask-thumbnail menu, and the Properties panel
