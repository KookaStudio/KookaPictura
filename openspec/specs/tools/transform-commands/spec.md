# tools/transform-commands Specification

## Purpose
The `Edit > Transform` submenu commands beyond Free Transform: constrained Scale
and Rotate sessions, the exact rotations and flips, and Transform Again.

## Requirements

### Requirement: Transform submenu

The `Edit > Transform` submenu SHALL list Again (`Shift+Ctrl+T`), Scale, Rotate,
Skew, Distort, Perspective, Warp, Rotate 180°, Rotate 90° Clockwise, Rotate 90°
Counter Clockwise, Flip Horizontal, and Flip Vertical in that order, separated
after Again, after Warp, and after the quarter turns. Every item SHALL be a live
command, enabled only when it can act on the active target.

#### Scenario: Menu order and separators [tc_menu]

- **WHEN** the `Edit > Transform` submenu is built
- **THEN** its items appear in the CS6 order with separators after Again, after Warp, and after Rotate 90° Counter Clockwise
- **AND** Again shows `Shift+Ctrl+T`

#### Scenario: Commands enable on a transformable target [tc_enable]

- **WHEN** a raster layer is the current layer
- **THEN** the Scale, Rotate, exact-rotation, and flip commands are enabled
- **WHEN** the Background layer is current
- **THEN** those commands are disabled

#### Scenario: Scale and Rotate disable where the switch cannot act [tc_enable_session]

- **WHEN** a Skew, Distort, Perspective, or Warp session is open
- **THEN** Scale and Rotate are disabled
- **WHEN** a Free Transform session is open
- **THEN** they are enabled

### Requirement: Constrained Scale and Rotate

`Edit > Transform > Scale` SHALL open a session whose handles scale and whose
rotate band is inert. `Edit > Transform > Rotate` SHALL open a session whose
handles rotate and whose scale state stays unchanged. Both SHALL keep the
reference point, numeric fields, and commit/cancel behaviour of Free Transform.

#### Scenario: Scale scales without rotating [tc_scale]

- **WHEN** Scale is chosen and the top-right handle is dragged outward
- **THEN** the box widens on that axis, its height and rotation are unchanged, and a press outside the box grabs nothing

#### Scenario: Rotate rotates without scaling [tc_rotate]

- **WHEN** Rotate is chosen and a handle is dragged a quarter turn about the reference point
- **THEN** the box rotates 90° and its scale stays 100%

#### Scenario: Choosing Scale or Rotate mid-session keeps the transform [tc_mode_switch]

- **WHEN** a Free Transform session is open with a scale applied and Scale is chosen
- **THEN** the session stays open with the same box and only the gesture constraint changes

### Requirement: Exact rotations and flips

`Rotate 180°`, `Rotate 90° Clockwise`, `Rotate 90° Counter Clockwise`, `Flip
Horizontal`, and `Flip Vertical` SHALL apply immediately to the active layer or,
with a selection, to the selected pixels. With a session open they SHALL update
the live box instead of committing. Each immediate command SHALL record exactly
one history state named for the command.

#### Scenario: Rotate 90° Clockwise swaps the box [tc_rot90]

- **WHEN** Rotate 90° Clockwise is invoked on a 12×10 layer
- **THEN** the layer's box becomes 10×12 and one `Rotate 90° Clockwise` state is added

#### Scenario: Rotate 180° and flips move the content [tc_rot180_flip]

- **WHEN** Rotate 180°, Flip Horizontal, and Flip Vertical are invoked in turn on a layer with an off-centre mark
- **THEN** the mark lands at the opposite corner, the mirrored horizontal position, and the mirrored vertical position respectively

#### Scenario: A selection transforms only its pixels [tc_selection]

- **WHEN** a selection covers part of a layer and Rotate 90° Clockwise is invoked
- **THEN** the selected pixels move and the unselected pixels stay

#### Scenario: An open session updates in place [tc_session]

- **WHEN** a Free Transform session is open and Rotate 90° Clockwise is invoked
- **THEN** the session stays open, its box rotates 90°, and no history state is added

### Requirement: Transform Again

`Edit > Transform > Again` SHALL repeat the last committed similarity transform
on the current target, including the reference point's relative position and
translation, and SHALL record one `Transform Again` state. It SHALL be disabled
until a repeatable transform has been committed and while a session is open.

#### Scenario: Again repeats the last transform [tc_again]

- **WHEN** Rotate 90° Clockwise is invoked and then Again
- **THEN** the layer is rotated 180° in total and the second state is `Transform Again`

#### Scenario: Again is unavailable before a transform [tc_again_disabled]

- **WHEN** a document is open and no transform has been committed
- **THEN** Again is disabled
