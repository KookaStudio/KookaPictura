# Spec Delta

## MODIFIED Requirements

### Requirement: Transform page control wiring

The page's controls SHALL write through to the session. The 3×3 locator SHALL
move the reference point within the target and SHALL highlight the cell that
holds the reference point; when the reference point is not on one of the nine
anchors, no cell SHALL be highlighted. The X/Y fields SHALL set the reference
point's document position and translate the target so the reference point lands
there, refreshing the canvas preview on every committed edit; in absolute mode
the values are document coordinates, and with relative positioning on they are
offsets from a fixed session origin. The show toggle sets `show_pivot`, W/H and
the link set the scale, the angle sets the rotation, H/V set the skew, and the
interpolation combo sets the session's resample method. The interpolation combo
SHALL list the six CS6 methods (Nearest Neighbor, Bilinear, Bicubic, Bicubic
Smoother, Bicubic Sharper, Bicubic Automatic). The warp button SHALL switch
between Free Transform and Warp modes and show as checked in Warp mode. Cancel
SHALL cancel the session and Apply SHALL commit it, both ending the session; the
commit SHALL record the session's own history label (`"Free Transform"` for a
layer, `"Transform Selection"` for a selection border).

#### Scenario: The locator moves the pivot

- **WHEN** the bottom-right locator square is clicked
- **THEN** the session's pivot equals the live quad's bottom-right corner

#### Scenario: The locator highlights the active cell [tob_pivot_highlight]

- **WHEN** a locator cell is clicked, and then the reference point is dragged between anchors
- **THEN** the clicked cell is highlighted and the others are not, and a pivot between anchors leaves every cell unhighlighted

#### Scenario: X and Y translate the target [tob_xy_moves_target]

- **WHEN** a document X or Y value is typed into the field
- **THEN** the target translates so the reference point lands at that position, the canvas preview shows the move before Apply, and Apply commits the moved result

#### Scenario: Apply commits and records one state

- **WHEN** a field changed the transform and Apply is clicked
- **THEN** the session ends, exactly one state labelled for the session's target is recorded, and the tool page returns

#### Scenario: Apply on a border session records Transform Selection [tob_apply_border]

- **WHEN** a field changes a selection-border transform and Apply is clicked
- **THEN** exactly one `"Transform Selection"` state is recorded and the tool page returns

#### Scenario: Cancel restores the document

- **WHEN** a field changed the transform and Cancel is clicked
- **THEN** the session ends with the document bit-identical to its pre-session value

#### Scenario: The interpolation combo reaches the session

- **WHEN** Nearest Neighbor is chosen in the combo
- **THEN** the session reports `nearest` as its interpolation

## ADDED Requirements

### Requirement: Split scale slider

The W and H fields' slider popups SHALL map their first half to the scale range
from the field minimum to 100% and their second half to 100%-1000%, so the
midpoint is exactly 100% and the maximum is 1000%. The typed range SHALL also
end at 1000%.

#### Scenario: The scale slider midpoint is 100% [tob_scale_slider_split]

- **WHEN** the W slider is set to its midpoint, to a quarter, and to its maximum
- **THEN** W is 100% at the midpoint, about 50% at a quarter, and 1000% at the maximum, and the H slider has the same range
