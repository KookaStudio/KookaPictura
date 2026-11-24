#include "api/selection_api.h"

#include "api/selection_bridge.h"

#include <rust/cxx.h>

#include <QtCore/QtMath>

#include <cstdint>

namespace pictura::selection_api {

QString SelectionBounds::toString() const
{
    if (!valid) {
        return QString();
    }
    return QStringLiteral("%1 %2 %3 %4").arg(x).arg(y).arg(width).arg(height);
}

bool SelectionContour::hasVertex(int x, int y) const
{
    for (const QPolygonF& loop : loops) {
        for (const QPointF& point : loop) {
            if (qRound(point.x()) == x && qRound(point.y()) == y) {
                return true;
            }
        }
    }
    return false;
}

QRect marqueeDragRect(const QPointF& a, const QPointF& b, Qt::KeyboardModifiers mods,
                      bool mirror, MarqueeStyle style, double fixedRatioW, double fixedRatioH,
                      int fixedSizeW, int fixedSizeH)
{
    const ::rust::Vec<std::int32_t> rect = pictura::selection_marquee_rect(
        a.x(), a.y(), b.x(), b.y(), mods.testFlag(Qt::ShiftModifier), mirror,
        static_cast<int>(style), fixedRatioW, fixedRatioH, fixedSizeW, fixedSizeH);
    if (rect.size() != 4) {
        return QRect();
    }
    return QRect(rect[0], rect[1], rect[2], rect[3]);
}

bool withinRadius(const QPointF& a, const QPointF& b, double radius)
{
    return pictura::selection_within_radius(a.x(), a.y(), b.x(), b.y(), radius);
}

QList<QPolygonF> clipSelectionShapeToCanvas(const QList<QPolygonF>& loops, const QSize& canvas)
{
    ::rust::Vec<std::uint32_t> lengths;
    ::rust::Vec<double> points;
    lengths.reserve(static_cast<std::size_t>(loops.size()));
    for (const QPolygonF& loop : loops) {
        lengths.push_back(static_cast<std::uint32_t>(loop.size()));
        for (const QPointF& point : loop) {
            points.push_back(point.x());
            points.push_back(point.y());
        }
    }
    const ::rust::Slice<const std::uint32_t> length_slice(lengths.data(), lengths.size());
    const ::rust::Slice<const double> point_slice(points.data(), points.size());
    const pictura::SelectionClipResult result = pictura::selection_clip_shape_to_canvas(
        length_slice, point_slice, double(canvas.width()), double(canvas.height()));
    if (!result.valid || result.points.size() % 2 != 0) {
        return {};
    }

    QList<QPolygonF> out;
    out.reserve(static_cast<qsizetype>(result.loop_lengths.size()));
    std::size_t point_index = 0;
    for (const std::uint32_t length : result.loop_lengths) {
        if (length > (result.points.size() - point_index) / 2) {
            return {};
        }
        QPolygonF loop;
        loop.reserve(static_cast<qsizetype>(length));
        for (std::uint32_t index = 0; index < length; ++index) {
            loop << QPointF(result.points[point_index], result.points[point_index + 1]);
            point_index += 2;
        }
        out.append(loop);
    }
    return point_index == result.points.size() ? out : QList<QPolygonF>{};
}

SelectionBounds bounds(const PictureView& view)
{
    const ::rust::Vec<int32_t> extents = view.selection_bounds_extents();
    if (extents.size() != 4) {
        return SelectionBounds{};
    }
    return SelectionBounds{int(extents[0]), int(extents[1]), int(extents[2]),
                           int(extents[3]), true};
}

SelectionContour contour(const PictureView& view)
{
    const ::rust::Vec<int32_t> extents = view.selection_contour_extents();
    SelectionContour out;
    out.revision = view.selection_revision();
    size_t i = 0;
    while (i < extents.size()) {
        const int count = int(extents[i++]);
        if (count < 0) {
            break;
        }
        QPolygonF loop;
        loop.reserve(count);
        for (int k = 0; k < count; ++k) {
            if (i + 1 >= extents.size()) {
                break;
            }
            loop << QPointF(double(extents[i]), double(extents[i + 1]));
            i += 2;
        }
        if (loop.size() >= 2) {
            out.loops.append(loop);
        }
    }
    return out;
}

} // namespace pictura::selection_api
