#include "api/transform_api.h"

#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"

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

} // namespace pictura::transform_api
