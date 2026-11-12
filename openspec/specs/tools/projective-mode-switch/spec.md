# tools/projective-mode-switch Specification

## Purpose
Choosing Skew, Distort, or Perspective while a transform session is open
switches its mode and keeps the pending transform, as Photoshop CS6 does.

## Requirements

### Requirement: Choosing a projective mode mid-session keeps the transform

While a transform session is open, `Edit > Transform > Skew`, `Distort`, and
`Perspective` SHALL switch the open session to that mode instead of starting a
new one. The box SHALL keep its corners and the reference point its document
position; the pending move, scale, rotation, and skew SHALL carry into the
projective quad, and a later commit SHALL apply them with the projective edit.
The switch SHALL be refused while a warp is being edited or is stored, and
Distort and Perspective SHALL be refused for a session on editable type, as
their begin is. Leaving a projective mode for Free, Scale, or Rotate SHALL stay
refused.

#### Scenario: Skew keeps the pending transform [pms_keep]

- **WHEN** a Free Transform session has a move, a 150 % scale, and a 30° rotation pending and Skew is chosen
- **THEN** the session stays open in Skew mode with the same box corners and the same reference point position

#### Scenario: The kept transform commits [pms_commit]

- **WHEN** a Free Transform session with a 150 % width is switched to Distort and committed without a drag
- **THEN** the layer is 150 % wide, as the Free Transform commit makes it

#### Scenario: Distort is refused for editable type [pms_type]

- **WHEN** a Free Transform session is open on an editable type layer and Distort is chosen
- **THEN** the session stays in Free mode and the Distort command is disabled

#### Scenario: A stored warp refuses the switch [pms_warp]

- **WHEN** a session holds a stored warp and Skew is requested
- **THEN** the mode does not change
