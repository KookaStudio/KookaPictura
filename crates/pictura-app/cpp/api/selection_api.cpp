#include "api/selection_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <rust/cxx.h>

#include <QtCore/QtMath>

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
