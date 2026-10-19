# document-lifecycle Specification (delta)

## ADDED Requirements

### Requirement: Open runs off the GUI thread and shows a loading tab

The application SHALL perform each document open's file read, decode, first
composite, and initial history capture off the GUI thread. The GUI thread SHALL
remain responsive to input and timers for the whole open. The application SHALL
insert a document tab titled with the file name as soon as an open is known to
produce a document. The tab SHALL be in a loading state: its page SHALL show an
empty workspace with no canvas and no document image, and the application SHALL
NOT make a document available for it before the open succeeds. When another
document is active, the application SHALL keep that document active and visible
and SHALL NOT switch to the loading tab. When no document is active, the
loading tab SHALL be the active tab and its empty workspace SHALL be shown. The
application SHALL show a busy cursor while at least one open is in flight and
SHALL restore the previous cursor when the last open finishes or is cancelled.
The loading-tab path SHALL cover File > Open, Open Recent, a file drop on the
workspace, File > Open As, File > Open As Smart Object, PSD/PSB, raster import,
PDF, and EPS.

#### Scenario: The GUI stays live while the worker runs [lt_off_thread]

- **WHEN** a document open is held in flight
- **THEN** the application processes events (the spinner advances and input is handled) and the document is not yet available

#### Scenario: An open inserts a loading tab [lt_loading_tab]

- **WHEN** an open starts for a file named `B`
- **THEN** a tab titled `B` exists in the loading state with an empty workspace, no canvas, and no document image

#### Scenario: A background open keeps the current document [lt_background]

- **WHEN** document `A` is active and an open starts for `B`
- **THEN** `A` stays active and visible, and `B` becomes active only when its open succeeds

#### Scenario: A first open shows an empty workspace [lt_empty_workspace]

- **WHEN** no document is active and an open starts for `B`
- **THEN** the loading tab is the active tab and its page shows no canvas and no document image

#### Scenario: The busy cursor spans concurrent opens [lt_busy_cursor]

- **WHEN** one or more opens are in flight and each finishes, fails, or is cancelled
- **THEN** the busy cursor is active while any open is in flight and the previous cursor is restored when the last one ends

#### Scenario: A generic PDF is probed without a loading tab [lt_generic_pdf]

- **WHEN** a generic PDF is opened
- **THEN** no loading tab exists during the format probe or the Import PDF dialog, and a loading tab exists when a chosen page or image render starts

#### Scenario: Every open entry point shares the path [lt_entry_points]

- **WHEN** a document is opened through File > Open, Open Recent, a workspace file drop, File > Open As, File > Open As Smart Object, PSD/PSB, raster import, PDF, or EPS
- **THEN** the open uses the loading-tab path and its completion rules

### Requirement: The loading tab shows a spinner and cancels

The loading tab SHALL replace its close button with an animated spinner while
the open is in flight. The spinner SHALL animate independently of the worker.
Hovering the spinner SHALL replace it with the normal close glyph. Clicking the
spinner SHALL cancel the open. Middle-clicking the loading tab SHALL cancel the
open. A normal document tab SHALL keep its close button and SHALL NOT be
cancelled by a middle-click. Cancelling SHALL remove the loading tab
immediately, discard the worker's result when it arrives, restore the
previously active tab or the empty workspace, and show no error.

#### Scenario: The spinner animates during the load [lt_spinner_animates]

- **WHEN** a loading tab is visible while its open is held in flight
- **THEN** the spinner's rotation advances without the worker completing

#### Scenario: Hovering the spinner shows the close glyph [lt_hover_close]

- **WHEN** the pointer enters the loading tab's spinner
- **THEN** the spinner paints the normal close glyph, and leaving restores the spinner

#### Scenario: Clicking the spinner cancels [lt_click_cancel]

- **WHEN** the loading tab's spinner is clicked
- **THEN** the tab is removed immediately, the worker result is discarded, the previous tab or empty workspace is restored, and no error is shown

#### Scenario: Middle-clicking the loading tab cancels [lt_middle_click_cancel]

- **WHEN** the loading tab is middle-clicked
- **THEN** the open is cancelled with the same result as clicking the spinner

#### Scenario: Normal tabs are unchanged [lt_normal_tab_unchanged]

- **WHEN** a normal document tab is hovered, closed, or middle-clicked
- **THEN** its close button and middle-click behaviour are unchanged (the loading filter only handles loading tabs)

#### Scenario: Cancel discards a late result [lt_cancel_discards]

- **WHEN** a loading tab is cancelled and the worker finishes afterwards
- **THEN** no document is added, no tab reappears, and no error is shown

### Requirement: Open completion fills or removes the tab

On success the application SHALL populate the loading tab with the document,
restore the normal close button, activate the tab unless the user selected
another tab during the load, and keep the existing status-bar notice. On
failure the application SHALL remove the loading tab, restore the previously
active tab or the empty workspace, keep the existing error path, and add no
document. The application SHALL restore the busy cursor on success, failure,
and cancel. A user cancel SHALL be distinguished from a failure so that a
cancel never takes the failure path.

#### Scenario: Success fills the tab [lt_success]

- **WHEN** an open succeeds
- **THEN** the tab shows the document, its close button is back, and the tab is active unless the user selected another tab during the load

#### Scenario: A user selection is not stolen [lt_success_keeps_selection]

- **WHEN** the user selects another tab while an open is in flight and the open then succeeds
- **THEN** the user's tab stays active and the completed tab is filled in the background

#### Scenario: Success keeps the status notice [lt_notice]

- **WHEN** an open succeeds and the document reports a notice such as `Converted from CMYK; saved as CMYK`
- **THEN** the status bar shows that notice

#### Scenario: Failure removes the tab [lt_failure]

- **WHEN** an open fails
- **THEN** the loading tab is removed, no document is added, the previous tab or empty workspace is restored, and the existing error path runs

#### Scenario: A cancel is not a failure [lt_cancel_distinct]

- **WHEN** an open is cancelled by the user
- **THEN** the cancellation path runs, no error is shown, and the failure path is not taken

### Requirement: The lower-left loading banner is removed

The application SHALL NOT show the lower-left `Opening <file>…` banner. The
banner's stylesheet rule, test hooks, and the `documentOpenStarted` signal
SHALL be removed.

#### Scenario: The banner is gone [lt_banner_removed]

- **WHEN** a document is opened
- **THEN** no `documentOpenBanner` widget or stylesheet rule exists and the open is indicated by the loading tab and spinner instead

## REMOVED Requirements

### Requirement: Open shows a loading banner

**Reason**: The open now runs off the GUI thread and shows a loading tab with a
spinner. The banner was a synchronous stopgap; issue #729 replaces it.

**Migration**: Read "Open runs off the GUI thread and shows a loading tab",
"The loading tab shows a spinner and cancels", and "Open completion fills or
removes the tab". The notice and every-entry-point guarantees move there; the
banner-specific style and `documentOpenStarted` seam are deleted.
