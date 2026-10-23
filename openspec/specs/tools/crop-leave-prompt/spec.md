# tools/crop-leave-prompt Specification

## Purpose
Confirmation when a tool switch leaves a pending crop, and Perspective Crop's Reset and menu spacing.

## Requirements

### Requirement: Leaving a pending crop asks

Switching away from the Crop tool with a pending crop, or from Perspective Crop
with a complete quad, SHALL ask "Crop the image?" with Crop, Cancel and Don't
Crop. Crop SHALL commit the crop and then switch; Don't Crop SHALL switch and
discard it; Cancel SHALL stay on the tool with the crop intact. A box that is
only the untouched default SHALL NOT ask.

#### Scenario: Cancel, Don't Crop and Crop [cl_prompt_choices]

- **WHEN** a Crop box is pending and another tool is chosen, answering Cancel, then Don't Crop, then (with a new box) Crop
- **THEN** the tool stays Crop with the box intact; then the tool switches and the document is unchanged; then the tool switches and the document is cropped to the box

#### Scenario: A default box does not ask [cl_prompt_default_box]

- **WHEN** the Crop tool holds only its default box and another tool is chosen
- **THEN** no question is asked

#### Scenario: Perspective Crop asks too [cl_prompt_perspective]

- **WHEN** a Perspective Crop quad is complete and another tool is chosen
- **THEN** Cancel stays on the tool and Don't Crop switches

### Requirement: Perspective Crop reset

Perspective Crop SHALL show a Reset button beside Cancel and Apply once the
quad is complete. Reset SHALL clear the quad and the output size and keep the
tool active.

#### Scenario: Reset clears the quad [cl_perspective_reset]

- **WHEN** the quad is complete and Reset is pressed
- **THEN** the quad and the output width and height are cleared, the three buttons hide, and the tool stays Perspective Crop

### Requirement: Crop menus share spacing

The guide-overlay menu SHALL use the same vertical padding and separator
spacing as the Settings menu.

#### Scenario: The menus look alike [cl_menu_spacing]

- **WHEN** the guide-overlay and Settings menus are opened
- **THEN** their rows and separators are spaced the same
