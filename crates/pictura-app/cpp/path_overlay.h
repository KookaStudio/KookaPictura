#pragma once

#include "image_view.h"

namespace pictura {

class PictureView;

// The Work Path as a canvas overlay: every subpath's curve, plus the anchors
// and handle lines of subpath `pointsOf`: every subpath's when it is -1, none
// when it names no subpath.
// Defined in tool_pen.cpp.
ImageView::PathOverlay workPathOverlay(const PictureView& v, int pointsOf);

// Draw the Work Path on `canvas` while a non-Path tool is active, or clear it
// when `view` has no document. Used by the Type tools, which set type on it.
void showWorkPathOverlay(ImageView& canvas, PictureView* view);

} // namespace pictura
