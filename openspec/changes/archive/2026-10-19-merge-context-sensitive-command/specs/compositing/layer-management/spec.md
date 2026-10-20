# Spec Delta

## REMOVED Requirements

### Requirement: Merge Down and Merge Layers

**Reason**: The separate `Layer > Merge Down` leaf was a regression against the
single context-sensitive CS6 command. It left `Merge Down` enabled with a
multi-selection and merged only the current row.

**Migration**: Use the single context-sensitive merge command. It reads
`Merge Down` with exactly one selected layer and `Merge Layers` with two or more,
and `Ctrl+E` dispatches whichever applies.

### Requirement: Merge Down as a distinct command

**Reason**: This requirement directly contradicts the single context-sensitive
CS6 command and the command-registry spec's `layer.merge.layers` entry. Keeping a
second leaf is what caused issue #744.

**Migration**: Use the single context-sensitive merge command. There is no
separate `Merge Down` command, action, or shortcut binding.

## ADDED Requirements

### Requirement: Context-sensitive Merge Down / Merge Layers

The system SHALL provide one selection-dependent merge command bound to `Ctrl+E`.
With exactly one selected layer the command SHALL be labelled **Merge Down** and
SHALL perform Merge Down. With two or more selected layers the same command SHALL
be labelled **Merge Layers** and SHALL perform Merge Layers; it MUST NOT perform
Merge Down in that state. With no selected layer the command SHALL be disabled
and SHALL read **Merge Layers**. The system SHALL NOT provide a separate
`Layer > Merge Down` command.

The merge SHALL validate before mutating: the merge target (the lower layer for
Merge Down, the bottommost selected layer for Merge Layers) MUST NOT be an
adjustment or fill-content layer, and Merge Down MUST have a layer directly below
the selected layer. Merge Down SHALL require exactly one selected layer and MUST
be refused when there is nothing below it. A refused merge MUST leave the
document unchanged and MUST NOT add a history state.

The command SHALL composite the inputs in stacking order through the document
compositor over the union of their content rectangles, apply each input's mask
into the resulting alpha, and replace the inputs with one pixel node. Clipping
coverage is folded into the base alpha only by `Merge Clipping Mask`; the
compositor has no clipping pass yet, so Merge Down / Merge Layers / Merge
Visible share that documented gap rather than diverging. For Merge Down the
result SHALL inherit the lower layer's name, blend mode, and opacity
*(inferred)*. For Merge Layers the result SHALL occupy the topmost selected
layer's position and its blend mode and opacity SHALL be reset to `Normal` and
`255` *(inferred)*. Sibling order and parent-group references SHALL be repaired
so the result sits where the inputs were removed. Every merge SHALL recomposite
and record exactly one undo state.

The Layers panel merge entry SHALL follow the same label and enablement rules as
the Layer-menu command: `Merge Down` with one selected layer, `Merge Layers` with
two or more, and disabled when the current selection cannot merge.

#### Scenario: Merge Down composites two layers

- **WHEN** a pixel layer sits directly above another pixel layer and Merge Down
  runs
- **THEN** the two layers are replaced by one pixel layer whose pixels equal the
  compositor's composite of the pair within the compositor tolerance, whose
  alpha is the union of their nontransparent areas, and which inherits the lower
  layer's name, blend mode, and opacity

#### Scenario: Merge Layers replaces the selection at the topmost position

- **WHEN** more than one layer is selected and `Ctrl+E` runs
- **THEN** the selected layers are replaced by one pixel layer at the topmost
  selected position whose pixels equal their composite and whose blend mode and
  opacity are `Normal` and `255`

#### Scenario: The merge label follows a single selection

- **WHEN** exactly one layer is selected and the Layer menu is opened
- **THEN** the merge command reads `Merge Down`

#### Scenario: The merge label follows a multiple selection

- **WHEN** two or more layers are selected and the Layer menu is opened
- **THEN** the same merge command reads `Merge Layers` and no separate
  `Merge Down` command is shown

#### Scenario: No separate Merge Down item remains

- **WHEN** the Layer menu is opened with exactly one selected layer
- **THEN** exactly one merge item is present, labelled `Merge Down`, and no
  second `Merge Down` entry exists

#### Scenario: The merge command is disabled with no selection

- **WHEN** no layer is selected
- **THEN** the merge command is disabled and reads `Merge Layers`

#### Scenario: Ctrl+E merges down with one selected layer

- **WHEN** exactly one layer is selected and `Ctrl+E` runs
- **THEN** that layer is merged into the layer directly below it and exactly one
  history state is recorded

#### Scenario: Ctrl+E merges only the selected layers

- **WHEN** two or more layers are selected and `Ctrl+E` runs
- **THEN** only the selected layers are replaced by one pixel layer and exactly
  one history state is recorded

#### Scenario: A multi-selection cannot run Merge Down

- **WHEN** two or more layers are selected
- **THEN** the merge command performs Merge Layers, the current row is not merged
  down on its own, and no Merge Down command is dispatchable

#### Scenario: The Layers panel merge label follows the selection

- **WHEN** the Layers panel menu is opened with exactly one selected layer
- **THEN** the merge entry reads `Merge Down`, and with two or more selected
  layers the same entry reads `Merge Layers`

#### Scenario: The Layers panel merge entry is disabled when the merge cannot run

- **WHEN** the Layers panel menu is opened with only the bottom layer selected
- **THEN** the merge entry is disabled and reads `Merge Down`

#### Scenario: An adjustment target is refused

- **WHEN** the merge target (the lower layer for Merge Down, the bottommost
  selected layer for Merge Layers) is an adjustment layer and merge runs
- **THEN** the merge is refused, the document is unchanged, and no history state
  is added

#### Scenario: Merge Down needs a layer below

- **WHEN** the bottom layer of the stack is the only selected layer and Merge
  Down runs
- **THEN** the merge is refused, the document is unchanged, and no history state
  is added

#### Scenario: A merge is one undo step

- **WHEN** a merge completes and the user undoes it
- **THEN** the exact pre-merge layer tree, kinds, names, masks, and pixels are
  restored
