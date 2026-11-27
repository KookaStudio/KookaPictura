# tools/eyedropper-modifiers Specification

## Purpose

Eyedropper modifier gestures select the colour destination and provide temporary sampling from eligible painting tools.

## Requirements

### Requirement: Eyedropper colour destination

The Eyedropper SHALL sample the foreground by default and the background when Alt is held at press. The destination SHALL remain fixed for a held drag, which SHALL sample continuously using the current Eyedropper size and scope. Sampling SHALL leave the other colour and document history unchanged.

#### Scenario: Alt samples background [eye_background]
- **WHEN** the Eyedropper Alt-drags across differently coloured pixels
- **THEN** the background follows the sampled colours, the foreground is unchanged, and no history state is added

#### Scenario: Plain sampling remains foreground [eye_foreground]
- **WHEN** the Eyedropper drags without modifiers
- **THEN** the foreground follows the sampled colours and the background is unchanged

### Requirement: Temporary Eyedropper for painting tools

Alt-press with Brush, Pencil, Color Replacement, Pattern Stamp, History Brush or Art History Brush SHALL start a temporary foreground Eyedropper gesture using the Eyedropper's sample size and scope. The gesture SHALL sample continuously until mouse release, even if Alt is released first, leave the selected tool and its options unchanged, create no stroke or history state, and keep the temporary cursor until mouse release. After mouse release, the cursor SHALL reflect the current modifiers. A gesture already in progress SHALL retain its owner. Existing Alt gestures for Clone Stamp, Healing Brush, Eraser and Mixer Brush SHALL remain available. Temporary sampling SHALL also work in Quick Mask for all six eligible tools, using the same document sampling as the Eyedropper and leaving the mask unchanged.

#### Scenario: Temporary sampling uses Eyedropper settings [eye_temporary]
- **WHEN** each eligible tool Alt-drags over an image with non-default Eyedropper sampling settings
- **THEN** the foreground follows the scoped samples, the selected tool and options remain unchanged, and no stroke or history state is created

#### Scenario: Temporary cursor and gesture restore [eye_restore]
- **WHEN** Alt is released while the mouse remains held during temporary sampling
- **THEN** sampling and the temporary cursor continue until mouse release without starting a stroke, after which the normal cursor and subsequent stroke behavior are restored

#### Scenario: Cancel temporary sampling [eye_cancel]
- **WHEN** the temporary sampling session is cancelled
- **THEN** the selected tool's cursor and subsequent normal stroke behavior are restored

#### Scenario: Temporary sampling in Quick Mask [eye_quick_mask]
- **WHEN** each eligible tool Alt-drags in Quick Mask
- **THEN** the foreground follows the same scoped document samples as the Eyedropper, while the mask, selected tool, and history remain unchanged

#### Scenario: Existing Alt gestures remain [eye_reserved]
- **WHEN** Alt is used with Clone Stamp, Healing Brush, Eraser or Mixer Brush
- **THEN** the tool's existing source, history, or reservoir gesture operates instead of temporary foreground sampling

#### Scenario: Mid-stroke Alt does not steal the stroke [eye_stroke]
- **WHEN** Alt is pressed after a painting stroke begins
- **THEN** the stroke retains its ownership through release and no temporary sampling starts
