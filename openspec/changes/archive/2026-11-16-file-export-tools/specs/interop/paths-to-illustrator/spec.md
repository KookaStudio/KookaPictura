# Spec Delta

## Purpose

Exports the document's Work Path and saved paths to an Illustrator-openable
`.ai` file, preserving path geometry so vector outlines can be reused outside
the document.

## ADDED Requirements

### Requirement: Paths command and dialog

`File ▸ Export ▸ Paths to Illustrator…` SHALL be enabled when a document is open
and SHALL open one modal dialog with a **Destination** file path and a
**Format** menu offering `Illustrator 8 (EPS-based)` (the default) and
`Illustrator (PDF-based)`. The written file SHALL use the `.ai` suffix,
appending it when the chosen destination lacks it. Opening the dialog and
cancelling it SHALL leave the document's pixels, path, modified state, and
history unchanged.

#### Scenario: The dialog presents both formats [pt_dialog]

- **WHEN** the user opens `File ▸ Export ▸ Paths to Illustrator…`
- **THEN** the dialog shows a destination and a Format menu with Illustrator 8 (EPS-based) selected by default and Illustrator (PDF-based) offered

#### Scenario: The ai suffix is appended [pt_suffix]

- **WHEN** the user confirms a destination named `logo` or `logo.eps`
- **THEN** the written file is `logo.ai` in both cases

#### Scenario: Opening the dialog changes nothing [pt_dialog_unchanged]

- **WHEN** the user opens the dialog and cancels
- **THEN** the document's pixels, path, modified state, and history are unchanged

#### Scenario: The command is disabled without a document [pt_no_document]

- **WHEN** no document is open
- **THEN** `File ▸ Export ▸ Paths to Illustrator…` is disabled

### Requirement: Every document path is exported

The export SHALL write the Work Path and every saved path, in Paths panel order.
Each path SHALL record its saved name in the file, and each subpath SHALL
preserve its anchors, incoming and outgoing cubic handles, and open or closed
state. A straight segment MAY be written as a line; a curved segment SHALL be
written as a cubic Bézier. A saved name with non-ASCII or control characters
SHALL be escaped so the written file remains ASCII. The document dimensions
SHALL become the page or bounding-box size, and document pixel coordinates
SHALL map one-to-one to PostScript points with the origin moved from the
document's top-left to the page's bottom-left.

#### Scenario: All paths are present [pt_all_paths]

- **WHEN** a document has a Work Path and two saved paths
- **THEN** the written file contains all three, including both recorded saved-path names

#### Scenario: Saved names are ASCII [pt_name_ascii]

- **WHEN** a saved path is named with a non-ASCII character or a newline
- **THEN** the file remains ASCII and the name is recorded in an escaped form

#### Scenario: Open and closed subpaths survive [pt_open_closed]

- **WHEN** a saved path has one open and one closed subpath
- **THEN** the open subpath is not closed by the file and the closed one is

#### Scenario: Cubic handles survive [pt_handles]

- **WHEN** a path has a smooth curved segment with distinct handles
- **THEN** the written curve uses those handles and the round trip through a PostScript or PDF parser returns the same anchors and handles within the fixed decimal precision

#### Scenario: The page matches the document [pt_page_size]

- **WHEN** the document is 640 × 480
- **THEN** the EPS bounding box and the PDF media box are 640 × 480 points

### Requirement: The EPS-based file is a valid Illustrator 8 EPS

The default format SHALL write an EPS document starting with
`%!PS-Adobe-3.0 EPSF-3.0`, carrying `%%BoundingBox` and `%%Pages: 1`, followed
by one path construction per document path and a hairline stroke, and ending
with `showpage` and `%%EOF`. The output SHALL be ASCII, and a second export of
the same document SHALL be byte-identical.

#### Scenario: The EPS header and trailer parse [pt_eps_header]

- **WHEN** the EPS-based file is opened
- **THEN** it begins with the EPSF-3.0 header, contains the document's bounding box and `%%Pages: 1`, and ends with `showpage` and `%%EOF`

#### Scenario: The EPS strokes the paths [pt_eps_stroke]

- **WHEN** a path is exported in the EPS format
- **THEN** the file contains the path construction operators and a stroke operator, so the geometry is visible when opened

#### Scenario: The export is deterministic [pt_deterministic]

- **WHEN** the same document is exported twice to the EPS format
- **THEN** both files are byte-identical

### Requirement: The PDF-based file is a one-page PDF

The PDF-based format SHALL write a valid one-page PDF whose media box is the
document size, whose content stream constructs every path with `m`, `l`, and `c`
operators and, for a closed subpath, `h`, and whose paths carry the same
hairline stroke. The output
SHALL open in a PDF reader and in Illustrator, and SHALL NOT contain raster
image data.

#### Scenario: The PDF structure is valid [pt_pdf_structure]

- **WHEN** the PDF-based file is opened
- **THEN** it has one page, a media box matching the document, and a content stream with path and stroke operators

#### Scenario: The PDF has no pixels [pt_pdf_no_raster]

- **WHEN** the PDF-based file is inspected
- **THEN** it contains no image XObject and no embedded raster data

### Requirement: A path-less document is refused

When the document has neither a Work Path nor a saved path, the export SHALL
report `No paths to export` and SHALL write no file.

#### Scenario: No paths writes nothing [pt_no_paths]

- **WHEN** the user runs Paths to Illustrator on a document with no Work Path and no saved paths
- **THEN** the export reports that there are no paths and no file is created

### Requirement: The export is atomic and non-mutating

The file SHALL be written atomically (temporary file plus rename). A failed
write SHALL leave any existing destination file untouched and SHALL NOT change
the document's path, modified state, history, or pixels.

#### Scenario: An unwritable destination leaves the old file [pt_atomic]

- **WHEN** the destination cannot be written
- **THEN** the export reports failure and no temporary or partial file remains

#### Scenario: A successful export leaves the document alone [pt_unchanged]

- **WHEN** the export completes
- **THEN** the document's path, modified state, history, and pixels are unchanged
