## ADDED Requirements

### Requirement: Row menu mask commands follow the mask state

The Layers panel row menu SHALL enable Add Layer Mask only when the clicked
layer has no layer mask. It SHALL enable Delete Layer Mask only when the layer
has one. It SHALL enable Enable Layer Mask only when that mask is disabled, and
Disable Layer Mask only when it is enabled.

#### Scenario: A layer with an enabled mask [lmc_row_menu_enabled_mask]

- **WHEN** the row menu opens on a layer whose layer mask is enabled
- **THEN** Delete Layer Mask and Disable Layer Mask are enabled, and Add Layer Mask and Enable Layer Mask are greyed out

#### Scenario: A layer without a mask, and with a disabled one [lmc_row_menu_states]

- **WHEN** the row menu opens on a layer without a mask, and then on one whose mask is disabled
- **THEN** only Add Layer Mask is enabled for the first, and Delete and Enable Layer Mask are enabled for the second

### Requirement: The mask button follows the active layer's masks

The Layers panel mask button SHALL read "Add layer mask" and add a layer mask
when the active layer has no layer mask. When the active layer has a layer mask
but no vector mask, it SHALL read "Add vector mask" and add a Reveal All vector
mask, or Hide All with Alt. When the active layer has both masks, it SHALL be
disabled and read "Add a mask". Deleting the layer mask SHALL return the button
to "Add layer mask".

#### Scenario: The button walks the mask states [lmc_mask_button]

- **WHEN** a layer gains a layer mask through the button, then a vector mask through the button, and then loses both
- **THEN** the tooltip reads "Add layer mask", then "Add vector mask", then "Add a mask" with the button disabled, and finally "Add layer mask" again

### Requirement: Refine Mask

Refine Mask… SHALL open the Refine Edge dialog titled "Refine Mask", with the
clicked layer's mask as its source and Layer Mask as its preselected output. The
source SHALL be the mask's stored coverage, whether or not the mask is enabled.
OK SHALL refine the mask with the dialog's settings and write the result to the
chosen output as one "Refine Mask" undo state. A Layer Mask output SHALL replace
the mask of any layer that has one, whatever the layer renders from, and SHALL
keep the existing mask's link, density and feather settings. A preview update
MUST NOT materialize the source at document size.

#### Scenario: Refining a hard mask [lmc_refine_mask]

- **WHEN** Refine Mask runs with a feather on a layer whose mask has a hard edge, and outputs to Layer Mask
- **THEN** the dialog is titled "Refine Mask", the mask edge becomes partial coverage, the mask stays linked, and one "Refine Mask" state is recorded

#### Scenario: Refining the mask of a fill layer [lmc_refine_mask_any_layer]

- **WHEN** Refine Mask runs on a solid-colour fill layer's mask and outputs to Layer Mask
- **THEN** the mask is replaced and one "Refine Mask" state is recorded

#### Scenario: The preview reads the source at preview size [lmc_refine_preview_size]

- **WHEN** a Refine Mask preview samples a mask
- **THEN** it equals the document-sized stored coverage sampled at the preview size, and a disabled mask reads its stored coverage

### Requirement: Layer Mask Display Options and the rubylith overlay

Mask Options… SHALL open Layer Mask Display Options, with a colour (red by
default) and an opacity from 0 to 100% (50% by default) for the rubylith
overlay. Alt+Shift-clicking a mask thumbnail SHALL toggle the overlay for that
layer: the mask's hidden areas tinted in the rubylith colour at the chosen
opacity. The overlay and its options SHALL be display-only and MUST NOT change
the composite or the document. The overlay SHALL belong to the current layer:
the toggle SHALL select the clicked row, and the overlay SHALL end when another
layer becomes current, when the layer stack is edited, or when the document is
replaced.

#### Scenario: Toggling the overlay [lmc_rubylith_toggle]

- **WHEN** a mask thumbnail is Alt+Shift-clicked twice
- **THEN** the canvas shows the rubylith overlay after the first click and none after the second, and the composite is unchanged throughout

#### Scenario: The overlay follows the options [lmc_rubylith_options]

- **WHEN** the display options are set to a colour and an opacity
- **THEN** a fully hidden mask pixel's overlay is that colour at that opacity, a fully revealed pixel's overlay is clear, and the defaults are red at 50%

#### Scenario: The overlay ends with its layer [lmc_rubylith_follows_the_layer]

- **WHEN** the overlay is shown and its layer is moved, or another layer becomes current
- **THEN** the overlay is off and the canvas draws none
