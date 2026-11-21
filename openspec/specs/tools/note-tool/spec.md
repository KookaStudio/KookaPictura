# tools/note-tool Specification

## Purpose
Notes: text pinned to document points, placed with the Note tool and edited in the Notes panel.

## Requirements

### Requirement: Notes

Notes SHALL be stored on `Document::annotations` as a document pixel position
and text, without limit. The Notes panel SHALL show the current note's text,
its position among the document's notes, previous/next buttons, and Delete;
text typed into it SHALL be committed as one "Edit Note" state when the editor
loses focus or the panel moves to another note, and unchanged text SHALL record
nothing.

#### Scenario: Editing a note

- **WHEN** the current note's text is changed in the Notes panel and committed
- **THEN** one "Edit Note" state is recorded and the note holds the new text

### Requirement: Note tool

The Note tool SHALL add a note at a click on empty canvas as one "New Note"
state, make a clicked note current, and delete a note on Alt-click or when
dragged off the canvas as one "Delete Note" state; a dragged note SHALL move as
one "Move Note" state on release. A note becoming current SHALL show it in the
Notes panel and make the panel visible. The options bar's Clear All SHALL delete
every note as one "Delete All Notes" state. The canvas SHALL show every note as
a note glyph with any tool active while `View > Show > Notes` and `View > Extras`
are both on, outlining the current one. The Note tool itself SHALL keep them
visible while it is active when Extras is on; Extras SHALL hide them regardless
of the active tool. `View > Show > Notes` SHALL be a checkable command, on by
default, persisted in the session, and applied to every canvas.

#### Scenario: Adding and reopening notes

- **WHEN** the `annotation_tools` self-test adds a note, adds a second, and clicks the first
- **THEN** each click on empty canvas records one "New Note" state and the Notes panel shows the clicked note's text

#### Scenario: Deleting notes

- **WHEN** a note is Alt-clicked and then Clear All is used
- **THEN** the note count drops by one and then to zero, and no note is current

#### Scenario: Show gates note glyphs for other tools [note_show_gate]

- **WHEN** notes exist and another tool is active with Show > Notes on, then off
- **THEN** the glyphs are visible and then hidden, while the Note tool itself keeps them visible

#### Scenario: Extras hides note glyphs [note_extras]

- **WHEN** Show > Notes is on and Extras is turned off, then on
- **THEN** the glyphs hide and return with the Show checkmark unchanged

#### Scenario: Extras wins over the Note tool [note_tool_extras]

- **WHEN** the Note tool is active with glyphs visible and Extras is turned off, then on
- **THEN** the glyphs hide while Extras is off even with the Note tool active, and return with Extras on
