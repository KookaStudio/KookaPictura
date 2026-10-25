# ui/character-styles-panel Specification

## Purpose

Provides the Character Styles panel: the active document's named character
styles, applying one to the active type layer, and the options dialog that
creates and edits them.

## ADDED Requirements

### Requirement: Character Styles panel lists and applies document styles

The Character Styles panel SHALL list the active document's character styles in
creation order. A single click on a row SHALL apply that style to the active type
layer through the type bridge, re-rendering the layer and recording exactly one
history state. When no type layer is active, a click SHALL record no state. An
empty document SHALL show an empty list, not an error.

#### Scenario: Panel lists the document styles

- **WHEN** a document with character styles named `Heading` and `Caption` is shown
- **THEN** the panel lists `Heading` then `Caption`

#### Scenario: Clicking a style applies it

- **WHEN** a character-style row is clicked with a type layer active and then undone once
- **THEN** the layer's character attributes reflect the style and the single undo restores the previous attributes

#### Scenario: No type layer records nothing

- **WHEN** a character-style row is clicked with no active type layer
- **THEN** no history state is recorded

#### Scenario: An empty list is valid

- **WHEN** the active document has no character styles
- **THEN** the panel shows an empty list and the delete action is disabled

### Requirement: Character Styles panel manages styles

The panel's footer SHALL create a new style through the Character Style Options
dialog and SHALL delete the selected style, each recording exactly one history
state. Double-clicking a row SHALL open it in the options dialog and, on accept,
edit the style, re-applying it to every type layer that applies it as one state.
The delete action SHALL be enabled only while a row is selected.

#### Scenario: Creating a style adds it to the document

- **WHEN** the footer's create action is invoked, the dialog is accepted with the name `Quote` and Faux Italic checked
- **THEN** the list contains `Quote` and reading the style reports Faux Italic

#### Scenario: Deleting a style removes it

- **WHEN** a style is selected and the delete action is invoked
- **THEN** the style is removed from the list and one undo restores it

#### Scenario: Editing a style updates every applying layer

- **WHEN** a style applied to a type layer is edited to a new size and the dialog is accepted
- **THEN** the layer re-renders at the new size and one undo restores the old size

#### Scenario: Renaming an applied style keeps it applied

- **WHEN** a style applied to a type layer is renamed and the dialog is accepted
- **THEN** the layer still applies the style under its new name and one undo restores the old name on the style and the layer

### Requirement: Character Style Options dialog edits character attributes

The Character Style Options dialog SHALL present the style's name and the
character attribute pages — Basic Character Formats, Advanced Character Formats,
and OpenType Features — with every control backed by the type model's character
attribute set. Accepting the dialog SHALL write every attribute back through the
bridge. While editing an existing style, the Preview checkbox SHALL be checked by
default, and a control change SHALL re-apply the style to every type layer that
applies it without recording a history state; cancelling SHALL restore the
style's opening attributes without recording history.

#### Scenario: The dialog offers the character pages

- **WHEN** an existing style is opened for editing
- **THEN** the page list holds the three character page titles and Preview is checked

#### Scenario: Accepted options round-trip into the style

- **WHEN** Faux Bold and OpenType Fractions are checked and the dialog is accepted
- **THEN** reading the style back reports both values

#### Scenario: Preview changes the style without history and Cancel restores it

- **WHEN** a size change is made with Preview checked and the dialog is cancelled
- **THEN** the style reports the changed size while the dialog is open, no history state is recorded, and the opening size is restored on cancel

### Requirement: Window and Type menus toggle the Character Styles panel

`Window > Panels > Character Styles` and `Type > Panels > Character Styles` SHALL
be enabled checkable commands with their own command ids. Checking one SHALL show
the panel in the panel column, unchecking SHALL hide it, and each menu item's
checked state SHALL reflect the panel's actual visibility.

#### Scenario: Both menu paths toggle the panel

- **WHEN** either menu entry is checked and then unchecked
- **THEN** the Character Styles panel becomes visible and then hidden, and that menu item's checked state follows

### Requirement: The Character Styles icon is bundled and recorded

The Character Styles panel icon SHALL use the command id
`window.panels.characterStyles` as its icon id, SHALL be bundled through the Qt
resource, SHALL resolve through `icon(id)` like every other panel icon, and
SHALL be recorded in `assets/icons/lucide-map.json` and the provenance register.

#### Scenario: The panel icon is registered [character_styles_icon]

- **WHEN** `icon("window.panels.characterStyles")` is requested at 16 px and the provenance guard runs
- **THEN** the icon is non-null and the asset has a recorded provenance entry
