# Proposal

## Why

Two CS6 parity gaps in the Crop tool:

- The options bar carries a disabled `Content-Aware` checkbox. Content-Aware
  crop arrived in Photoshop CC 2015.5; CS6's bar ends at `Delete Cropped
  Pixels` (#105 asked to implement it, but it is not a CS6 feature).
- Choosing the tool shows no box in Classic mode (the default) and a dashed box
  without handles in Modern. CS6 frames the whole image with its corner and
  edge handles as soon as the tool is chosen, ready to drag.

## What Changes

- Remove the `Content-Aware` checkbox.
- Both modes start with the centered, ratio-fitted preview box, drawn with its
  eight handles (no rule-of-thirds guides until the box is active).
- A handle drag on the preview resizes it and makes it the active crop; a drag
  inside still draws a new box, a click still adopts it, and Escape still
  clears to no box.
- A Crop tool commit's History state names its guide overlay
  (`Crop: Rule of Thirds`, `Crop: Grid`, ...); `Image > Crop` stays `Crop`.
- Rotating the box shows a dense rotate grid in place of the guide overlay and
  an `∠: <angle>°` readout beside the pointer, both gone on release.
- Each overlay in the grid menu has its own icon (six custom Lucide-style
  icons replace the generic `crop.grid`), and the button shows the selected one.
- **BREAKING**: none.

## Capabilities

- Modified `tools/tool-framework` (Crop options-bar controls).
- New `tools/crop-overlay-feedback`.
- Modified `tools/canvas-tools` (Crop tool: the History name).
- Modified `tools/crop-straighten` (Crop preview and active states; Straighten
  from the init state).
