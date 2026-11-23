# Spec Delta

## MODIFIED Requirements

### Requirement: Paragraph Styles panel manages styles

The panel's footer SHALL create a new style through the Paragraph Style Options
dialog and SHALL delete the selected style, each recording exactly one history
state. Double-clicking a row SHALL open it in the options dialog and, on accept,
apply the dialog's name and attribute changes as one named-style edit that
re-resolves, where the document's type layers are editable, every type layer
applying the style under its new name, recording exactly one history state.
Accepting a rename to a name already used by another style, a rename of
`Basic Paragraph`, or a rename of another style to
`Basic Paragraph` SHALL be refused: the panel SHALL return failure to its
caller, leave the style list unchanged, and restore the style and its applying
layers to the attributes they carried when the dialog opened, recording no
history state. The default `Basic Paragraph` style SHALL NOT be deletable.

#### Scenario: Deleting the default style is refused

- **WHEN** the `Basic Paragraph` row is selected and the delete action is invoked
- **THEN** the style remains and no history state is recorded

#### Scenario: Deleting another style removes it

- **WHEN** a non-default style is selected and the delete action is invoked
- **THEN** the style is removed from the list and one undo restores it

#### Scenario: Renaming an applied style keeps it applied

- **WHEN** a style applied to a type layer is renamed in the options dialog and the dialog is accepted
- **THEN** the layer still applies the style under its new name, the style's changed attributes reach the layer, and one undo restores the old name and attributes on the style and the layer

#### Scenario: A refused rename leaves the style unchanged

- **WHEN** the dialog is accepted with a target name that already belongs to another style, or with `Basic Paragraph` as the source or target of a rename
- **THEN** the panel returns failure to its caller, the list still shows the original style, the applying layers keep their attributes from before the dialog opened, and no history state is recorded
