// CPU-warped live projective preview. Qt's raster engine can drop one triangle
// of an image drawn under a projective QTransform at some angles; warping the
// preview layer here keeps the whole shape and matches the committed engine.
// ponytail: the warp runs on the GUI thread, bounded by the visible canvas at
// the view's scale (about one sample per screen pixel); move it off-thread if a
// 4K viewport lags. Zoomed-out samples are point-bilinear, not area-averaged.
// A crossed (bowtie) target quad fills its own outline, two straight-edged
// triangles, matching the engine's `crossed_quad` (Photoshop's look).

#include "image_view.h"

#include <QtGui/QImage>
#include <QtGui/QPolygonF>

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

// `p` through `c` (QTransform constructor order), dividing by the homogeneous
// divisor whatever its sign: QTransform::map clamps a negative divisor to a tiny
// positive one, which loses a front side lying behind w < 0.
QPointF project(const std::array<double, 9>& c, const QPointF& p)
{
    const double w = c[2] * p.x() + c[5] * p.y() + c[8];
    return QPointF((c[0] * p.x() + c[3] * p.y() + c[6]) / w,
                   (c[1] * p.x() + c[4] * p.y() + c[7]) / w);
}

// The inverse of the 3×3 `m` (any consistent layout), false when singular.
bool invert3(const std::array<double, 9>& m, std::array<double, 9>& out)
{
    const double c0 = m[4] * m[8] - m[5] * m[7];
    const double c1 = m[5] * m[6] - m[3] * m[8];
    const double c2 = m[3] * m[7] - m[4] * m[6];
    const double det = m[0] * c0 + m[1] * c1 + m[2] * c2;
    if (!std::isfinite(det) || std::abs(det) < 1e-12) {
        return false;
    }
    out = {c0 / det,
           (m[2] * m[7] - m[1] * m[8]) / det,
           (m[1] * m[5] - m[2] * m[4]) / det,
           c1 / det,
           (m[0] * m[8] - m[2] * m[6]) / det,
           (m[2] * m[3] - m[0] * m[5]) / det,
           c2 / det,
           (m[1] * m[6] - m[0] * m[7]) / det,
           (m[0] * m[4] - m[1] * m[3]) / det};
    return true;
}

double orient(const QPointF& a, const QPointF& b, const QPointF& c)
{
    return (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x());
}

bool segmentsCross(const QPointF& a, const QPointF& b, const QPointF& c, const QPointF& d)
{
    return orient(a, b, c) * orient(a, b, d) < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0;
}

// Barycentric weights of `p` in (a, b, c); false outside it.
bool barycentric(const QPointF& a, const QPointF& b, const QPointF& c, const QPointF& p,
                 double (&w)[3])
{
    constexpr double kSlack = 1e-9;
    const double area = orient(a, b, c);
    if (!std::isfinite(area) || std::abs(area) < 1e-12) {
        return false;
    }
    w[0] = orient(b, c, p) / area;
    w[1] = orient(c, a, p) / area;
    w[2] = orient(a, b, p) / area;
    return w[0] >= -kSlack && w[1] >= -kSlack && w[2] >= -kSlack;
}

// The engine's `crossed_quad::Hourglass`: a crossed quad fills its own outline,
// the two straight-edged triangles meeting where its edges cross. The source
// splits at row `split` (a column for a quad whose top and bottom edges cross).
struct Hourglass {
    QPointF q[4];
    QPointF apex;
    double split = 0.5;
    bool transposed = false;
    bool valid = false;

    explicit Hourglass(const QPointF (&quad)[4])
    {
        if (segmentsCross(quad[0], quad[3], quad[1], quad[2])) {
            for (int i = 0; i < 4; ++i) {
                q[i] = quad[i];
            }
        } else if (segmentsCross(quad[0], quad[1], quad[3], quad[2])) {
            q[0] = quad[0];
            q[1] = quad[3];
            q[2] = quad[2];
            q[3] = quad[1];
            transposed = true;
        } else {
            return;
        }
        const QPointF d0 = q[3] - q[0];
        const QPointF d1 = q[2] - q[1];
        const QPointF r = q[1] - q[0];
        const double den = d0.x() * d1.y() - d0.y() * d1.x();
        if (std::abs(den) < 1e-12) {
            return;
        }
        const double s = (r.x() * d1.y() - r.y() * d1.x()) / den;
        const double t = (r.x() * d0.y() - r.y() * d0.x()) / den;
        split = std::clamp((s + t) / 2.0, 1e-6, 1.0 - 1e-6);
        apex = q[0] + d0 * s;
        valid = true;
    }

    // The source fractions (u, v) landing on `p`; false outside both triangles.
    bool inverse(const QPointF& p, double& u, double& v) const
    {
        double w[3];
        double ru = 0.0;
        double rv = 0.0;
        if (barycentric(q[0], q[1], apex, p, w)) {
            const double rest = 1.0 - w[2];
            ru = rest > 1e-12 ? w[1] / rest : 0.5;
            rv = std::clamp(w[2], 0.0, 1.0) * split;
        } else if (barycentric(apex, q[3], q[2], p, w)) {
            const double m = 1.0 - w[0];
            ru = m > 1e-12 ? w[2] / m : 0.5;
            rv = split + std::clamp(m, 0.0, 1.0) * (1.0 - split);
        } else {
            return false;
        }
        ru = std::clamp(ru, 0.0, 1.0);
        u = transposed ? rv : ru;
        v = transposed ? ru : rv;
        return true;
    }
};

} // namespace

WarpedTransformPreview warpTransformPreview(const QImage& source, const QPointF& sourceOrigin,
                                            const std::array<double, 9>& coefficients,
                                            const QRectF& sourceRect,
                                            const QRectF& visibleDocument, double scale)
{
    WarpedTransformPreview out;
    if (source.isNull() || sourceRect.isEmpty() || visibleDocument.isEmpty()
        || !(scale > 0.0)) {
        return out;
    }
    std::array<double, 9> inverse{};
    if (!invert3(coefficients, inverse)) {
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
    int ahead = 0;
    for (const QPointF& corner : corners) {
        ahead += w(corner.x(), corner.y()) > 0.0 ? 1 : 0;
    }
    const double front = ahead >= 3   ? 1.0
        : ahead <= 1                  ? -1.0
        : (w((sourceRect.left() + sourceRect.right()) / 2.0,
             (sourceRect.top() + sourceRect.bottom()) / 2.0)
               >= 0.0
               ? 1.0
               : -1.0);
    QPointF target[4];
    bool targetFinite = true;
    for (int i = 0; i < 4; ++i) {
        target[i] = project(coefficients, corners[i]);
        targetFinite = targetFinite && std::isfinite(target[i].x()) && std::isfinite(target[i].y());
    }
    const Hourglass hourglass(target);
    const bool crossed = targetFinite && hourglass.valid;
    out.crossed = crossed;
    const QRectF visibleLocal = visibleDocument.translated(-sourceOrigin);
    QRectF destination = visibleLocal;
    // A source that crosses the horizon maps its front side outside its own
    // projected corners, so only a one-sided source is bounded by them; the
    // per-sample front-side check rejects the far side.
    if (crossed) {
        destination = QPolygonF({target[0], target[1], target[2], target[3]})
                          .boundingRect()
                          .intersected(visibleLocal);
    } else if (ahead == 0 || ahead == 4) {
        double minX = std::numeric_limits<double>::infinity();
        double minY = std::numeric_limits<double>::infinity();
        double maxX = -std::numeric_limits<double>::infinity();
        double maxY = -std::numeric_limits<double>::infinity();
        for (const QPointF& corner : corners) {
            const QPointF mapped = project(coefficients, corner);
            if (!std::isfinite(mapped.x()) || !std::isfinite(mapped.y())) {
                return out;
            }
            minX = std::min(minX, mapped.x());
            minY = std::min(minY, mapped.y());
            maxX = std::max(maxX, mapped.x());
            maxY = std::max(maxY, mapped.y());
        }
        destination = QRectF(minX, minY, maxX - minX, maxY - minY).intersected(visibleLocal);
    }
    if (destination.isEmpty()) {
        return out;
    }
    const QRect pixels =
        QRectF(destination.topLeft() * scale, destination.size() * scale).toAlignedRect();
    if (pixels.isEmpty()) {
        return out;
    }
    QImage src = source.format() == QImage::Format_ARGB32_Premultiplied
        ? source
        : source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QImage result(pixels.size(), QImage::Format_ARGB32_Premultiplied);
    result.fill(Qt::transparent);
    bool any = false;
    for (int y = 0; y < pixels.height(); ++y) {
        QRgb* row = reinterpret_cast<QRgb*>(result.scanLine(y));
        for (int x = 0; x < pixels.width(); ++x) {
            const QPointF sample =
                QPointF(pixels.left() + x + 0.5, pixels.top() + y + 0.5) / scale;
            QPointF sourcePoint;
            if (crossed) {
                double u = 0.0;
                double v = 0.0;
                if (!hourglass.inverse(sample, u, v)) {
                    continue;
                }
                sourcePoint = QPointF(
                    sourceRect.left() + std::min(u * sourceRect.width(), sourceRect.width() - 1e-9),
                    sourceRect.top()
                        + std::min(v * sourceRect.height(), sourceRect.height() - 1e-9));
            } else {
                sourcePoint = project(inverse, sample);
                if (!std::isfinite(sourcePoint.x()) || !std::isfinite(sourcePoint.y())) {
                    continue;
                }
                if (w(sourcePoint.x(), sourcePoint.y()) * front <= 0.0) {
                    continue;
                }
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
    out.origin = QPointF(pixels.left(), pixels.top()) / scale;
    out.scale = scale;
    return out;
}

void ImageView::refreshTransformWarp()
{
    QRectF visible;
    if (width() > 0 && height() > 0) {
        const QPointF a = widgetToImage(QPointF(0.0, 0.0));
        const QPointF b = widgetToImage(QPointF(width(), 0.0));
        const QPointF c = widgetToImage(QPointF(width(), height()));
        const QPointF d = widgetToImage(QPointF(0.0, height()));
        const double left = std::min({a.x(), b.x(), c.x(), d.x()});
        const double top = std::min({a.y(), b.y(), c.y(), d.y()});
        const double right = std::max({a.x(), b.x(), c.x(), d.x()});
        const double bottom = std::max({a.y(), b.y(), c.y(), d.y()});
        visible = QRectF(QPointF(left, top), QPointF(right, bottom));
    }
    if (visible.isEmpty() && !docSize_.isEmpty()) {
        visible = QRectF(QPointF(0.0, 0.0), QSizeF(docSize_));
    }
    // Zoomed out, one sample per device pixel; zoomed in, one per document
    // pixel (the painter magnifies it like the plain layer).
    const double scale = std::min(1.0, zoom_ * devicePixelRatioF());
    if (!transformWarpedStale_ && visible == transformWarpedVisible_
        && scale == transformWarpedScale_) {
        return;
    }
    const QRectF sourceRect = transformProjectiveSource_.isEmpty()
        ? QRectF(QPointF(0.0, 0.0), QSizeF(moveLayer_.size()))
        : transformProjectiveSource_;
    transformWarped_ = warpTransformPreview(moveLayer_, moveLayerPos_,
                                            transformProjectiveCoefficients_, sourceRect,
                                            visible, scale);
    transformWarpedVisible_ = visible;
    transformWarpedScale_ = scale;
    transformWarpedStale_ = false;
}

} // namespace pictura
