# tools/slice-tool Specification

## Purpose
Web-export slices: user slices drawn with the Slice tool and the auto slices that tile the rest of the canvas.

## Requirements

### Requirement: User and auto slices

User slices SHALL be stored on `Document::slices`. `pictura_render::resolve_slices`
SHALL return the user slices clipped to the canvas (skipping, but keeping the
index of, any entirely off-canvas) plus auto slices that tile the rest of the
canvas without overlap, merging runs along a row, and SHALL number all of them
from 1 in reading order. An unsliced document SHALL resolve to one auto slice.
`add_slice` SHALL refuse an empty rect.

#### Scenario: Slices tile the canvas

- **WHEN** two user slices are added
- **THEN** the resolved slices cover the canvas area exactly with no overlap

#### Scenario: An empty band merges

- **WHEN** one user slice sits in the middle of the top edge
- **THEN** the band beneath it is a single full-width auto slice

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

### Requirement: Editing user slices

`pictura_render::set_slice(doc, index, rect)` SHALL replace user slice `index`
and `remove_slice(doc, index)` SHALL delete it; both SHALL refuse without change
an out-of-range index, and `set_slice` SHALL refuse an empty rect.

#### Scenario: Set and remove address the right slice

- **WHEN** slice 1 of two is set to a new rect and slice 0 is then removed
- **THEN** the remaining slice is the new rect, and an out-of-range index or empty rect is refused

### Requirement: Slice Select tool

The Slice Select tool SHALL select the user slice under a click (user slices
before auto slices) and mark it with orange handles; a click on an auto slice
or empty canvas SHALL deselect. Dragging inside the selected slice SHALL move
it and dragging an edge or corner handle (within 6 screen pixels) SHALL resize
it, updating the canvas live and recording one "Edit Slice" state on release
when the rect changed. Delete or Backspace SHALL remove the selected slice as
one "Delete Slice" state; Escape SHALL deselect. The pointer SHALL show the move
or matching resize cursor over the selected slice's body or handles.

#### Scenario: Select, move, resize, and delete

- **WHEN** the `crop_group` self-test selects a 16×16 user slice, drags it by (1, 1), drags its right edge 4 px inward, presses Escape, reselects it, and presses Delete
- **THEN** the move and the resize each record one "Edit Slice" state with the expected rects, Escape clears the selection, and Delete records one "Delete Slice" state leaving only the auto slice

### Requirement: Slice Select pointer

With the Slice Select tool active, the canvas pointer SHALL be the precise
crosshair (the marquee's cursor, hot spot at its centre) except over the
selected slice's body or handles, where the move or matching resize cursor
applies.

#### Scenario: Crosshair away from the selected slice

- **WHEN** the pointer hovers the canvas away from any selected slice
- **THEN** the pointer is the precise crosshair

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
