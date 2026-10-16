## MODIFIED Requirements

### Requirement: Accented Edges

The system SHALL implement `Filter::AccentedEdges { edge_width, edge_brightness, smoothness }` as edge accentuation: edges are traced into lines from the smoothed brightness and laid over the image dark when `edge_brightness` is below 25 and light (chalk) when it is above, harder the further from 25; at 25 the edges SHALL NOT be accented.

#### Scenario: Edge brightness polarity

- **WHEN** Accented Edges is applied to an edged image at `edge_brightness` 0 and at 50
- **THEN** the 0 output is darker near the edge than the input and the 50 output is lighter near the edge than the input

#### Scenario: Edge width retains a detectable edge

- **WHEN** Accented Edges is applied at `edge_width` 1 and at 14
- **THEN** both outputs preserve a detectable luminance change across the source edge

#### Scenario: Edge brightness 25 is neutral

- **WHEN** Accented Edges is applied to an edged image at `edge_brightness` 0, 25, and 50
- **THEN** at the edge the 25 output is lighter than the 0 output and darker than the 50 output
