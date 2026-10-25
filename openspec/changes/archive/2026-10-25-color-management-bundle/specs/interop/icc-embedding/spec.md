# Spec Delta

## Purpose

Give the user a per-save choice over embedding the document's ICC profile, with a clear format support matrix and no change to the saved pixels.

## ADDED Requirements

### Requirement: Save As offers Embed Color Profile for the formats that support it

`File > Save As` SHALL offer an `Embed Color Profile` option when the chosen destination is PSD, PSB, JPEG, or TIFF. The option SHALL default to on when the document carries a non-sRGB RGB working profile, and for a CMYK source document saving to PSD or PSB. It SHALL default to off for an untagged sRGB document and for a CMYK source document saving to JPEG or TIFF, because those raster pixels are the sRGB working pixels. The option SHALL NOT be offered for PNG, WebP, or BMP; Photoshop does not offer profile embedding for PNG, and the other two formats are outside this capability. Photoshop PDF SHALL continue to use its own profile-inclusion policy. Changing the option SHALL NOT alter the document, its history, or its dirty state.

#### Scenario: A tagged document defaults to embedding

- **WHEN** Save As is opened for a document whose RGB working profile is Adobe RGB
- **THEN** Embed Color Profile is checked

#### Scenario: An untagged sRGB document defaults to no embedding

- **WHEN** Save As is opened for a document with no working profile
- **THEN** Embed Color Profile is unchecked

#### Scenario: PNG has no embed option

- **WHEN** Save As is opened with PNG selected
- **THEN** no Embed Color Profile control is offered

#### Scenario: Cancelling Save As changes nothing

- **WHEN** the Save As dialog is cancelled
- **THEN** no file is written and the document's pixels, path, dirty state, and history are unchanged

### Requirement: The setting controls the written profile bytes

With `Embed Color Profile` on, a PSD/PSB save SHALL write the document's working RGB profile as image resource `1039` (and SHALL keep a CMYK source profile attached to a CMYK source document); a JPEG save SHALL write the profile in an APP2 `ICC_PROFILE` marker segment; and a TIFF save SHALL write the profile in tag `34675`. With the option off, the output SHALL contain no embedded ICC profile for those formats. Turning the option on or off SHALL NOT change the colour channel bytes written; it only adds or removes the profile. For a CMYK source document saved as JPEG or TIFF, the raster pixels are the sRGB working pixels and the CMYK source profile SHALL NOT be embedded; such a document keeps its CMYK profile only in PSD/PSB.

#### Scenario: A PSD save with embedding on carries resource 1039

- **WHEN** an Adobe RGB document is saved as PSD with Embed Color Profile on and reopened
- **THEN** the reopened document has the Adobe RGB working profile and its pixels match the saved working pixels

#### Scenario: A PSD save with embedding off is untagged

- **WHEN** an Adobe RGB document is saved as PSD with Embed Color Profile off
- **THEN** the output's image resources contain no resource 1039 and the working pixels are unchanged

#### Scenario: A JPEG save embeds the profile in APP2

- **WHEN** an Adobe RGB document is saved as JPEG with Embed Color Profile on
- **THEN** the JPEG contains an APP2 ICC_PROFILE segment whose bytes parse as the Adobe RGB profile

#### Scenario: A TIFF save embeds the profile in tag 34675

- **WHEN** an Adobe RGB document is saved as TIFF with Embed Color Profile on
- **THEN** the TIFF's tag 34675 contains the Adobe RGB profile bytes

#### Scenario: A TIFF save with embedding off has no profile tag

- **WHEN** an Adobe RGB document is saved as TIFF with Embed Color Profile off
- **THEN** the TIFF has no ICC profile tag and its pixels are unchanged

#### Scenario: A CMYK source document is not mis-tagged in a raster save

- **WHEN** a CMYK PSD is saved as JPEG with Embed Color Profile on
- **THEN** the JPEG's pixels are the sRGB working pixels and no CMYK profile is embedded

### Requirement: Save reuses the per-document choice

`File > Save` on an already saved document SHALL use the Embed Color Profile choice last accepted for that document in Save As, defaulting as for Save As when no choice has been made. A save SHALL NOT re-open the options dialog.

#### Scenario: Save keeps the last choice

- **WHEN** a document has been saved as PSD with embedding off and is saved again
- **THEN** the new save also omits resource 1039

#### Scenario: Save on a newly opened tagged document defaults to embedding

- **WHEN** a tagged Adobe RGB PSD is opened and saved without a prior Save As choice
- **THEN** the save embeds the Adobe RGB profile
