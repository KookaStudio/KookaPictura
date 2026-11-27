# ui/rgb-channel-interaction Specification

## Purpose
Define how the Channels panel selects and displays RGB channels, how eye
toggles control visibility, and how channel state affects adjustment availability.
Use the Layers panel conventions for row and thumbnail presentation.

## Requirements

### Requirement: RGB row selection and visibility
The Channels panel SHALL select RGB component rows and update their visibility independently of eye clicks.

#### Scenario: Composite selection
- **WHEN** the user clicks the RGB composite row
- **THEN** all RGB components are visible and highlighted as active

#### Scenario: Component selection
- **WHEN** all RGB components are visible and the user clicks a component row
- **THEN** only that component is selected and visible

#### Scenario: Existing partial visibility
- **WHEN** a subset of RGB components is visible and the user clicks a visible component row
- **THEN** the clicked row is selected and the visibility subset remains unchanged

#### Scenario: Hidden component selection
- **WHEN** a subset of RGB components is visible and the user clicks a hidden component row
- **THEN** only the clicked component is selected and visible

#### Scenario: Eye click
- **WHEN** the user clicks an eye icon
- **THEN** its visibility changes without changing the active row or unrelated channels

### Requirement: RGB adjustment availability
The application SHALL restrict color-dependent Image Adjustments when a component is active or only a subset is visible, subject to existing command requirements.

#### Scenario: Partial RGB
- **WHEN** a component is active or fewer than all RGB components are visible
- **THEN** color-dependent adjustments are disabled and the specified tonal adjustments retain their ordinary eligibility

#### Scenario: Composite RGB
- **WHEN** the RGB composite is active and all components are visible
- **THEN** normal adjustment eligibility is restored

### Requirement: Channel list presentation
The Channels panel SHALL use the Layers panel visual conventions and provide two pixels of vertical padding around channel thumbnails.

#### Scenario: RGB rows displayed
- **WHEN** RGB channels are displayed
- **THEN** the list has a consistent eye column, thumbnail spacing and active-row appearance
