# document-lifecycle

## ADDED Requirements

### Requirement: New Document background color swatch

The New Document dialog SHALL show a color swatch to the right of its Background
Contents list. The list SHALL offer White, Background Color, and Transparent,
and SHALL offer Custom only after a custom color has been chosen. The swatch
SHALL show the fill color the current choice would use (white, the toolbox
Background Color, or the chosen custom color) and SHALL be disabled while
Transparent is selected. Clicking an enabled swatch SHALL open the color picker
for the shown color; accepting a color SHALL add or select the Custom choice and
make that color the new document's initial fill; cancelling SHALL leave the
choice and the swatch unchanged. Confirming the dialog with Custom SHALL create
the document filled with the chosen color as its locked Background layer.

#### Scenario: The swatch follows Background Contents [ndb_swatch_follows_contents]

- **WHEN** the New Document dialog opens, the user chooses Background Color, and
  then chooses Transparent
- **THEN** the list offers White, Background Color, and Transparent, the swatch
  shows white, then the toolbox Background Color, and is disabled for
  Transparent

#### Scenario: A picked color becomes the initial background [ndb_swatch_pick]

- **WHEN** the user clicks the swatch, picks a color, and confirms the dialog
- **THEN** the list adds and selects Custom, the swatch shows the picked color,
  and the new document's locked Background layer is filled with it

#### Scenario: Cancelling the picker keeps the choice [ndb_swatch_cancel]

- **WHEN** the user clicks the swatch from Custom and cancels the color picker
- **THEN** Custom stays selected, and the swatch and the initial fill are
  unchanged

#### Scenario: Transparent disables the swatch [ndb_swatch_transparent]

- **WHEN** the user chooses Transparent in Background Contents
- **THEN** the swatch is disabled and the new document has a transparent
  initial layer
