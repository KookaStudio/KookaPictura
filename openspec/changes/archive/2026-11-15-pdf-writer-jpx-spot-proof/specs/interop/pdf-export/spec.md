# Spec Delta: interop/pdf-export

## MODIFIED Requirements

### Requirement: Compression and image quality

Compression SHALL be None, ZIP (Flate), JPEG, or JPEG 2000 with an Image
Quality of Minimum, Low, Medium, High, or Maximum. JPEG SHALL be encoded at the
Qt boundary. JPEG 2000 SHALL be selectable only when the compatibility level is
Acrobat 6 (PDF 1.5) or later, the written image is 8-bit continuous tone, and
the standard is not PDF/A-1b; otherwise the setting SHALL fall back to ZIP
(Flate) and the dialog SHALL report JPEG 2000 unavailable. JPEG 2000 SHALL be
written as a JPEG 2000 codestream carried by `JPXDecode`, with the Tile Size
setting controlling its tiles. A soft mask SHALL always be lossless.

#### Scenario: ZIP is lossless [pdf_zip]

- **WHEN** a document is written with ZIP compression
- **THEN** the image stream uses `FlateDecode` and decodes to the composite exactly

#### Scenario: JPEG uses DCT at the chosen quality [pdf_jpeg]

- **WHEN** a document is written with JPEG compression at Maximum and at Low
- **THEN** both images use `DCTDecode`, both render close to the composite, and the Low file is smaller

#### Scenario: No compression writes raw samples [pdf_no_compression]

- **WHEN** a document is written with compression None
- **THEN** the image stream has no filter and its length is width × height × components

#### Scenario: JPEG 2000 writes a codestream [pdf_jpeg2000]

- **WHEN** an 8-bit document is written at Acrobat 6 with JPEG 2000 at Maximum quality
- **THEN** the image stream uses `JPXDecode`, a PDF image lister reports the JPX filter, and a rendered page is close to the composite

#### Scenario: JPEG 2000 quality changes size [pdf_jpeg2000_quality]

- **WHEN** the same document is written with JPEG 2000 at Low and at Maximum
- **THEN** both render close to the composite and the Low file is smaller

#### Scenario: JPEG 2000 is gated by level, depth, and standard [pdf_jpeg2000_gated]

- **WHEN** JPEG 2000 is selected at Acrobat 5, or with Convert 16 Bit/Channel Image To 8 Bit/Channel off, or under PDF/A-1b:2005
- **THEN** the image stream uses `FlateDecode`

#### Scenario: JPEG2000 is unavailable [pdf_jpeg2000_disabled]

- **WHEN** the compatibility level is Acrobat 5 (PDF 1.4), or a 16-bit document is shown with Convert 16 Bit/Channel Image To 8 Bit/Channel off, or the standard is PDF/A-1b:2005, and the Compression page is shown
- **THEN** JPEG 2000 is listed but cannot be selected

### Requirement: Preserve Photoshop Editing Capabilities

With Preserve Photoshop Editing Capabilities on, the page SHALL carry the
document's PSD serialization in a private `/PieceInfo /Pictura` stream, and
`File > Open` of that PDF SHALL restore the layered document. The payload SHALL
honour the Layers (off: flattened), Alpha Channels, Spot Colors, and Notes
options. Spot channels SHALL be told apart from alpha channels by the
document's stored channel display information, not by channel order. Alpha
Channels off SHALL drop only non-spot extra channels; Spot Colors off SHALL drop
only spot extra channels; the two options SHALL keep the other kind. The
payload's channel-name and channel-display resources SHALL describe exactly the
channels the payload carries. A layered document saved with the option off SHALL
warn that the format cannot hold its features. This payload is Pictura's own and
is not claimed to reopen layered in Photoshop.

#### Scenario: The PSD is embedded [pdf_embeds_psd]

- **WHEN** a two-layer document is written with Preserve Editing on
- **THEN** the page's `/PieceInfo /Pictura /Private /PSD` stream decodes to a PSD with two layers

#### Scenario: Reopening restores the layers [pdf_reopen_layers]

- **WHEN** that PDF is opened with `File > Open`
- **THEN** the document has the same two layers and composite

#### Scenario: Layers off embeds a flattened PSD [pdf_layers_off]

- **WHEN** the two-layer document is written with Preserve Editing on and Layers off
- **THEN** the embedded PSD has no layer tree beyond the background

#### Scenario: Spot Channels are kept [pdf_spot_colors]

- **WHEN** a document with one spot channel and one alpha channel is written with Preserve Editing on, Alpha Channels on, and Spot Colors on
- **THEN** the embedded PSD carries both extra channels and both stored names

#### Scenario: Spot Colors off drops only the spot [pdf_spot_colors_off]

- **WHEN** the same document is written with Spot Colors off and Alpha Channels on
- **THEN** the embedded PSD carries the alpha channel but not the spot channel, and the remaining channel keeps its name

#### Scenario: Alpha Channels off keeps the spot [pdf_alpha_off_keeps_spot]

- **WHEN** the same document is written with Alpha Channels off and Spot Colors on
- **THEN** the embedded PSD carries the spot channel under the spot's name and not the alpha channel

#### Scenario: Preserve Editing off warns for a layered document [pdf_flat_warns]

- **WHEN** a two-layer document is confirmed in the PDF dialog with Preserve Editing off
- **THEN** the format-features warning is shown before writing

#### Scenario: A PDF without the payload is refused on open [pdf_open_foreign_refused]

- **WHEN** a PDF with no Pictura payload is opened
- **THEN** the open fails without replacing the document

### Requirement: The Save Adobe PDF dialog

The dialog SHALL show the Save options, Adobe PDF Preset, Standard, and
Compatibility controls and a page list General, Compression, Output, Security,
Summary. General SHALL hold the Save options Layers, Notes, Alpha Channels,
Spot Colors, ICC Profile, and Use Proof Setup, plus Description, Preserve
Photoshop Editing Capabilities, Embed Page Thumbnails, Optimize For Fast Web
View, and View PDF After Saving. Compression SHALL hold the downsampling method,
resolution, threshold, compression, image quality, tile size, and Convert 16
Bit/Channel Image To 8 Bit/Channel. Output SHALL hold Color Conversion,
Destination, Profile Inclusion Policy, and the PDF/X output intent fields.
Summary SHALL list the effective settings and any warnings. Settings that the
level, standard, or the open document's channels force SHALL be disabled.

#### Scenario: The dialog has the five pages [pdf_dialog_pages]

- **WHEN** the Save Adobe PDF dialog opens
- **THEN** its page list is General, Compression, Output, Security, Summary, and each page shows its controls

#### Scenario: Save options follow the document [pdf_save_options]

- **WHEN** the dialog opens on a document that has a spot channel, and on one that has none, and opens with a stored proof setup active
- **THEN** Spot Colors is enabled on the first and disabled on the second, and Use Proof Setup names the active proof setup

#### Scenario: Compatibility gates controls [pdf_compat_gates]

- **WHEN** Compatibility is set to Acrobat 4 (PDF 1.3)
- **THEN** Convert 16 Bit/Channel Image To 8 Bit/Channel is checked and disabled and the Security page names RC4 40-bit encryption

#### Scenario: Summary lists the settings [pdf_summary]

- **WHEN** the Summary page is shown for `[PDF/X-1a:2001]`
- **THEN** it lists the standard, compatibility, compression, and colour conversion, and warns that transparency is flattened

## ADDED Requirements

### Requirement: Use Proof Setup for the page colour

The General page SHALL offer Use Proof Setup, labelled with the active proof
setup's name. When it is selected and Color Conversion is No Conversion, the
page image and live text SHALL be converted from the document profile to the
proof setup's device profile using the proof setup's rendering intent and
black-point compensation, and SHALL be tagged with that profile's ICC bytes
regardless of the Profile Inclusion Policy. The proof conversion SHALL write
8-bit samples and SHALL force Convert 16 Bit/Channel Image To 8 Bit/Channel on,
overriding a 16-bit document. When Color Conversion is Convert To Destination,
the Output page SHALL take precedence: the proof conversion SHALL NOT apply, the
control SHALL be disabled, and the Summary SHALL name the override.

#### Scenario: Use Proof Setup converts to the proof device [pdf_use_proof_setup]

- **WHEN** an sRGB document is written with Use Proof Setup on (Working CMYK), No Conversion, and Include Destination Profile
- **THEN** the image is ICCBased with four components and the bundled CMYK profile and renders close to the composite

#### Scenario: Output conversion overrides the proof option [pdf_use_proof_override]

- **WHEN** Convert To Destination to sRGB is selected while Use Proof Setup is on
- **THEN** the image follows the Output page's sRGB conversion and tagging, Use Proof Setup is disabled, and the Summary warns that the Output page overrides it

#### Scenario: Use Proof Setup off keeps the document tag [pdf_use_proof_off]

- **WHEN** Use Proof Setup is off, Color Conversion is No Conversion, and ICC Profile is on
- **THEN** the image is tagged with the document profile as before

#### Scenario: Use Proof Setup forces 8-bit output [pdf_use_proof_16bit]

- **WHEN** a 16-bit document is written with Use Proof Setup on
- **THEN** Convert 16 Bit/Channel Image To 8 Bit/Channel is forced on and the image samples are 8-bit

#### Scenario: Don't Include Profiles still writes the proof profile [pdf_use_proof_no_profile]

- **WHEN** Use Proof Setup is on (Working CMYK) with the Profile Inclusion Policy set to Don't Include Profiles
- **THEN** the image is still tagged with the proof profile's ICC bytes

#### Scenario: Live text follows the proof conversion [pdf_use_proof_live_text]

- **WHEN** a type layer is written as live text with Use Proof Setup on
- **THEN** its fill colour is converted to the proof device like the page image
