# ui/layers-panel

## ADDED Requirements

### Requirement: Inline rename editor hugs the rendered label

When an inline rename starts, the system SHALL size and position the editor to
the row's rendered label area: the label's left edge (after the visibility
gutter, disclosure slot, thumbnail block, name gap, and clipping indent) through
the rendered name text plus the name gap, capped at the label band's right edge
(the space left by the lock, fx, link, Blend If, and list-toggle badges actually
present). The editor SHALL NOT stretch across the empty remainder of the label
band, and SHALL NOT cover the row's trailing icons, which SHALL keep their
painted positions and stay visible while the edit is open. The editor SHALL use
the same rendered label geometry the row paints, so paint, the double-click hit
target, and the editor geometry cannot drift.

#### Scenario: The rename editor hugs the label and stops before the icons [lpr_rename_editor_bounds]

- **WHEN** the user starts an inline rename on a styled layer row whose name is
  shorter than the label band
- **THEN** the editor starts at the rendered label's left edge, is only as wide
  as the rendered label plus the name gap, and stops before the fx and
  list-toggle badges, which remain visible

#### Scenario: A name that fills the band caps the editor at the badges [lpr_rename_editor_band_cap]

- **WHEN** the user starts an inline rename on a layer whose rendered name fills
  the label band
- **THEN** the editor covers the rendered label and stops at the band's right
  cap, before any trailing badges
