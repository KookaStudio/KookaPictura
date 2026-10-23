# psd-icc-conversion Specification

## Purpose
Applies an embedded non-sRGB ICC profile on read and saves ICC-normalized documents untagged.

## Requirements

### Requirement: An embedded non-sRGB ICC profile is applied on read

`read_psd` SHALL convert an RGB document to the sRGB working space when its
image resources carry an ICC profile (resource `1039`) that parses as an ICC
profile and is not sRGB: the composite's color planes and every layer's color
channels SHALL be converted from the embedded profile to sRGB with the
relative-colorimetric intent, and the original profile bytes SHALL be recorded
in `Document.source_icc`. "Is sRGB" SHALL be decided by the profile's
`Description` tag containing `srgb`, case-insensitively. A document that is not
RGB (for example Grayscale), a profile that is absent, does not parse, is sRGB,
or cannot be transformed by the engine, a malformed resource section, or a layer
missing a color channel SHALL leave the pixels and resources unchanged and SHALL
NOT set `source_icc`. The document's `mode` and `depth` SHALL be unchanged by
this pass.

#### Scenario: An Adobe-RGB document is converted

- **WHEN** an RGB file carries an Adobe RGB (non-sRGB) ICC profile and a known pixel layer
- **THEN** `read_psd` returns that layer's pixels converted to sRGB (matching an independent lcms2 conversion), `source_icc` is the embedded profile bytes, and `mode` is unchanged

#### Scenario: A Grayscale document with a profile is untouched

- **WHEN** a Grayscale file carries a non-sRGB profile
- **THEN** the pixels and the resource bytes are unchanged and `source_icc` is `None`

#### Scenario: Other resources survive the conversion

- **WHEN** the file also carries an XMP resource
- **THEN** the XMP bytes are unchanged in the output resources while resource `1039` is removed

#### Scenario: A layer's color channels are converted

- **WHEN** the file has a pixel layer with color channels in the non-sRGB space
- **THEN** those channels are converted too, so the composited result matches the profile-correct color

#### Scenario: An sRGB document is untouched

- **WHEN** the embedded profile is sRGB (or names sRGB) or there is no profile
- **THEN** every pixel and the image-resource bytes are unchanged from a profile-free read

#### Scenario: An undecodable profile is ignored

- **WHEN** resource `1039` is present but does not parse as an ICC profile
- **THEN** the pixels and resources are unchanged and `source_icc` is `None`

### Requirement: An ICC-normalized document saves untagged

`write_psd` SHALL write an image-resource section without resource `1039` when
the document was ICC-normalized on read (`source_icc` is `Some`), so the sRGB
pixels are not re-tagged with the source profile. Every other resource SHALL be
re-emitted byte-for-byte.

#### Scenario: The saved file drops the source profile

- **WHEN** an ICC-normalized document is written and re-read
- **THEN** its image resources no longer contain resource `1039` and its pixels are the converted sRGB values

#### Scenario: Other resources survive

- **WHEN** the source file also carries an EXIF or XMP resource
- **THEN** those resources are present with identical bytes in the output

### Requirement: The app reports an ICC conversion

The app SHALL show a status-bar notice when an opened document was normalized
from an embedded ICC profile, naming the source profile's description.

#### Scenario: Opening a converted document shows the notice

- **WHEN** a document with a non-sRGB embedded profile is opened
- **THEN** the view reports an "embedded ICC profile" conversion notice

### Requirement: The ICC fixture is proven by an independent converter

The committed fixture SHALL embed the synthesized profile
`crates/pictura-codec/tests/fixtures/psd_icc_rgb.icc` (produced by
`cargo run -p pictura-color --example dump_adobe_rgb`) as the image-resource
profile of `crates/pictura-codec/tests/fixtures/icc_profile.psd`, and a test
SHALL compare the converted composite against an independent lcms2 conversion of
a known source color.

#### Scenario: The converted composite matches lcms2

- **WHEN** the fixture is read and an independent converter converts its source color from the embedded profile to sRGB
- **THEN** the composite's pixel is within 1 LSB of that conversion

### Requirement: A saved ICC profile matches the output color mode

`write_psd` SHALL emit image resource `1039` only when the profile's ICC
data-space signature matches the output header color mode: `RGB ` for RGB,
`GRAY` for Grayscale, `CMYK` for CMYK, and `Lab ` for Lab. A framable preserved
profile whose data-space signature differs SHALL be dropped from the emitted
section, so a document normalized from a CMYK or Lab source is not saved with the
mismatched profile still tagged. A framable profile whose signature matches, and
a profile shorter than the 20-byte ICC header that carries no data-space
signature, SHALL be re-emitted byte-for-byte, as SHALL every other image resource
and any bytes the section parser could not frame; a document that carries no
resource `1039` SHALL be unchanged.

#### Scenario: A 16-bit CMYK source saves untagged

- **WHEN** a 16/32-bit CMYK file is read (normalized to RGB, its CMYK resource `1039` preserved) and written
- **THEN** the output's header color mode is RGB and its image resources carry no resource `1039`

#### Scenario: An 8-bit CMYK source keeps its CMYK profile

- **WHEN** an 8-bit CMYK document carrying a CMYK resource `1039` is written
- **THEN** the output's header color mode is CMYK and its resource `1039` data is byte-identical to the input's

#### Scenario: A 16-bit Lab source saves untagged

- **WHEN** a 16/32-bit Lab file is read (normalized to RGB, its Lab resource `1039` preserved) and written
- **THEN** the output's header color mode is RGB and its image resources carry no resource `1039`

#### Scenario: A Grayscale document keeps its GRAY profile

- **WHEN** a Grayscale document carrying a `GRAY` resource `1039` is written
- **THEN** the output's header color mode is Grayscale and its resource `1039` data is byte-identical to the input's

#### Scenario: A matching RGB profile survives

- **WHEN** an RGB document carrying an RGB resource `1039` is written
- **THEN** the output's resource `1039` data is byte-identical to the input's

#### Scenario: Other resources survive the filter

- **WHEN** a document carrying resource `1039` and an XMP resource is written and its `1039` is dropped
- **THEN** the XMP resource is present with identical bytes in the output

#### Scenario: A profile too short to classify is preserved

- **WHEN** a document carries a resource `1039` whose data is shorter than the ICC data-space field
- **THEN** its image-resource section is re-emitted byte-for-byte

### Requirement: Convert to Profile transforms retained native samples

When a 16- or 32-bit RGB document is converted to another profile, the retained
native color samples SHALL be transformed at the stored depth together with the
8-bit working copies. The document's `source_planes` color planes and every
layer's `source_channels` color planes (channel ids `0..3`) SHALL be converted;
alpha (id `-1`), mask (id `-2`), and document extra planes MUST be copied
unchanged. The
destination profile SHALL then be assigned. A save after conversion MUST re-emit
the converted samples: a 16-bit ramp MUST retain more than 256 distinct levels in
the destination profile, and 32-bit float samples outside `[0, 1]` MUST NOT be
clamped by the conversion.

#### Scenario: A 16-bit conversion keeps native levels [picc_convert_16bit]

- **WHEN** a 16-bit RGB PSD with a 0..65535 ramp is converted from sRGB to Adobe RGB, saved, and reopened
- **THEN** the saved file MUST carry more than 256 distinct levels in the destination profile and the working copy MUST match the converted native samples

#### Scenario: A 32-bit conversion does not clamp [picc_convert_32bit]

- **WHEN** a 32-bit float RGB document with color samples outside `[0, 1]` is converted to another profile
- **THEN** the converted native samples MUST stay 32-bit floats and MUST NOT be pre-clamped to `[0, 1]`, while alpha and mask planes MUST remain byte-for-byte equal
