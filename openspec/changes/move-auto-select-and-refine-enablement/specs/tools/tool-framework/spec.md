## MODIFIED Requirements

### Requirement: Move options-bar controls

The Move tool's options bar SHALL offer an `Auto-Select` checkbox, a
Group/Layer choice that is enabled only while `Auto-Select` is checked
(unchecked by default), a `Show Transform Controls` checkbox, the align and
distribute button sets, and a three-dots menu drawn with the Lucide `ellipsis`
icon listing the align and distribute actions plus an `Align To:` choice of
Selection or Canvas. Choosing `Align To: Canvas` SHALL align to the canvas
bounds; `Selection` SHALL align to the active selection. While `Auto-Select` is
checked, a Move press on the canvas with no active selection SHALL make the
topmost visible layer whose content covers the press point the active layer —
the layer itself in Layer mode, its top-level ancestor group in Group mode — and
SHALL keep the active layer when no layer covers the point.

#### Scenario: Align To Canvas [ltf_move_align_canvas]

- **WHEN** `Align To: Canvas` is chosen and an align action is invoked
- **THEN** the selected layers are aligned to the canvas bounds

#### Scenario: Auto-Select and transform controls are present [ltf_move_controls]

- **WHEN** the Move options bar is shown
- **THEN** the `Auto-Select` checkbox, the Group/Layer choice, `Show Transform
  Controls`, align, distribute, and ellipsis-icon three-dots controls are
  present, and the Group/Layer choice is disabled while `Auto-Select` is
  unchecked

#### Scenario: Auto-Select Layer picks the topmost covering layer [ltf_move_auto_select]

- **WHEN** Auto-Select is on in Layer mode and the Move tool is pressed on a
  point covered by two stacked pixel layers
- **THEN** the topmost layer covering the point becomes the active layer

#### Scenario: Auto-Select Group picks the ancestor group [ltf_move_auto_select_group]

- **WHEN** Auto-Select is on in Group mode and the Move tool is pressed on a
  point covered by a layer inside a group
- **THEN** the top-level ancestor group becomes the active layer

## ADDED Requirements

### Requirement: Options-bar action buttons mirror command enablement

An options-bar button that dispatches a command SHALL be enabled exactly when
that command is enabled for the active document, so a click can never be a
silent no-op. The selection tools' `Select and Mask…` button SHALL therefore be
disabled without a document or without a non-empty selection, matching
`Select ▸ Refine Edge…`.

#### Scenario: The button follows the command [ltf_options_command_enablement]

- **WHEN** the active document has no non-empty selection
- **THEN** the `Select and Mask…` button is disabled, and it becomes enabled
  once a non-empty selection exists
