# Spec Delta

## Purpose

Loads several image files into one new layered document, matching Photoshop
CS6's `File ▸ Scripts ▸ Load Files into Stack` and its **Load Layers** dialog.

## ADDED Requirements

### Requirement: Load Layers command and dialog

`File ▸ Scripts ▸ Load Files into Stack…` SHALL be enabled with or without an
open document and SHALL open one modal **Load Layers** dialog. The dialog SHALL
contain a **Source Files** group with the explanatory text, a **Use** menu
offering `Files` and `Folder`, a source file list, the buttons `Browse…`,
`Remove`, and `Add Open Files`, the checkboxes **Attempt to Automatically Align
Source Images** and **Create Smart Object after Loading Layers**, and the
buttons `OK` and `Cancel`. Both checkboxes SHALL be off by default and the
**Use** menu SHALL default to `Files`. `OK` SHALL be disabled while the source
list is empty. Opening and cancelling the dialog SHALL leave every open
document's pixels, path, modified state, and history unchanged.

#### Scenario: The dialog presents the Load Layers controls [lfs_dialog]

- **WHEN** the user opens `File ▸ Scripts ▸ Load Files into Stack…`
- **THEN** the dialog shows Source Files, Use, the file list, Browse…, Remove, Add Open Files, both checkboxes, OK, and Cancel

#### Scenario: The defaults match CS6 [lfs_defaults]

- **WHEN** the dialog opens
- **THEN** Use is Files, both checkboxes are off, and the source list is empty

#### Scenario: The Use menu offers Files and Folder [lfs_use_menu]

- **WHEN** the Use menu is opened
- **THEN** it offers Files and Folder and no other source kind

#### Scenario: OK needs at least one source [lfs_empty_ok]

- **WHEN** the source list is empty
- **THEN** OK is disabled, and it becomes enabled after a source is added

#### Scenario: Cancel changes nothing [lfs_cancel]

- **WHEN** the dialog is opened and cancelled
- **THEN** every open document's pixels, path, modified state, and history are unchanged

### Requirement: Source selection

In `Files` mode, `Browse…` SHALL add the chosen image files to the source list.
`Remove` SHALL remove the selected rows. `Add Open Files` SHALL append the file
paths of open documents that have been saved to disk. A path SHALL appear at
most once in the list. In `Folder` mode, `Browse…` SHALL choose a folder and
populate the list with that folder's supported image files, and `Remove` SHALL
be disabled. The list SHALL preserve the order in which files were added.

#### Scenario: Browse adds files [lfs_browse]

- **WHEN** two image files are added through the file browser
- **THEN** both appear in the source list in that order

#### Scenario: Remove drops the selection [lfs_remove]

- **WHEN** one row is selected and Remove is pressed
- **THEN** only that row leaves the list

#### Scenario: Add Open Files appends saved documents [lfs_add_open]

- **WHEN** Add Open Files is pressed while a document saved as `photo.png` is open
- **THEN** `photo.png` is appended to the list

#### Scenario: Duplicates are ignored [lfs_dedupe]

- **WHEN** the same path is added twice
- **THEN** the list contains it once

#### Scenario: Folder mode lists the folder's images [lfs_folder]

- **WHEN** Use is Folder and a folder containing images and a text file is chosen
- **THEN** the list holds the folder's supported images and not the text file

### Requirement: Stack construction

`OK` SHALL decode every listed file and create one new untitled document whose
canvas width and height are the largest source width and height. Each decodable
file SHALL become one raster layer at its native size, positioned at the
canvas's top-left corner and named with the file's name including its
extension. The first listed file SHALL be the topmost layer. A file that cannot
be read or decoded SHALL be skipped and reported after the document is created.
When no listed file can be decoded, no document SHALL be created. The new
document SHALL be unmodified after creation and SHALL carry one
`Load Files into Stack` history state.

#### Scenario: A new document holds one layer per file [lfs_new_document]

- **WHEN** two readable images are loaded into a stack
- **THEN** a new untitled document with two layers is created

#### Scenario: The canvas is the largest source [lfs_canvas]

- **WHEN** the sources are 640 × 480 and 320 × 200
- **THEN** the new document is 640 × 480

#### Scenario: The first file is on top [lfs_order]

- **WHEN** `first.png` is listed before `second.png`
- **THEN** the topmost layer is named `first.png` and the layer below it `second.png`

#### Scenario: Unreadable files are skipped and reported [lfs_failures]

- **WHEN** one readable image and one missing path are loaded
- **THEN** the document has one layer and the missing path is reported

#### Scenario: No decodable file creates no document [lfs_all_failed]

- **WHEN** every listed file is missing or unreadable
- **THEN** no document is created

#### Scenario: No document is required [lfs_no_document]

- **WHEN** the command runs with no document open
- **THEN** a new stacked document is created

### Requirement: Auto-Align option

When **Attempt to Automatically Align Source Images** is checked and at least
two layers are loaded, the stack SHALL be aligned with the Auto projection
before the document is shown. When it is unchecked, every layer SHALL keep its
source position.

#### Scenario: Checked sources are aligned [lfs_align]

- **WHEN** two overlapping copies of the same content are loaded with the option on
- **THEN** the layers are repositioned to match their overlapping content

#### Scenario: Unchecked sources keep their positions [lfs_no_align]

- **WHEN** the same sources are loaded with the option off
- **THEN** every layer stays at the canvas's top-left corner

### Requirement: Smart Object option

When **Create Smart Object after Loading Layers** is checked, the loaded (and
aligned) layers SHALL be combined into one embedded smart object layer on the
same canvas, and the embedded source SHALL preserve the individual layers.
When it is unchecked, the layers SHALL remain separate.

#### Scenario: Checked stack becomes one smart object [lfs_smart_object]

- **WHEN** a two-file stack is loaded with the option on
- **THEN** the document holds one smart-object layer whose embedded source contains the two layers

#### Scenario: Unchecked stack stays layered [lfs_no_smart_object]

- **WHEN** a two-file stack is loaded with the option off
- **THEN** the document holds two separate raster layers
