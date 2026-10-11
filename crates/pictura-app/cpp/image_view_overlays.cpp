// ImageView overlays for the Crop tool group: the crop box, the Perspective
// Crop quad, and the slice map. Split from image_view.cpp to keep each translation unit small.
// Ported from photorust's CanvasView::paintCropQuad / paintSlices.

#include "image_view.h"

#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

// The active crop guide overlay: 0 Rule of Thirds, 1 Grid, 2 Diagonal,
// 3 Triangle, 4 Golden Ratio, 5 Golden Spiral.
void drawCropGuides(QPainter& p, const QRectF& box, int mode)
{
    const QPointF tl = box.topLeft();
    const QPointF tr = box.topRight();
    const QPointF bl = box.bottomLeft();
    const auto vline = [&](double fx) { p.drawLine(QPointF(fx, box.top()), QPointF(fx, box.bottom())); };
    const auto hline = [&](double fy) { p.drawLine(QPointF(box.left(), fy), QPointF(box.right(), fy)); };
    switch (mode) {
    case 1: // Grid: eighths.
        for (int i = 1; i < 8; ++i) {
            hline(box.top() + box.height() * i / 8.0);
            vline(box.left() + box.width() * i / 8.0);
        }
        break;
    case 2: // Diagonal.
        p.drawLine(tl, box.bottomRight());
        p.drawLine(tr, bl);
        break;
    case 3: { // Triangle: diagonals plus top-corner fans to the bottom centre.
        const QPointF bc(box.center().x(), box.bottom());
        p.drawLine(tl, box.bottomRight());
        p.drawLine(tr, bl);
        p.drawLine(tl, bc);
        p.drawLine(tr, bc);
        break;
    }
    case 4: // Golden Ratio lines.
        for (double f : {0.382, 0.618}) {
            hline(box.top() + box.height() * f);
            vline(box.left() + box.width() * f);
        }
        break;
    case 5: { // Golden Spiral about the box centre (an approximation of CS6's).
        p.save();
        p.setClipRect(box);
        constexpr double kPi = 3.14159265358979323846;
        const double k = std::log(1.6180339887498949) / (kPi / 2.0);
        const double span = 4.0 * kPi;
        const double rMin = std::exp(-k * span);
        const double scale = (std::min(box.width(), box.height()) * 0.5) / (1.0 - rMin);
        QPainterPath path;
        const int n = 320;
        for (int i = 0; i <= n; ++i) {
            const double theta = span * i / double(n);
            const double r = std::exp(-k * (span - theta)) * scale;
            const QPointF pt(box.center().x() + r * std::cos(theta),
                             box.center().y() + r * std::sin(theta));
            if (i == 0) {
                path.moveTo(pt);
            } else {
                path.lineTo(pt);
            }
        }
        p.setBrush(Qt::NoBrush);
        p.drawPath(path);
        p.restore();
        break;
    }
    default: // 0 Rule of Thirds.
        for (int i = 1; i <= 2; ++i) {
            hline(box.top() + box.height() * i / 3.0);
            vline(box.left() + box.width() * i / 3.0);
        }
        break;
    }
}

} // namespace

void ImageView::setCropBox(const QRectF& box)
{
    cropBox_ = box.normalized();
    update();
}

void ImageView::setCropPreview(bool preview)
{
    if (cropPreview_ == preview) {
        return;
    }
    cropPreview_ = preview;
    update();
}

void ImageView::setCropOverlay(int overlay)
{
    if (cropOverlay_ == overlay) {
        return;
    }
    cropOverlay_ = overlay;
    update();
}

void ImageView::setCropContentOffset(const QPointF& offset)
{
    if (cropContentOffset_ == offset) {
        return;
    }
    cropContentOffset_ = offset;
    update();
}

void ImageView::clearCropBox()
{
    cropBox_ = QRectF();
    cropPreview_ = false;
    cropContentOffset_ = QPointF();
    cropStraighten_ = 0.0;
    cropStraightenPivot_ = QPointF();
    cropStraightenLine_ = QLineF();
    update();
}

void ImageView::setCropStraighten(double degrees, const QPointF& pivot)
{
    cropStraighten_ = degrees;
    cropStraightenPivot_ = pivot;
    update();
}

void ImageView::setCropStraightenLine(const QLineF& line)
{
    cropStraightenLine_ = line;
    update();
}

void ImageView::clearCropStraightenLine()
{
    cropStraightenLine_ = QLineF();
    update();
}

// The preview canvas: the image rect when straight, else the axis-aligned
// bounding box of the image rect rotated about the session pivot. Matches
// `rotate_document_in`'s canvas growth so the preview and the commit agree.
QRectF ImageView::cropCanvasImageRect() const
{
    // The content offset (Modern pan) moves the composite, so the frame follows
    // the shifted image rect.
    const QRectF image =
        QRectF(0.0, 0.0, docSize_.width(), docSize_.height()).translated(cropContentOffset_);
    if (docSize_.isEmpty()) {
        return image;
    }
    QRectF canvas = image;
    if (cropStraighten_ != 0.0 && !cropBox_.isNull()) {
        const double rad = cropStraighten_ * M_PI / 180.0;
        const double c = std::cos(rad);
        const double s = std::sin(rad);
        const QPointF pivot = cropStraightenPivot_;
        double min_x = 0.0;
        double min_y = 0.0;
        double max_x = 0.0;
        double max_y = 0.0;
        bool first = true;
        for (const QPointF& corner : {image.topLeft(), image.topRight(), image.bottomRight(),
                                      image.bottomLeft()}) {
            const QPointF u = corner - pivot;
            const double qx = pivot.x() + c * u.x() - s * u.y();
            const double qy = pivot.y() + s * u.x() + c * u.y();
            if (first) {
                min_x = max_x = qx;
                min_y = max_y = qy;
                first = false;
            } else {
                min_x = std::min(min_x, qx);
                min_y = std::min(min_y, qy);
                max_x = std::max(max_x, qx);
                max_y = std::max(max_y, qy);
            }
        }
        canvas = QRectF(min_x, min_y, max_x - min_x, max_y - min_y);
    }
    // The canvas follows a crop box dragged beyond its edge, so the grown area
    // is visible and shaded instead of clipped at the original edge.
    if (!cropBox_.isNull()) {
        canvas = canvas.united(cropBox_);
    }
    return canvas;
}

QPolygonF ImageView::cropBoxWidgetForTest() const
{
    const auto toWidget = [this](const QPointF& p) { return p * zoom_ + offset_; };
    QPolygonF poly;
    if (!cropBox_.isNull()) {
        poly << toWidget(cropBox_.topLeft()) << toWidget(cropBox_.topRight())
             << toWidget(cropBox_.bottomRight()) << toWidget(cropBox_.bottomLeft());
    }
    return poly;
}

void ImageView::setPerspectiveCropQuad(const QPolygonF& quad)
{
    perspectiveQuad_ = quad.size() == 4 ? quad : QPolygonF();
    update();
}

void ImageView::clearPerspectiveCropQuad()
{
    perspectiveQuad_.clear();
    update();
}

void ImageView::setSliceOverlay(const QList<SliceOverlay>& slices, const QRectF& dragging)
{
    sliceOverlay_ = slices;
    sliceDrag_ = dragging.normalized();
    update();
}

void ImageView::clearSliceOverlay()
{
    sliceOverlay_.clear();
    sliceDrag_ = QRectF();
    update();
}

void ImageView::setSamplingRing(double diameter, const QPointF& imagePos)
{
    if (diameter <= 0.0) {
        clearSamplingRing();
        return;
    }
    samplingRingActive_ = true;
    samplingRingDiameter_ = diameter;
    samplingRingImagePos_ = imagePos;
    update();
}

void ImageView::clearSamplingRing()
{
    if (!samplingRingActive_) {
        return;
    }
    samplingRingActive_ = false;
    update();
}

// Drawn in widget space so lines, handles, and badges keep their screen size at
// any zoom.
void ImageView::paintCropGroupOverlays(QPainter& painter)
{
    if (perspectiveQuad_.size() != 4 && sliceOverlay_.isEmpty() && sliceDrag_.isNull()
        && cropBox_.isNull() && cropStraightenLine_.isNull() && !samplingRingActive_) {
        return;
    }
    const auto toWidget = [this](const QPointF& p) { return p * zoom_ + offset_; };
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);

    if (!sliceOverlay_.isEmpty() || !sliceDrag_.isNull()) {
        // CS6 colours: user slices saturated blue and solid, auto slices grey
        // and dotted.
        const QColor userColor(0x2c, 0x6f, 0xd6);
        const QColor autoColor(0x8a, 0x8a, 0x8a);
        QFont badgeFont = painter.font();
        badgeFont.setPixelSize(9);
        painter.setFont(badgeFont);
        const QFontMetrics metrics(badgeFont);
        painter.setRenderHint(QPainter::Antialiasing, false);
        for (const SliceOverlay& slice : sliceOverlay_) {
            const QRectF box(toWidget(slice.rect.topLeft()), toWidget(slice.rect.bottomRight()));
            const QColor color = slice.user ? userColor : autoColor;
            painter.setPen(QPen(color, 1, slice.user ? Qt::SolidLine : Qt::DotLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawRect(box);
            // The numbered badge sits inside the top-left corner, skipped on a
            // slice too small to hold it.
            const QString label = QStringLiteral("%1").arg(slice.number, 2, 10, QLatin1Char('0'));
            const QRectF badge(box.left() + 1, box.top() + 1,
                               metrics.horizontalAdvance(label) + 6, 12);
            if (box.width() >= badge.width() + 2 && box.height() >= badge.height() + 2) {
                painter.setPen(Qt::NoPen);
                painter.setBrush(color);
                painter.drawRect(badge);
                painter.setPen(Qt::white);
                painter.drawText(badge, Qt::AlignCenter, label);
            }
            if (slice.selected) {
                // CS6 marks the selected slice with orange handles.
                const QColor handle(0xf5, 0x9e, 0x0b);
                painter.setPen(QPen(handle, 1));
                painter.setBrush(handle);
                const QPointF c = box.center();
                for (const QPointF& h :
                     {box.topLeft(), QPointF(c.x(), box.top()), box.topRight(),
                      QPointF(box.right(), c.y()), box.bottomRight(), QPointF(c.x(), box.bottom()),
                      box.bottomLeft(), QPointF(box.left(), c.y())}) {
                    painter.drawRect(QRectF(h.x() - 2.5, h.y() - 2.5, 5, 5));
                }
            }
        }
        if (!sliceDrag_.isNull()) {
            painter.setPen(QPen(userColor, 1));
            painter.setBrush(Qt::NoBrush);
            painter.drawRect(QRectF(toWidget(sliceDrag_.topLeft()),
                                    toWidget(sliceDrag_.bottomRight())));
        }
    }

    if (!cropBox_.isNull()) {
        const QRectF box(toWidget(cropBox_.topLeft()), toWidget(cropBox_.bottomRight()));
        painter.setRenderHint(QPainter::Antialiasing, false);
        // The crop shield: what is about to be thrown away, dimmed. Only the
        // canvas outside the box is dimmed, not the workspace around it; under
        // straighten the canvas is the rotated composite's bounding box.
        const QRectF canvasDoc = cropCanvasImageRect();
        const QRectF canvas(toWidget(canvasDoc.topLeft()), toWidget(canvasDoc.bottomRight()));
        QPainterPath shield;
        shield.addRect(canvas);
        shield.addRect(box);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0, 0, 0, 150));
        painter.drawPath(shield);
        painter.setBrush(Qt::NoBrush);
        if (cropPreview_) {
            // The preview state: a dashed outline only, no guides or handles.
            painter.setPen(QPen(QColor(255, 255, 255, 220), 1, Qt::DashLine));
            painter.drawRect(box);
        } else {
            // The active guide overlay, chosen from the options-bar menu.
            painter.setPen(QPen(QColor(255, 255, 255, 90), 1));
            drawCropGuides(painter, box, cropOverlay_);
            painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
            painter.drawRect(box);
            painter.setPen(QPen(QColor(40, 40, 40), 1));
            painter.setBrush(Qt::white);
            const QPointF c = box.center();
            for (const QPointF& h :
                 {box.topLeft(), QPointF(c.x(), box.top()), box.topRight(), QPointF(box.right(), c.y()),
                  box.bottomRight(), QPointF(c.x(), box.bottom()), box.bottomLeft(),
                  QPointF(box.left(), c.y())}) {
                painter.drawRect(QRectF(h.x() - 3, h.y() - 3, 6, 6));
            }
        }
    }

    // The straighten line tool's live horizon: a plain ruled line from press to
    // the current pointer, drawn in widget space.
    if (!cropStraightenLine_.isNull()) {
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
        painter.drawLine(QLineF(toWidget(cropStraightenLine_.p1()),
                                toWidget(cropStraightenLine_.p2())));
    }

    if (perspectiveQuad_.size() == 4) {
        QPolygonF quad;
        for (const QPointF& p : perspectiveQuad_) {
            quad << toWidget(p);
        }
        painter.setRenderHint(QPainter::Antialiasing, true);
        // Shade everything outside the quad, as the rectangular crop does.
        QPainterPath shield;
        shield.addRect(viewRect());
        QPainterPath inside;
        inside.addPolygon(quad);
        inside.closeSubpath();
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0, 0, 0, 150));
        painter.drawPath(shield.subtracted(inside));
        // A 3x3 grid interpolated along the edges, so it follows the
        // perspective and shows how the warp will land.
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 90), 1));
        for (int i = 1; i <= 2; ++i) {
            const double t = i / 3.0;
            painter.drawLine(quad[0] + (quad[1] - quad[0]) * t, quad[3] + (quad[2] - quad[3]) * t);
            painter.drawLine(quad[0] + (quad[3] - quad[0]) * t, quad[1] + (quad[2] - quad[1]) * t);
        }
        painter.setPen(QPen(QColor(255, 255, 255, 220), 1));
        painter.drawPolygon(quad);
        painter.setPen(QPen(QColor(40, 40, 40), 1));
        painter.setBrush(Qt::white);
        for (const QPointF& p : quad) {
            painter.drawRect(QRectF(p.x() - 3.5, p.y() - 3.5, 7, 7));
        }
    }
    if (samplingRingActive_) {
        // The eyedropper's sampling ring: a white/black pair so it reads on any
        // document, centred on the hovered pixel in widget space.
        const QPointF c = toWidget(samplingRingImagePos_);
        const double r = samplingRingDiameter_ * 0.5 * zoom_;
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 0));
        painter.drawEllipse(c, r + 1.0, r + 1.0);
        painter.setPen(QPen(Qt::black, 0));
        painter.drawEllipse(c, r, r);
    }

    painter.restore();
}

} // namespace pictura
