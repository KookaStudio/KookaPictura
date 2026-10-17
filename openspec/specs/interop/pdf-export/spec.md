# interop/pdf-export Specification

## Purpose
`File ▸ Save As ▸ Photoshop PDF`: a Qt-free engine writer that saves the
document composite as a one-page PDF with its top type layers as live text
(colour by mode, compression,
downsampling, transparency, standards, security, Fast Web View), the Save
Adobe PDF dialog and `.joboptions` presets that configure it, and the reopen of
Pictura's own Photoshop PDFs from their embedded PSD.

## Requirements

### Requirement: Photoshop PDF is a Save As format

Save As SHALL offer `Photoshop PDF (*.PDF *.PDP)`. Saving to a `.pdf` or `.pdp`
path SHALL open the Save Adobe PDF dialog before anything is written; Cancel
SHALL write nothing and leave the document unchanged. A confirmed save SHALL
write the file atomically (temp file plus rename), record the `.pdf` path as
the document's path, and clear the modified state. Save on a document whose
path is a `.pdf` SHALL open the Save Adobe PDF dialog again, seeded with the
options that document was last saved with. A refused or failed write SHALL
leave any existing file intact and report failure.

#### Scenario: Save As lists Photoshop PDF [pdf_save_as_format]

- **WHEN** the Save As format list is shown
- **THEN** it contains `Photoshop PDF (*.PDF *.PDP)` and a path chosen with it gets the `.pdf` suffix

#### Scenario: A PDF save records the path and clears modified [pdf_path_becomes_pdf]

- **WHEN** a modified document is saved to `poster.pdf` through the PDF options
- **THEN** a PDF is written at `poster.pdf`, the document's path is `poster.pdf`, and it is no longer modified

#### Scenario: Save on a PDF path reopens the dialog with its settings [pdf_resave_shows_dialog]

- **WHEN** a document saved as a PDF with JPEG compression is edited and saved again
- **THEN** the Save Adobe PDF dialog opens with JPEG compression selected

#### Scenario: Cancel writes nothing [pdf_cancel_writes_nothing]

- **WHEN** the Save Adobe PDF dialog is cancelled
- **THEN** no file is written and the document's path and modified state are unchanged

#### Scenario: A failed write leaves the old file [pdf_atomic_failure]

- **WHEN** a PDF save targets a directory that does not exist
- **THEN** it reports failure and creates no file

### Requirement: The page holds the composite as one image

The writer SHALL produce one page whose MediaBox is the document size in points
at its resolution (`pixels × 72 / ppi`, 72 ppi when the resolution is unknown)
and whose content draws the document's rendered composite as one image filling
the page, less the type layers written as live text, which are drawn over it.
The file SHALL be a structurally valid PDF whose header version is the selected
compatibility level's.

#### Scenario: Page size follows the resolution [pdf_page_size]

- **WHEN** a 600 × 300 px document at 300 ppi is written
- **THEN** the MediaBox is 144 × 72 points

#### Scenario: The page renders the composite [pdf_renders_composite]

- **WHEN** an RGB document with ZIP compression is written and rendered by a PDF renderer at the document's resolution
- **THEN** the rendered pixels equal the CPU composite

#### Scenario: The file is structurally valid [pdf_valid_structure]

- **WHEN** a written PDF is checked by a PDF validator
- **THEN** it reports no structural errors and the header names the selected PDF version

### Requirement: Type layers are written as live text

A visible type layer that no other content covers SHALL be written as PDF text
set in its embedded font over the page image, not as pixels in the image, so
its text can be selected, searched, copied, and edited in a PDF editor; the
page image SHALL then be the composite without that layer. A type layer
qualifies when every visible layer above it in the stack is itself a
qualifying type layer, reached only through groups that do not change
compositing (pass-through, or Normal at full opacity, without a mask, vector
mask, or layer effects), and the layer itself is Normal, unclipped, has no
mask, vector mask, layer effects, or warp, and sets its text in one style.
Its font SHALL be embedded whole as a `Type0` font with `Identity-H` encoding
and a `ToUnicode` map: TrueType outlines as `FontFile2`, CFF outlines as
`FontFile3 /OpenType` (PDF 1.6 and later only). Its fill colour SHALL be
written in the page image's colour space. Layer opacity below 100% SHALL be
written as a constant-alpha graphics state where transparency is kept. A type
layer that does not qualify, whose font forbids embedding, whose CFF font the
compatibility level cannot carry, or whose opacity the level cannot keep, SHALL
stay in the page image. The Preserve Photoshop Editing payload and the page
thumbnail SHALL hold the full document.

#### Scenario: A top type layer is live text [pdf_live_text]

- **WHEN** an RGB document with a type layer reading `Kooka Pictura` above a pixel layer is written
- **THEN** the page uses a `Type0` `Identity-H` font with an embedded `FontFile2`, a text extractor reads `Kooka Pictura`, and the page image equals the composite without the type layer

#### Scenario: Live text renders like the composite [pdf_live_text_renders]

- **WHEN** that PDF is rendered by a PDF renderer at the document's resolution
- **THEN** the rendered pixels match the full composite, glyph edges aside

#### Scenario: Covered type stays in the image [pdf_live_text_covered]

- **WHEN** a pixel layer lies above the type layer
- **THEN** no font is written and the page image equals the full composite

#### Scenario: Faded type follows the transparency rule [pdf_live_text_opacity]

- **WHEN** the type layer is at 50% opacity and written at Acrobat 6, then at Acrobat 4
- **THEN** Acrobat 6 draws it as live text under a graphics state with `/ca 0.5`, and Acrobat 4 keeps it in the page image

#### Scenario: Live text colour follows the output conversion [pdf_live_text_colour]

- **WHEN** a document with red type is written with Convert to Destination, Photocraft Coated CMYK, Include Destination Profile
- **THEN** the text is filled in the image's ICC-based CMYK colour space with the converted red

#### Scenario: Saving from the app keeps type live [pdf_live_text_app]

- **WHEN** a document with a type layer is saved as Photoshop PDF from the app
- **THEN** the written file embeds the layer's font and carries its text

### Requirement: Colour follows the document mode

The image colour space SHALL follow the output mode: Bitmap as 1-bit gray with
CCITT Group 4, Grayscale as gray, RGB and Duotone as RGB, a CMYK source as
DeviceCMYK, a Lab source as CIE Lab (D50), and an unedited Indexed source as an
indexed RGB palette. A Multichannel document SHALL be refused. With No
Conversion and the ICC Profile option on, a Gray or RGB image SHALL be tagged
with the document's profile (sRGB when untagged); with it off, it SHALL use the
device colour space.

#### Scenario: Grayscale writes a gray image [pdf_gray]

- **WHEN** a Grayscale document is written
- **THEN** the image has one colour component and renders as the gray composite

#### Scenario: A CMYK source writes DeviceCMYK [pdf_cmyk]

- **WHEN** a document read from a CMYK file is written with No Conversion
- **THEN** the image is DeviceCMYK and renders to the same colours as the composite

#### Scenario: A Lab source writes a Lab image [pdf_lab]

- **WHEN** a document read from a Lab file is written
- **THEN** the image colour space is `/Lab` with a D50 white point

#### Scenario: An Indexed source writes its palette [pdf_indexed]

- **WHEN** an unedited Indexed document is written with ZIP compression
- **THEN** the image is `/Indexed` over its 256-entry palette and renders as the composite

#### Scenario: Bitmap uses CCITT Group 4 [pdf_bitmap_ccitt]

- **WHEN** an unedited Bitmap document is written
- **THEN** the image is 1 bit per component with `CCITTFaxDecode` `K -1` and renders to the same black and white pixels

#### Scenario: Multichannel is refused [pdf_multichannel_refused]

- **WHEN** a Multichannel document is written as Photoshop PDF
- **THEN** the write fails with a message that Photoshop PDF does not support Multichannel and writes no file

#### Scenario: The ICC profile is embedded [pdf_icc_embed]

- **WHEN** an Adobe RGB document is written with the ICC Profile option on, and again with it off
- **THEN** the first image is ICCBased with the Adobe RGB profile bytes and the second is DeviceRGB

### Requirement: Transparency is kept as a soft mask where allowed

A composite with transparent pixels SHALL be written with a soft mask at PDF
1.4 and later unless the standard forbids transparency (PDF/X-1a, PDF/X-3,
PDF/A-1b), in which case and at PDF 1.3 the composite SHALL be flattened onto
white.

#### Scenario: Alpha becomes a soft mask [pdf_alpha_smask]

- **WHEN** a document with a transparent pixel is written at Acrobat 6
- **THEN** the image carries an `SMask` whose sample at that pixel is 0

#### Scenario: Acrobat 4 flattens onto white [pdf_alpha_flatten]

- **WHEN** the same document is written at Acrobat 4 (PDF 1.3)
- **THEN** the image has no soft mask and that pixel is white

### Requirement: Compression and image quality

Compression SHALL be None, ZIP (Flate), or JPEG with an Image Quality of
Minimum, Low, Medium, High, or Maximum. JPEG SHALL be encoded at the Qt
boundary. JPEG2000 SHALL be listed and disabled. A soft mask SHALL always be
lossless.

#### Scenario: ZIP is lossless [pdf_zip]

- **WHEN** a document is written with ZIP compression
- **THEN** the image stream uses `FlateDecode` and decodes to the composite exactly

#### Scenario: JPEG uses DCT at the chosen quality [pdf_jpeg]

- **WHEN** a document is written with JPEG compression at Maximum and at Low
- **THEN** both images use `DCTDecode`, both render close to the composite, and the Low file is smaller

#### Scenario: No compression writes raw samples [pdf_no_compression]

- **WHEN** a document is written with compression None
- **THEN** the image stream has no filter and its length is width × height × components

#### Scenario: JPEG2000 is unavailable [pdf_jpeg2000_disabled]

- **WHEN** the Compression page is shown
- **THEN** JPEG2000 is listed but cannot be selected

### Requirement: Downsampling

Downsampling SHALL be Do Not Downsample, Average, Subsampling, or Bicubic, to a
target resolution, applied only when the document resolution exceeds the
"for images above" threshold. It SHALL change the image's pixel size and never
the page size.

#### Scenario: Above the threshold the image is downsampled [pdf_downsample]

- **WHEN** a 300 ppi document is written with Bicubic to 150 ppi for images above 225 ppi
- **THEN** the image is half the document's pixel size and the page size is unchanged

#### Scenario: Below the threshold nothing changes [pdf_downsample_threshold]

- **WHEN** a 200 ppi document is written with the same settings
- **THEN** the image keeps the document's pixel size

#### Scenario: Each method resamples [pdf_downsample_methods]

- **WHEN** a two-tone checker is downsampled by 2 with Average, Subsampling, and Bicubic
- **THEN** Average yields the mean, Subsampling a source sample, and Bicubic a weighted value

### Requirement: 16-bit output

With Convert 16 Bit/Channel Image To 8 Bit/Channel off, a 16-bit Gray or RGB
document SHALL be written with 16 bits per component and ZIP compression only.
The option SHALL be forced on below PDF 1.5.

#### Scenario: 16-bit is kept [pdf_16bit]

- **WHEN** an unedited 16-bit RGB document is written at Acrobat 6 with the conversion off
- **THEN** the image has 16 bits per component and its samples are the source samples

#### Scenario: Older levels force 8-bit [pdf_16bit_forced]

- **WHEN** the same document is written at Acrobat 5
- **THEN** the image has 8 bits per component and the option reports locked

### Requirement: Preserve Photoshop Editing Capabilities

With Preserve Photoshop Editing Capabilities on, the page SHALL carry the
document's PSD serialization in a private `/PieceInfo /Pictura` stream, and
`File > Open` of that PDF SHALL restore the layered document. The payload SHALL
honour the Layers (off: flattened), Alpha Channels (off: no extra channels), and
Notes options. Spot Colors SHALL be disabled. A layered document saved with the
option off SHALL warn that the format cannot hold its features. This payload is
Pictura's own and is not claimed to reopen layered in Photoshop.

#### Scenario: The PSD is embedded [pdf_embeds_psd]

- **WHEN** a two-layer document is written with Preserve Editing on
- **THEN** the page's `/PieceInfo /Pictura /Private /PSD` stream decodes to a PSD with two layers

#### Scenario: Reopening restores the layers [pdf_reopen_layers]

- **WHEN** that PDF is opened with `File > Open`
- **THEN** the document has the same two layers and composite

#### Scenario: Layers off embeds a flattened PSD [pdf_layers_off]

- **WHEN** the two-layer document is written with Preserve Editing on and Layers off
- **THEN** the embedded PSD has no layer tree beyond the background

#### Scenario: Preserve Editing off warns for a layered document [pdf_flat_warns]

- **WHEN** a two-layer document is confirmed in the PDF dialog with Preserve Editing off
- **THEN** the format-features warning is shown before writing

#### Scenario: A PDF without the payload is refused on open [pdf_open_foreign_refused]

- **WHEN** a PDF with no Pictura payload is opened
- **THEN** the open fails without replacing the document

### Requirement: Notes, thumbnails, metadata, and Fast Web View

The Notes option SHALL write each note as a PDF text annotation at its
position. Embed Page Thumbnails SHALL attach an RGB thumbnail at most 106 px on
its long side. Optimize For Fast Web View SHALL write a linearized file. The
document information SHALL carry the producer, creation and modification
dates, and the file name as title. View PDF After Saving SHALL open the saved
file with the system viewer.

#### Scenario: Notes become text annotations [pdf_notes]

- **WHEN** a document with a note "check the sky" is written with Notes on, and again with Notes off
- **THEN** the first page has a `/Text` annotation with that contents and the second has none

#### Scenario: A thumbnail is embedded [pdf_thumbnail]

- **WHEN** a 400 × 200 document is written with Embed Page Thumbnails
- **THEN** the page has a `/Thumb` image of 106 × 53

#### Scenario: Fast Web View linearizes [pdf_linearized]

- **WHEN** a document is written with Optimize For Fast Web View, with and without security
- **THEN** a linearization checker reports the file correctly linearized

#### Scenario: View After Saving opens the file [pdf_view_after]

- **WHEN** a PDF is saved with View PDF After Saving on
- **THEN** the system viewer is asked to open the saved file

### Requirement: Output colour conversion

Color Conversion SHALL be No Conversion or Convert To Destination. Converting
SHALL transform the composite from the document profile to the destination
(sRGB, Adobe RGB, ProPhoto RGB, Gray Gamma 2.2, Photocraft Coated CMYK, or the
document profile). The Profile Inclusion Policy (Include Destination Profile,
Don't Include Profiles) SHALL decide the tagging of a converted image.

#### Scenario: Convert to CMYK [pdf_convert_cmyk]

- **WHEN** an RGB document is written with Convert To Destination to Photocraft Coated CMYK and Include Destination Profile
- **THEN** the image is ICCBased with four components, the profile bytes are the bundled CMYK profile, and it renders close to the composite

#### Scenario: Don't Include Profiles writes device colour [pdf_profile_inclusion]

- **WHEN** the same conversion uses Don't Include Profiles
- **THEN** the image is DeviceCMYK

### Requirement: PDF/X and PDF/A standards

The Standard SHALL be None, PDF/X-1a:2001, PDF/X-3:2002, PDF/X-4:2008, or
PDF/A-1b:2005. Choosing one SHALL force its compatibility level and lock the
settings it fixes. Every PDF/X file SHALL carry a `GTS_PDFX` output intent with
the Output Intent Profile and its Output Condition, Output Condition Identifier,
and Registry Name, `GTS_PDFXVersion`, `/Trapped /False`, a title, and a TrimBox.
PDF/X-1a SHALL convert to CMYK device colour. PDF/X-4 and PDF/A-1b SHALL carry
XMP identifying the standard. PDF/A-1b SHALL carry a `GTS_PDFA1` output intent.
No standard SHALL be combined with security or, except PDF/X-4, transparency.

#### Scenario: PDF/X-1a converts to CMYK [pdf_x1a]

- **WHEN** an RGB document is written as PDF/X-1a:2001
- **THEN** the file is PDF 1.3, the image is DeviceCMYK, the output intent is GTS_PDFX with the CMYK profile, and the info has `GTS_PDFXVersion (PDF/X-1:2001)`, `/Trapped /False`, and a title

#### Scenario: PDF/X-3 keeps colour-managed RGB [pdf_x3]

- **WHEN** an RGB document is written as PDF/X-3:2002
- **THEN** the file is PDF 1.3, the image is ICCBased RGB, and the info has `GTS_PDFXVersion (PDF/X-3:2002)`

#### Scenario: PDF/X-4 carries XMP and keeps transparency [pdf_x4]

- **WHEN** a document with transparency is written as PDF/X-4:2008
- **THEN** the file is PDF 1.6, the image keeps its soft mask, and the XMP has `pdfxid:GTS_PDFXVersion` `PDF/X-4`

#### Scenario: PDF/A-1b carries its identification [pdf_a1b]

- **WHEN** a document is written as PDF/A-1b:2005
- **THEN** the file is PDF 1.4, has no soft mask and no encryption, its XMP has `pdfaid:part 1` and `pdfaid:conformance B`, and its output intent is GTS_PDFA1

#### Scenario: A standard locks its settings [pdf_standard_locks]

- **WHEN** PDF/X-1a:2001 is selected in the dialog
- **THEN** Compatibility shows Acrobat 4 (PDF 1.3) and is disabled, and the Security page controls are disabled

### Requirement: Security

The Security page SHALL offer a document open password, a permissions password
with Printing Allowed (None, Low Resolution (150 dpi), High Resolution), Changes
Allowed (None; Inserting, deleting, and rotating pages; Filling in form fields
and signing; Commenting, filling in form fields, and signing; Any except
extracting pages), Enable copying of text, images, and other content, Enable
text access for screen reader devices, and Enable plaintext metadata. The
encryption level SHALL follow Compatibility: RC4 40-bit (Acrobat 4), RC4 128-bit
(Acrobat 5 and 6), AES 128-bit (Acrobat 7), AES 256-bit (Acrobat X). Presets
SHALL NOT store passwords.

#### Scenario: An open password encrypts the file [pdf_open_password]

- **WHEN** a document is written at each compatibility level with open password `kooka`
- **THEN** a PDF reader refuses it without a password, opens it with `kooka`, and reports the level's encryption

#### Scenario: Permissions restrict printing and copying [pdf_permissions]

- **WHEN** a document is written with a permissions password, Printing Allowed None, and copying off
- **THEN** a PDF reader opens it without a password and reports printing and copying not allowed

#### Scenario: Presets drop passwords [pdf_security_not_in_presets]

- **WHEN** options with passwords are saved as a preset
- **THEN** the preset file contains no password or permission keys

### Requirement: Adobe PDF presets

The dialog SHALL offer the built-in presets `[Photoshop Default]`,
`[High Quality Print]`, `[Press Quality]`, `[Smallest File Size]`,
`[PDF/X-1a:2001]`, `[PDF/X-3:2002]`, `[PDF/X-4:2008]`, `[PDF/A-1b:2005]`,
`[Standard]`, `[Rich Content PDF]`, `[Illustrator Default]`, and
`[Oversized Pages]` (values inferred), plus the user's presets. Changing a
setting SHALL mark the preset modified. Save Preset SHALL write a `.joboptions`
file. `Edit > Adobe PDF Presets…` SHALL list every preset and New, Edit,
Delete, Load, and Save As user presets; built-in presets SHALL be read-only.
Loading SHALL accept Acrobat `.joboptions` files and ignore unknown keys.

#### Scenario: Built-in presets are offered [pdf_builtin_presets]

- **WHEN** the preset list is shown
- **THEN** it begins with the twelve built-in presets in brackets, and choosing `[Smallest File Size]` sets JPEG Low and Bicubic 100 ppi

#### Scenario: Editing marks the preset modified [pdf_preset_modified]

- **WHEN** a setting is changed after choosing a preset
- **THEN** the preset name gains ` (Modified)`

#### Scenario: Save Preset writes joboptions [pdf_save_preset]

- **WHEN** Save Preset is used with the name `Web proof`
- **THEN** `Web proof.joboptions` is written to the presets folder and listed

#### Scenario: joboptions round-trips [pdf_joboptions_roundtrip]

- **WHEN** options are written as joboptions and parsed back
- **THEN** every preset setting is equal

#### Scenario: Acrobat joboptions load [pdf_joboptions_acrobat]

- **WHEN** a joboptions file with comments, nested dictionaries, a `/Description << /ENU <FEFF…> >>`, and unknown keys is loaded
- **THEN** the known settings and the description are read and the unknown keys are ignored

#### Scenario: The preset manager edits user presets [pdf_presets_manager]

- **WHEN** a user preset is created, edited, and deleted in `Edit > Adobe PDF Presets…`
- **THEN** its file is written, rewritten, and removed, and built-in presets cannot be edited or deleted

### Requirement: The Save Adobe PDF dialog

The dialog SHALL show the Save options, Adobe PDF Preset, Standard, and
Compatibility controls and a page list General, Compression, Output, Security,
Summary. General SHALL hold Description, Preserve Photoshop Editing
Capabilities, Embed Page Thumbnails, Optimize For Fast Web View, and View PDF
After Saving. Compression SHALL hold the downsampling method, resolution,
threshold, compression, image quality, tile size, and Convert 16 Bit/Channel
Image To 8 Bit/Channel. Output SHALL hold Color Conversion, Destination,
Profile Inclusion Policy, and the PDF/X output intent fields. Summary SHALL list
the effective settings and any warnings. Settings that the level or standard
forces SHALL be disabled.

#### Scenario: The dialog has the five pages [pdf_dialog_pages]

- **WHEN** the Save Adobe PDF dialog opens
- **THEN** its page list is General, Compression, Output, Security, Summary, and each page shows its controls

#### Scenario: Compatibility gates controls [pdf_compat_gates]

- **WHEN** Compatibility is set to Acrobat 4 (PDF 1.3)
- **THEN** Convert 16 Bit/Channel Image To 8 Bit/Channel is checked and disabled and the Security page names RC4 40-bit encryption

#### Scenario: Summary lists the settings [pdf_summary]

- **WHEN** the Summary page is shown for `[PDF/X-1a:2001]`
- **THEN** it lists the standard, compatibility, compression, and colour conversion, and warns that transparency is flattened
