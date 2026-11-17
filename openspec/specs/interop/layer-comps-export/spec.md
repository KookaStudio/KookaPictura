# interop/layer-comps-export Specification

## Purpose
Exports every saved layer comp as its own file or as one page of a multi-page
PDF, without changing the open document.

## Requirements
### Requirement: Layer Comps to Files command and dialog

`File > Scripts > Layer Comps to Files` SHALL be enabled when a document is open
and SHALL open one modal dialog with a Destination folder (defaulting to the
document's folder), a File Name Prefix (defaulting to the document's file stem),
a File Type, and Include ICC Profile. The File Type menu SHALL offer PSD, JPEG,
TIFF, BMP, and PDF, defaulting to PSD. PSD, JPEG, TIFF, and PDF SHALL enable
Include ICC Profile; BMP SHALL disable it. The dialog SHALL be closed by Cancel
and Export. Cancelling SHALL write nothing and leave the document unchanged.
Both comps export commands SHALL be enabled through the application's command
registry when a document is open.

#### Scenario: The dialog presents its controls [lce_dialog]

- **WHEN** the command is chosen on an open document
- **THEN** the dialog shows Destination, File Name Prefix, File Type, Include ICC Profile, Cancel, and Export

#### Scenario: The defaults [lce_dialog_defaults]

- **WHEN** the dialog opens on `poster.psd`
- **THEN** the destination is the document's folder, the prefix is `poster`, PSD is selected, and Include ICC Profile is off

#### Scenario: The ICC control follows the format [lce_dialog_icc]

- **WHEN** the File Type is BMP
- **THEN** Include ICC Profile is disabled, and it is enabled for the other four formats

#### Scenario: Cancel writes nothing [lce_dialog_cancel]

- **WHEN** the dialog is cancelled
- **THEN** no file is written and the document's pixels, path, modified state, and history are unchanged

#### Scenario: No document [lce_dialog_no_document]

- **WHEN** no document is open
- **THEN** the command is disabled

### Requirement: Each comp is exported as an applied document

The export SHALL write exactly one output file per comp, in panel order. Each
output SHALL be the document with that comp's recorded classes applied, rendered
at the document's pixel dimensions. The written file SHALL use the selected File
Type and its corresponding serializer: PSD through the PSD writer, JPEG, TIFF,
and BMP through the raster save edge, and PDF through the PDF writer. JPEG and
BMP SHALL be flattened onto white; TIFF and PSD SHALL preserve transparency.
Include ICC Profile SHALL embed the document's working profile in PSD, JPEG,
TIFF, and PDF, and SHALL write them untagged when off. The file suffix SHALL be
`.psd`, `.jpg`, `.tif`, `.bmp`, or `.pdf` according to the chosen format. The
export SHALL refuse or settle an active transform or other blocking edit session
before it reads the document, so it never exports a mid-session state.

#### Scenario: One file per comp [lce_one_per_comp]

- **WHEN** a document with three comps is exported to Files
- **THEN** the destination contains exactly three files

#### Scenario: Each file reflects its comp [lce_comp_content]

- **WHEN** two comps record different layer visibility and both are exported
- **THEN** each output decodes to its own comp's rendered visibility

#### Scenario: The document is untouched [lce_document_unchanged]

- **WHEN** a Files export completes
- **THEN** the open document's layer states, pixels, history, path, and modified state are unchanged

#### Scenario: The ICC option follows the format [lce_icc]

- **WHEN** Include ICC Profile is on and the working profile is Adobe RGB
- **THEN** the written PSD, JPEG, TIFF, and PDF carry that profile

#### Scenario: No comps [lce_files_empty]

- **WHEN** the document has no comps and the export is requested
- **THEN** the export writes nothing and reports that there are no comps

#### Scenario: A transform session blocks export [lce_session_gate]

- **WHEN** a free-transform session is active and a comps export is requested
- **THEN** the export is refused or waits until the session is settled and does not export the mid-session document

### Requirement: Output names are sanitized and unique

The output file name SHALL be the File Name Prefix followed by the comp's name.
Both parts SHALL be sanitized by replacing `/`, `\`, `:`, `*`, `?`, `"`, `<`,
`>`, `|`, and control characters with `_`, and removing trailing dots and
spaces. The comp-name part SHALL fall back to `Layer Comp` when empty. A name
collision SHALL append ` 2`, ` 3`, and so on before the suffix, in panel order.
The final file name component SHALL be at most 255 UTF-8 bytes. Truncation SHALL
preserve the sanitized prefix and truncate the comp-name part on a character
boundary; when the prefix alone is too long it SHALL be truncated first.

#### Scenario: Illegal characters are replaced [lce_name_sanitize]

- **WHEN** the prefix is `Export` and the comp is named `a/b:c?`
- **THEN** the file is `Exporta_b_c_.tif` when TIFF is selected

#### Scenario: Collisions get a suffix [lce_name_collision]

- **WHEN** two comps sanitize to the same name
- **THEN** the second file gets ` 2` before its suffix and no file is overwritten

#### Scenario: An empty name falls back [lce_name_empty]

- **WHEN** the prefix and the sanitized comp name are both empty
- **THEN** the file name starts with `Layer Comp`

#### Scenario: Long names are truncated [lce_name_long]

- **WHEN** the assembled file name component exceeds 255 UTF-8 bytes
- **THEN** the prefix is preserved and the comp-name part is truncated on a character boundary before the suffix

#### Scenario: A long prefix is truncated first [lce_name_long_prefix]

- **WHEN** the sanitized prefix alone exceeds the 255-byte component cap
- **THEN** the prefix is truncated on a character boundary and the comp name is omitted or reduced so the suffix still fits

### Requirement: Layer Comps to PDF writes one page per comp

`File > Scripts > Layer Comps to PDF` SHALL be enabled when a document is open
and SHALL ask for a destination PDF. It SHALL write one PDF with one page per
comp, in panel order, each page the document's physical size (`pixels × 72 /
ppi`, 72 ppi when the resolution is unknown) and showing that comp applied as a
flattened composite. The page count SHALL equal the comp count. A document with
no comps SHALL write nothing and report that there are no comps. Cancel SHALL
write nothing.

#### Scenario: Page count equals comp count [lce_pdf_pages]

- **WHEN** a document with three comps is exported to PDF
- **THEN** the PDF has three pages

#### Scenario: Page size follows the document [lce_pdf_size]

- **WHEN** a 600 × 300 px document at 300 ppi is exported to PDF
- **THEN** each page's MediaBox is 144 × 72 points

#### Scenario: Each page shows its comp [lce_pdf_content]

- **WHEN** two comps record different visibility and the PDF is rendered
- **THEN** each page matches its comp's composite

#### Scenario: Cancel writes nothing [lce_pdf_cancel]

- **WHEN** the destination dialog is cancelled
- **THEN** no PDF is written and the document is unchanged

#### Scenario: No comps [lce_pdf_empty]

- **WHEN** the document has no comps and Layer Comps to PDF is chosen
- **THEN** no file is written and a clear message is shown

### Requirement: Exports are cancellable and never mutate the document

The Files export SHALL report progress and allow cancellation before the next
file. A destination that does not exist or is not writable SHALL be refused
before any file is written. A directory at a computed output path SHALL count as
a failure and stop the batch; a failed write SHALL report the comp and path and
stop the batch. An existing file at a computed path SHALL be replaced, matching
Export Layers. A cancelled or failed export SHALL leave files already written in
place and SHALL leave the document's pixels, path, modified state, history, and
layer comps unchanged, adding no history state. The PDF export SHALL obey the
same non-mutation rule.

#### Scenario: Cancel stops the batch [lce_cancel]

- **WHEN** the user cancels a multi-comp Files export
- **THEN** the export stops before the next file and the document is unchanged

#### Scenario: A bad destination writes nothing [lce_bad_destination]

- **WHEN** the destination does not exist or is not writable
- **THEN** no file is written and a failure is reported

#### Scenario: A directory target stops the batch [lce_directory_target]

- **WHEN** a computed output path is an existing directory
- **THEN** the export stops, reports the comp and path, and the document is unchanged

#### Scenario: An existing file is replaced [lce_replace]

- **WHEN** the destination already contains the computed file name as a file
- **THEN** the file is replaced with the comp's output

#### Scenario: A failed write stops the batch [lce_write_failure]

- **WHEN** a file cannot be written partway through
- **THEN** the export stops and reports the failing comp and path

#### Scenario: No history or dirty state [lce_no_history]

- **WHEN** either export finishes or is cancelled
- **THEN** the document's history depth and modified state are unchanged
