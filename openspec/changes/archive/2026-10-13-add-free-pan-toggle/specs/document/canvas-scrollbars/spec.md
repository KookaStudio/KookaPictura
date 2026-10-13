## MODIFIED Requirements

### Requirement: Scrollbars are always visible

Both the horizontal and the vertical scrollbar SHALL be visible at all times,
regardless of whether the document fits the viewport, so the canvas is freely
pannable while `view.freePan` is checked. Each bar SHALL remain a pure projection
of the canvas offset and zoom through the shared range helper, and the canvas
SHALL remain the single source of truth. Hiding a bar by an as-needed policy is
removed; the bars SHALL NOT collapse to hidden when the document fits. When
`view.freePan` is unchecked and an axis fits the viewport, that bar's range
collapses to a single point, so the bar is still visible but offers no pan range
on that axis.

#### Scenario: Bars stay visible when the document fits [lcs_always_visible]

- **WHEN** the document is smaller than the viewport along both axes
- **THEN** both scrollbars are still shown and, with Free Pan on, the canvas can
  still be panned within the shared range

#### Scenario: Bars are visible when the document overflows

- **WHEN** the canvas is zoomed so the document is larger than the viewport along
  an axis
- **THEN** both scrollbars are shown with that axis's pan range

## ADDED Requirements

### Requirement: Scrollbars project the free-pan range

The workspace scrollbars SHALL derive their range and value from the same shared
range helper that clamps the canvas pan, using the canvas's current free-pan
state, so the bars never offer a range the canvas will refuse. When Free Pan is
off and the scaled document fits an axis, that bar's range SHALL equal the
centred value on both ends and its value SHALL read zero. The bars SHALL remain
visible and the canvas offset SHALL remain the single source of truth; the bars
SHALL NOT store their own pan state.

#### Scenario: A constrained axis collapses [lcs_free_pan_collapse]

- **WHEN** Free Pan is off and the document fits the viewport along an axis
- **THEN** that scrollbar's range is a single point and the canvas offset equals
  the centred value

#### Scenario: The bars follow the toggle [lcs_free_pan_follow]

- **WHEN** Free Pan is toggled
- **THEN** both scrollbars re-derive their range and value from the shared helper
  with no second stored pan state
