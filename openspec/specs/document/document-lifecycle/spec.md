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
The system SHALL open a PSD/PSB file chosen from a file dialog into a new
document, and SHALL offer the most recently opened files through an Open Recent
list. A successful open SHALL leave the document unmodified with its path
recorded.

#### Scenario: Open a file adds a document
- **WHEN** the user opens a valid PSD
- **THEN** a new tab shows the composited document and the command line argument path is recorded

#### Scenario: Open failure does not add a document
- **WHEN** the chosen file cannot be read
- **THEN** no tab is added and the application remains usable

#### Scenario: Recent files
- **WHEN** a document is opened successfully
- **THEN** it appears at the front of the bounded Open Recent list

### Requirement: Save and Save As

The system SHALL write the active document in its current output format, chosen
by the save path's extension, writing atomically so a failed write never
truncates an existing file. A `psd` path SHALL write the document's composite
and layer tree through the PSD codec; a `psb` path SHALL force a version-2 PSB
container through the codec. A supported raster path (PNG, JPEG, TIFF, WebP, or
BMP) SHALL write the flattened composite through the Qt-boundary raster encode
edge. Save As SHALL default its format to PSD unless the document is a single
pixel or background layer imported from a supported raster, in which case it
SHALL default to that source format. When the document's recorded path is a
raster format and the document has more than one layer, a group, an adjustment,
or a type layer, Save SHALL behave as Save As rather than silently flattening
into that raster file. Saving a document that has more than one layer, a group,
an adjustment, or a type layer to a format that cannot hold them SHALL warn
before writing and then write the flattened composite. The `composite`
serialized to PSD SHALL be the active document's current rendered composite, so
a save after any edit round-trips to the edited pixels rather than to a pre-edit
merged image. The `write_psd` byte layout and its use of `doc.composite.channels`
to derive the PSD colour-plane count MUST NOT change; an RGB document's merged
composite is RGBA (four planes) after a canvas rebuild, while a grayscale
document keeps its composite colour-plane count. Save on an untitled document
SHALL behave as Save As. A successful save SHALL record the file path and clear
the modified state.

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

### Requirement: Modified state tracking
The system SHALL expose `is_dirty()` for the active document. Every successful
mutating command SHALL mark the document modified; opening and saving SHALL clear
the modified state. A rejected command SHALL NOT change the modified state.

#### Scenario: A mutating command marks modified
- **WHEN** a filter, adjustment, document op, or selection command succeeds
- **THEN** the document reports modified

#### Scenario: Save clears modified
- **WHEN** a modified document is saved successfully
- **THEN** the document reports unmodified

#### Scenario: Open clears modified
- **WHEN** a document is opened
- **THEN** it reports unmodified

### Requirement: Unsaved-changes prompt
The system SHALL prompt Save / Discard / Cancel before closing, reverting, or
replacing a modified document. Cancel SHALL abort the operation and leave the
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
The system SHALL revert a saved document by reloading it from its path and
discarding in-memory changes. Revert SHALL be unavailable for an untitled
document.

#### Scenario: Revert reloads from disk
- **WHEN** the user reverts a modified document with a path
- **THEN** the document matches the file on disk and reports unmodified

#### Scenario: Revert unavailable when untitled
- **WHEN** the active document has no path
- **THEN** Revert is disabled

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

### Requirement: Open shows a loading banner

The application SHALL display a loading banner while a document is being opened:
a boxed indicator at the window's lower-left corner showing `Opening <file
name>…`. The banner SHALL use the theme's notice style — a rounded box on the
notice yellow with the dark notice text — so it stays legible over the canvas.
The banner SHALL be shown for every open entry point — File > Open, Open Recent,
a file drop, and Open As Smart Object — and for both PSD/PSB (native reader) and
raster (image import) sources. The application SHALL paint the banner before the
blocking read/decode begins, so it is visible while the open runs; the load
itself MAY remain synchronous on the GUI thread. When the open finishes the
application SHALL hide the banner and restore the wait cursor, whether the open
succeeded or was refused. A successful open that reports a notice (for example
`Converted from CMYK; saved as CMYK`) SHALL show that notice in the status bar.

#### Scenario: Every open entry point shows the loading banner [olb_banner]

- **WHEN** a document is opened through File > Open, Open Recent, a file drop, or Open As Smart Object, from a PSD/PSB or a raster source
- **THEN** while the read/decode is running the lower-left banner is visible showing `Opening <file name>…` for that file and the wait cursor is active

#### Scenario: The banner is a rounded notice-coloured box [olb_style]

- **WHEN** the application stylesheet is built
- **THEN** the loading banner has a rule giving it the notice background, the dark notice text, and rounded corners

#### Scenario: The banner clears when the open finishes [olb_clears]

- **WHEN** an open finishes, whether it succeeded or was refused
- **THEN** the banner is hidden and the wait cursor is restored

#### Scenario: A notice replaces the banner [olb_notice]

- **WHEN** an open succeeds and the opened document reports a notice
- **THEN** the banner is hidden and the status bar shows that notice
