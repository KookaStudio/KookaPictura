## ADDED Requirements

### Requirement: Dragging outside the text moves it

While a Type tool session is open, the pointer outside the text, and outside
paragraph type's box and handles, SHALL show the Move tool's cursor. A drag that
starts there SHALL move the whole text with the pointer: the preview, caret,
placement point, baseline guides, and box SHALL follow live, and nothing SHALL
be recorded until the session commits, which places the type where it was
dropped. A press and release outside the text without a drag SHALL commit the
session, as before.

#### Scenario: Drag the text while typing [type_session_move]

- **WHEN** `tst_type_tools::dragOutsideMovesTheText` types "Move me", hovers away from the text, drags from there by (30, 20), and commits
- **THEN** the pointer zone reads move away from the text and not over it, the placement point and baseline guide moved by (30, 20) with no history recorded during the drag, and the committed layer sits 30 px right and 20 px lower than an unmoved one

#### Scenario: A click outside still commits [type_session_click_commits]

- **WHEN** `tst_type_tools::typeKeysAndCancel` types and then clicks away without dragging
- **THEN** the session commits one "Horizontal Type" state
