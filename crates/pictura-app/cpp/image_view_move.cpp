// ImageView feedback for the Move tool: the moving layer's content box as
// marching ants, the ΔX / ΔY readout in the rulers' unit, and the idle
// `Show Transform Controls` box.

#include "image_view.h"

#include "canvas_ruler.h"

#include <QtGui/QPainter>

namespace pictura {

void ImageView::setMeasureUnit(RulerUnit unit, bool traditionalPoints)
{
    measureUnit_ = int(unit);
    measureTraditional_ = traditionalPoints;
}

void ImageView::setPpiProvider(std::function<double()> ppi)
{
    measurePpi_ = std::move(ppi);
}

QString ImageView::offsetReadout(const QPointF& delta) const
{
    return measureReadout(QStringLiteral("ΔX"), delta.x(), QStringLiteral("ΔY"), delta.y());
}

QString ImageView::measureReadout(const QString& first, double firstPx, const QString& second,
                                  double secondPx) const
{
    const double ppi = measurePpi_ ? measurePpi_() : 72.0;
    const QSize doc = documentSize();
    const auto unit = RulerUnit(measureUnit_);
    return QStringLiteral("%1: %2\n%3: %4")
        .arg(first, formatRulerLength(firstPx, unit, ppi, measureTraditional_, doc.width()),
             second, formatRulerLength(secondPx, unit, ppi, measureTraditional_, doc.height()));
}

void ImageView::setControlsBox(const QPolygonF& box)
{
    if (box == controlsBox_) {
        return;
    }
    controlsBox_ = box;
    update();
}

void ImageView::setMoveOutline(const QRectF& bounds)
{
    moveOutline_ = bounds;
    updateAntsTimer();
    update();
}

QRect ImageView::antsUpdateRect() const
{
    QRectF imageBounds;
    if (selectionEdgesVisible_) {
        for (const QPolygonF& loop : selectionContours_) {
            imageBounds = imageBounds.united(
                loop.boundingRect().translated(selectionContourOffset_));
        }
    }
    for (const QPolygonF& loop : previewContours_) {
        imageBounds = imageBounds.united(loop.boundingRect());
    }
    if (!moveOutline_.isEmpty() && movePreviewActive_) {
        imageBounds = imageBounds.united(moveOutline_.translated(moveDelta_));
    }
    if (imageBounds.isEmpty() || zoom_ <= 0.0) {
        return rect();
    }
    // The painter maps image coordinates through viewRotation() after the
    // offset/zoom, so build the same widget-space box before clipping.
    const QRectF scaled(imageBounds.x() * zoom_ + offset_.x(),
                        imageBounds.y() * zoom_ + offset_.y(),
                        imageBounds.width() * zoom_, imageBounds.height() * zoom_);
    const QRectF widget = viewRotation().mapRect(scaled);
    return widget.adjusted(-2.0, -2.0, 2.0, 2.0)
        .toAlignedRect()
        .intersected(rect());
}

void ImageView::paintMoveOutline(QPainter& painter)
{
    if (moveOutline_.isEmpty() || !movePreviewActive_) {
        return;
    }
    painter.save();
    painter.setClipping(false);
    painter.setTransform(QTransform::fromScale(zoom_, zoom_)
                         * QTransform::fromTranslate(offset_.x(), offset_.y()) * viewRotation());
    // The selection's marching ants: white under an animated black dash.
    const QRectF box = moveOutline_.translated(moveDelta_);
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(Qt::white, 0));
    painter.drawRect(box);
    QPen ants(Qt::black, 0, Qt::DashLine);
    ants.setDashOffset(antsPhase_);
    painter.setPen(ants);
    painter.drawRect(box);
    painter.restore();
}

} // namespace pictura
