# ui/brush-panel Specification

## MODIFIED Requirements

### Requirement: Brush panel

`Window > Panels > Brush` (shortcut F5) SHALL show or hide a Brush panel whose
Brush Tip Shape page edits the brush shared by every painting tool: Size
(1–5000 px), Flip X, Flip Y, Angle (−180–180°), Roundness (1–100 %),
Hardness (0–100 %), and Spacing (1–1000 % of the size), plus a row of round
tip presets that set Size and Hardness. The page SHALL show a tip-shape
indicator and a stroke preview painted by the engine with the current tip, and
SHALL follow tip changes made elsewhere.

The option-set list SHALL also offer Shape Dynamics, Scattering, Color
Dynamics, Transfer, and Brush Pose. Each SHALL have an enable checkbox that
turns the set on or off without discarding its values, a page on the right with
its controls, and a padlock that retains the set across preset switches. The
list SHALL show the set's values when selected and SHALL follow changes made
elsewhere. Texture and Dual Brush SHALL remain visible and disabled until their
changes land.

#### Scenario: Roundness reaches the brush

- **WHEN** the `brush_panel` self-test sets Roundness to 30 %
- **THEN** the controller's roundness is 30 and the stroke preview has painted pixels

#### Scenario: A flattened tip paints wide

- **WHEN** a 20 px, 30 % round, 0° Brush dab lands on white
- **THEN** pixels 7 px beside the centre are painted and pixels 7 px below it are not

#### Scenario: A dynamics row opens its page

- **WHEN** Shape Dynamics is selected in the option-set list
- **THEN** its size, angle, roundness, and minimum controls are shown and editable

#### Scenario: A set checkbox writes the model

- **WHEN** the Scattering checkbox is cleared
- **THEN** the stroke uses no scatter or count dynamics and the stored values remain

#### Scenario: A padlock retains a set

- **WHEN** the Color Dynamics padlock is clicked and a brush preset is applied
- **THEN** the lock shows on the row and the preset leaves Color Dynamics unchanged
