# ui/color-picker-drag Specification

## Purpose
The Color Picker's colour services reuse their colour transforms, so a drag
through the field or ramp follows the cursor.

## Requirements

### Requirement: Picker colour services reuse their transforms

The picker's sRGB↔working-CMYK services (the gamut test, the closest printable
colour, and the CMYK readout and edit conversions) SHALL build their colour
transforms once for the installed working CMYK profile and SHALL reuse them on
every later call. They SHALL rebuild them only when Color Settings installs a
working CMYK profile. Every service MUST return the same value as a one-shot
conversion through the same profile, intent and black point compensation.

#### Scenario: Repeated calls build no transforms [cpd_reuse]

- **WHEN** the gamut test, closest colour, and CMYK conversions run for many
  colours with no Color Settings change
- **THEN** no transform is built after the first call

#### Scenario: Installing a working CMYK profile rebuilds once [cpd_rebuild]

- **WHEN** Color Settings installs a working CMYK profile
- **THEN** the transform pair is rebuilt exactly once, and later calls convert
  through the new pair

#### Scenario: Cached conversions match a one-shot conversion [cpd_exact]

- **WHEN** a colour is converted through the reusable transform and through a
  one-shot conversion with the same profiles, intent and black point
  compensation
- **THEN** the output bytes are identical

### Requirement: A picker drag follows the cursor

Each drag sample in the Color Picker dialog's colour field SHALL update the
field, every readout and both alerts. A sample MUST NOT parse a colour profile or
build a colour transform.

#### Scenario: A field drag updates the readouts on every sample [cpd_drag]

- **WHEN** the left button is held and the cursor moves across the colour field
  in many steps
- **THEN** after each step the hex readout shows the colour under the cursor,
  and the full drag finishes within the interactive budget

### Requirement: The Color Picker's eyedropper samples the canvas

While the Color Picker dialog is open, the canvas SHALL show the eyedropper
cursor and a left-button press over the image SHALL set the picker's colour to
the active document's composite at that pixel. With the button held, the
picker's colour SHALL follow the pointer as it moves, and releasing SHALL end
that gesture. A press outside the document, or a colour that reads as invalid,
SHALL leave the colour unchanged. Sampling SHALL NOT run the active tool or
record a history state. Closing the picker SHALL disarm the sampler, so a later
click reaches the active tool. This SHALL hold for every entry point that opens
the picker without its own sampler, including the toolbox and Color panel
foreground and background swatches; an entry point that supplies its own
sampler keeps it, and a picker opened while another dialog's sampler is armed
SHALL leave that sampler armed.

#### Scenario: A swatch picker arms the eyedropper [cpe_arm]

- **WHEN** the picker is opened from a colour swatch and the pointer moves over
  the document
- **THEN** the canvas shows the eyedropper cursor and the canvas sampler is
  armed

#### Scenario: A canvas click sets the picker colour [cpe_click]

- **WHEN** the user left-clicks a pixel of the image while the picker is open
- **THEN** the picker's colour becomes that pixel's composite colour, no
  history state is recorded, and the active tool does not run

#### Scenario: A held drag follows the pointer [cpe_drag]

- **WHEN** the user presses on one pixel and drags with the left button held
  across other pixels
- **THEN** the picker's colour becomes the composite colour under the pointer
  after each move

#### Scenario: Closing the picker disarms the sampler [cpe_release]

- **WHEN** the picker is closed
- **THEN** the canvas sampler is disarmed and a later click reaches the active
  tool again
