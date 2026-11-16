## ADDED Requirements

### Requirement: Duplicate Channel command

The Channels panel menu SHALL offer `Duplicate Channel…` and SHALL enable it
only when the current row is a colour or alpha channel row of an open document.
The command SHALL open a dialog with a Name field, a Destination list, and an
Invert checkbox. The Destination list SHALL contain every open document with the
same pixel dimensions as the source, including the source, followed by a `New`
entry. The default name SHALL be `<source channel name> copy`, and the default
destination SHALL be the active document.

Accepting the dialog SHALL add one extra channel to the destination with the
source coverage, the requested name, and `255 - coverage` when Invert is
checked. A same-document or same-size cross-document duplicate SHALL record
exactly one `Duplicate Channel` history state in the destination and SHALL NOT
record history in the source. The `New` destination SHALL create a grayscale
document holding the coverage as its extra channel. A destination at the
56-channel limit SHALL refuse without adding a channel or recording history.

#### Scenario: The menu opens the dialog [cp_duplicate_menu]

- **WHEN** an alpha channel row is current and `Duplicate Channel…` is invoked
- **THEN** the dialog lists the same-size documents and `New`, defaults the name
  to `<source> copy`, and defaults the destination to the active document

#### Scenario: A colour row duplicates [cp_duplicate_color_row]

- **WHEN** the current row is a colour row and the dialog is accepted
- **THEN** the new extra channel holds that row's grayscale plane

#### Scenario: Duplicate within the document [cp_duplicate_same_doc]

- **WHEN** the dialog is accepted with the source document as the destination
- **THEN** a new channel with the requested name holds the source coverage and
  the document gains exactly one `Duplicate Channel` state

#### Scenario: Duplicate into another document [cp_duplicate_cross_doc]

- **WHEN** the dialog is accepted with another same-size document
- **THEN** the target gains the named channel and one `Duplicate Channel` state,
  and the source document's channel count and history are unchanged

#### Scenario: Duplicate into a new document [cp_duplicate_new_doc]

- **WHEN** the dialog is accepted with `New`
- **THEN** a grayscale document opens with the coverage as its named extra
  channel

#### Scenario: Invert complements the channel [cp_duplicate_invert]

- **WHEN** the dialog is accepted with Invert checked
- **THEN** the new channel holds `255 - source` for every pixel

#### Scenario: Dragging onto New Channel duplicates [cp_duplicate_button_drag]

- **WHEN** a colour or alpha row is dropped on the Create New Channel button
- **THEN** the current document gains one named copy of that row and exactly one
  `Duplicate Channel` state

#### Scenario: Dragging onto another document duplicates [cp_duplicate_document_drag]

- **WHEN** a colour or alpha row is dropped on another document's tab or canvas
- **THEN** the target gains one named copy of the row and exactly one
  `Duplicate Channel` state, the source is unchanged, and a different target
  size scales the coverage to fit and centre it

#### Scenario: The channel cap refuses [cp_duplicate_cap]

- **WHEN** the destination already holds 56 channels
- **THEN** the duplicate is refused with no new channel and no history state

### Requirement: Cross-document channel drag

Dragging a colour or alpha channel row SHALL carry the source document, row,
alpha index, visible-channel mask, and display name. Dropping it on another
document's tab or canvas SHALL duplicate that row into the target without
requiring equal dimensions. The target coverage SHALL be the source row coverage
scaled to fit and centred when the sizes differ, and exact when they match. The
source channel SHALL remain unchanged. Dropping on the source document or
outside a document SHALL do nothing.

#### Scenario: A same-size channel drag is exact [cp_channel_drag_same_size]

- **WHEN** a channel row is dragged onto a same-size document
- **THEN** the target channel bytes equal the source row bytes

#### Scenario: A different-size channel drag fits [cp_channel_drag_fit]

- **WHEN** a channel row is dragged onto a different-size document
- **THEN** the target channel holds the source coverage scaled to fit and
  centred, with the uncovered border empty

#### Scenario: The source channel is unchanged [cp_channel_drag_source]

- **WHEN** a channel row is dragged to another document
- **THEN** the source channel count, channel bytes, and history are unchanged
