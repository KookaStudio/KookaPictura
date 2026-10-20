## MODIFIED Requirements

### Requirement: Transform page control wiring

The page's controls SHALL write through to the session: the locator and the X/Y
fields move the reference point, the show toggle sets `show_pivot`, W/H and the
link set the scale, the angle sets the rotation, H/V set the skew, and the
interpolation combo sets the session's resample method. The interpolation combo
SHALL list the six CS6 methods (Nearest Neighbor, Bilinear, Bicubic, Bicubic
Smoother, Bicubic Sharper, Bicubic Automatic). The warp button SHALL switch
between Free Transform and Warp modes and show as checked in Warp mode. Cancel
SHALL cancel the session and Apply SHALL commit it, both ending the session.

#### Scenario: The locator moves the pivot

- **WHEN** the bottom-right locator square is clicked
- **THEN** the session's pivot equals the live quad's bottom-right corner

#### Scenario: Apply commits and records one state

- **WHEN** a field changed the transform and Apply is clicked
- **THEN** the session ends, exactly one `"Free Transform"` state is recorded, and the tool page returns

#### Scenario: Cancel restores the document

- **WHEN** a field changed the transform and Cancel is clicked
- **THEN** the session ends with the document bit-identical to its pre-session value

#### Scenario: The interpolation combo reaches the session

- **WHEN** Nearest Neighbor is chosen in the combo
- **THEN** the session reports `nearest` as its interpolation

## ADDED Requirements

### Requirement: Warp mode fields

In Warp mode the transform page SHALL hide the reference-point, X/Y, W/H, angle
and skew fields and show the Warp fields, ported from photocraft's Warp bar:
- a style menu with None, Custom, and the fifteen presets in CS6 order;
- an orientation toggle;
- Bend, H, and V percentages in `[-100, 100]`.

Choosing Custom SHALL keep the current shape as a mesh. Choosing None SHALL
reset to the untouched one-patch grid, which reads as Custom. Choosing a preset
SHALL start it at the current bend, or at 50% from a non-preset, with no
distortion. The orientation toggle and the Bend, H, and V fields SHALL be
enabled only for a preset and SHALL write through to it.

#### Scenario: Warp mode swaps the fields [tob_warp_fields]

- **WHEN** a session enters Warp mode
- **THEN** the warp toggle is checked, the Warp fields are shown, and the angle field is hidden

#### Scenario: A preset style takes its values from the bar [tob_warp_values]

- **WHEN** Arc is chosen, Bend is typed as -25, H as 10, and the orientation toggle is clicked
- **THEN** the session's warp is Arc with bend -25, H 10, and vertical orientation, and choosing None disables the Bend field again
