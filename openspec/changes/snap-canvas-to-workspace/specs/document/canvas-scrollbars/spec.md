## MODIFIED Requirements

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
