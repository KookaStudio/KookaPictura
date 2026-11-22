# Proposal

Tracking issue: #1533.

## Why

A layer row elides its name twice: `nameRect` elides the name to the available
label band and reports the elided text's width, then `paint()` elides the full
name again to that width. Qt's `QFontMetrics::elidedText` treats text whose
advance equals the available width as not fitting, so the second pass drops the
tail of a name that already fit and appends an ellipsis. Short names such as
`r1 r2`, `f2-1`, or `f2 x` render as `r1 …`, `f2…`, or `f…` while the row has
plenty of space to spare.

## What Changes

- `LayerRowDelegate::nameRect` reports the elided text through an optional
  out-parameter, so paint can draw that exact string.
- The regular and synthetic row paint paths draw the reported text instead of
  re-eliding it to `nameRect`'s width.
- A Qt Test asserts a fitting name paints in full and an overlong name still
  elides once with a trailing ellipsis.

## Capabilities

### Added Requirements

- `ui/layers-panel`: a layer name paints with a single elision, so a name that
  fits is shown in full.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h`: `nameRect` and the
  two paint call sites.
- `crates/pictura-app/cpp/panels/layers_panel.h`,
  `crates/pictura-app/cpp/panels/layers_panel_test.cpp`: a rendered-name test
  seam.
- `crates/pictura-app/cpp/tests/tst_layers_panel_bundle.cpp`: coverage.
