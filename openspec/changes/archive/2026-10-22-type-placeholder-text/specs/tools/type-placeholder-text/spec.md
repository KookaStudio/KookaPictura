# tools/type-placeholder-text Specification

## Purpose
New type starting with selected placeholder text, and the Type preference that
turns it off. Not a CS6 behavior: an owner decision, labelled non-parity.

## ADDED Requirements

### Requirement: New type starts with selected placeholder text

When the Fill New Type Layers With Placeholder Text preference is on, a new
session of the Horizontal or Vertical Type tool, opened by a click or by
dragging a paragraph box, SHALL start with the text `Lorem Ipsum Pictura`, all
of it selected. Typed or pasted text SHALL replace the selection. Committing
without editing SHALL create a type layer with the placeholder text. A Type
Mask session, and a session that reopens an existing type layer, SHALL NOT
insert the placeholder.

#### Scenario: A dragged box is filled and selected [tph_box]

- **WHEN** `tst_type_placeholder::dragFillsBox` drags a box with the
  Horizontal Type tool
- **THEN** the session text is `Lorem Ipsum Pictura` and the whole text is
  selected

#### Scenario: A click is filled and selected [tph_click]

- **WHEN** `tst_type_placeholder::clickFillsPointType` clicks with the
  Horizontal Type tool
- **THEN** the session text is `Lorem Ipsum Pictura` and the whole text is
  selected

#### Scenario: Typing replaces the placeholder [tph_typing]

- **WHEN** `tst_type_placeholder::typingReplacesPlaceholder` drags a box, types
  `Hello`, and commits
- **THEN** the new layer's text is `Hello`

#### Scenario: An untouched placeholder is committed [tph_commit]

- **WHEN** `tst_type_placeholder::untouchedPlaceholderCommits` clicks and
  commits without typing
- **THEN** a type layer with the text `Lorem Ipsum Pictura` is added

#### Scenario: Mask tools and reopened layers start as before [tph_exempt]

- **WHEN** `tst_type_placeholder::maskAndReopenAreExempt` clicks with the
  Horizontal Type Mask tool, and clicks an existing type layer with the
  Horizontal Type tool
- **THEN** the mask session starts empty, and the reopened session holds the
  layer's own text with nothing selected

### Requirement: The Type preference turns the placeholder off

`Edit > Preferences > Type` SHALL open a Type page with a Fill New Type Layers
With Placeholder Text checkbox, on by default. Turning it off SHALL make new
type sessions start empty. The choice SHALL be saved with the session and
restored on the next start.

#### Scenario: The Type page carries the preference [tph_pref_page]

- **WHEN** `tst_type_placeholder::typePreferencePage` dispatches
  `Edit > Preferences > Type`
- **THEN** the Preferences dialog shows the Type page with the checkbox
  checked

#### Scenario: Turning it off starts empty type [tph_pref_off]

- **WHEN** `tst_type_placeholder::preferenceOffStartsEmpty` clears the
  checkbox and clicks with the Horizontal Type tool
- **THEN** the session starts with empty text

#### Scenario: The preference survives a restart [tph_pref_saved]

- **WHEN** `tst_type_placeholder::preferenceIsSaved` clears the checkbox and
  reads the saved session back
- **THEN** the saved session holds the placeholder preference off
