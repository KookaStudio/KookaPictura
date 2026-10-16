# Proposal: red-eye-tool

## Why

The Red Eye tool (issue #14) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `healing::red_eye_region` behind
`Document::remove_red_eye`. This change ports it onto Kooka's healing layer
wrapper and tool framework, completing the `J` group.

## What Changes

- `pictura_paint::healing::red_eye_layer`: inside a document box, pixels where
  red dominates green and blue (Pupil Size widens the ratio) take the
  green/blue level and are darkened by Darken Amount; other pixels are left
  alone.
- `cxxqt_object/healing.rs`: `red_eye`, one `"Red Eye Tool"` history state.
- `tool_redeye.cpp`: drag a box over the eye (a click gets a 24 px box).
- Options-bar row: Pupil Size and Darken Amount (default 50 / 50). Catalog row
  enabled; the `J` cycle ends Content-Aware Move → Red Eye.
- C++ self-test `red_eye_tool` (539); `shift_plain` (117) asserts the J cycle;
  the unimplemented-tool guard (98) now probes Clone Stamp.

## Capabilities

### New Capabilities

- `tools/red-eye-tool`: the Red Eye tool and its engine entry point.

## Impact

- `pictura-paint` (`healing/red_eye.rs`), `pictura-app` bridge and C++ as above.
- No new dependency.

## Provenance

Ported from photorust's `core/src/healing.rs` (`red_eye_region`),
`core/src/document.rs` (`remove_red_eye`), and `shell/src/canvas/CanvasView.cpp`
(the gesture) (<https://github.com/perfecto25/photorust>). Behavioural parity
only: CS6's red-eye detector is closed.
