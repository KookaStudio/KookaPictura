# canvas-scrollbars Specification

## Purpose
Scrollbars that project the view state and follow pan, zoom, fit, and the Navigator.

## Requirements

### Requirement: Scrollbars project the view state

The workspace SHALL wrap the canvas in a thin container carrying a horizontal and
a vertical scrollbar. Each scrollbar's range SHALL be derived from the same range
helper that clamps the canvas pan, and its value SHALL be derived from the
canvas's current offset and zoom, so the scrollbars are a pure projection. The
canvas offset SHALL remain the single source of truth and the scrollbars SHALL
NOT store pan or zoom. Dragging a scrollbar SHALL pan the canvas, and the canvas
SHALL clamp the resulting offset through the shared range.

#### Scenario: A scrollbar drag pans the canvas [lcs_drag]

- **WHEN** the user drags the horizontal scrollbar
- **THEN** the canvas pans horizontally and the vertical scrollbar and the
  Navigator proxy reflect the new visible region

#### Scenario: Range and value round-trip [lcs_roundtrip]

- **WHEN** the canvas is panned and zoomed to a known state and the scrollbar
  value is set to a derived position
- **THEN** the canvas offset equals the clamped derived position and the
  scrollbar reads back that value

#### Scenario: The offset stays authoritative [lcs_single_source]

- **WHEN** the canvas offset is changed by a pan, zoom, fit, or the Navigator
  rather than by a scrollbar
- **THEN** the scrollbars update to match and no second stored pan state is used
  to compute the canvas transform

### Requirement: Scrollbars follow pan, zoom, fit, and the Navigator

The scrollbars SHALL update whenever the canvas view changes by any path — a pan,
a zoom, a fit or 100 % command, or a Navigator proxy drag — through a view-change
signal emitted by the canvas, so the bars never lag the actual view.

#### Scenario: Zoom updates the bars [lcs_zoom]

- **WHEN** the canvas zoom changes
- **THEN** the scrollbar ranges and visible-handle size are recomputed from the
  new zoom

#### Scenario: The Navigator and the bars agree [lcs_navigator]

- **WHEN** the user drags the Navigator's proxy rectangle
- **THEN** the scrollbars move to the corresponding range positions and the
  canvas offset matches the proxy region

### Requirement: Scrollbars are always visible

Both the horizontal and the vertical scrollbar SHALL be visible whenever the
canvas is pannable, regardless of whether the document fits the viewport, so the
canvas is freely pannable while `view.freePan` is checked. Each bar SHALL remain
a pure projection of the canvas offset and zoom through the shared range helper,
and the canvas SHALL remain the single source of truth. Hiding a bar by an
as-needed policy is removed; the bars SHALL NOT collapse to hidden when the
document fits. When `view.freePan` is unchecked and an axis fits the viewport,
that bar's range collapses to a single point, so the bar is still visible but
offers no pan range on that axis. When `view.snapCanvasToWorkspace` is checked
and the scaled document fits the viewport on both axes, the canvas is pinned and
offers no pan affordance, so the workspace SHALL hide both bars and their corner;
the bars SHALL reappear as soon as the document overflows an axis or the flag is
unchecked.

#### Scenario: Bars stay visible when the document fits [lcs_always_visible]

- **WHEN** the document is smaller than the viewport along both axes
- **THEN** both scrollbars are still shown and, with Free Pan on, the canvas can
  still be panned within the shared range

#### Scenario: Bars are visible when the document overflows

- **WHEN** the canvas is zoomed so the document is larger than the viewport along
  an axis
- **THEN** both scrollbars are shown with that axis's pan range

#### Scenario: The bars hide under the workspace snap lock [lcs_ws_snap_hidden]

- **WHEN** Snap Canvas To Workspace is on and the document fits the viewport on
  both axes
- **THEN** both scrollbars and their corner are hidden

#### Scenario: The bars return past the workspace [lcs_ws_snap_shown]

- **WHEN** the canvas is zoomed so the document overflows the viewport while Snap
  Canvas To Workspace is on
- **THEN** both scrollbars are shown again and project the pan range

### Requirement: Scrollbars project the free-pan range

The workspace scrollbars SHALL derive their range and value from the same shared
range helper that clamps the canvas pan, using the canvas's current free-pan
state, so the bars never offer a range the canvas will refuse. When Free Pan is
off and the scaled document fits an axis, that bar's range SHALL equal the
centred value on both ends and its value SHALL read zero. The bars SHALL remain
visible except while the workspace snap lock hides them (see the always-visible
requirement), and the canvas offset SHALL remain the single source of truth; the
bars SHALL NOT store their own pan state.

#### Scenario: A constrained axis collapses [lcs_free_pan_collapse]

- **WHEN** Free Pan is off and the document fits the viewport along an axis
- **THEN** that scrollbar's range is a single point and the canvas offset equals
  the centred value

#### Scenario: The bars follow the toggle [lcs_free_pan_follow]

- **WHEN** Free Pan is toggled
- **THEN** both scrollbars re-derive their range and value from the shared helper
  with no second stored pan state

### Requirement: Scrollbars move the canvas against the handle

The workspace scrollbars SHALL map their value onto the canvas offset so that the
handle travels against the content. A bar's value SHALL be the distance from the
shared range's maximum offset, so the bar's minimum shows the start of the
document and its maximum shows its end, and raising the value SHALL lower the
canvas offset on that axis. The bar's range SHALL remain the range the shared
range helper derives, so every position the canvas can be panned to — the
reveal margin beyond the document included — has a handle position, and the
handle SHALL rest at the end of its track only when the canvas has reached the
pannable limit. The canvas offset SHALL keep its meaning as the document's
top-left in widget space; the pan, the zoom, the rulers and the Navigator SHALL
be unaffected.

#### Scenario: The handle travels against the content [lcs_direction]

- **WHEN** the user raises a scrollbar's value
- **THEN** the canvas offset on that axis falls, so the content travels against
  the handle

#### Scenario: The bar's ends are the document's ends [lcs_ends]

- **WHEN** the user reads a scrollbar at its minimum and at its maximum
- **THEN** the canvas offset is the shared range's maximum at the bar's minimum,
  so the start of the document is visible, and the shared range's minimum at the
  bar's maximum, so the end of the document is visible

#### Scenario: The bar covers the reveal margin [lcs_margin]

- **WHEN** the canvas is panned into the reveal margin beyond the document
- **THEN** the scrollbar's value moves with the canvas and reaches the end of
  its range only when the canvas reaches the pannable limit
