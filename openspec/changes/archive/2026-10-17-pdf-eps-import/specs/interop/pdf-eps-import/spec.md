## ADDED Requirements

### Requirement: Opening a generic PDF shows the Import PDF dialog

`File > Open`, Open Recent, and drag-and-drop SHALL recognize a PDF by its
`%PDF-` signature in the first 1 KiB, whatever its extension. A PDF that
carries Pictura's embedded Photoshop document SHALL reopen with its layers and
no dialog. Any other PDF SHALL show the **Import PDF** dialog before anything
opens; Cancel SHALL open nothing.

#### Scenario: A generic PDF opens the dialog [pdf_import_dialog_shown]

- **WHEN** a PDF without Pictura's editing payload is opened
- **THEN** the Import PDF dialog is shown and no document opens until OK

#### Scenario: A Pictura Photoshop PDF skips the dialog [pdf_pictura_skips_dialog]

- **WHEN** a PDF saved by Pictura with Preserve Photoshop Editing Capabilities is opened
- **THEN** it opens with its layers and the Import PDF dialog is not shown

#### Scenario: A PDF is recognized by signature [pdf_sniff_signature]

- **WHEN** a PDF file named `art.ai` is opened
- **THEN** the Import PDF dialog is shown

#### Scenario: Cancel opens nothing [pdf_cancel]

- **WHEN** the Import PDF dialog is cancelled
- **THEN** no document is added

### Requirement: Import PDF dialog layout and defaults

The dialog SHALL show, under **Select**, the radios **Pages**, **Images**, and
**3D** (3D disabled), a thumbnail grid numbered from 1, the text `<k> of <n>
page(s) selected` (or `image(s)` in Images mode), and **Thumbnail Size**
(Small, Large, Fit Page; default Small). **Page Options** SHALL hold **Name**
(the file name), **Crop To** (Bounding Box, Media Box, Crop Box, Bleed Box,
Trim Box, Art Box; default Bounding Box), and **Anti-aliased** (on). **Image
Size** SHALL show the uncompressed size of the result and hold **Width** and
**Height** with units (Pixels, Inches, Centimeters, Millimeters, Points,
Picas; default Inches), **Constrain Proportions** (on), **Resolution** (300,
Pixels/Inch or Pixels/Centimeter), **Mode** (Grayscale, RGB Color, CMYK Color,
Lab Color; default RGB Color), and **Bit Depth** (8 bit, 16 bit; default 8).
**Suppress Warnings** SHALL be off. The first page SHALL be selected.

#### Scenario: Defaults match Photoshop [pdf_dialog_defaults]

- **WHEN** a one-page PDF named `sample.pdf` is opened
- **THEN** Pages is checked, 3D is disabled, the label reads `1 of 1 page(s) selected`, Name is `sample.pdf`, Crop To is Bounding Box, Anti-aliased is on, units are Inches, Resolution is 300 Pixels/Inch, Mode is RGB Color, Bit Depth is 8 bit, Thumbnail Size is Small, and Suppress Warnings is off

#### Scenario: Thumbnail size changes the grid [pdf_thumbnail_size]

- **WHEN** Thumbnail Size is changed from Small to Large to Fit Page
- **THEN** the thumbnails grow from 64 px to 128 px to the grid's width

### Requirement: Crop To selects the page box

Width and Height SHALL follow the chosen box of the first selected page at
the current resolution. The Media, Crop, Bleed, Trim, and Art boxes SHALL come
from the page with the PDF defaults (CropBox defaults to MediaBox; Bleed, Trim,
and Art default to CropBox; all clipped to MediaBox) and SHALL follow the
page's `/Rotate`. Bounding Box SHALL be the smallest rectangle holding the
page's painted marks; a blank page SHALL use its Crop Box.

#### Scenario: Page boxes and defaults [pdf_page_boxes]

- **WHEN** a page has MediaBox `[0 0 612 792]`, CropBox `[36 36 576 756]`, and TrimBox `[72 72 540 720]` and no BleedBox or ArtBox
- **THEN** Media Box is 612 × 792 pt, Crop Box and Bleed Box and Art Box are 540 × 720 pt, and Trim Box is 468 × 648 pt

#### Scenario: Bounding Box hugs the marks [pdf_bounding_box]

- **WHEN** a 144 × 72 pt page paints only a rectangle from (20, 20) to (80, 50)
- **THEN** the Bounding Box is 60 × 30 pt and the dialog shows Width 0.833 and Height 0.417 inches

#### Scenario: Rotated pages swap width and height [pdf_page_rotate]

- **WHEN** a 144 × 72 pt page has `/Rotate 90`
- **THEN** its Media Box is 72 pt wide and 144 pt tall and the render is portrait

### Requirement: Image Size controls

With Constrain Proportions on, editing Width SHALL rescale Height by the box's
aspect ratio and the reverse. Changing Resolution SHALL keep the physical
width and height and change the pixel count. Changing the unit SHALL convert
the value without changing the pixel size. The Image Size label SHALL be
`width × height × channels × bytes per channel`, formatted with K/M suffixes.
With several pages selected, Width and Height SHALL show the largest selected
page box, and each page SHALL scale by the same factor.

#### Scenario: Constrain keeps the aspect ratio [pdf_constrain_proportions]

- **WHEN** Crop To is Media Box on a 144 × 72 pt page and Width is set to 4 inches
- **THEN** Height becomes 2 inches; with Constrain Proportions off, Height stays where it was

#### Scenario: Resolution keeps the physical size [pdf_resolution_keeps_size]

- **WHEN** Resolution changes from 300 to 150 with sizes in Inches
- **THEN** Width and Height in inches are unchanged and the Image Size label quarters

#### Scenario: Several pages show the largest box [pdf_multipage_max_size]

- **WHEN** pages of 144 × 72 pt and 288 × 144 pt are both selected with Crop To Media Box
- **THEN** Width shows 4 inches and Height 2 inches, and opening scales the smaller page to 600 × 300 px at 300 ppi

### Requirement: Opening pages

OK SHALL open one document per selected page, in page order. Each document
SHALL be untitled, hold one transparent layer named `Layer 1` with the page
rendered at the chosen pixel size from the chosen box, record the chosen
resolution, and be in the chosen mode and bit depth. Areas the page does not
paint SHALL stay transparent. Anti-aliased off SHALL render edges without
intermediate coverage. A single document SHALL be named by Name; several SHALL
be named by Name's stem plus `-<page number>` and Name's extension.

#### Scenario: The page renders at the chosen size [pdf_render_page]

- **WHEN** a 144 × 72 pt page that paints a red rectangle from (20, 20) to (80, 50) pt is opened with Crop To Media Box at 72 ppi
- **THEN** the document is 144 × 72 px at 72 ppi with one layer `Layer 1`, pixels inside the rectangle are opaque red, and pixels outside are transparent

#### Scenario: Anti-aliasing toggles edge coverage [pdf_antialias]

- **WHEN** a page with a diagonal edge is opened with Anti-aliased on, then off
- **THEN** the first render has partially transparent edge pixels and the second has only fully opaque or fully transparent pixels

#### Scenario: Mode and bit depth apply [pdf_mode_depth]

- **WHEN** a page is opened as CMYK Color at 16 bit, and again as Grayscale at 8 bit
- **THEN** the documents are CMYK 16-bit and Grayscale 8-bit

#### Scenario: Several pages open as several documents [pdf_multi_select_documents]

- **WHEN** pages 1 and 3 of a three-page `deck.pdf` are selected and opened
- **THEN** two documents open, named `deck-1.pdf` and `deck-3.pdf`, and the label read `2 of 3 page(s) selected`

### Requirement: Images mode opens embedded images

In Images mode the grid SHALL list each distinct image XObject the PDF's pages
draw, including those inside Form XObjects, in page order. Page Options and
Image Size SHALL be disabled. OK SHALL open each selected image as its own
document at its native pixel size, with its soft mask as transparency, at the
chosen mode and depth. An encrypted PDF SHALL disable the Images radio.

#### Scenario: Images are listed [pdf_images_mode_lists]

- **WHEN** a page draws a 40 × 30 px image twice and a 16 × 16 px image inside a Form XObject, and Images is chosen
- **THEN** two images are listed and the label reads `1 of 2 image(s) selected`

#### Scenario: An image opens at its own size [pdf_images_mode_opens]

- **WHEN** the 40 × 30 px Flate RGB image is opened from Images mode
- **THEN** a 40 × 30 px document opens whose pixels equal the image's samples

#### Scenario: The object reader survives a broken xref [pdf_objects_broken_xref]

- **WHEN** a PDF's `startxref` offset points past the end of the file
- **THEN** its objects are still found by scanning for `obj` headers

### Requirement: Encrypted PDFs ask for a password

Opening a PDF that needs a password SHALL prompt for it before the dialog. A
wrong password SHALL prompt again; Cancel SHALL open nothing.

#### Scenario: The right password opens [pdf_password_prompt]

- **WHEN** a PDF encrypted with user password `kooka` is opened and `kooka` is entered
- **THEN** the Import PDF dialog shows its page

#### Scenario: A wrong password asks again [pdf_wrong_password]

- **WHEN** `wrong` is entered, then the prompt is cancelled
- **THEN** the prompt is shown a second time and no document opens

### Requirement: Warnings after import

Recoverable problems Ghostscript reports while rendering SHALL be shown in one
message after the import unless Suppress Warnings is checked.

#### Scenario: Suppress Warnings hides the message [pdf_suppress_warnings]

- **WHEN** a damaged PDF that Ghostscript repairs is opened with Suppress Warnings off, then on
- **THEN** a warning message is shown the first time and not the second, and both open the page

### Requirement: Ghostscript is required

PDF and EPS rasterization SHALL run Ghostscript as a separate process with
`-dSAFER`. When it is absent or fails, opening SHALL report a clear error
naming Ghostscript and open nothing, without crashing.

#### Scenario: Missing Ghostscript reports an error [gs_missing_reports]

- **WHEN** Ghostscript cannot be found and a generic PDF or an EPS is opened
- **THEN** an error naming Ghostscript is reported and no document is added

### Requirement: Opening an EPS rasterizes it

An EPS, recognized by `%!PS-Adobe` or the DOS EPS header `C5 D0 D3 C6`, SHALL
show the **Rasterize Generic EPS Format** dialog: **Image Size**, **Width** and
**Height** (default the `%%BoundingBox` at the resolution, unit Inches),
**Resolution** (300 Pixels/Inch), **Mode** (Grayscale, RGB Color, CMYK Color,
Lab Color; default RGB Color), **Constrain Proportions** (on), and
**Anti-aliased** (on). OK SHALL open one untitled document with one
transparent layer rendered from the bounding box; Cancel SHALL open nothing.
`%%HiResBoundingBox` SHALL be preferred when present.

#### Scenario: EPS dialog defaults [eps_dialog_defaults]

- **WHEN** an EPS with `%%BoundingBox: 0 0 144 72` is opened
- **THEN** the dialog shows Width 2 and Height 1 inches at 300 Pixels/Inch, RGB Color, Constrain Proportions and Anti-aliased on

#### Scenario: EPS renders at the chosen size [eps_render]

- **WHEN** that EPS, painting a blue rectangle over its whole bounding box, is opened at 72 ppi
- **THEN** a 144 × 72 px document opens whose pixels are opaque blue

#### Scenario: DOS binary EPS is read [eps_dos_binary_header]

- **WHEN** an EPS wrapped in a DOS binary header with a TIFF preview is opened
- **THEN** its PostScript section's bounding box sizes the dialog and the render

#### Scenario: Cancel opens nothing [eps_cancel]

- **WHEN** the Rasterize Generic EPS Format dialog is cancelled
- **THEN** no document is added
