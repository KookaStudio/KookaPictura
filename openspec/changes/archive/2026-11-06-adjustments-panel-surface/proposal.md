# Proposal

## Why

Issue #1205: the Adjustments panel's background is darker than the other
panels in the right column. `AdjustmentsPanel` wraps its icon grid in a
`QScrollArea`; the viewport defaults to `QPalette::Base` (`${surface-field}`,
`#404040` at the default level), while the panel column and the other panels
paint `${surface}` (`#4d4d4d`). Nothing in the theme targets the scroll
viewport or the unnamed grid host, so the whole panel reads as a recessed
field instead of a panel surface.

## What Changes

- The Adjustments panel's scroll area, its viewport, and its grid host gain
  object names so the theme can address them.
- The theme paints all three with `${surface}`, the same token the Styles panel
  and the panel column use.
- A Qt test renders the real window with the Adjustments tab active and with
  the Styles tab active, samples both panel backgrounds, and asserts they are
  the same `${surface}` colour; a second check re-applies the theme at another
  brightness level and asserts the panel follows the new token.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/adjustments-panel`: adds an "Adjustments panel surface" requirement with
  two scenarios (the surface matches the other panels, and it follows the
  active theme).

## Impact

- `crates/pictura-app/cpp/panels/adjustments_panel.cpp` (object names on the
  scroll area, viewport, and grid host).
- `crates/pictura-app/cpp/theme.cpp` (three `#adjustments*` background rules).
- `crates/pictura-app/cpp/tests/tst_adjustments_panel.cpp` (two Qt tests).
- No engine, document, codec, dependency, or `docs/` change.
