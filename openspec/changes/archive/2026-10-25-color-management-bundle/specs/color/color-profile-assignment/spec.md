# Spec Delta

## MODIFIED Requirements

### Requirement: Convert to Profile transforms pixels and retags

`pictura-codec` SHALL expose an operation that, given an RGB document, a
destination profile, and `ConversionOptions`, converts the document composite
and every layer's color channels from the document's current working profile (or
sRGB when `document_icc` is `None`) to the destination using the supplied
rendering intent and black-point-compensation setting, then sets the working
profile and resource 1039 as Assign Profile does. The transform SHALL be applied
with the existing planar color conversion and SHALL dither only 8-bit color
channels; 16- and 32-bit retained native samples SHALL be converted at their own
depth and SHALL ignore dither. A document whose color channels are incomplete
SHALL be left with those channels unconverted rather than failing. When the
requested operation also flattens, the document SHALL be flattened to a single
opaque Background before the color transform, and the result SHALL carry no
other layers. A non-RGB document, a bad destination profile, or a working
profile that cannot be parsed SHALL be refused without mutating the document.

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

## ADDED Requirements

### Requirement: Convert to Profile previews on the canvas

The Convert to Profile dialog SHALL offer a Preview option that shows the
converted document on the canvas without recording history. While the preview is
open, changing the destination or conversion options SHALL re-preview from the
pre-preview document, never from the previous preview. Cancelling the dialog
SHALL restore the pre-preview document exactly. Accepting the dialog SHALL
discard the preview and perform the conversion once as a single undo state.

#### Scenario: Preview shows the conversion without history

- **WHEN** Preview is enabled in Convert to Profile
- **THEN** the canvas shows the converted composite and the history count does not change

#### Scenario: Changing an option re-previews from the base

- **WHEN** Preview is on and the destination is changed from Adobe RGB to ProPhoto RGB
- **THEN** the new preview is the conversion of the original document, not of the Adobe RGB preview

#### Scenario: Cancel restores the original

- **WHEN** Preview is on and the dialog is cancelled
- **THEN** the document's pixels, working profile, and history are exactly the pre-preview values

#### Scenario: Accept records one state

- **WHEN** Preview is on and the dialog is accepted
- **THEN** the final document is the converted result and exactly one Convert to Profile history state is added
