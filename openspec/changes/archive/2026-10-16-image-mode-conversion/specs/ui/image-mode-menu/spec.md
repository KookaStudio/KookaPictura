# ui/image-mode-menu Specification

## Purpose
Wires the `Image ▸ Mode` submenu onto the mode and bit-depth conversions: check
marks for the current mode and depth, the CS6 prompts, the Indexed Color and
Bitmap dialogs, and one history state per conversion.

## ADDED Requirements

### Requirement: The Mode submenu reflects the document

The Bitmap, Grayscale, Indexed Color, RGB Color, CMYK Color, Lab Color, and
8/16/32 Bits/Channel entries SHALL be checkable commands. The entry for the
document's mode and the entry for its depth SHALL be checked. An entry SHALL be
enabled when the conversion is available or it is the checked entry, and
selecting the checked entry SHALL change nothing. Duotone, Multichannel, and
Color Table SHALL remain disabled stubs. Color Table SHALL be listed after the
depth entries. Without a document every entry SHALL be disabled.

#### Scenario: An RGB document

- **WHEN** an 8-bit RGB document is active
- **THEN** RGB Color and 8 Bits/Channel are checked; Grayscale, Indexed Color,
  CMYK Color, Lab Color, and 16/32 Bits/Channel are enabled; Bitmap, Duotone,
  Multichannel, and Color Table are disabled

### Requirement: Conversions prompt as CS6 does

Converting a color document to Grayscale SHALL first ask to discard color
information (Discard / Cancel). Converting a layered document to Grayscale,
CMYK, or Lab SHALL offer Flatten / Don't Flatten / Cancel. Converting a layered
document to Indexed Color or Bitmap SHALL ask to flatten layers (OK / Cancel),
since the conversion flattens. Indexed Color SHALL open a dialog with Palette
(Exact only when the image has at most 256 colors, Web, Local Perceptual/
Selective/Adaptive), Colors, Dither (None, Diffusion, Pattern, Noise), Amount,
and Preview. Preview SHALL show the conversion without recording history, and
Cancel SHALL restore the document exactly. Bitmap SHALL open a dialog offering
50% Threshold, Pattern Dither, and Diffusion Dither. Leaving a 32-bit document
SHALL open the HDR Conversion dialog. Cancelling any prompt or dialog SHALL
change nothing.

#### Scenario: Indexed preview then cancel

- **WHEN** the Indexed Color dialog previews a layered document and is
  cancelled
- **THEN** the layers and mode are restored and no history state was added

### Requirement: One Convert Mode state per conversion

Each completed conversion through the `image_mode` bridge (`convert_image_mode`,
`convert_image_depth`, `convert_to_indexed`, `convert_to_bitmap`) SHALL
recomposite and record exactly one history state labelled `Convert Mode`. A
refused conversion SHALL record nothing. Undo SHALL restore the exact prior
document, including its mode and depth.

#### Scenario: Grayscale and undo

- **WHEN** an RGB document is converted to Grayscale and then undone
- **THEN** one `Convert Mode` state was added and the composite pixels equal the
  original RGB
