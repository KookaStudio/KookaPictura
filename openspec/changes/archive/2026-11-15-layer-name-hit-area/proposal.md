# Proposal

Tracking issue: #896.

## Why

The layer row's name hit rect spans from the thumbnail block to the right-edge
badges, so a double-click in empty space to the right of a short layer name
starts inline rename. Photoshop CS6 treats only the painted label as the rename
target; the rest of the row opens Layer Style.

## What Changes

- `LayerRowDelegate::nameRect` becomes the rendered label rect: the elided name
  text width inside the available label band, floored to one pixel.
- `paint()` draws the name through that same rect, so hit-testing and rendering
  cannot drift.
- Double-clicking the row outside the rendered label reaches the existing Layer
  Style branch instead of `tree_->edit()`.
- A Qt Test covers a short name: inside the label opens the editor; to the right
  of it opens Layer Style and opens no editor.

## Capabilities

### Modified Capabilities

- `ui/layers-panel`: inline rename starts only on the rendered name label;
  double-clicking elsewhere in the row invokes the layer-style affordance.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h`: the shared label rect
  and paint.
- `crates/pictura-app/cpp/panels/layers_panel_events.cpp`: behavior follows the
  narrower rect; no code change expected.
- `crates/pictura-app/cpp/panels/layers_panel.h`,
  `layers_panel_test.cpp`, and `layers_panel_test_extra.cpp`: a test seam for a
  double-click to the right of the label.
- `crates/pictura-app/cpp/tests/tst_layers_panel_bundle.cpp`: coverage.
- `crates/pictura-app/cpp/selftest_layers_round3.cpp`: wording only.
