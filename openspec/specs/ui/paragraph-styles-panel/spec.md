# ui/paragraph-styles-panel Specification

## Purpose
Provides the Paragraph Styles panel: the document's named paragraph styles,
applied to the active type layer, and the options dialog that creates and
edits them.

## Requirements

### Requirement: Paragraph Styles panel lists and applies document styles

The Paragraph Styles panel SHALL list the active document's paragraph styles in
panel order, with the default `Basic Paragraph` style first. A single click on a
row SHALL apply that style to the active type layer through the type bridge,
re-rendering the layer and recording exactly one history state. When no type
layer is active, a click SHALL record no state.

#### Scenario: Panel lists the document styles

- **WHEN** a document with the default style and a style named `Heading` is shown
- **THEN** the panel lists `Basic Paragraph` then `Heading`

#### Scenario: Clicking a style applies it

- **WHEN** a style row is clicked with a type layer active and then undone once
- **THEN** the layer's character attributes reflect the style and the single undo restores the previous attributes

#### Scenario: No type layer records nothing

- **WHEN** a style row is clicked with no active type layer
- **THEN** no history state is recorded

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

### Requirement: Window and Type menus toggle the Paragraph Styles panel

`Window > Panels > Paragraph Styles` and `Type > Panels > Paragraph Styles` SHALL
be enabled checkable commands with their own command ids. Checking one SHALL show
the panel in the panel column, unchecking SHALL hide it, and each menu item's
checked state SHALL reflect the panel's actual visibility.

#### Scenario: Both menu paths toggle the panel

- **WHEN** either menu entry is checked and then unchecked
- **THEN** the Paragraph Styles panel becomes visible and then hidden, and that menu item's checked state follows

### Requirement: Paragraph Style Options dialog edits every modelled attribute

The Paragraph Style Options dialog SHALL present the style's name and all seven
CS6 pages — Basic Character Formats, Advanced Character Formats, OpenType
Features, Indents and Spacing, Composition, Justification, and Hyphenation —
with each control backed by the type model's character and paragraph attribute
sets. Accepting the dialog SHALL write every attribute back through the bridge.
While editing an existing style, the Preview checkbox SHALL be checked by
default, and a control change SHALL re-apply the style to every type layer that
applies it without recording a history state; cancelling SHALL restore the
style's opening attributes without recording history.

#### Scenario: The dialog offers all seven pages

- **WHEN** an existing style is opened for editing
- **THEN** the page list holds the seven CS6 page titles and Preview is checked

#### Scenario: Accepted options round-trip into the style

- **WHEN** the OpenType Fractions and Faux Bold boxes are checked, auto leading is set, and the dialog is accepted
- **THEN** reading the style back reports those values

#### Scenario: Preview changes the style without history and Cancel restores it

- **WHEN** a size change is made with Preview checked and the dialog is cancelled
- **THEN** the style reports the changed size while the dialog is open, no history state is recorded, and the opening size is restored on cancel
