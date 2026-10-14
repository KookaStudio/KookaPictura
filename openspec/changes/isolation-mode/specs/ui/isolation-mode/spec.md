# isolation-mode Specification

## Purpose

Isolation mode focuses the Layers panel and the Move tool on one set of layers:
the panel's `Selected` filter shows only those layers, Auto-Select only picks
among them, and a layer can be released or the whole mode exited without
changing the document.

## ADDED Requirements

### Requirement: The Selected filter isolates

The Layers panel filter SHALL offer a `Selected` dimension in addition to the
existing dimensions, with no criteria control of its own — its criterion is the
isolated set. Entering isolation SHALL capture the current selection as a fixed
set. With `Selected` chosen and the filter toggle on, the panel SHALL show only
the layers in the isolated set and their group ancestors, and every other layer
SHALL be hidden from the panel but not from the document. The set SHALL change
only through Release, so selecting a shown row SHALL NOT add or remove a layer
from it. Choosing `Selected` with the toggle on SHALL be isolation mode.

#### Scenario: The Selected filter shows the isolated set [iso_selected]

- **WHEN** two layers are selected and isolation is entered
- **THEN** the panel shows only those two layers (and any ancestor groups) and
  every other layer is hidden

#### Scenario: Selecting a shown row keeps the set [iso_fixed]

- **WHEN** isolation is active and the user selects a different shown row
- **THEN** the isolated set is unchanged and every isolated row stays visible

#### Scenario: Toggling the filter off restores every row [iso_toggle]

- **WHEN** isolation is active and the filter toggle is turned off
- **THEN** every layer is shown again and isolation ends

### Requirement: Move Auto-Select is scoped while isolated

While isolation is active, the Move tool's Auto-Select SHALL only pick a layer
that is in the isolated set, or a descendant of one. A press whose topmost hit
is outside the isolated set SHALL keep the active layer, and a press on a member
or its descendant SHALL select it as usual.

#### Scenario: A hit outside the set keeps the active layer [iso_scope_out]

- **WHEN** isolation is active on one layer and the Move tool Auto-Select is
  pressed over a different layer
- **THEN** the active layer does not change

#### Scenario: A hit inside the set selects [iso_scope_in]

- **WHEN** isolation is active on a layer and the Move tool Auto-Select is
  pressed over that layer
- **THEN** it becomes the active layer

#### Scenario: A group member is reachable [iso_scope_group]

- **WHEN** isolation is active on a group and the Move tool Auto-Select is
  pressed over a child of that group
- **THEN** the child is a valid hit

### Requirement: Release from Isolation

The Layers panel row context menu SHALL offer `Release from Isolation` for a row
that is in the isolated set. Choosing it SHALL remove that row from the set
(deselecting it in the panel). Releasing the last isolated layer SHALL end
isolation. The command SHALL NOT change the document and SHALL NOT add a history
state.

#### Scenario: Release removes one layer from the set [iso_release_one]

- **WHEN** two layers are isolated and `Release from Isolation` is chosen for
  one of them
- **THEN** that layer is no longer selected or isolated, the other remains, and
  no history state is recorded

#### Scenario: Releasing the last layer ends isolation [iso_release_last]

- **WHEN** one layer is isolated and `Release from Isolation` is chosen for it
- **THEN** isolation ends and the panel shows every layer

### Requirement: Enter and exit isolation

With the Move tool active, the canvas right-click menu SHALL offer `Isolation
Mode`, which SHALL isolate the current selection. Pressing Escape, turning the
filter toggle off, choosing another filter dimension, or switching the active
document SHALL end isolation. Isolation SHALL be view-only: it SHALL NOT mutate
or serialize the document and SHALL NOT add a history state.

#### Scenario: The canvas entry isolates the selection [iso_enter_canvas]

- **WHEN** the Move tool is active, one or more layers are selected, and
  `Isolation Mode` is chosen from the canvas right-click menu
- **THEN** the Selected filter turns on for the current selection

#### Scenario: Escape ends isolation [iso_escape]

- **WHEN** isolation is active and Escape is pressed
- **THEN** the Selected filter is cleared and the panel shows every layer,
  before any other Escape action

#### Scenario: Switching documents ends isolation [iso_reset]

- **WHEN** isolation is active and the active document changes
- **THEN** isolation is reset and the new document has no Selected filter
