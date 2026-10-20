#include "api/transform_api.h"

#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/warp.cxxqt.h"

#include <rust/cxx.h>

#include <QtCore/QStringList>

namespace pictura::transform_api {

namespace {

Matrix matrixFromExtents(const ::rust::Vec<double>& extents)
{
    Matrix out;
    if (extents.size() != 9) {
        return out;
    }
    for (size_t i = 0; i < 9; ++i) {
        out.coefficients[i] = extents[i];
    }
    out.valid = true;
    return out;
}

// Points per grid curve in the warp bridge's flattened lines.
constexpr size_t kCurvePoints = 41;

QList<QPolygonF> curvesFromExtents(const ::rust::Vec<double>& extents)
{
    QList<QPolygonF> out;
    const size_t stride = 2 * kCurvePoints;
    for (size_t base = 0; base + stride <= extents.size(); base += stride) {
        QPolygonF curve;
        for (size_t i = base; i < base + stride; i += 2) {
            curve << QPointF(extents[i], extents[i + 1]);
        }
        out << curve;
    }
    return out;
}

} // namespace

QString LayerRect::toString() const
{
    if (!valid) {
        return QString();
    }
    return QStringLiteral("%1 %2 %3 %4").arg(left).arg(top).arg(right).arg(bottom);
}

QString Quad::toString() const
{
    QStringList parts;
    for (const QPointF& corner : corners) {
        parts << QStringLiteral("%1,%2")
                     .arg(corner.x(), 0, 'f', 2)
                     .arg(corner.y(), 0, 'f', 2);
    }
    return parts.join(QLatin1Char(' '));
}

LayerRect layerRect(const PictureView& view, const QString& path)
{
    const ::rust::Vec<int32_t> extents = layer_rect_extents(view, path);
    if (extents.size() != 4) {
        return LayerRect{};
    }
    return LayerRect{int(extents[0]), int(extents[1]), int(extents[2]), int(extents[3]), true};
}

LayerRect contentRect(const PictureView& view, const QString& path)
{
    const ::rust::Vec<int32_t> extents = layer_content_bounds_extents(view, path);
    if (extents.size() != 4) {
        return LayerRect{};
    }
    return LayerRect{int(extents[0]), int(extents[1]), int(extents[2]), int(extents[3]), true};
}

Quad quad(const PictureView& view)
{
    const ::rust::Vec<double> extents = transform_quad_extents(view);
    Quad out;
    for (size_t i = 0; i + 1 < extents.size(); i += 2) {
        out.corners << QPointF(extents[i], extents[i + 1]);
    }
    return out;
}

Matrix previewMatrix(const PictureView& view)
{
    return matrixFromExtents(transform_preview_matrix_extents(view));
}

Matrix previewLocalMatrix(const PictureView& view)
{
    return matrixFromExtents(transform_preview_local_matrix_extents(view));
}

WarpOverlay warpOverlay(const PictureView& view)
{
    WarpOverlay out;
    out.grid = curvesFromExtents(transform_warp_grid_lines(view));
    out.guides = curvesFromExtents(transform_warp_guide_lines(view));
    const ::rust::Vec<double> points = transform_warp_points(view);
    for (size_t i = 0; i + 5 <= points.size(); i += 5) {
        WarpPoint point;
        point.kind = WarpPoint::Kind(int(points[i]));
        point.pos = QPointF(points[i + 1], points[i + 2]);
        point.anchor = QPointF(points[i + 3], points[i + 4]);
        out.points << point;
    }
    return out;
}

} // namespace pictura::transform_api
