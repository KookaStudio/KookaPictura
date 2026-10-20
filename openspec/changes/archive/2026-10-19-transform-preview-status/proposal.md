# Transform preview appearance and action status

## Why

During a Free Transform / Skew session the live preview drew only the target
layer's raw channels. A layer whose visible color comes from a layer style, a
layer mask, a vector mask, a fill adjustment, or editable type lost that
appearance while dragging and only got it back on commit (#747 follow-up).

The status bar also had no readout naming the transform that is in progress, so
the user had to infer the active mode from the options bar.

## What Changes

- When the target's raw channels are not its whole appearance, render the
  transform preview layer through the compositor (one scratch composite at
  session begin), with the layer's opacity and blend neutralized and the result
  cropped to its rect. The preview still applies the layer opacity once.
- Add a status-bar action readout that names an active transform session:
  `Free Transform in progress`, `Skew in progress`, `Distort in progress`, or
  `Perspective in progress`. It clears when the session ends.

## Impact

- Capabilities: `document/free-transform`, `ui/application-shell` (MODIFIED).
- Code: `impl_transform/session.rs`, `frame.cpp`, `frame_build.cpp`,
  `frame.h`, `frame_includes.h`.
- Tests: `tst_transform_skew` gains the styled-layer preview and status-readout
  cases.
