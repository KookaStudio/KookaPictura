# Spec Delta

## MODIFIED Requirements

### Requirement: Slice tool

The Slice tool SHALL add a user slice from a drag of at least 1×1 pixel as one
"Slice" history state; a click SHALL add nothing. While the tool is active the
canvas SHALL show every resolved slice (user slices solid blue, auto slices
dotted grey, each with a numbered badge when it fits) and SHALL re-read them
after every edit or undo. `View > Show > Slices` SHALL be a checkable command,
on by default, persisted in the session, and applied to every canvas. With
another tool active, the same overlay SHALL remain visible while Show > Slices
and `View > Extras` are both on, and SHALL be hidden otherwise; leaving the
Slice tool SHALL NOT hide slices that Show > Slices keeps on.

#### Scenario: Drawing and undoing a slice

- **WHEN** the `crop_group` self-test drags a slice, clicks once, and undoes
- **THEN** the drag adds one "Slice" state and a numbered user slice, the click adds none, and after Undo the overlay shows the single auto slice

#### Scenario: The overlay follows the tool

- **WHEN** a user slice exists, another tool is selected with Show > Slices on, and the Slice tool is left
- **THEN** the slice overlay stays visible with the other tool and leaving the Slice tool does not hide it

#### Scenario: Show and Extras hide the overlay [slice_show_off]

- **WHEN** Show > Slices is turned off with another tool active, then turned on, then Extras is turned off
- **THEN** the overlay hides, returns, and hides again with the Show checkmark unchanged

#### Scenario: Show > Slices persists [slice_show_persist]

- **WHEN** Show > Slices is turned off, the session is saved and reloaded, and two documents are open
- **THEN** both canvases keep the overlay hidden with the Show checkmark cleared

## ADDED Requirements

### Requirement: Lock slices

`View > Lock Slices` SHALL be a checkable command, off by default, global, and
persisted in the session. While it is on, the Slice and Slice Select tools SHALL
NOT add, move, resize, or delete a user slice and SHALL record no history state;
the slice overlay SHALL remain visible. Turning it off SHALL restore editing
without changing the stored slices. `View > Clear Slices` SHALL remain available
while slices are locked because it is an explicit command.

#### Scenario: Locking blocks slice edits [slice_lock]

- **WHEN** the user locks slices, drags a new slice, drags an existing slice and its edge, and presses Delete
- **THEN** the slice count and rectangles are unchanged, no history state is recorded, and the overlay is still visible

#### Scenario: The lock persists [slice_lock_persist]

- **WHEN** the user locks slices, saves and reloads the session, and restarts
- **THEN** Lock Slices is checked and slice edits stay blocked

#### Scenario: Clear works while locked [slice_clear_locked]

- **WHEN** slices are locked and the user dispatches Clear Slices
- **THEN** every user slice is removed in one "Clear Slices" state

### Requirement: Clear slices

`View > Clear Slices` SHALL remove every user slice of the active document as
one "Clear Slices" history state and SHALL leave the derived auto slices
covering the canvas. It SHALL be disabled when the document has no user slices
or no document is open. Undo SHALL restore every removed user slice.

#### Scenario: Clear removes every user slice [slice_clear]

- **WHEN** two user slices exist and the user dispatches Clear Slices, then undoes
- **THEN** the first call leaves only auto slices in one "Clear Slices" state, and Undo restores both user slices

#### Scenario: Clear needs a user slice [slice_clear_disabled]

- **WHEN** the document has only the derived auto slice, or no document is open
- **THEN** Clear Slices is disabled and dispatch changes nothing
