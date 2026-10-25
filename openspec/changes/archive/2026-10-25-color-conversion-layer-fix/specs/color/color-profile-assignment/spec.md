## MODIFIED Requirements

### Requirement: Convert to Profile transforms pixels and retags

`pictura-codec` SHALL expose an operation that, given an RGB document, a
destination profile, and `ConversionOptions`, converts the document composite
and every layer's color channels from the document's current working profile (or
sRGB when `document_icc` is `None`) to the destination using the supplied
rendering intent and black-point-compensation setting, then sets the working
profile and resource 1039 as Assign Profile does. The transform SHALL be applied
with the existing planar color conversion and SHALL dither only 8-bit color
channels; 16- and 32-bit retained native samples in the working RGB space SHALL
be converted at their own depth and SHALL ignore dither. A retained native store
that encodes a recorded source mode (CMYK or Lab) SHALL NOT be converted as RGB;
the 8-bit working channels already carry that conversion. A document whose color
channels are incomplete SHALL be left with those channels unconverted rather
than failing. When the requested operation also flattens, the document SHALL be
flattened to a single opaque Background before the color transform, and the
result SHALL carry no other layers. A non-RGB document, a bad destination
profile, or a working profile that cannot be parsed SHALL be refused without
mutating the document.

#### Scenario: Convert transforms the pixels

- **WHEN** Convert to Profile is applied to a document whose composite is a
  known color patch, with Perceptual intent and black-point compensation on
- **THEN** the composite bytes equal the reference lcms2 conversion of that
  patch with the same intent and flag, within one LSB per channel

#### Scenario: Convert preserves appearance and tags the destination

- **WHEN** a document is converted from sRGB to Adobe RGB and then displayed
- **THEN** the displayed bytes are close to the pre-conversion display and the
  document carries a 1039 resource for Adobe RGB

#### Scenario: Dither changes only 8-bit output

- **WHEN** the same conversion is run with dither on and off on an 8-bit document
- **THEN** the outputs differ deterministically and the dithered output is
  stable across two runs; a 16-bit document converted with dither on equals its
  undithered output

#### Scenario: Flatten leaves one Background

- **WHEN** Convert to Profile is applied with Flatten Image on to a document with
  several layers
- **THEN** the result holds exactly one Background layer and its pixels are the
  converted composite

#### Scenario: A non-RGB document is refused

- **WHEN** Convert to Profile is requested for a CMYK or Grayscale document
- **THEN** the operation returns false without mutating the document

#### Scenario: A source-mode native store is left alone

- **WHEN** Convert to Profile is applied to an RGB-mode document read from a
  16-bit CMYK PSD with pixel layers
- **THEN** the 8-bit layer channels and composite convert to the destination,
  while the retained CMYK planes and layer stores remain in the source profile
