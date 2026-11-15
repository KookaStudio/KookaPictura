# Spec Delta

## Purpose

Write a sliced document for the web: one image per resolved slice, plus the
HTML table or CSS layout that reassembles those images at their original
positions, through the Save Optimized As format choice.

## ADDED Requirements

### Requirement: Slice images

When Save for Web is confirmed on a document that has at least one user slice,
the system SHALL resolve the user slices into non-overlapping export cells with
`pictura_render::resolve_export_slices`: later-created slices in
`Document::slices` order own any overlap, the remainder of an earlier rectangle
is split into rectangles, and auto slices fill the rest. The canvas overlay
SHALL keep the overlapping `resolve_slices` result unchanged.

The system SHALL write one image per resolved slice, user and auto, cropped from
the composite at the Image Size dimensions and encoded with the active pane's
format and settings. A slice whose width or height is zero after Image Size
scaling SHALL be omitted from both the images and the layout, and the remaining
slices SHALL still cover the canvas. Each file SHALL be named
`<document>_NN.<ext>`, where `NN` is the slice's reading-order number
zero-padded to at least two digits, and SHALL be written into an `images/`
subfolder beside the chosen output path. A slice's rect edges SHALL scale with
Image Size so adjacent slices keep their shared edges. A document with no user
slices SHALL keep the existing single-image Save behaviour.

#### Scenario: Two slices write two cropped images [slice-images]

- **WHEN** a 64 x 48 document with two user slices is saved as GIF with HTML and Images
- **THEN** `images/<document>_01.gif` and `images/<document>_02.gif` are written, each decodes to its slice's width and height, and each holds the matching region of the composite

#### Scenario: Overlapping slices export disjointly [slice-overlap]

- **WHEN** a document with two overlapping user slices is saved with HTML and Images
- **THEN** the written images do not overlap, together they cover the canvas, and the later-created slice owns the overlapping region

#### Scenario: Image Size scales the slices [slice-images-size]

- **WHEN** the same document is halved through Image Size and saved
- **THEN** every non-zero slice image is half its previous width and height, rounded, zero-area slices are omitted from the images and layout, and the remaining slices still cover the 32 x 24 canvas without gaps

### Requirement: HTML table reassembly

The HTML and Images and HTML Only formats SHALL write an HTML file beside the
images whose body holds a `<table id="Table_01">` of exactly the non-zero
export cells: one `<td>` per non-zero slice, with `colspan` and `rowspan` when
the slice spans several columns or rows, containing an `<img
src="images/<name>" width=W height=H alt="">`. The file SHALL use UTF-8, a
`<title>` naming the document, the Content-Type meta tag, zero body margins,
and Save for Web Slices comments around the table. Text and attribute values
SHALL be HTML-escaped (`&`, `<`, `>`, `"`), image paths SHALL be URL-encoded,
and CSS ids SHALL replace every character outside `[A-Za-z0-9_-]` with `_`, and
a sanitized id that starts with a digit SHALL be prefixed with `s_`. The
rendered table SHALL occupy the optimised canvas exactly with no gaps or
overlaps.

#### Scenario: The table reassembles a jigsaw [slice-html-table]

- **WHEN** a 4 x 4-slice grid with an L-shaped arrangement of rectangular slices (one tall slice spanning two rows plus two short ones) is exported as HTML
- **THEN** the table's cells plus their spans cover the canvas rectangle exactly, and every image path names a file written beside the HTML

#### Scenario: Names and paths are escaped [slice-html-escape]

- **WHEN** a document whose name contains `&"<>` is saved as HTML with HTML and Images
- **THEN** the title and attribute values escape `&`, `<`, `>`, and `"`, image paths are URL-encoded, and CSS ids replace other characters with `_` (prefixing a digit-leading id with `s_`)

#### Scenario: HTML Only writes no images [slice-html-only]

- **WHEN** the same document is saved with Format HTML Only
- **THEN** the HTML file is written with the same table and image paths, and no `images/` files are created

### Requirement: CSS layout

The HTML and Images and HTML Only formats SHALL also offer a CSS layout that
emits a positioned wrapper `<div id="Table_01">` with `position:relative` and,
for each non-zero slice, an absolutely positioned `<div>` at the slice's left,
top, width, and height holding the same `<img>`. The positioning rules SHALL
appear in a `<style>` block in the document head, keyed by the slice names, and
the images, comments, title, and charset SHALL match the table format. Text and
attribute values SHALL be HTML-escaped, image paths URL-encoded, and CSS ids
sanitized as in the table format.

#### Scenario: CSS places every slice [slice-css]

- **WHEN** a sliced document is exported with the CSS layout
- **THEN** the HTML contains no table, one positioned div per non-zero slice whose left/top/width/height equal the slice rect, and each img path names the written slice image

### Requirement: Save Optimized As output choice

On Save, a document with user slices SHALL open Save Optimized As offering a
file name, Format = HTML and Images, Images Only, or HTML Only, Slices = All
Slices, and Layout = Table or CSS. HTML and Images SHALL write the HTML file
and the slice images; Images Only SHALL write the slice images without an HTML
file; HTML Only SHALL write the HTML file without images. The chosen file name
SHALL name the HTML file in the HTML formats and supply the `<document>` prefix
for the slice file names. Cancelling SHALL write nothing. These choices SHALL
NOT change the document, its path, its modified state, or its history.

#### Scenario: The three formats write the right files [slice-save-formats]

- **WHEN** a sliced document is saved as HTML and Images, then as Images Only, then as HTML Only
- **THEN** the first writes an HTML file and the slice images, the second writes only the slice images, and the third writes only the HTML file

#### Scenario: Cancelling writes nothing [slice-save-cancel]

- **WHEN** Save Optimized As is cancelled
- **THEN** no file is written and the document's pixels, path, modified state, and history are unchanged

#### Scenario: An unsliced document keeps the image save [slice-unsliced]

- **WHEN** a document with no user slices is saved
- **THEN** one optimised image is written at the chosen path and no HTML file or images folder is created
