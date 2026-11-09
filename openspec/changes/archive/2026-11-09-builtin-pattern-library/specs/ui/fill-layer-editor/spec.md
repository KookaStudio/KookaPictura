# Spec Delta

## MODIFIED Requirements

### Requirement: Pattern Fill editor

The system SHALL edit a Pattern Fill layer through a dialog listing the patterns
decoded from the open document followed by the built-in pattern library, with
Scale, Link With Layer, and Snap To Origin. Accepting SHALL replace the layer's
`PtFl` content in one undoable state; cancelling SHALL leave the layer unchanged.
When the document has no embedded patterns, the built-in library SHALL still
provide entries, so the dialog SHALL show its empty state only when no pattern is
available at all. Choosing a built-in pattern SHALL embed it in the document's
`Patt` block in the same edit, before the layer references it, so the fill
renders and the saved PSD carries it. A document pattern with the same id SHALL
win over the built-in. `Edit > Define Pattern` remains outside this capability.

#### Scenario: A pattern fill is edited [fle_pattern_apply]

- **WHEN** the document contains a pattern, the user selects it with a scale and link choice in the Pattern Fill editor, and accepts
- **THEN** the layer's `PtFl` block carries that pattern id, scale, and link flag, the composite updates, and one history state is recorded

#### Scenario: Snap To Origin resets the offset [fle_pattern_snap]

- **WHEN** the user activates Snap To Origin and accepts
- **THEN** the pattern's origin offset is `(0, 0)`

#### Scenario: A blank document offers the built-ins [fle_pattern_builtins]

- **WHEN** the document has no pattern resources
- **THEN** the Pattern Fill editor lists the built-in patterns rather than the empty state, and creating or editing a pattern fill with one embeds it in the document

#### Scenario: No patterns disables the editor [fle_pattern_empty]

- **WHEN** no document pattern and no built-in pattern are available
- **THEN** the Pattern Fill editor reports the empty state and Pattern creation and editing are unavailable
