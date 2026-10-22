#include "selection_geometry.h"

#include <QtGui/QPainterPath>

#include <algorithm>
#include <cmath>

namespace pictura {

QRect dragRect(const QPointF& a, const QPointF& b)
{
    const QRectF rect = QRectF(a, b).normalized();
    return QRect(qRound(rect.left()), qRound(rect.top()), qRound(rect.width()),
                 qRound(rect.height()));
}

QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH)
{
    if (style == MarqueeStyle::FixedSize) {
        const int w = fixedSizeW;
        const int h = fixedSizeH;
        return QRect(qRound(a.x()) - w / 2, qRound(a.y()) - h / 2, w, h);
    }
    if (style == MarqueeStyle::FixedRatio) {
        const double ratio = fixedRatioW / fixedRatioH;
        const double dx = b.x() - a.x();
        const double dy = b.y() - a.y();
        double w = std::abs(dx);
        double h = std::abs(dy);
        if (ratio > 0.0) {
            if (h <= 0.0 || w / ratio >= h) {
                h = w / ratio;
            } else {
                w = h * ratio;
            }
        }
        const int left = dx >= 0.0 ? qRound(a.x()) : qRound(a.x() - w);
        const int top = dy >= 0.0 ? qRound(a.y()) : qRound(a.y() - h);
        return QRect(left, top, qRound(w), qRound(h));
    }

    double dx = b.x() - a.x();
    double dy = b.y() - a.y();
    if (mods.testFlag(Qt::ShiftModifier)) {
        const double m = std::max(std::abs(dx), std::abs(dy));
        dx = std::copysign(m, dx);
        dy = std::copysign(m, dy);
    }
    const QPointF end = a + QPointF(dx, dy);
    if (mirror) {
        return QRect(qRound(a.x() - std::abs(dx)), qRound(a.y() - std::abs(dy)),
                     qRound(2 * std::abs(dx)), qRound(2 * std::abs(dy)));
    }
    return dragRect(a, end);
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
