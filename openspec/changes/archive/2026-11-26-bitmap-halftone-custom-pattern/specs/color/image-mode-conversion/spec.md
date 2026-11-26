## ADDED Requirements

### Requirement: Halftone Screen Bitmap conversion

Halftone Screen SHALL reduce flattened 8-bit Grayscale to depth-1 Bitmap with a periodic screen at the requested physical frequency and angle. The system SHALL accept 1.000–999.999 lines/inch or 0.400–400.00 lines/cm, angle −180° through 180°, and Round, Diamond, Ellipse, Line, Square, and Cross shapes. Screen spacing SHALL use the document resolution, defaulting to 72 PPI when missing. Black and white input values SHALL remain black and white.

#### Scenario: Halftone settings control a periodic one-bit screen [bitmap_halftone]

- **WHEN** uniform mid-gray is converted at 72 PPI using the Line shape, 9 lines/inch, and 0° or 90°
- **THEN** an eight-pixel screen repeats, the orientation rotates with the angle, and the result retains a packed depth-1 Bitmap plane

#### Scenario: Screen units, shapes, and endpoints are honored [bitmap_screen_geometry]

- **WHEN** screens use equivalent lines/inch and lines/cm settings, each of the six shapes, a non-right angle, or black and white input
- **THEN** equivalent frequencies give the same pixels, each shape follows the specified ranked-cell geometry and rotation, and black and white remain unchanged

#### Scenario: Screen option boundaries are enforced [bitmap_screen_bounds]

- **WHEN** frequency or angle is at a documented boundary, outside its range, or nonfinite
- **THEN** valid boundary values are accepted and invalid values are refused without changing the document

### Requirement: Custom Pattern Bitmap conversion

Custom Pattern SHALL use a selected pattern's grayscale values as spatial thresholds. The system SHALL repeat the tile from the top-left canvas pixel. Black and white input values SHALL remain black and white. The system SHALL decline conversion when the pattern cannot be resolved or has invalid dimensions or samples.

#### Scenario: A chosen pattern repeats across both canvas axes [bitmap_custom]

- **WHEN** a rectangular grayscale tile is used to convert a uniform mid-gray canvas larger than the tile
- **THEN** the tile's lighter and darker thresholds produce a repeating one-bit result across both axes

#### Scenario: Invalid conversion options leave the document unchanged [bitmap_invalid]

- **WHEN** a Halftone Screen option is out of range or a Custom Pattern tile is missing or invalid
- **THEN** the operation SHALL fail without changing the source mode or document

#### Scenario: Missing patterns and endpoint samples are handled [bitmap_pattern_edges]

- **WHEN** the selected pattern is missing or invalid, or black and white input is converted with a valid tile
- **THEN** an unresolved or invalid tile is refused without changing the document, and valid conversion preserves black and white

### Requirement: Bitmap method controls and replay

The Bitmap dialog SHALL offer Halftone Screen and Custom Pattern along with its three existing methods, and SHALL show only the controls applicable to the selected new method. The system SHALL record the selected method and options for silent and modal replay and make a successful conversion one Convert Mode history state. Modal replay SHALL prefill the recorded options, apply accepted edits and return them to the command, and cancel without conversion or changes to the recorded options. Silent replay SHALL refuse an unresolved pattern without changing the document.

#### Scenario: The Bitmap dialog exposes controls for the selected method [bitmap_controls]

- **WHEN** the user switches between Halftone Screen, Custom Pattern, and an existing method
- **THEN** the dialog shows only the selected method's applicable options

#### Scenario: A recorded Bitmap conversion reuses its options [bitmap_replay]

- **WHEN** a recorded conversion with non-default Halftone Screen options or a custom pattern is replayed on an equivalent source
- **THEN** the one-bit output matches the initial conversion and the successful replay adds one Convert Mode history state

#### Scenario: Modal replay pre-fills, edits, and cancels [bitmap_modal_replay]

- **WHEN** a recorded new-method conversion is replayed with its dialog enabled
- **THEN** the dialog is prefilled with recorded options, acceptance applies and returns edited options, and cancellation changes neither document nor recorded options

#### Scenario: Silent replay refuses a missing pattern [bitmap_replay_missing]

- **WHEN** a recorded Custom Pattern conversion is replayed silently and its pattern cannot be resolved
- **THEN** replay fails without changing the document or adding a history state
