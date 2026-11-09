#pragma once

#include "tools.h"

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/Qt>
#include <QtGui/QPolygonF>

namespace pictura {

// Screen-space radius for the Polygonal and Magnetic Lasso close-click and
// double-click tests. The document-space radius is kPolygonCloseRadius / zoom.
constexpr double kPolygonCloseRadius = 6.0;

// Style constrains the drag geometry before rasterisation: Normal follows the
// drag (Shift squares, `mirror` centres about the press point); Fixed Ratio
// keeps the entered width:height; Fixed Size puts its top-left at the pointer.
// The math lives in `pictura-select`; this is the Qt adapter.
QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH);

// Whether `b` lies within `radius` document pixels of `a`, boundary included.
bool within_radius(const QPointF& a, const QPointF& b, double radius);

/// The closed selection shapes intersected with the canvas rectangle, closed
/// along its border: a shape that leaves the canvas is drawn as the part of it
/// inside the canvas, so the live outline and the committed one agree at the
/// edge. A loop with fewer than three points (a trace that has not closed yet)
/// and a loop whose part inside the canvas encloses nothing (a stroke that
/// doubles back on itself) are returned unchanged; a loop wholly outside the
/// canvas is dropped.
QList<QPolygonF> clipSelectionShapeToCanvas(const QList<QPolygonF>& loops, const QSize& canvas);

} // namespace pictura
