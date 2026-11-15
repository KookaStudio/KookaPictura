# interop/pdf-eps-import Specification

## Purpose
Generic PDF and EPS import: `File ▸ Open` on a PDF without Pictura's editing
payload shows the Import PDF dialog (pages or embedded images, page box,
raster size, mode, depth), an EPS shows the Rasterize Generic EPS Format
dialog, and Ghostscript, run as a separate process, rasterizes the choice into
untitled documents.

## Requirements

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

In Images mode the grid SHALL list the image XObjects a PDF's pages draw,
including those inside Form XObjects, in page order, followed by the inline
images (`BI` / `ID` / `EI`) in page content-stream order; only page content
streams SHALL be scanned, so inline images inside Form XObject content streams
SHALL NOT be listed. Every inline occurrence SHALL be listed, and images inside
patterns SHALL NOT be listed. Content streams with an unsupported filter SHALL
yield no inline images, though their XObjects SHALL still be listed. Page
Options SHALL be disabled; Width, Height, their units, Constrain Proportions,
and Resolution SHALL be disabled; Mode and Bit Depth SHALL stay live. OK SHALL
open each selected image as its own document at its native pixel size, with its
soft mask as transparency, at the chosen mode and depth. For an encrypted PDF,
Images SHALL stay disabled until the entered password unlocks the file.

#### Scenario: Images are listed [pdf_images_mode_lists]

- **WHEN** a page draws a 40 × 30 px image twice and a 16 × 16 px image inside a Form XObject, and Images is chosen
- **THEN** two images are listed and the label reads `1 of 2 image(s) selected`

#### Scenario: An image opens at its own size [pdf_images_mode_opens]

- **WHEN** the 40 × 30 px Flate RGB image is opened from Images mode
- **THEN** a 40 × 30 px document opens whose pixels equal the image's samples

#### Scenario: The object reader survives a broken xref [pdf_objects_broken_xref]

- **WHEN** a PDF's `startxref` offset points past the end of the file
- **THEN** its objects are still found by scanning for `obj` headers

#### Scenario: Encrypted images need the password [pdf_images_encrypted]

- **WHEN** a PDF encrypted with user password `kooka` is opened, `kooka` is entered, and Images is chosen
- **THEN** the Images radio is enabled and the grid lists the PDF's images

#### Scenario: Inline images are listed [pdf_inline_images_listed]

- **WHEN** a page draws one image XObject and one inline image and Images is chosen
- **THEN** both are listed in page order, the XObject before the inline image, and the label counts two images

#### Scenario: An inline image opens [pdf_inline_image_opens]

- **WHEN** the inline image is opened from Images mode
- **THEN** a document opens at the inline image's declared width and height whose pixels equal its samples

#### Scenario: Inline images with an unsupported filter [pdf_inline_images_skip_unknown_filters]

- **WHEN** a page's content stream uses an unsupported filter and draws one image XObject and one inline image, and Images is chosen
- **THEN** the page still lists its image XObject and the inline image is not listed

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
Lab Color; default RGB Color), **Bit Depth** (8 bit, 16 bit; default 8 bit),
**Constrain Proportions** (on), and **Anti-aliased** (on). OK SHALL open one
untitled document with one transparent layer rendered from the bounding box;
Cancel SHALL open nothing. `%%HiResBoundingBox` SHALL be preferred when present.

#### Scenario: EPS dialog defaults [eps_dialog_defaults]

- **WHEN** an EPS with `%%BoundingBox: 0 0 144 72` is opened
- **THEN** the dialog shows Width 2 and Height 1 inches at 300 Pixels/Inch, RGB Color, Bit Depth is 8 bit, Constrain Proportions and Anti-aliased on

#### Scenario: EPS renders at the chosen size [eps_render]

- **WHEN** that EPS, painting a blue rectangle over its whole bounding box, is opened at 72 ppi
- **THEN** a 144 × 72 px document opens whose pixels are opaque blue

#### Scenario: DOS binary EPS is read [eps_dos_binary_header]

- **WHEN** an EPS wrapped in a DOS binary header with a TIFF preview is opened
- **THEN** its PostScript section's bounding box sizes the dialog and the render

#### Scenario: Cancel opens nothing [eps_cancel]

- **WHEN** the Rasterize Generic EPS Format dialog is cancelled
- **THEN** no document is added

### Requirement: Photoshop's own PDFs reopen with their layers

A PDF whose first page carries Photoshop's editing data
(`/PieceInfo /AdobePhotoshop /Private` with `/ImageResources`,
`/StandardImageFileData`, and `/CompositeImage`) SHALL open with its layers,
without the Import PDF dialog: the data SHALL be rebuilt into a PSD and read
as one, so pixel layers keep their pixels and type layers stay type layers.
A PDF without that data, or one whose editing data cannot be rebuilt, SHALL
fall back to the Import PDF dialog.

#### Scenario: A Photoshop PDF keeps its type layer [pdf_photoshop_layers]

- **WHEN** a PDF holding Photoshop editing data for a background and a type layer `PDF FROM PHOTOSHOP` is opened
- **THEN** a document opens without the Import PDF dialog with the same two layers, the type layer is a type layer with that text, and the layer pixels and composite match

#### Scenario: Other PDFs still import [pdf_photoshop_absent]

- **WHEN** a PDF without Photoshop editing data is opened
- **THEN** no PSD is rebuilt and the Import PDF dialog is shown

### Requirement: Password-protected Photoshop PDFs reopen with their layers

Opening a PDF encrypted with the standard security handler (RC4 40/128-bit,
AES-128, or AES-256) SHALL ask for its password before anything else, and
either the open (user) or the permissions (owner) password SHALL open it. A
file whose editing data (Pictura's or Photoshop's) the password unlocks SHALL
open with its layers, without the Import PDF dialog; one without editing data
SHALL continue to the Import PDF dialog without asking again. A file with only
a permissions password SHALL open without a prompt. Revert SHALL reopen the
file without asking for the password again.

#### Scenario: A password-protected Photoshop PDF keeps its layers [pdf_encrypted_layers]

- **WHEN** a two-layer document is saved as Photoshop PDF with open password `kooka` at any compatibility level, and reopened by entering `wrong`, then `kooka`
- **THEN** the prompt is shown twice and the document opens with both layers, without the Import PDF dialog

#### Scenario: The permissions password also opens it [pdf_owner_password_opens]

- **WHEN** that file, also given permissions password `owner`, is opened with `owner`
- **THEN** its editing data opens with both layers

#### Scenario: A permissions password alone needs no prompt [pdf_permissions_only_layers]

- **WHEN** a two-layer document saved with only a permissions password is opened
- **THEN** it opens with both layers and no password prompt

#### Scenario: Revert keeps the password [pdf_encrypted_revert]

- **WHEN** a reopened password-protected Photoshop PDF is edited and reverted
- **THEN** it reopens with its layers without a prompt

### Requirement: Native mode and depth rendering

A page, embedded image, or EPS opened with CMYK mode or 16-bit depth SHALL be
rasterized by Ghostscript natively. CMYK and RGB SHALL render in that mode and
depth through Ghostscript's PSD devices; 16-bit Grayscale SHALL derive from the
native RGB 16-bit render at 16-bit precision, and a Lab document SHALL keep the
existing Lab encoding. The native samples SHALL carry more precision than eight
bits, and a CMYK document SHALL keep the ink values Ghostscript rendered instead
of values derived from an 8-bit RGB conversion. Transparency SHALL be preserved:
areas the page does not paint SHALL stay transparent, and Anti-aliased SHALL
produce partial coverage as it does for other imports. Every Crop To box SHALL
keep working, and the chosen resolution and document naming SHALL be unchanged.
The RGB display SHALL be derived from the stored native samples; for CMYK, the
displayed RGB SHALL be derived from the stored CMYK values.

#### Scenario: 16-bit RGB keeps more than eight bits [pdf_native_rgb16_precision]

- **WHEN** a page painting a smooth tonal ramp is opened as RGB Color at 16 bit
- **THEN** the document is RGB 16-bit and its pixel data holds values that are not multiples of 257

#### Scenario: CMYK keeps its rendered inks [pdf_native_cmyk16]

- **WHEN** a page painting a DeviceCMYK rectangle is opened as CMYK Color at 16 bit
- **THEN** the document is CMYK 16-bit, its stored ink values match the rendered rectangle, not the GCR conversion of the displayed RGB, and the RGB display is derived from the stored ink values

#### Scenario: 16-bit Grayscale keeps more than eight bits [pdf_native_gray16_precision]

- **WHEN** a tonal page is opened as Grayscale at 16 bit
- **THEN** the document is Grayscale 16-bit and its pixel data holds values that are not multiples of 257

#### Scenario: Native renders stay transparent [pdf_native_transparency]

- **WHEN** a page that paints only part of its canvas is opened as CMYK Color at 16 bit
- **THEN** pixels the page paints are opaque and pixels it does not paint have alpha 0

#### Scenario: Native renders anti-alias [pdf_native_antialias]

- **WHEN** a page with a diagonal edge is opened as CMYK Color at 16 bit with Anti-aliased on, then off
- **THEN** the first document has partially transparent edge pixels and the second has only fully opaque or fully transparent pixels

#### Scenario: Native renders crop [pdf_native_crop]

- **WHEN** a 144 × 72 pt page painting a rectangle from (20, 20) to (80, 50) pt is opened as RGB Color at 16 bit with Crop To Bounding Box, then a rotated page is opened with Crop To Crop Box at 144 ppi and a document Name
- **THEN** each document is the same cropped size and placement as the 8-bit import of the same page and box, and the second keeps the chosen Name and 144 ppi Resolution

#### Scenario: An embedded image renders natively [pdf_image_native]

- **WHEN** a 16-bit image XObject is opened from Images mode as RGB Color at 16 bit
- **THEN** the document is 16-bit and its samples hold values that are not multiples of 257

#### Scenario: EPS renders natively [eps_native_cmyk]

- **WHEN** an EPS painting a process-colour rectangle is opened as CMYK Color at 16 bit
- **THEN** the document is CMYK 16-bit with the rendered ink values, and areas outside the painted rectangle are transparent
