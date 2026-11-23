# Proposal

Tracking issue: #1545.

## Why

Renaming a layer in the Layers panel replaces the whole row with a text input,
covering the lock, fx, filter, and list-toggle icons while the edit is open.
#896 / PR #1375 narrowed the double-click target to the rendered label, but the
inline editor still takes the delegate's default geometry, which is the entire
row.

## What Changes

- `LayerRowDelegate` gains a shared `nameBandRect` helper: the label band from
  the rendered label's left edge to the right-edge badge caps.
- `nameRect` uses that band for its elision, so paint, hit-testing, and the
  editor cannot drift.
- `LayerRowDelegate::updateEditorGeometry` sizes the inline rename editor to the
  rendered label plus the name gap, capped at the label band, instead of the
  full row. A short name no longer stretches across the panel, and the trailing
  icons keep their positions and stay visible.
- Qt Tests cover a short styled row, a plain row, a locked row, and a name that
  fills the band.

## Capabilities

### Added Requirements

- `ui/layers-panel`: the inline rename editor hugs the rendered label and stops
  before the row's trailing icons.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h`: the band helper, the
  name rect, and the editor geometry declaration.
- `crates/pictura-app/cpp/panels/layers_panel_events.cpp`: the shared band and
  the editor geometry.
- `crates/pictura-app/cpp/tests/tst_layers_panel_bundle.cpp`: coverage.
