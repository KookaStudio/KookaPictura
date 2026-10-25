# Spec Delta

## MODIFIED Requirements

### Requirement: An embedded non-sRGB ICC profile is applied on read

`read_psd` SHALL convert an RGB document to the sRGB working space when its
image resources carry an ICC profile (resource `1039`) that parses as an ICC
profile and is not sRGB: the composite's color planes and every layer's color
channels SHALL be converted from the embedded profile to sRGB with the
relative-colorimetric intent, and the original profile bytes SHALL be recorded
in `Document.source_icc`. "Is sRGB" SHALL be decided by the profile's
`Description` tag containing `srgb`, case-insensitively. A document that is not
RGB (for example Grayscale or CMYK), a profile that is absent, does not parse, is
sRGB, or cannot be transformed by the engine, a malformed resource section, or a
layer missing a color channel SHALL leave the pixels and resources unchanged and
SHALL NOT set `source_icc`. The document's `mode` and `depth` SHALL be unchanged
by this pass. A CMYK document's profile SHALL be applied by the color-mode
normalization specified in `codec/psd-color-modes`, not by this RGB pass.

#### Scenario: An Adobe-RGB document is converted

- **WHEN** an RGB file carries an Adobe RGB (non-sRGB) ICC profile and a known pixel layer
- **THEN** `read_psd` returns that layer's pixels converted to sRGB (matching an independent lcms2 conversion), `source_icc` is the embedded profile bytes, and `mode` is unchanged

#### Scenario: A Grayscale document with a profile is untouched

- **WHEN** a Grayscale file carries a non-sRGB profile
- **THEN** the pixels and the resource bytes are unchanged and `source_icc` is `None`

#### Scenario: A CMYK document is left to the color-mode pass

- **WHEN** a CMYK file carries a CMYK ICC profile
- **THEN** this RGB pass leaves the document unchanged and the CMYK profile is applied by the color-mode normalization

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
an RGB document was ICC-normalized on read (`source_icc` is `Some`), so the sRGB
pixels are not re-tagged with the source profile. Every other resource SHALL be
re-emitted byte-for-byte. A CMYK document that was Convert-normalized keeps a
matching CMYK profile in `1039`: the working CMYK profile replaces the source
bytes, and the resource is not removed. The application's Embed Color Profile
choice may omit `1039` independently of this rule.

#### Scenario: The saved file drops the source profile

- **WHEN** an ICC-normalized RGB document is written and re-read
- **THEN** its image resources no longer contain resource `1039` and its pixels are the converted sRGB values

#### Scenario: Other resources survive

- **WHEN** the source file also carries an EXIF or XMP resource
- **THEN** those resources are present with identical bytes in the output

#### Scenario: A Convert-normalized CMYK document keeps its working profile

- **WHEN** a CMYK document read with the Convert policy is written unchanged
- **THEN** resource `1039` carries the working CMYK profile and the retained CMYK planes are byte-identical
