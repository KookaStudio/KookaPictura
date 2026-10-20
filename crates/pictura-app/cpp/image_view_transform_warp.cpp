// CPU-warped live projective preview. Qt's raster engine can drop one triangle
// of an image drawn under a projective QTransform at some angles; warping the
// preview layer here keeps the whole shape and matches the committed engine.
// ponytail: the warp runs on the GUI thread per move, bounded by the visible
// canvas; cap its resolution if 4K canvases lag.

#include "image_view.h"

#include <QtGui/QImage>
#include <QtGui/QTransform>

#include <algorithm>
#include <cmath>
#include <limits>

namespace pictura {

namespace {

QRgb bilinear(const QImage& image, double x, double y)
{
    const double sx = x - 0.5;
    const double sy = y - 0.5;
    const int x0 = int(std::floor(sx));
    const int y0 = int(std::floor(sy));
    const double fx = sx - x0;
    const double fy = sy - y0;
    const auto at = [&image](int px, int py) {
        px = std::clamp(px, 0, image.width() - 1);
        py = std::clamp(py, 0, image.height() - 1);
        return reinterpret_cast<const QRgb*>(image.constScanLine(py))[px];
    };
    const QRgb p00 = at(x0, y0);
    const QRgb p10 = at(x0 + 1, y0);
    const QRgb p01 = at(x0, y0 + 1);
    const QRgb p11 = at(x0 + 1, y0 + 1);
    const auto mix = [fx, fy](int c00, int c10, int c01, int c11) {
        const double top = c00 + (c10 - c00) * fx;
        const double bottom = c01 + (c11 - c01) * fx;
        return int(std::lround(top + (bottom - top) * fy));
    };
    return qRgba(mix(qRed(p00), qRed(p10), qRed(p01), qRed(p11)),
                 mix(qGreen(p00), qGreen(p10), qGreen(p01), qGreen(p11)),
                 mix(qBlue(p00), qBlue(p10), qBlue(p01), qBlue(p11)),
                 mix(qAlpha(p00), qAlpha(p10), qAlpha(p01), qAlpha(p11)));
}

} // namespace

WarpedTransformPreview warpTransformPreview(const QImage& source, const QPointF& sourceOrigin,
                                            const std::array<double, 9>& coefficients,
                                            const QRectF& sourceRect,
                                            const QRectF& visibleDocument)
{
    WarpedTransformPreview out;
    if (source.isNull() || sourceRect.isEmpty() || visibleDocument.isEmpty()) {
        return out;
    }
    const QTransform forward(coefficients[0], coefficients[1], coefficients[2],
                             coefficients[3], coefficients[4], coefficients[5],
                             coefficients[6], coefficients[7], coefficients[8]);
    bool invertible = false;
    const QTransform inverse = forward.inverted(&invertible);
    if (!invertible) {
        return out;
    }
    const auto w = [&coefficients](double x, double y) {
        return coefficients[2] * x + coefficients[5] * y + coefficients[8];
    };
    const QPointF corners[4] = {
        QPointF(sourceRect.left(), sourceRect.top()),
        QPointF(sourceRect.right(), sourceRect.top()),
        QPointF(sourceRect.right(), sourceRect.bottom()),
        QPointF(sourceRect.left(), sourceRect.bottom()),
    };
    double minX = std::numeric_limits<double>::infinity();
    double minY = std::numeric_limits<double>::infinity();
    double maxX = -std::numeric_limits<double>::infinity();
    double maxY = -std::numeric_limits<double>::infinity();
    int ahead = 0;
    for (const QPointF& corner : corners) {
        const QPointF mapped = forward.map(corner);
        if (!std::isfinite(mapped.x()) || !std::isfinite(mapped.y())) {
            return out;
        }
        minX = std::min(minX, mapped.x());
        minY = std::min(minY, mapped.y());
        maxX = std::max(maxX, mapped.x());
        maxY = std::max(maxY, mapped.y());
        ahead += w(corner.x(), corner.y()) > 0.0 ? 1 : 0;
    }
    const QRectF bounds(minX, minY, maxX - minX, maxY - minY);
    const double front = ahead >= 3   ? 1.0
        : ahead <= 1                  ? -1.0
        : (w((sourceRect.left() + sourceRect.right()) / 2.0,
             (sourceRect.top() + sourceRect.bottom()) / 2.0)
               >= 0.0
               ? 1.0
               : -1.0);
    const QRectF visibleLocal = visibleDocument.translated(-sourceOrigin);
    const QRectF destination = bounds.intersected(visibleLocal);
    if (destination.isEmpty()) {
        return out;
    }
    const QRect pixels = destination.toAlignedRect();
    QImage src = source.format() == QImage::Format_ARGB32_Premultiplied
        ? source
        : source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QImage result(pixels.size(), QImage::Format_ARGB32_Premultiplied);
    result.fill(Qt::transparent);
    bool any = false;
    for (int y = 0; y < pixels.height(); ++y) {
        QRgb* row = reinterpret_cast<QRgb*>(result.scanLine(y));
        for (int x = 0; x < pixels.width(); ++x) {
            const QPointF sourcePoint =
                inverse.map(QPointF(pixels.left() + x + 0.5, pixels.top() + y + 0.5));
            if (!std::isfinite(sourcePoint.x()) || !std::isfinite(sourcePoint.y())) {
                continue;
            }
            if (w(sourcePoint.x(), sourcePoint.y()) * front <= 0.0) {
                continue;
            }
            if (sourcePoint.x() < sourceRect.left() || sourcePoint.x() >= sourceRect.right()
                || sourcePoint.y() < sourceRect.top()
                || sourcePoint.y() >= sourceRect.bottom()
                || sourcePoint.x() < 0.0 || sourcePoint.y() < 0.0
                || sourcePoint.x() >= src.width() || sourcePoint.y() >= src.height()) {
                continue;
            }
            const QRgb pixel = bilinear(src, sourcePoint.x(), sourcePoint.y());
            if (qAlpha(pixel) == 0) {
                continue;
            }
            row[x] = pixel;
            any = true;
        }
    }
    if (!any) {
        return out;
    }
    out.image = result;
    out.origin = QPointF(pixels.left(), pixels.top());
    return out;
}

} // namespace pictura
