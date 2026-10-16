## ADDED Requirements

### Requirement: Brush panel

`Window > Panels > Brush` (shortcut F5) SHALL show or hide a Brush panel whose
Brush Tip Shape page edits the brush shared by every painting tool: Size
(1–5000 px), Flip X, Flip Y, Angle (−180–180°), Roundness (1–100 %),
Hardness (0–100 %), and Spacing (1–1000 % of the size), plus a row of round
tip presets that set Size and Hardness. The page SHALL show a tip-shape
indicator and a stroke preview painted by the engine with the current tip, and
SHALL follow tip changes made elsewhere. The brush dynamics option sets,
Brush Presets, and turning Spacing off SHALL be shown disabled as not
implemented.

#### Scenario: Roundness reaches the brush

- **WHEN** the `brush_panel` self-test sets Roundness to 30 %
- **THEN** the controller's roundness is 30 and the stroke preview has painted pixels

#### Scenario: A flattened tip paints wide

- **WHEN** a 20 px, 30 % round, 0° Brush dab lands on white
- **THEN** pixels 7 px beside the centre are painted and pixels 7 px below it are not

### Requirement: Shared brush tip

The Brush, Pencil, Color Replacement, Mixer Brush, Clone Stamp, Pattern Stamp,
and History Brush SHALL paint with the panel's Roundness, Angle, and Spacing.
A single Flip X or Flip Y SHALL mirror the painted tip's angle, and both
together SHALL leave it unchanged.

#### Scenario: One flip mirrors the angle

- **WHEN** the angle is 30° and only Flip X is on
- **THEN** the tools paint with an angle of −30°
