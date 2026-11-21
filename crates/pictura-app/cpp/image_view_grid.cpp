// View > Show > Grid, Pixel Grid, Layer Edges and the persistent Target Path
// curve (frame_view.cpp). The interactive path overlay stays in
// image_view_paths.cpp.

#include "image_view.h"

#include <QtGui/QPainter>
#include <QtGui/QPen>

#include <algorithm>
#include <cmath>

namespace pictura {

void ImageView::setTargetPathOverlay(const PathOverlay& overlay)
{
    if (targetPathOverlay_.curve == overlay.curve
        && targetPathOverlay_.anchors == overlay.anchors
        && targetPathOverlay_.activeAnchor == overlay.activeAnchor
        && targetPathOverlay_.anchorsSolid == overlay.anchorsSolid
        && targetPathOverlay_.handles == overlay.handles
        && targetPathOverlay_.preview == overlay.preview
        && targetPathOverlay_.bounds == overlay.bounds) {
        return;
    }
    targetPathOverlay_ = overlay;
    update();
}

void ImageView::setTargetPathVisible(bool on)
{
    if (targetPathVisible_ == on) {
        return;
    }
    targetPathVisible_ = on;
    update();
}

void ImageView::setLayerEdges(const QRectF& rect)
{
    if (layerEdges_ == rect) {
        return;
    }
    layerEdges_ = rect;
    update();
}

void ImageView::setLayerEdgesVisible(bool on)
{
    if (layerEdgesVisible_ == on) {
        return;
    }
    layerEdgesVisible_ = on;
    update();
}

void ImageView::setGrid(bool visible, const QColor& color, int style, double everyX,
                        double everyY, int subdivisions)
{
    gridVisible_ = visible;
    gridColor_ = color;
    gridStyle_ = qBound(0, style, 2);
    gridEveryX_ = everyX;
    gridEveryY_ = everyY;
    gridSubdivisions_ = std::max(1, subdivisions);
    update();
}

void ImageView::setPixelGridVisible(bool on)
{
    if (pixelGridVisible_ == on) {
        return;
    }
    pixelGridVisible_ = on;
    update();
}

void ImageView::setNotesVisible(bool on)
{
    if (notesVisible_ == on) {
        return;
    }
    notesVisible_ = on;
    update();
}

void ImageView::setCountVisible(bool on)
{
    if (countVisible_ == on) {
        return;
    }
    countVisible_ = on;
    update();
}

void ImageView::setSamplersVisible(bool on)
{
    if (samplersVisible_ == on) {
        return;
    }
    samplersVisible_ = on;
    update();
}

void ImageView::setSlicesVisible(bool on)
{
    if (slicesVisible_ == on) {
        return;
    }
    slicesVisible_ = on;
    update();
}

void ImageView::paintTargetPathOverlay(QPainter& painter)
{
    if (!targetPathVisible_ || targetPathOverlay_.curve.isEmpty()) {
        return;
    }
    const QTransform toWidget = QTransform::fromScale(zoom_, zoom_)
        * QTransform::fromTranslate(offset_.x(), offset_.y());
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setBrush(Qt::NoBrush);
    const QPainterPath curve = toWidget.map(targetPathOverlay_.curve);
    painter.setPen(QPen(Qt::white, 2.6));
    painter.drawPath(curve);
    painter.setPen(QPen(QColor(0x2c, 0x6f, 0xd6), 1.1));
    painter.drawPath(curve);
    painter.restore();
}

void ImageView::paintLayerEdges(QPainter& painter)
{
    if (!layerEdgesVisible_ || layerEdges_.isNull()) {
        return;
    }
    const QTransform toWidget = QTransform::fromScale(zoom_, zoom_)
        * QTransform::fromTranslate(offset_.x(), offset_.y());
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(0x2c, 0x6f, 0xd6), 1.0, Qt::DashLine));
    painter.drawRect(toWidget.mapRect(layerEdges_));
    painter.restore();
}

void ImageView::paintGrid(QPainter& painter)
{
    if (!gridVisible_ || docSize_.isEmpty() || gridEveryX_ <= 0.0 || gridEveryY_ <= 0.0
        || zoom_ <= 0.0) {
        return;
    }
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    const QRectF docRect(offset_.x(), offset_.y(), docSize_.width() * zoom_,
                         docSize_.height() * zoom_);
    painter.setClipRect(docRect);
    const auto xToWidget = [this](double x) { return x * zoom_ + offset_.x(); };
    const auto yToWidget = [this](double y) { return y * zoom_ + offset_.y(); };
    const double left = std::max(0.0, -offset_.x() / zoom_);
    const double top = std::max(0.0, -offset_.y() / zoom_);
    const double right = std::min(double(docSize_.width()), (width() - offset_.x()) / zoom_);
    const double bottom = std::min(double(docSize_.height()), (height() - offset_.y()) / zoom_);
    if (right <= left || bottom <= top) {
        painter.restore();
        return;
    }
    const Qt::PenStyle style = gridStyle_ == 1 ? Qt::DashLine
        : gridStyle_ == 2 ? Qt::DotLine : Qt::SolidLine;
    QColor minor = gridColor_;
    minor.setAlpha(130);
    const int subs = std::max(1, gridSubdivisions_);
    const double minorX = gridEveryX_ / subs;
    const double minorY = gridEveryY_ / subs;
    const auto first = [](double value, double step) {
        return std::floor(value / step) * step;
    };
    if (minorX * zoom_ >= 2.0) {
        painter.setPen(QPen(minor, 0, style));
        for (double x = first(left, minorX); x <= right; x += minorX) {
            painter.drawLine(QPointF(xToWidget(x), yToWidget(top)),
                             QPointF(xToWidget(x), yToWidget(bottom)));
        }
    }
    if (minorY * zoom_ >= 2.0) {
        painter.setPen(QPen(minor, 0, style));
        for (double y = first(top, minorY); y <= bottom; y += minorY) {
            painter.drawLine(QPointF(xToWidget(left), yToWidget(y)),
                             QPointF(xToWidget(right), yToWidget(y)));
        }
    }
    painter.setPen(QPen(gridColor_, 0, style));
    if (gridEveryX_ * zoom_ >= 1.0) {
        for (double x = first(left, gridEveryX_); x <= right; x += gridEveryX_) {
            painter.drawLine(QPointF(xToWidget(x), yToWidget(top)),
                             QPointF(xToWidget(x), yToWidget(bottom)));
        }
    }
    if (gridEveryY_ * zoom_ >= 1.0) {
        for (double y = first(top, gridEveryY_); y <= bottom; y += gridEveryY_) {
            painter.drawLine(QPointF(xToWidget(left), yToWidget(y)),
                             QPointF(xToWidget(right), yToWidget(y)));
        }
    }
    painter.restore();
}

void ImageView::paintPixelGrid(QPainter& painter)
{
    if (!pixelGridVisible_ || zoom_ <= 5.0 || docSize_.isEmpty()) {
        return;
    }
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    const QRectF docRect(offset_.x(), offset_.y(), docSize_.width() * zoom_,
                         docSize_.height() * zoom_);
    painter.setClipRect(docRect);
    const auto xToWidget = [this](double x) { return x * zoom_ + offset_.x(); };
    const auto yToWidget = [this](double y) { return y * zoom_ + offset_.y(); };
    const double left = std::max(0.0, -offset_.x() / zoom_);
    const double top = std::max(0.0, -offset_.y() / zoom_);
    const double right = std::min(double(docSize_.width()), (width() - offset_.x()) / zoom_);
    const double bottom = std::min(double(docSize_.height()), (height() - offset_.y()) / zoom_);
    painter.setPen(QPen(QColor(90, 90, 90, 110), 0));
    for (int x = int(std::ceil(left)); x <= int(std::floor(right)); ++x) {
        painter.drawLine(QPointF(xToWidget(x), yToWidget(top)),
                         QPointF(xToWidget(x), yToWidget(bottom)));
    }
    for (int y = int(std::ceil(top)); y <= int(std::floor(bottom)); ++y) {
        painter.drawLine(QPointF(xToWidget(left), yToWidget(y)),
                         QPointF(xToWidget(right), yToWidget(y)));
    }
    painter.restore();
}

} // namespace pictura
