# Spec Delta

## Purpose

Batch-exports every layer of a document as its own file, in a chosen format,
with each layer rendered alone on a document-sized transparency canvas.

## ADDED Requirements

### Requirement: Export Layers command and dialog

`File ▸ Scripts ▸ Export Layers to Files…` SHALL be enabled when a document is
open and SHALL open one modal dialog with a **Destination** folder (defaulting
to the document's folder), a **File Name Prefix** (defaulting to the document's
file stem), **Visible Layers Only** (off by default), a **File Type** menu, and
**Include ICC Profile** (off by default), closed by `Cancel` and `Export`. The
File Type menu SHALL offer `PSD`, `JPEG`, `TIFF`, `BMP`, and `PDF`, defaulting
to `PSD`; the PSD, JPEG, TIFF, and PDF choices SHALL enable Include ICC Profile
and the BMP choice SHALL disable it. Opening the dialog and cancelling it SHALL
leave the document's pixels, path, modified state, and history unchanged.

#### Scenario: The dialog presents the export controls [elf_dialog]

- **WHEN** the user opens `File ▸ Scripts ▸ Export Layers to Files…`
- **THEN** the dialog shows Destination, File Name Prefix, Visible Layers Only, File Type, Include ICC Profile, and Cancel and Export

#### Scenario: The dialog carries the documented defaults [elf_defaults]

- **WHEN** the dialog opens on a document saved as `poster.psd`
- **THEN** Destination is the document's folder, the prefix is `poster`, PSD is selected, Visible Layers Only and Include ICC Profile are off

#### Scenario: The format menu matches the supported set [elf_format_menu]

- **WHEN** the File Type menu is opened
- **THEN** it offers PSD, JPEG, TIFF, BMP, and PDF and no other format

#### Scenario: The ICC control follows the format [elf_icc_control]

- **WHEN** the File Type is BMP
- **THEN** Include ICC Profile is disabled, and it is enabled for PSD, JPEG, TIFF, and PDF

#### Scenario: Opening the dialog changes nothing [elf_dialog_unchanged]

- **WHEN** the user opens the dialog and cancels
- **THEN** the document's pixels, path, modified state, and history are unchanged

#### Scenario: The command is disabled without a document [elf_no_document]

- **WHEN** no document is open
- **THEN** `File ▸ Scripts ▸ Export Layers to Files…` is disabled

### Requirement: Every layer is exported alone

The export SHALL traverse the whole layer tree in the document's stored order,
visiting a group's children in place and in order. Every non-group node SHALL be
one output target: a pixel, smart-object, type, shape, fill, or adjustment
layer. A group SHALL NOT be a target itself; its children SHALL be. Each target
SHALL be rendered alone on a canvas of the document's pixel dimensions, at its
original position, with its own mask, effects, opacity, blend mode, and fill
preserved, and with transparency everywhere else. A layer inside a group SHALL
NOT receive the group's opacity, blend mode, or effects.

#### Scenario: One file per layer [elf_one_file_per_layer]

- **WHEN** a document has three pixel layers and the export runs
- **THEN** the destination contains exactly three files, one per layer

#### Scenario: The canvas is the document size [elf_document_canvas]

- **WHEN** a 640 × 480 document has one 100 × 100 layer
- **THEN** that layer's output is 640 × 480 with the layer at its original position and transparency around it

#### Scenario: Group children are targets, the group is not [elf_group_children]

- **WHEN** a document has one group holding two pixel layers
- **THEN** the export writes the two children as separate files and writes no file for the group

#### Scenario: Group effects are not applied to a child [elf_group_effects]

- **WHEN** a group has an effect or reduced opacity and one child is exported alone
- **THEN** the child's output shows only the child's own appearance

#### Scenario: Adjustment layers are targets [elf_adjustment]

- **WHEN** the document contains an adjustment layer
- **THEN** it produces a target; a raster format writes a fully transparent image for it and PSD or PDF carries the adjustment layer

### Requirement: Visibility follows the layer tree

With **Visible Layers Only** off, every layer SHALL be exported, including
hidden layers; the exported copy SHALL be rendered as visible. With it on, a
layer SHALL be exported only when the layer and all of its ancestors are
visible.

#### Scenario: Hidden layers export when the option is off [elf_hidden_off]

- **WHEN** Visible Layers Only is off and the document has a hidden layer
- **THEN** that layer produces a non-empty file

#### Scenario: Hidden layers are skipped when the option is on [elf_hidden_on]

- **WHEN** Visible Layers Only is on and the document has a hidden layer
- **THEN** no file is written for it

#### Scenario: A child of a hidden group is skipped [elf_hidden_group]

- **WHEN** Visible Layers Only is on and a visible child sits inside a hidden group
- **THEN** the child produces no file

### Requirement: Each target uses the chosen format

A target SHALL be written in the selected File Type using the corresponding
document serializer: PSD through the PSD writer, JPEG, TIFF, and BMP through the
raster save edge, and PDF through the PDF writer. JPEG and BMP SHALL be
flattened onto white; TIFF and PSD SHALL preserve transparency. The output's
pixel dimensions SHALL be the document's pixel dimensions. When Include ICC
Profile is on, JPEG, TIFF, PSD, and PDF SHALL carry the document's working RGB
profile, and their decoded pixels SHALL be the document's working RGB values
rather than an sRGB-normalized copy. When it is off, JPEG, TIFF, PSD, and PDF
SHALL be written untagged. The written suffix SHALL be `.psd`, `.jpg`, `.tif`,
`.bmp`, or `.pdf` for the five formats, independent of case in the chosen
prefix and layer name.

#### Scenario: PSD keeps the single layer [elf_psd_layer]

- **WHEN** the File Type is PSD and a pixel layer is exported
- **THEN** the file opens as a PSD with one layer at the layer's original position

#### Scenario: A raster format renders the layer alone [elf_raster_format]

- **WHEN** the File Type is TIFF and a layer is exported
- **THEN** the TIFF decodes to the document size with the layer's pixels and transparency elsewhere

#### Scenario: JPEG is flattened onto white [elf_jpeg_white]

- **WHEN** the File Type is JPEG and the layer has transparent pixels
- **THEN** the written JPEG is opaque and its transparent areas are white

#### Scenario: PDF carries the single layer [elf_pdf]

- **WHEN** the File Type is PDF and a layer is exported
- **THEN** the output is a one-page PDF whose page is the document's physical size (pixels × 72 / ppi) and whose editing payload contains that layer

#### Scenario: The ICC option embeds the profile [elf_icc]

- **WHEN** Include ICC Profile is on and the document's working profile is Adobe RGB
- **THEN** the written JPEG, TIFF, PSD, and PDF carry the Adobe RGB profile, and the JPEG and TIFF pixels decode to the document's Adobe RGB values

#### Scenario: The ICC option off writes untagged [elf_icc_off]

- **WHEN** Include ICC Profile is off
- **THEN** the written JPEG, TIFF, PSD, and PDF carry no embedded profile

### Requirement: Output names are sanitized and unique

The output file name SHALL be the File Name Prefix followed by the layer's
name, sanitized by replacing `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`, and
control characters with `_`, removing trailing dots and spaces, and falling
back to `Layer` when the result is empty. A name collision SHALL append ` 2`,
` 3`, and so on before the suffix, in traversal order. The final file name
component SHALL be at most 255 UTF-8 bytes, truncating the layer-name part on a
character boundary when needed.

#### Scenario: Illegal characters are replaced [elf_name_sanitize]

- **WHEN** the prefix is `Export` and the layer is named `a/b:c?`
- **THEN** the file is `Exporta_b_c_.tif` when TIFF is selected

#### Scenario: Collisions get a suffix [elf_name_collision]

- **WHEN** two layers sanitize to the same name
- **THEN** the second file gets ` 2` before its suffix and no file is overwritten

#### Scenario: An empty name falls back [elf_name_empty]

- **WHEN** the prefix and the sanitized layer name are both empty
- **THEN** the file name starts with `Layer`

#### Scenario: Long names are truncated [elf_name_long]

- **WHEN** the assembled file name component exceeds 255 UTF-8 bytes
- **THEN** it is truncated on a character boundary before the suffix and the write succeeds

### Requirement: The batch is cancellable and non-mutating

The export SHALL report progress as files are written, SHALL allow cancellation,
and SHALL stop before the next file after a cancel request. Files already
written SHALL remain. A destination that does not exist or is not writable SHALL
be refused before any file is written. A computed name that already exists as a
file SHALL be replaced; a directory at that path SHALL count as a write failure.
A failed write SHALL stop the batch and report the failing layer and path. A
cancelled or failed export SHALL leave the document's path, modified state,
history, and pixels unchanged and SHALL add no history state.

#### Scenario: Cancel stops the batch [elf_cancel]

- **WHEN** the user cancels while a multi-layer export is running
- **THEN** the export stops before the next file, files already written remain, and the document is unchanged

#### Scenario: A bad destination writes nothing [elf_bad_dest]

- **WHEN** the destination does not exist or is not writable
- **THEN** the export reports failure and writes no file

#### Scenario: An existing file is replaced [elf_replace]

- **WHEN** the destination already contains the computed name as a file
- **THEN** the file is replaced with the layer's output

#### Scenario: A directory at the target stops the batch [elf_directory_target]

- **WHEN** the destination contains a directory with the computed name
- **THEN** the export stops, reports the layer and path, and the document is unchanged

#### Scenario: A failed file stops the batch [elf_write_failure]

- **WHEN** a file cannot be written partway through the batch
- **THEN** the export stops, reports the layer and path, and the document is unchanged

#### Scenario: A successful batch leaves the document alone [elf_unchanged]

- **WHEN** the export completes
- **THEN** the document's path, modified state, history, and pixels are unchanged

### Requirement: A minimal document exports one file

A document with a single Background layer SHALL produce exactly one output file.

#### Scenario: A single-layer document writes one file [elf_single_layer]

- **WHEN** the export runs on a one-layer document
- **THEN** the destination contains exactly one file
