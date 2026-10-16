## ADDED Requirements

### Requirement: Properties mask section with refinement controls

When the active layer carries a layer mask, the Properties panel SHALL show a
Mask section containing a mask name row and the actions Enable, Disable, Link,
Unlink, Delete, and Apply, each wired to the layer-mask bridge and each one
undoable step, plus a Density control (0-100 %), a Feather control
(0-250 px, one decimal), and an Invert button. Density and Feather MUST show
the mask's stored values, preview while dragged, and record one undo step per
committed change; Invert MUST flip the mask pixels in one undo step. The
section SHALL be absent for a layer without a mask, and the existing adjustment
page behavior SHALL be unchanged.

#### Scenario: A masked layer shows the section [lmk_props]

- **WHEN** the active layer carries a mask
- **THEN** the Properties panel shows the Mask section with the mask name row
  and the Enable/Disable, Link/Unlink, Delete, and Apply actions

#### Scenario: The section is absent without a mask [lmk_props_absent]

- **WHEN** the active layer has no mask
- **THEN** the Properties panel shows no Mask section

#### Scenario: Delete clears the mask from the panel [lmk_props_delete]

- **WHEN** the user activates Delete in the Mask section
- **THEN** the mask is removed from the active layer in one undo step and the section disappears

#### Scenario: Density and Feather edit the mask [lmk_props_refine]

- **WHEN** the user sets Density to 50 % and Feather to 4 px in the Mask section
- **THEN** the active layer's mask stores density 128 and feather 4, each change one undo step

#### Scenario: Invert flips the mask [lmk_props_invert]

- **WHEN** the user activates Invert in the Mask section
- **THEN** the mask's pixels are inverted in one undo step

## REMOVED Requirements

### Requirement: Properties mask section

**Reason**: Its disabled Density, Feather, and Invert placeholders become live controls; the requirement is restated as "Properties mask section with refinement controls".
