## ADDED Requirements

### Requirement: Freeform Pen tool

A Freeform Pen drag SHALL add a subpath of anchors following the stroke within
the Curve Fit tolerance (0.5-10 px), a higher Curve Fit never keeping more
anchors for the same stroke; a drag ending near its start SHALL close the
subpath. A drag SHALL record exactly one "Freeform Pen" state and a click SHALL
record nothing.

#### Scenario: Drawing a closed loop

- **WHEN** the `tst_pen_tools` Freeform Pen test drags a wobbly square back to its start
- **THEN** one "Freeform Pen" state is recorded, the subpath is closed with four to six anchors, and a wave drawn at Curve Fit 10 keeps fewer anchors than at 0.5
