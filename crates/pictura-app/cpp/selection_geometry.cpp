#include "selection_geometry.h"

#include "api/selection_bridge.h"

#include <QtGui/QPainterPath>

#include <cstdint>
#include <rust/cxx.h>

namespace pictura {

QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH)
{
    const ::rust::Vec<::std::int32_t> rect = selection_marquee_rect(
        a.x(), a.y(), b.x(), b.y(), mods.testFlag(Qt::ShiftModifier), mirror,
        static_cast<int>(style), fixedRatioW, fixedRatioH, fixedSizeW, fixedSizeH);
    if (rect.size() != 4) {
        return QRect();
    }
    return QRect(rect[0], rect[1], rect[2], rect[3]);
}

bool within_radius(const QPointF& a, const QPointF& b, double radius)
{
    return selection_within_radius(a.x(), a.y(), b.x(), b.y(), radius);
}

QList<QPolygonF> clipSelectionShapeToCanvas(const QList<QPolygonF>& loops, const QSize& canvas)
{
    QList<QPolygonF> out;
    if (canvas.isEmpty()) {
        return loops;
    }
    const QRectF area(0.0, 0.0, canvas.width(), canvas.height());
    for (const QPolygonF& loop : loops) {
        if (loop.size() < 3) {
            // A trace that has not closed yet: there is no shape to close.
            out << loop;
            continue;
        }
        const QRectF bounds = loop.boundingRect();
        // An explicit comparison, not QRectF::contains: contains is false for a
        // null rectangle, which a one-point trace has.
        if (bounds.left() >= area.left() && bounds.right() <= area.right()
            && bounds.top() >= area.top() && bounds.bottom() <= area.bottom()) {
            out << loop;
            continue;
        }
        // The same for the outside test: QRectF::intersects is false for a
        // flat bounding box, which a horizontal stroke has.
        if (bounds.right() < area.left() || bounds.left() > area.right()
            || bounds.bottom() < area.top() || bounds.top() > area.bottom()) {
            continue;
        }
        QPainterPath shape;
        shape.addPolygon(loop);
        QPainterPath areaPath;
        areaPath.addRect(area);
        // The intersection closes the shape along the canvas border, which is
        // what the committed outline, traced from the clipped mask, does.
        const QList<QPolygonF> parts = shape.intersected(areaPath).toSubpathPolygons();
        if (parts.isEmpty()) {
            // The shape encloses nothing inside the canvas, which a stroke
            // that doubles back on itself does: keep the trace, which the
            // paint crops at the border, rather than dropping the band.
            out << loop;
            continue;
        }
        for (const QPolygonF& part : parts) {
            if (part.size() >= 2) {
                out << part;
            }
        }
    }
    return out;
}

} // namespace pictura
