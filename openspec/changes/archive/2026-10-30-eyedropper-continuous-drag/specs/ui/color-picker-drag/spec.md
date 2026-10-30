## ADDED Requirements

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
