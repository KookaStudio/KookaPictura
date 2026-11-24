#include "selection_geometry.h"

#include "api/selection_api.h"

#include <cstdint>

namespace pictura {

QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH)
{
    return selection_api::marqueeDragRect(a, b, mods, mirror, style, fixedRatioW, fixedRatioH,
                                          fixedSizeW, fixedSizeH);
}

bool within_radius(const QPointF& a, const QPointF& b, double radius)
{
    return selection_api::withinRadius(a, b, radius);
}

QList<QPolygonF> clipSelectionShapeToCanvas(const QList<QPolygonF>& loops, const QSize& canvas)
{
    return selection_api::clipSelectionShapeToCanvas(loops, canvas);
}

} // namespace pictura
