# tools/shape-geometry-options Specification

## Purpose
Snap To Pixels and Proportional geometry in the shape tools' geometry gear.

## ADDED Requirements

### Requirement: Snap To Pixels

The geometry gear SHALL offer Snap To Pixels for Rectangle and Rounded
Rectangle. When on, a drawn box SHALL snap its edges to whole document pixels
before the outline is generated. Other shape tools SHALL ignore it.

#### Scenario: A fractional rectangle snaps to whole pixels

- **WHEN** Snap To Pixels is on and a rectangle is dragged from (10.4, 10.4) to (40.4, 30.6)
- **THEN** its outline lies on whole-pixel coordinates

### Requirement: Proportional geometry

The geometry gear SHALL offer Proportional for the box tools with a width and
height ratio. A drag under Proportional SHALL keep that ratio unless Shift
squares the box.

#### Scenario: A proportional drag keeps the ratio

- **WHEN** Proportional is chosen with a 2:1 ratio and a rectangle is dragged wider than 2:1
- **THEN** the resulting box has a 2:1 width-to-height ratio

#### Scenario: Shift still squares

- **WHEN** Proportional is chosen with a 2:1 ratio and Shift is held during the drag
- **THEN** the resulting box is square
