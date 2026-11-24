# document-lifecycle Specification

## Purpose
New, Open, Save, Save As, modified-state tracking, the unsaved-changes prompt, and Revert.

## Requirements

### Requirement: New document creation
The system SHALL create a new document from a New Document dialog that specifies
name, width, height, color mode, bit depth, and background. When the system
clipboard holds an image, the dialog SHALL open with its Document Type
preselected to Clipboard and its width and height preloaded from that image's
pixel dimensions; with no image on the clipboard the dialog SHALL open on its
default type and leave the Clipboard type unavailable. Creation SHALL be
supported for 8-bit Grayscale and 8-bit RGB with a white or transparent
background; requests outside that set SHALL be rejected rather than produce a
document the codec cannot write. A new document SHALL be untitled, unmodified,
with empty history and no selection. A document SHALL be created only by an
explicit New, Open, or Place action; launch SHALL NOT fabricate a document, so a
normal launch starts with no document open and the document-requiring commands
disabled.

#### Scenario: Create an RGB document
- **WHEN** the user confirms New with an 8-bit RGB document, white background, and dimensions
- **THEN** a new untitled tab opens showing a document of those dimensions filled white

#### Scenario: Create a transparent document
- **WHEN** the user confirms New with a transparent background
- **THEN** the new document's initial composite is fully transparent

#### Scenario: Unsupported mode or depth is rejected
- **WHEN** a document is requested with a mode or depth the codec cannot write
- **THEN** creation fails and no document is added

#### Scenario: Launch fabricates no document [ldl_no_launch_doc]
- **WHEN** the application is launched with no document argument
- **THEN** no document exists until the user runs New, Open, or Place

#### Scenario: An explicit New creates a document after launch [ldl_new_after_launch]
- **WHEN** the user runs New after a launch with no document
- **THEN** a document is created and the document-requiring commands become
  enabled

#### Scenario: An image on the clipboard preloads the New size
- **WHEN** the system clipboard holds an image and the user opens File > New
- **THEN** the Document Type is Clipboard and the width and height fields show the clipboard image's pixel dimensions

#### Scenario: An empty clipboard keeps the default size
- **WHEN** the system clipboard holds no image and the user opens File > New
- **THEN** the Document Type is the default size and the Clipboard type is disabled

### Requirement: Open documents

The system SHALL open one or more files chosen from a file dialog into new
documents, and SHALL offer the most recently opened files through an Open Recent
list. The `File > Open` dialog SHALL accept a multi-file selection, including a
Shift-click range and Ctrl/Cmd-click individual files, and SHALL open one
document per selected file. A file that fails to open SHALL NOT prevent the
remaining selected files from opening. Each document SHALL follow the
document-window host rules, including the `Open Documents As Tabs` preference. A
successful open SHALL leave the document unmodified with its path recorded.

#### Scenario: Open a file adds a document

- **WHEN** the user opens a valid PSD
- **THEN** a new tab shows the composited document and the command line argument path is recorded

#### Scenario: Open failure does not add a document

- **WHEN** the chosen file cannot be read
- **THEN** no tab is added and the application remains usable

#### Scenario: Recent files

- **WHEN** a document is opened successfully
- **THEN** it appears at the front of the bounded Open Recent list

#### Scenario: Multi-select opens one document per file [dlg_open_multiple]

- **WHEN** the user selects several files in `File > Open` and confirms
- **THEN** one document opens for each selected file, each records its own path,
  and every document follows the tabs preference

#### Scenario: A failed file does not stop the rest [dlg_open_multiple_partial]

- **WHEN** the selection includes a file that cannot be read and other readable
  files
- **THEN** the readable files open and the failed file adds no document

#### Scenario: A single selection still opens one document [dlg_open_single]

- **WHEN** the user selects one file in `File > Open` and confirms
- **THEN** one document opens for it

### Requirement: Save and Save As

For an editable document, the system SHALL write the active document in its current output format, chosen
by the save path's extension, writing atomically so a failed write never
truncates an existing file. A `psd` path SHALL write the document's composite
and layer tree through the PSD codec; a `psb` path SHALL force a version-2 PSB
container through the codec. A supported raster path (PNG, JPEG, TIFF, WebP,
BMP, or GIF) SHALL write the flattened composite: PNG, JPEG, TIFF, WebP, and BMP
through the Qt-boundary raster encode edge, and GIF through the engine GIF89a
encoder (`interop/gif-save`). Save As SHALL default its format to PSD unless the
document is a single pixel or background layer imported from a supported raster,
in which case it SHALL default to that source format. When the document's
recorded path is a raster format and the document has more than one layer, a
group, an adjustment, or a type layer, Save SHALL behave as Save As rather than
silently flattening into that raster file. Saving a document that has more than
one layer, a group, an adjustment, or a type layer to a format that cannot hold
them SHALL warn before writing and then write the flattened composite. The
`composite` serialized to PSD SHALL be the active document's current rendered
composite, so a save after any edit round-trips to the edited pixels rather than
to a pre-edit merged image. The `write_psd` byte layout and its use of
`doc.composite.channels` to derive the PSD colour-plane count MUST NOT change; an
RGB document's merged composite is RGBA (four planes) after a canvas rebuild,
while a grayscale document keeps its composite colour-plane count. Save on an
untitled document SHALL behave as Save As. On success the save SHALL record the
file path. It SHALL clear the modified state only when the document is the same
document that was snapshotted when the write started — the same history
identity, content revision, native persisted-state identity, and document
generation — and that snapshot is still current. A document changed after the
snapshot SHALL keep the recorded path and remain modified. When the document generation changed during the save (the
document was replaced), the replacement document's path and modified state SHALL
be unchanged. On a pre-publication failure the path and modified state SHALL
be unchanged and no temporary file SHALL remain. An unconfirmed publication
SHALL retain the path and establish the protection obligation specified below.

#### Scenario: Save As writes a file

- **WHEN** the user runs Save As and chooses a path
- **THEN** a valid file of the chosen path's format is written at that path and the document is no longer modified

#### Scenario: Save uses the recorded path

- **WHEN** a document has a path and the user runs Save
- **THEN** the file at that path is overwritten atomically, in that path's format

#### Scenario: Save on an untitled document prompts for a path

- **WHEN** Save is invoked on an untitled document
- **THEN** a Save As path is requested before any write

#### Scenario: Save after an edit writes the edited composite

- **WHEN** an edit changes the canvas and the document is saved and read back
- **THEN** the read-back document's composite equals the edited rendered composite, not the pre-edit merged image, and its colour-plane count matches the saved document's colour-plane count

#### Scenario: Save As defaults to the import source format

- **WHEN** a single-layer PNG imported through `File > Open` is saved with Save As
- **THEN** the Save As dialog preselects PNG as the format

#### Scenario: A raster save clears modified and re-saves the same format

- **WHEN** a document is saved to `photo.png` and then edited and saved again
- **THEN** the second save writes a PNG to `photo.png` and clears the modified state

#### Scenario: A layered document saved to a flat format warns and flattens

- **WHEN** the user saves a document with more than one layer to a PNG
- **THEN** a warning is shown that the format cannot hold the document's features, and on confirmation the flattened composite is written as a PNG

#### Scenario: A layered document defaults Save As to PSD

- **WHEN** a PNG imported through `File > Open` has a layer or an adjustment added and Save As is invoked
- **THEN** the Save As dialog preselects PSD, not PNG

#### Scenario: Save on a layered raster path behaves as Save As

- **WHEN** a layered document recorded at a `.png` path is saved
- **THEN** the format-aware Save As dialog opens (defaulting to PSD) instead of overwriting the PNG with a flattened composite

#### Scenario: A PSB path writes a PSB container

- **WHEN** a document is saved to a `.psb` path
- **THEN** the written file is a version-2 PSB

#### Scenario: A completed save writes the snapshot and clears modified [dl_save_current]

- **WHEN** a modified document is saved through the worker and the document does not change while it runs
- **THEN** the file reopens with the snapshot's layers and composite, the recorded path is the saved path, and the document is unmodified

#### Scenario: An edit during a save leaves the document modified [dl_save_stale]

- **WHEN** the document's history or content revision changes while the save worker runs
- **THEN** the file holds the saved snapshot, the recorded path is the saved path, and the document remains modified

#### Scenario: A failed write leaves the document untouched [dl_save_failed]

- **WHEN** the target cannot be written
- **THEN** the save returns false, the path and modified state are unchanged, and no temporary file remains

#### Scenario: A replaced document keeps its path and dirty state [dl_save_replaced]

- **WHEN** the document is replaced while the save worker runs
- **THEN** the file holds the original document's snapshot, and the replacement document keeps its own path and modified state

### Requirement: Modified state tracking

The system SHALL expose `is_dirty()` for the active document. Every successful
mutating command SHALL mark the document modified. A native saved-state identity
SHALL include both history state and persisted session metadata, including
Vanishing Point planes, even where a metadata change creates no history entry
or content revision. Save completion SHALL also compare the snapshot content
revision to the live content revision before clearing modified. Saving records
the snapshot identity; history navigation
SHALL clear modified only if both the resulting history state and current
persisted metadata match the saved identity and there is no outstanding
publication protection obligation. A later metadata-only edit remains modified
across navigation. Opening, New, and successful Revert establish an unmodified
state. A dropped saved history state remains modified. A rejected command SHALL
NOT change modified state. A view with no document SHALL report unmodified.

#### Scenario: A mutating command marks modified

- **WHEN** a filter, adjustment, document op, or selection command succeeds
- **THEN** the document reports modified

#### Scenario: Save clears modified

- **WHEN** a modified document is saved successfully and does not change while the save runs
- **THEN** the document reports unmodified

#### Scenario: Open clears modified

- **WHEN** a document is opened
- **THEN** it reports unmodified

#### Scenario: History navigation marks modified [ml_history_navigation]

- **WHEN** a saved document is edited and then navigated by undo, redo, history jump, or named-snapshot restore to a state other than the saved state
- **THEN** the document reports modified

#### Scenario: Returning to the saved state clears modified [ml_return_to_saved]

- **WHEN** a saved document is edited and history navigation returns to the saved state
- **THEN** the document reports unmodified

#### Scenario: A dropped saved state stays modified [ml_saved_state_dropped]

- **WHEN** enough new edits or bounded-history drops remove the saved state from the retained history
- **THEN** the document reports modified and no navigation can return it to unmodified

### Requirement: Unsaved-changes prompt
The system SHALL prompt Save / Discard / Cancel before closing, reverting, or
replacing a modified document or one with an outstanding publication protection obligation. Cancel SHALL abort the operation and leave the
document open and modified.

#### Scenario: Discard closes without saving
- **WHEN** the user chooses Discard in the prompt
- **THEN** the document closes and no file is written

#### Scenario: Save then close
- **WHEN** the user chooses Save in the prompt
- **THEN** the document is saved and then closed

#### Scenario: Cancel aborts
- **WHEN** the user chooses Cancel
- **THEN** the document remains open and modified

### Requirement: Revert
The system SHALL revert a saved document by reloading it from its path into the
existing document identity. Revert SHALL decode the file before changing the
document; if the file cannot be read or decoded, including when the stored PDF
password no longer opens it, Revert SHALL return failure and leave the document,
pixels, selection, dirty state, path, history, and any live session unchanged.
On success, Revert SHALL cancel the live session, clear the outgoing selection
and the rest of the document-scoped state, install the reloaded document,
preserve the existing history and its named snapshots, append exactly one
current history state labelled `Revert`, and mark the document unmodified. Undo
SHALL restore the pre-Revert document. Revert SHALL be unavailable for an
untitled document.

#### Scenario: Revert reloads from disk
- **WHEN** the user reverts a modified document with a path
- **THEN** the document matches the file on disk and reports unmodified

#### Scenario: Revert unavailable when untitled
- **WHEN** the active document has no path
- **THEN** Revert is disabled

#### Scenario: A successful Revert is undoable [dl_revert_undoable]

- **WHEN** a modified document with prior history and named snapshots is reverted from an intact file
- **THEN** the history retains the pre-Revert states and named snapshots and appends one current `Revert` state; undo then restores the pre-Revert pixels

#### Scenario: A failed Revert keeps the document [dl_revert_failure_keeps]

- **WHEN** Revert runs and the file is missing, truncated, cannot be decoded, or its stored PDF password no longer opens it
- **THEN** Revert returns failure and the document, pixels, selection, dirty state, path, history, and any live session are unchanged

### Requirement: Recent files persistence
The system SHALL persist the recent-files list in the session store, bounded to a
maximum length, and SHALL drop entries whose files can no longer be found.

#### Scenario: Recent list survives restart
- **WHEN** a document is opened and the application restarts
- **THEN** the file appears in the Open Recent list

#### Scenario: Missing recent entry is dropped
- **WHEN** a recent file no longer exists
- **THEN** it is not offered in Open Recent

### Requirement: Open Recent menu

File > Open Recent SHALL follow Open As Smart Object and SHALL be rebuilt each
time it opens, so a file opened in this session appears immediately. It SHALL
list the existing recent files most recent first, each labelled with its file
name and carrying its full path as a tooltip, followed by a separator and Clear
Recent File List; with no recent files it SHALL show only a disabled "No Recent
Files" row. Choosing an entry SHALL open it as File > Open would (PSD/PSB
through the codec, other images through the image importer), and a successful
image open SHALL also be recorded. Clear Recent File List SHALL empty and
persist the list.

#### Scenario: A file opened this session is offered at once

- **WHEN** the `recent_files` self-test opens an image and shows Open Recent
- **THEN** the image is the first row, labelled by name with its path as the tooltip, and Clear Recent File List is present

#### Scenario: Choosing a recent image reopens it

- **WHEN** that row is chosen
- **THEN** a new document opens

#### Scenario: Clearing empties the list

- **WHEN** Clear Recent File List is chosen
- **THEN** the list is empty and the submenu shows only "No Recent Files"

### Requirement: File dialog format filters

The Open, Save, Place, and Open As Smart Object file dialogs SHALL share one set
of format filters: one entry per format, named for the format and listing its
extensions in uppercase, with the patterns separated by spaces. The Open dialog
SHALL offer an `All Formats` entry listing every supported import format and
SHALL select it by default; the Save As type list SHALL also offer `All Formats`,
which names no single format and therefore falls back to the document's smart
default when the typed name has no extension. The Export As dialog SHALL present
its format via its own combo and SHALL filter the file dialog with a single
uppercase entry for the chosen format; the path's extension remains the
authoritative format. The "File type" combo
SHALL be non-editable, so it cannot be cleared, and SHALL support rolling
type-ahead: a printable key selects the first filter whose name starts with that
character (case-insensitively) and repeating the same key within the
keyboard-input interval advances to the next matching filter, wrapping. The
format written to disk SHALL use a lowercase extension regardless of the
filter's case.

#### Scenario: Save As lists one filter per format

- **WHEN** the Save As dialog is opened
- **THEN** its type list contains a separate uppercase entry for each of Photoshop, Photoshop Large Format, PNG, JPEG, TIFF, WebP, and BMP

#### Scenario: Open selects All Formats

- **WHEN** the Open dialog is opened
- **THEN** its type list contains an `All Formats` entry and that entry is selected

#### Scenario: Open As Smart Object uses the shared filters

- **WHEN** the Open As Smart Object dialog is opened
- **THEN** its type list matches the Open dialog's, including an `All Formats` entry and the raster formats

#### Scenario: The type combo rolls with a repeated key

- **WHEN** the "File type" combo has focus and the user presses a letter, then the same letter again within the keyboard-input interval
- **THEN** the selection moves to the next filter whose name starts with that letter, wrapping at the end

#### Scenario: The type combo cannot be cleared

- **WHEN** the "File type" combo is edited by keyboard
- **THEN** it selects a filter and never becomes empty

#### Scenario: Extensions are written lowercase

- **WHEN** a document is saved with a filter whose pattern is uppercase and the typed name has no extension
- **THEN** the appended extension is lowercase (for example `.png`)

### Requirement: Open As Smart Object accepts raster sources

`File > Open As Smart Object` SHALL accept a PSD/PSB source as-is and SHALL accept
a supported raster source (PNG, JPEG, GIF, BMP, TIFF, WebP) by embedding its
decoded layer as a smart object. The resulting document SHALL have a single
smart-object layer whose contents can be edited; a file that is neither PSD/PSB
nor a supported raster SHALL be refused without changing the view.

#### Scenario: A raster opens as a single smart-object layer

- **WHEN** File > Open As Smart Object is used on a PNG
- **THEN** the document has a single smart-object layer whose contents can be edited

#### Scenario: An unsupported source is refused

- **WHEN** File > Open As Smart Object is used on a file that is neither PSD/PSB nor a supported raster
- **THEN** no document is created

### Requirement: File dialog places

The Open, Save, Place, Open As Smart Object, and Export As file dialogs SHALL
expose a Places sidebar containing the filesystem root, the mounted volumes, the
XDG user folders, the desktop bookmark directories (KDE `user-places.xbel` and
GTK `gtk-3.0/bookmarks`), and the application's recent locations. Only existing
directories SHALL be offered, and the list SHALL be deduplicated by canonical
path. When the application runs inside a sandbox (Flatpak or Snap), the dialogs
SHALL instead delegate to the platform/portal file chooser, which supplies the
host's places.

#### Scenario: Root and home are offered

- **WHEN** any file dialog is opened
- **THEN** its Places sidebar contains the filesystem root and the user's home folder

#### Scenario: Mounted volumes and bookmarks are offered

- **WHEN** the system has a mounted volume or a KDE/GTK bookmark pointing at a directory
- **THEN** that directory appears in the sidebar

#### Scenario: Missing places are omitted

- **WHEN** a bookmark points at a path that no longer exists or is not a directory
- **THEN** that entry is left out of the sidebar

#### Scenario: Recent locations are offered

- **WHEN** a document has been saved through the application
- **THEN** its parent directory appears among the recent locations in the sidebar

#### Scenario: A sandboxed run delegates to the platform chooser

- **WHEN** the application runs inside a Flatpak or Snap sandbox
- **THEN** the file dialogs use the platform/portal chooser rather than the built-in dialog

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
open. A normal document tab SHALL keep its close button and SHALL close on a
middle-click. Cancelling SHALL remove the loading tab
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

#### Scenario: A middle-click closes a normal tab [lt_normal_tab_middle_click]

- **WHEN** a normal document tab is middle-clicked
- **THEN** its close button is present and the tab closes (the loading filter only handles loading tabs)

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

### Requirement: The loading tab shows phase progress

While an open is in flight, the loading tab's empty workspace SHALL show a
centered progress card containing the file name, the current phase, a
determinate progress bar from 0 to 100, the percentage, and a Cancel button.
The application SHALL report the phases `Reading file`, `Decoding`,
`Compositing`, and `Preparing layers`, and for PDF/EPS imports
`Rendering page`, `Rendering image`, or `Rendering EPS`, at the boundaries of
the work they name. The percentages SHALL be phase weights rather than measured
work; the application SHALL NOT claim sub-progress inside a phase. The card
SHALL be hidden when the tab is filled or removed. Its Cancel button SHALL
cancel the open exactly as the tab spinner does, restoring the previous tab and
showing no error. A generic PDF's probe and the Import PDF dialog SHALL show no
card; the card SHALL appear when a chosen page or image render starts.

#### Scenario: The card shows the current phase and percentage [lt_phase_progress]

- **WHEN** a document open is held in flight after its decode
- **THEN** the loading tab shows a centered card with the file name, a phase such as `Preparing layers…`, a bar between 0 and 100, a percentage, and a Cancel button

#### Scenario: The card's Cancel cancels the open [lt_phase_cancel]

- **WHEN** the card's Cancel button is clicked
- **THEN** the loading tab is removed, the worker result is discarded, the previous tab or empty workspace is restored, and no error is shown

#### Scenario: A generic PDF probe shows no card [lt_phase_no_probe]

- **WHEN** a generic PDF is opened
- **THEN** no progress card exists during the probe or the Import PDF dialog, and a card appears when a chosen page or image render starts

### Requirement: Raster Open records the file path

A raster opened as a document through File > Open, Open Recent, or a drop on the
workspace SHALL record the opened file as the document's current path. Save
SHALL then write that file in place without a Save As dialog, and Revert SHALL be
available. A raster reload performed by Revert SHALL record the same path again.
The path-less imports (`Open As Smart Object` and `Place`) SHALL remain
untitled.

#### Scenario: A raster open keeps its path

- **WHEN** a PNG is opened through `openDocumentAtPath`, edited, and saved with `saveActive()`
- **THEN** `documentPath(0)` equals the PNG path, `saveActive()` returns true without a Save As dialog, the file is rewritten in place, and Revert reloads the same path

### Requirement: Save As format choice drives the codec

When Save As is used and a single-format filter is selected, the application
SHALL write that filter's format. A conflicting suffix typed in the file name
SHALL be replaced with the filter's canonical suffix (`.psd`, `.psb`, `.png`,
`.jpg`, `.tif`, `.webp`, `.bmp`, `.gif`, or `.pdf`); the typed suffix SHALL NOT
silently choose a different codec. The `All Formats` entry SHALL keep the typed
suffix and choose the codec from it. The document's recorded path SHALL be the
path actually written.

#### Scenario: The chosen format wins over a typed suffix

- **WHEN** Save As has the PNG filter selected and the user types `report.jpg`
- **THEN** the written file is `report.png`, it decodes as PNG, and `report.jpg` is not created

### Requirement: Native saves run off the GUI thread

After the destination and format are chosen, a native `.psd` or `.psb` save
SHALL capture an immutable document snapshot and perform the composite refresh,
encode, and atomic write on a worker thread. The worker SHALL NOT touch the live
document. The GUI thread SHALL keep processing events. Interactive Save and
Save As SHALL accept editing input while Save in Background is enabled.
Synchronous callers SHALL wait for completion and retain their completion
result and input exclusion. With Save in Background disabled, interactive
saves SHALL also wait, exclude input, and show a busy indicator for a write
that outlasts 250 ms. Synchronous and background saves of identical inputs
SHALL write identical bytes.

#### Scenario: The event loop stays live during a save [dl_save_worker]

- **WHEN** a native save is held in flight by the test seam
- **THEN** a repeating timer keeps firing with no gap over 50 ms and the canvas keeps repainting

#### Scenario: The synchronous API writes the worker bytes [dl_save_sync_bytes]

- **WHEN** two identical documents are saved, one through the synchronous API and one through a held worker save
- **THEN** the two files are byte-identical

### Requirement: Compatibility read-only state protects the opened document

When the document version policy requires read-only access, the document SHALL remain inspectable and identify its read-only state and reason. It SHALL use the source merged image when the image is present, decodable, and carries real merged data. When a faithful preview is unavailable, the application SHALL explain that limitation. Browsing and viewport navigation SHALL remain available.

Read-only access restricts the ordinary editing, Save and Save As flows for that document. Document changes and ordinary saves SHALL be refused across menu, direct tool, action playback, and programmatic application entry points. A refused operation SHALL leave file bytes, document content, path, modified state, and history unchanged. Opening or reverting to an editable document SHALL restore the normal lifecycle for that document.

#### Scenario: Read-only documents reject mutations and ordinary saves [dvp_read_only_guards]
- **WHEN** a read-only document is targeted by a menu edit, a canvas edit, an action step, an app save API, or Save or Save As
- **THEN** nothing is edited or written and the file, path, modified state, and history are unchanged

#### Scenario: Compatibility view uses available original merged pixels [dvp_preview]
- **WHEN** read-only documents open with a real usable merge and without one
- **THEN** the first shows the original merged image, and the second reports that a faithful preview is unavailable

#### Scenario: Document replacement refreshes access [dvp_replace]
- **WHEN** an editable document replaces a read-only session through open or revert
- **THEN** editing and saves follow the replacement document's policy

#### Scenario: Every output protects the compatibility source [dvp_output_source_guard]
- **WHEN** any output operation, including control-server `export_png` or an operation from a different active tab, targets the source path or a filesystem alias of any open read-only compatibility document
- **THEN** the operation refuses before writing and leaves source bytes and session unchanged; unrelated output destinations retain their ordinary behavior subject to their own copy restrictions

#### Scenario: Source composite becomes unavailable [dvp_source_decode_failure]
- **WHEN** source-backed merged pixels cannot be decoded because data is corrupt or the source changes after open
- **THEN** viewing reports the limitation and a compatibility copy fails with a reason without creating or modifying its destination

### Requirement: Background save preference and progress

File Handling SHALL expose Save in Background, enabled by default and persisted
across launches. Changing it SHALL apply to subsequent saves. Automatically
Save Recovery Information SHALL operate independently. During an interactive
background save the originating document tab SHALL display a percentage and
its lower-left status area SHALL show a percentage and progress bar when that
document is visible. Progress SHALL reflect completed phases, stay below 100
until the write succeeds, and clear on completion or failure. Switching tabs
SHALL keep each document's progress associated with that document.

#### Scenario: The background save preference persists [bs_preference]

- **WHEN** a fresh session opens File Handling, turns Save in Background off, and relaunches
- **THEN** the initial setting is on, it reloads as off, an interactive native save uses the existing waiting indicator, and the recovery preference and interval are unchanged

#### Scenario: Input and progress follow the document [bs_input_progress]

- **WHEN** the user starts a held native save, edits that document through the UI, and switches to and edits another document
- **THEN** both edits are accepted, progress is shown only for the originating document, and the progress indicators clear when the job finishes

### Requirement: Background save completion is bound to the saved document

A background save SHALL apply path, dirty-state, recent-file and action
recording effects to its original transaction when appropriate. A stale
success on the same document SHALL update its path and identify the written
history state as the saved state. It SHALL leave newer live work dirty,
including a live state that was clean relative to the previous file. A
completion for a replaced document SHALL leave the replacement's path and
dirty state unchanged. The application SHALL serialize writes to each
destination within the instance. A second interactive save for a document
with an outstanding job or to an outstanding job's destination SHALL be
refused with a save-in-progress notice.

A native save SHALL sync its temporary file and the destination directory
before reporting durable success. If publication fails, the live path and
dirty state SHALL stay unchanged on a pre-rename failure and a failure notice
SHALL identify the save. If rename succeeded but directory sync failed, the
notice SHALL indicate uncertain publication and the document SHALL acquire a
sticky protection obligation: it SHALL be treated as needing an unsaved-changes
prompt despite history navigation or its previous clean state. With recovery
enabled, retain or create recovery data for the latest committed live state,
including a state formerly clean, until a later durable current-state save or
an explicit Discard resolves the obligation. A successful Revert resolves it
only after the unsaved-changes decision. The path remains unchanged.

#### Scenario: Save As completes after a newer edit and a tab switch [bs_stale_save_as]

- **WHEN** the user starts native Save As for revision A, edits to B, and selects another document while the write is held
- **THEN** the destination contains A, the originating document has the new path and stays dirty at B, and the selected document is unaffected

#### Scenario: Returning to the previous saved state during a save [bs_previous_saved_state]

- **WHEN** the user saves edited revision A and undoes to the previously saved state before A lands
- **THEN** the destination contains A, the live state is dirty, and returning to the written history state clears dirty

#### Scenario: Competing saves respect document and destination [bs_competing_saves]

- **WHEN** a save is held and a user attempts another save of that document or a save to its destination from another tab
- **THEN** the conflicting requests show a save-in-progress notice and only the first snapshot is written

#### Scenario: An unsuccessful write leaves the live document intact [bs_durable_failure]

- **WHEN** a native save fails before rename or its destination directory sync fails after rename
- **THEN** the original remains intact on a pre-rename failure, the pre-rename failure preserves path, dirty state, and recovery; the post-rename failure preserves the path, reports uncertain publication, and protects the live state

#### Scenario: An uncertain publication protects the previously clean state [bs_uncertain_clean]

- **WHEN** save A is held, the user undoes to previously clean B, rename succeeds and directory sync fails, and the user later closes
- **THEN** the application retains or creates recovery for B, prompts even after history navigation, Cancel preserves B, and a later durable save of B or explicit Discard resolves the obligation

#### Scenario: A metadata-only change during a save stays modified [bs_metadata_stale]

- **WHEN** save A is held and the user commits only a Vanishing Point plane change B, then the save completes and history is navigated away and back
- **THEN** the file contains A and the live document remains modified with B until B is durably saved

### Requirement: Background save action recording preserves dispatch order

When recording is active at dispatch, a background save SHALL reserve its position
in that recording session. A successful save SHALL fill that position with the
original command and destination even if later commands were recorded or recording
was stopped or switched before completion. Failure SHALL remove the pending position
without recording a save step. Recording playback SHALL preserve dispatch order and
SHALL NOT execute an unresolved pending save step.

#### Scenario: Recording survives background completion and failure [bs_recording_order]

- **WHEN** a held save is dispatched while recording, a later command is recorded, recording is stopped or switched, and the save succeeds; repeat with a failed save
- **THEN** replay of the original recording runs the successful save before the later command with its original destination, the switched recording receives no save step, and the failed save contributes no step

### Requirement: Destructive actions resolve outstanding saves

Close, Revert, quit, and restart SHALL resolve any applicable pending save
before deciding whether to prompt for unsaved changes. After a successful
write they SHALL recheck the affected document. After a failed write they
SHALL keep it open unless the user explicitly discards the changes.
Choosing Save in an unsaved-changes prompt SHALL complete the save before
the operation proceeds and SHALL recheck the document for unsaved changes.

#### Scenario: Close rechecks work after the pending save [bs_close_recheck]

- **WHEN** the user requests close while a save of A is pending and the live document contains newer B, or the pending write fails
- **THEN** the application resolves the write before the close decision, prompts for the remaining unsaved work, and keeps the document open on Cancel

#### Scenario: Quit, restart, and Revert resolve pending writes [bs_lifecycle_recheck]

- **WHEN** the user invokes quit, restart, or Revert during an outstanding save for an affected document
- **THEN** the operation waits for the write outcome and the unsaved-changes decision, and Cancel leaves the document open with its edits

### Requirement: Native Open reports a terminal operation status

A native PSD/PSB loading-tab request SHALL carry its status on completion. The request SHALL distinguish successful open, malformed or unsupported native data, file-access failure, and an unwinding panic in the guarded open work. The completion and memory-budget refusal reason SHALL be tied to the request that produced them. Both ControlServer open routes SHALL use that request-owned reason while preserving existing protocol codes and cancellation precedence. User cancellation SHALL complete once as `OP_ERR_CANCELLED`. A later result from that request SHALL be discarded.

#### Scenario: Native Open classifies per-request I/O and codec failures [os_open_error_kinds]

- **WHEN** a malformed PSD and a different PSD that becomes unreadable after routing are opened concurrently
- **THEN** the malformed request completes with `OP_ERR_CODEC` and the unreadable request completes with `OP_ERR_IO`, each reported against its own file with no loading tab or document left for either request

#### Scenario: A panic completes a native Open [os_open_panic]

- **WHEN** a native Open worker panics while staging a document and the caller awaits the request
- **THEN** the request completes once with `OP_ERR_PANIC`, the loading tab is removed, the previous document stays active, the busy cursor is restored if this was the last request, and a later native Open succeeds

#### Scenario: Cancellation wins over an unpublished native Open result [os_open_cancel]

- **WHEN** the user cancels a native Open whose guarded worker has yet to deliver its result
- **THEN** the request completes once with `OP_ERR_CANCELLED`, no failure is shown, and the late worker result leaves the tabs and current document as they were after cancellation

#### Scenario: Memory refusal stays with its native Open request [os_open_memory_concurrent]

- **WHEN** a native Open refused by the memory budget through a ControlServer open route overlaps an independent application or GUI open that fails or is cancelled; each control completion adapter is exercised separately
- **THEN** each completion and protocol response belongs to its own request, the memory refusal retains its existing protocol code, and cancellation takes precedence over a late failure

### Requirement: Native Save reports an error and its publication outcome

Native PSD/PSB Save, Save As and Save a Copy SHALL carry the operation status separately from the file publication outcome. A codec refusal SHALL use `OP_ERR_CODEC`, a file I/O failure SHALL use `OP_ERR_IO`, and a guarded unwind SHALL use `OP_ERR_PANIC`. A panic before rename SHALL have the same preservation and temporary-file cleanup guarantees as any other pre-rename failure. A panic after rename and before durable acknowledgement SHALL be treated as uncertain publication. For Save and Save As, the existing uncertainty notice and recovery rules SHALL apply only to the still-current originating session. Save a Copy SHALL follow the copy-specific uncertainty rule in `interop/save-a-copy`; a replacement session SHALL NOT inherit the original save's protection obligations.

#### Scenario: Native Save distinguishes a codec refusal from an I/O failure [os_save_error_kinds]

- **WHEN** one native save fails in the encoder and another fails to write to the destination
- **THEN** Qt receives `OP_ERR_CODEC` for the encoder refusal and `OP_ERR_IO` for the file error, and both saves report pre-rename failure without changing the original file

#### Scenario: A native Save panics before rename [os_save_panic_before_rename]

- **WHEN** a native Save As worker panics while writing its temporary file
- **THEN** Qt receives `OP_ERR_PANIC` with pre-rename failure, the temp file is gone, and the destination bytes, live path, dirty state, and recovery data are unchanged

#### Scenario: A native Save panics after rename [os_save_panic_after_rename]

- **WHEN** a native save of the current document panics after replacing the file and before durable acknowledgement
- **THEN** Qt receives `OP_ERR_PANIC` with uncertain publication, the destination contains the written snapshot, the live document keeps its path and requires unsaved-changes protection, and a later durable save of the current state clears that obligation

#### Scenario: An uncertain save completes after its session is replaced [os_save_replaced_panic]

- **WHEN** a native save panics after rename and before durable acknowledgement, and its originating session is replaced before completion
- **THEN** the originating attempt reports `OP_ERR_PANIC` and uncertain publication while the replacement session keeps its path, dirty state, history and recovery data unchanged

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
