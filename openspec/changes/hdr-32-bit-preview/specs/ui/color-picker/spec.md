# Spec Delta

## ADDED Requirements

### Requirement: HDR color picker

When a 32-bit document is active, the foreground/background swatch entry points SHALL open an HDR color picker instead of the ordinary picker. The HDR picker SHALL provide the ordinary colour field, ramp, comparison swatch, and numeric models, plus:

- an Intensity slider in stops and editable 32-bit floating-point R, G, B fields;
- a Preview area of swatches at exposures spaced by `Preview Stop Size`, with a
  `Relative to Document` option that folds in the document's preview exposure;
- Add to Swatches, which stores the float triple rather than a clamped integer
  colour.

The float value SHALL be the picker colour converted from sRGB to linear light
and multiplied by `2^intensity_stops`. A value above `1.0` SHALL be
representable. Non-finite float input SHALL be clamped to the finite boundary;
unparsable input SHALL leave the current colour unchanged. The `QColor` mirror
used by 8-bit tools SHALL hold the clamped sRGB representation; painting
remains 8-bit. The ordinary picker SHALL remain unchanged for 8- and 16-bit
documents.

#### Scenario: HDR mode opens for a 32-bit document

- **WHEN** a 32-bit document is active and the foreground swatch is clicked
- **THEN** the HDR picker opens with the Intensity slider and float RGB fields

#### Scenario: Intensity scales the linear colour

- **WHEN** the intensity is set to `+2` stops on a base colour
- **THEN** the resulting float RGB values are the base linear-light values multiplied by `4`

#### Scenario: Float fields accept values above white

- **WHEN** `2.5`, `1.0`, and `0.25` are entered in the 32-bit fields
- **THEN** the picker's colour is that float triple and the values are not clamped to `1.0`

#### Scenario: Non-finite input is clamped

- **WHEN** `NaN` or an infinite value is committed in a 32-bit field
- **THEN** the value is clamped to the finite boundary rather than producing a non-finite colour

#### Scenario: Preview swatches follow stop size

- **WHEN** Preview Stop Size is `3` and Relative to Document is off
- **THEN** the preview shows swatches at `−9`, `−6`, `−3`, `+3`, `+6`, and `+9` stops around the current colour

#### Scenario: Relative to Document folds in the preview

- **WHEN** Relative to Document is on and the document has a non-zero preview exposure
- **THEN** each preview swatch includes that document exposure

#### Scenario: Add to Swatches keeps the float value

- **WHEN** Add to Swatches is confirmed for an HDR colour
- **THEN** the new swatch retains the float triple and the owning swatches capability round-trips it

#### Scenario: 8-bit documents keep the ordinary picker

- **WHEN** an 8- or 16-bit document is active and a swatch is clicked
- **THEN** the ordinary colour picker opens with no Intensity slider or float fields

## MODIFIED Requirements

### Requirement: Picker entry points

The toolbox's foreground and background swatches SHALL open the color picker
dialog for that colour. For a 32-bit document they SHALL open the HDR color
picker; for an 8- or 16-bit document they SHALL open the ordinary picker. The
Color panel SHALL embed the picker's colour field and hue ramp inline so clicking
them sets the active swatch, and clicking an active Color panel swatch SHALL open
the depth-appropriate picker.

#### Scenario: Toolbox swatch

- **WHEN** a toolbox foreground/background swatch is clicked and a colour chosen
- **THEN** that swatch adopts the chosen colour

#### Scenario: Color panel field

- **WHEN** the Color panel's field or ramp is clicked
- **THEN** the active foreground/background colour follows it

#### Scenario: A 32-bit toolbox swatch opens the HDR picker

- **WHEN** a toolbox swatch is clicked with a 32-bit document active
- **THEN** the HDR picker opens instead of the ordinary picker
