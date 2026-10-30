# Proposal

## Why

The Eyedropper samples only on a click (#1040). CS6 samples continuously:
"press the mouse button over the image, drag to the target, release"
(`docs/03-tools/eyedropper-color-sampler-ruler.md`). The Color Picker dialog's
eyedropper also does nothing from the toolbox or Color panel swatches: those
entry points call `ColorPickerDialog::getColor` without a `ToolController` and
a `colorAt`, so `sampleCanvas` is never armed. With no sampler armed,
`DialogInputBlocker` swallows every left-button event over the canvas, so the
pointer never becomes the eyedropper and a click never samples
(`docs/07-color-painting/color-picker.md`, parity criterion 8). Only the Photo
Filter dialog passes a sampler today.

## What Changes

- `ToolController` samples an armed canvas sampler again on every mouse move
  while the left button is held, so the Color Picker, Replace Color, and Color
  Range eyedroppers follow the pointer through a drag.
- The Eyedropper tool (`ToolId::Eyedropper`) samples on move while its press
  gesture is live, so the foreground colour follows a drag.
- `ColorPickerDialog` gains a frame-installed default canvas sampler. A picker
  opened without an explicit `tools`/`colorAt` (the toolbox and Color panel
  swatches, options bars, and filter dialogs) then arms the eyedropper and
  samples the active document's composite, as CS6's picker does. A picker
  opened with an explicit sampler (Photo Filter) keeps it, and a picker opened
  while another dialog's sampler is already armed leaves that sampler alone.
- Sampling reads the maintained composite (`PictureView::sample_argb`), so a
  drag sample is O(1) and does not recomposite the document.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/canvas-tools`: the Eyedropper samples continuously while the button is
  held.
- `ui/color-picker-drag`: the Color Picker's eyedropper samples the canvas on
  click and drag from every entry point.

## Impact

- `crates/pictura-app/cpp/tools.{h,cpp}`: the sampler drag state.
- `crates/pictura-app/cpp/tool_eyedropper.cpp`: drag sampling.
- `crates/pictura-app/cpp/color_picker_dialog.{h,cpp}`: the default sampler.
- `crates/pictura-app/cpp/frame_build.cpp`: install the default sampler.
- New `crates/pictura-app/cpp/tests/tst_eyedropper.cpp`; one new
  `tst_image_adjustments` case; `cpp/tests/CMakeLists.txt` registration.
- `ROADMAP.md`: tick the shipped behaviour.
- No new dependency, no `docs/` change.
