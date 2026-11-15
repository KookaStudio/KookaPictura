# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
