#pragma once

#include "tools.h"

#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/Qt>

namespace pictura {

// Document-space radius for the Polygonal Lasso close-click and for treating a
// second rapid press as a double-click. Document pixels, so it shrinks visually
// when zoomed out.
constexpr double kPolygonCloseRadius = 6.0;

// The normalized rectangle spanned by two image points.
QRect dragRect(const QPointF& a, const QPointF& b);

// Style constrains the drag geometry before rasterisation: Normal follows the
// drag (Shift squares, `mirror` centres about the press point); Fixed Ratio
// keeps the entered width:height; Fixed Size is centred on the mousedown.
QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH);

} // namespace pictura
