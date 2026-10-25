# color-profile-assignment Specification

## Purpose
A document working profile plus Assign Profile and Convert to Profile as single undoable steps.

## Requirements

### Requirement: A document carries a working color profile

`pictura-core` `Document` SHALL expose `document_icc: Option<Vec<u8>>` holding
the ICC bytes of the profile the document's stored pixels are in, where `None`
means the sRGB working space. A document opened with the Convert incoming policy
(`read_psd`'s behaviour), constructed, or assigned/converted to sRGB SHALL have
`document_icc` equal to `None`; a document opened with the Preserve policy for a
non-sRGB embedded profile SHALL have `document_icc` equal to the embedded bytes,
because its stored pixels remain in that profile. `document_icc` SHALL be
distinct from `source_icc`, which records the profile a read normalised away.

#### Scenario: A freshly opened document is in the sRGB working space

- **WHEN** a PSD with no embedded profile (or with an sRGB profile) is read
- **THEN** `document_icc` is `None`

#### Scenario: A read-normalised document records the source separately

- **WHEN** a PSD with a non-sRGB embedded profile is read with the Convert policy
- **THEN** `source_icc` holds the embedded bytes and `document_icc` is still
  `None`

#### Scenario: A preserved document carries the embedded profile

- **WHEN** a PSD with a non-sRGB embedded profile is read with the Preserve policy
- **THEN** `document_icc` holds the embedded bytes, the pixels are unchanged, and
  `source_icc` is `None`

### Requirement: Assign Profile retags a document without changing pixels

`pictura-codec` SHALL expose an operation that, given a document and a target
profile, sets the document's working profile to that target and rewrites its
image-resource section so it carries resource 1039 framed with the target's ICC
bytes, or omits 1039 when the target is the sRGB working space. It SHALL NOT
modify the document composite or any layer's color-channel bytes.

#### Scenario: Assign leaves every pixel byte unchanged

- **WHEN** Assign Profile is applied to a document with a target profile
- **THEN** `doc.composite` and every layer's channel data are byte-identical to
  before the operation

#### Scenario: Assign tags the target profile for saving

- **WHEN** Assign Profile to Adobe RGB is applied and the document is written
- **THEN** the image-resource section contains a 1039 block whose data is Adobe
  RGB's ICC bytes

#### Scenario: Assigning the sRGB working space removes the tag

- **WHEN** Assign Profile is applied with the sRGB working space as the target
- **THEN** the document has no 1039 resource and `document_icc` is `None`

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

### Requirement: An assigned or converted document is displayed in the working space

The application SHALL convert the composite it shows on the canvas from the
document's working profile to the sRGB working space before display, while
keeping `doc.composite` and layer channels in the document's working profile. A
document with no working profile SHALL be displayed unchanged.

#### Scenario: Assign changes the displayed appearance

- **WHEN** a document is assigned a non-sRGB profile that reinterprets its
  numbers
- **THEN** the displayed canvas pixels change while the stored pixels do not

#### Scenario: Convert preserves the displayed appearance

- **WHEN** a document is converted to a different profile
- **THEN** the displayed canvas pixels are within the color-conversion tolerance
  of the pre-conversion display

### Requirement: Assign Profile and Convert to Profile are single undo steps

Each operation SHALL record exactly one history state on the active document and
mark it dirty. Undoing a Convert SHALL restore the pre-conversion composite and
layer pixels exactly; undoing an Assign SHALL restore the previous profile and
image-resource section.

#### Scenario: Convert is a single reversible step

- **WHEN** Convert to Profile is applied and then undone
- **THEN** the composite, every layer's pixel bytes, and the working profile
  equal their pre-conversion values, and the operation added exactly one
  history state

#### Scenario: Assign records no pixel change but one state

- **WHEN** Assign Profile is applied
- **THEN** exactly one history state is added and undoing it restores the prior
  profile

### Requirement: Repeated working-space conversion reuses the parsed profile

Converting a document's pixels to its working/display color space SHALL reuse the
parsed working profile (`document_icc`) and its built transform across repeated
conversions of the same document state, rather than reparsing the profile and
rebuilding the transform each time. The converted pixels MUST be byte-identical
to a conversion that reparses the profile, and a change to the document's working
profile (assign or convert) SHALL invalidate the reuse.

#### Scenario: Repeated conversions are byte-identical to a fresh parse

- **WHEN** a document's pixels are converted to the working space twice without a
  change to its working profile
- **THEN** both conversions return the same bytes, equal to a conversion that
  reparses the profile

#### Scenario: A working-profile change is picked up

- **WHEN** a document's working profile is changed and its pixels are converted
  again
- **THEN** the conversion uses the new profile and equals a fresh conversion under
  that profile

#### Scenario: A document in the sRGB working space is not parsed

- **WHEN** a document whose `document_icc` is `None` has its pixels converted to
  the working space
- **THEN** the pixels are returned unchanged and no profile is parsed

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
