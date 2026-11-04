# Proposal

## Why

Issue #1151. On a 16507×16196 document, the debounced panel refresh
(`cxx_refreshPanels`, 120 ms after a change) was measured at 230–490 ms. The
issue suspected the Histogram and Navigator of reading level 0. They do not:
both already read the coarsest view-pyramid level, and every thumbnail samples
O(thumbnail) pixels.

A per-panel timing pass (optimized build, `scripts/profile-tools.py --size
16507x16196`, brush, paint bucket, marquee, new layer, duplicate, shape) placed
the time:

| Debounced refresh | Before |
|---|---|
| First run after open | 527 ms, of which `registerTypeFallbacks` 525 ms |
| Dock retarget (all panels) | 7–25 ms |
| Every later run | 2–3 ms |

- **`registerTypeFallbacks` ran at the top of every refresh.** Its once-only
  body opens a `QRawFont` for every installed family for seven scripts, to
  find Missing Glyph Protection fallback faces. Its cost does not depend on
  document size, and a session that never sets type paid it on its first
  edit.
- **The Agent Control command's enabled provider searched the whole widget
  tree** (`findChild` is recursive) on every command refresh: 188 ms the
  first time and 15–19 ms after, in the `changed` refresh.

## What Changes

- **The fallback scan waits until type needs it.** The debounced refresh
  registers the fallbacks only when the active document holds a type layer,
  and every `type_api` call that lays out text registers them first. A
  session without type never scans the installed fonts.
- **The Agent Control provider checks only the frame's direct children**,
  where `AgentControl` lives.
- **Each panel's refresh and retarget is timed by name** under
  `PICTURA_PAINT_TIMING`, and the debounced refresh is a `panels_refresh`
  `PICTURA_METRICS` phase.

## Capabilities

### New Capabilities

- `ui/panel-refresh-responsiveness`

### Modified Capabilities

None.

## Impact

`pictura-render` (`has_type_layers`), `pictura-app` (`type_document_has_type`
bridge, `type_api`, `type_fonts`, `frame_session`, `frame_preferences`,
`control_server_type`).
