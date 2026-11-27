# ui/advanced-blending-controls Specification

## Purpose
Expose layer effect and mask blending flags in the Layer Style dialog.

## Requirements

### Requirement: Advanced blending flag controls

The Layer Style Blending Options page SHALL present Transparency Shapes Layer, Layer Mask Hides Effects, and Vector Mask Hides Effects after the existing group blending controls, in that order. Each checkbox SHALL reflect and edit its corresponding layer flag.

#### Scenario: Values and order
- **WHEN** the dialog opens for a layer with any combination of the three flags
- **THEN** the checkboxes appear in the specified order and show those values

#### Scenario: Live edit and cancel
- **WHEN** a checkbox changes and the dialog is cancelled
- **THEN** the change takes effect in the preview and cancellation restores the original flags

#### Scenario: Accept and undo
- **WHEN** the flags are changed and the dialog is accepted
- **THEN** the changes are a single undoable Layer Style edit and reopening displays the accepted flags

#### Scenario: Save and reopen
- **WHEN** the flags are changed and the document is saved as PSD or PSB and reopened
- **THEN** the dialog displays the saved values
