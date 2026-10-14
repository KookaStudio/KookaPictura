#pragma once

namespace pictura {
class PicturaMainWindow;

// crop_tool (534): choosing Crop places a canvas-sized box (not a pending crop,
// so Image > Crop is not captured); dragging the bottom-right handle resizes
// it, dragging inside moves it, Escape and the options bar's Cancel reset it,
// a 1:1 ratio locks a new box square; Enter crops to the box as one "Crop: <overlay>"
// state, trimming the layer to the canvas with Delete Cropped Pixels on and
// keeping its off-canvas pixels with it off; a double-click inside also
// commits.
int runCropToolChecks(PicturaMainWindow& frame);
} // namespace pictura
