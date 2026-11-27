# tools/color-sampler Specification

## Purpose
Color samplers: up to four document points whose composite color the Info panel reads out, placed and edited with the Color Sampler tool.

## Requirements

### Requirement: Color samplers

Color samplers SHALL be stored on `Document::annotations` as document pixel
positions, at most four; adding a fifth SHALL be refused without change. The
Info panel SHALL list each sampler, numbered from 1 in placement order, with its
position and the composite RGB at that pixel, and SHALL refresh after edits.

#### Scenario: The fifth sampler is refused

- **WHEN** four samplers are placed and a fifth placement is attempted
- **THEN** the document still holds four samplers and no history state is added

#### Scenario: The Info panel reads a sampler

- **WHEN** a sampler is placed on a black pixel
- **THEN** the Info panel lists `#1` with its position and R 0 G 0 B 0

### Requirement: Color Sampler tool

The Color Sampler tool SHALL place a sampler at the clicked pixel as one "Color
Sampler" state, move a dragged sampler as one "Move Color Sampler" state on
release, delete a sampler on Alt-click or when dragged off the canvas as one
"Delete Color Sampler" state, and remove all samplers from the options bar's
Clear as one "Clear Color Samplers" state. The canvas SHALL show every sampler
as a numbered crosshair with any tool active.

#### Scenario: Placing, moving, and deleting samplers

- **WHEN** the `annotation_tools` self-test places, drags, Alt-clicks, drags off, and clears samplers
- **THEN** each step records its one named state and the overlay count follows

#### Scenario: Samplers survive undo and tool changes

- **WHEN** Clear is undone and the Move tool is selected
- **THEN** the samplers are back on the document and still drawn on the canvas


### Requirement: Place a sampler with the Eyedropper

Shift-click with the Eyedropper SHALL place one Color Sampler at the clicked document pixel using the existing sampler limit and history behavior. The gesture SHALL leave foreground and background colours unchanged and SHALL NOT sample on drag. Shift takes precedence when Alt is also held.

#### Scenario: Shift-click places a sampler [eye_sampler]
- **WHEN** the Eyedropper Shift-clicks a document pixel
- **THEN** one sampler is added at that pixel and neither colour changes

#### Scenario: Fifth Shift-click is refused [eye_sampler_limit]
- **WHEN** four samplers exist and the Eyedropper Shift-clicks another pixel
- **THEN** the sampler count and history remain unchanged and the refusal is reported

#### Scenario: Shift and Alt together place a sampler [eye_sampler_priority]
- **WHEN** the Eyedropper is clicked with Shift and Alt held
- **THEN** one sampler is placed and neither colour changes
