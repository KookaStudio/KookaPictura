# Proposal

## Why

The Layers panel's bottom action strip spreads its seven buttons apart when the
panel is widened, instead of keeping a fixed, even gap and staying centred. The
row is a `QHBoxLayout` with zero spacing and no stretch items. `QToolButton` is
already `Fixed`/`Fixed`, so the buttons keep their widths; with nothing else to
grow, Qt's layout engine distributes the panel's surplus width into the spacers:
the leading and trailing edges and the six inter-button gaps. This is the
manual-pass defect recorded in #439.

## What Changes

- Give the action strip a constant 2 px inter-button gap, matching the other
  compact icon strips (the panel column and the toolbox footer).
- Bracket the strip with equal stretches so the leading and trailing space
  absorbs the panel's surplus width and the group stays horizontally centred.
- Leave the buttons' size policies alone: `QToolButton` already uses
  `QSizePolicy::Fixed` on both axes.
- Add a Qt Test that resizes the panel and asserts the gaps and button widths do
  not change and the row remains centred.
- Add one `ui/layers-panel` requirement for the strip's geometry.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: adds a requirement for the action strip's fixed-gap,
  centred layout as the panel width changes.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel.cpp` — the strip's layout
  construction only; object names, icons, labels, click handlers,
  `layerDropAction` properties, and drop event filters stay unchanged.
- `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` — new Qt Test case.
  This file is chosen over `tst_layers_panel.cpp`, which is already at 1376 of
  its 1400-line test cap.
- No bridge, API, dependency, or data-format changes. The strip's contents and
  behaviour are unchanged; only the row geometry changes.
- No `design.md`: this is a single-widget layout change with no cross-cutting,
  dependency, or migration decision to record.
- The test checks constant 2 px gaps and each button's unchanged width rather
  than a constant x-step, so it does not assume the Fill/Adjustment button has
  the same size hint as the other six.
