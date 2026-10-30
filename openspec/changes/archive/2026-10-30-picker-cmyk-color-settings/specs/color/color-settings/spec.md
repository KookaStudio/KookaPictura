## MODIFIED Requirements

### Requirement: Conversion options are persisted and applied

The application SHALL persist the rendering intent, black-point-compensation
flag, and dither flag as application preferences separate from any document. It
SHALL apply the rendering intent and the black-point-compensation flag to the
incoming Convert policy, Convert to Profile, and the picker's working-CMYK
evaluation, and the dither flag to the incoming Convert policy and Convert to
Profile. Dither SHALL NOT apply to the picker, which evaluates a single colour
rather than a plane. Changing them SHALL NOT create a history state or mark a
document dirty; it SHALL affect subsequent conversions only.

#### Scenario: The defaults match CS6

- **WHEN** the application starts with no stored conversion preference
- **THEN** the reported intent is Relative Colorimetric, black-point compensation is on, and dither is on

#### Scenario: A stored option survives a restart

- **WHEN** the intent is set to Perceptual and the session is saved and reloaded
- **THEN** the reported intent is Perceptual

#### Scenario: Changing an option is not a document edit

- **WHEN** an option is changed with a document open
- **THEN** the document's history count and dirty state are unchanged

#### Scenario: The picker follows a changed option [cs_picker_follows_options]

- **WHEN** the rendering intent or black-point compensation changes
- **THEN** the picker's CMYK readout and gamut alert change to that option's
  conversion, and the document's history count and dirty state are unchanged
