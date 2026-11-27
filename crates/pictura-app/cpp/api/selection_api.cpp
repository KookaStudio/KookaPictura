#include "api/selection_api.h"

#include "api/selection_bridge.h"

#include <rust/cxx.h>

#include <QtCore/QtMath>

#include <algorithm>
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

static QList<QPolygonF> decodeContour(const ::rust::Vec<int32_t>& extents)
{
    QList<QPolygonF> out;
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
            out.append(loop);
        }
    }
    return out;
}

SelectionContour contour(const PictureView& view)
{
    return {decodeContour(view.selection_contour_extents()), view.selection_revision()};
}

static QPainterPath contourPath(const QList<QPolygonF>& loops)
{
    QPainterPath path;
    for (const QPolygonF& loop : loops) {
        path.addPolygon(loop);
        path.closeSubpath();
    }
    return path;
}

QPainterPath committedContourPath(const PictureView& view)
{
    return contourPath(contour(view).loops);
}

QList<QPolygonF> rectPreviewContour(const PictureView& view, const QRect& rect,
                                   const QString& mode, double feather,
                                   const QPainterPath& committed)
{
    if (feather <= 0 && mode == QStringLiteral("new")) {
        return {QPolygonF(QRectF(rect))};
    }
    const auto preview = selection_rect_preview_contour(
        view, rect.x(), rect.y(), rect.width(), rect.height(), mode, feather);
    const auto loops = decodeContour(preview.contour);
    if (preview.width <= 0 || preview.height <= 0 || committed.isEmpty()) {
        return loops;
    }
    QPainterPath window;
    window.addRect(QRectF(preview.left, preview.top, preview.width, preview.height));
    return committed.subtracted(window).united(contourPath(loops)).toSubpathPolygons();
}

QRect singleLineBand(const QSize& size, const QPointF& pos, bool row)
{
    if (size.isEmpty()) {
        return {};
    }
    const int limit = row ? size.height() : size.width();
    const int index = qFloor(std::clamp(row ? pos.y() : pos.x(), 0.0, double(limit - 1)));
    return row ? QRect(0, index, size.width(), 1) : QRect(index, 0, 1, size.height());
}

} // namespace pictura::selection_api
