// ImageView feedback for the Move tool: the moving layer's content box as
// marching ants, the ΔX / ΔY readout in the rulers' unit, the idle
// `Show Transform Controls` box, and the Move preview's state.

#include "image_view.h"

#include "canvas_ruler.h"

#include <QtGui/QPainter>

#include <algorithm>

namespace pictura {

void ImageView::beginMovePreview(const QImage& base, const QImage& layer, const QPointF& layerPos,
                                 double opacity, std::optional<QPoint> basePos, int scale,
                                 const QImage& above, std::optional<QPoint> abovePos,
                                 int aboveScale)
{
    moveBase_ = base;
    moveBasePos_ = basePos;
    moveLayer_ = layer;
    moveLayerPos_ = layerPos;
    moveDelta_ = QPointF();
    moveOpacity_ = opacity;
    moveScale_ = std::max(1, scale);
    moveAbove_ = above;
    moveAbovePos_ = abovePos;
    moveAboveScale_ = std::max(1, aboveScale);
    lastMoveRect_ = QRect();
    movePreviewActive_ = true;
    update();
}

void ImageView::setMovePreviewLayer(const QImage& layer, const QPointF& layerPos)
{
    moveLayer_ = layer;
    moveLayerPos_ = layerPos;
    update();
}

void ImageView::setMovePreviewDelta(const QPointF& delta)
{
    moveDelta_ = delta;
    update();
}

void ImageView::endMovePreview()
{
    setMoveOutline(QRectF());
    movePreviewActive_ = false;
    moveBase_ = QImage();
    moveLayer_ = QImage();
    moveAbove_ = QImage();
    moveAbovePos_.reset();
    moveAboveScale_ = 1;
    lastMoveRect_ = QRect();
    moveDelta_ = QPointF();
    moveOpacity_ = 1.0;
    moveScale_ = 1;
    update();
}

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

QRect ImageView::moveImageWidgetRect() const
{
    if (!movePreviewActive_ || moveLayer_.isNull() || zoom_ <= 0.0) {
        return QRect();
    }
    const QRectF moving(moveLayerPos_ + moveDelta_,
                        QSizeF(moveLayer_.size()) * moveScale_);
    const QRectF scaled(moving.x() * zoom_ + offset_.x(), moving.y() * zoom_ + offset_.y(),
                        moving.width() * zoom_, moving.height() * zoom_);
    return viewRotation()
        .mapRect(scaled)
        .adjusted(-2.0, -2.0, 2.0, 2.0)
        .toAlignedRect()
        .intersected(rect());
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
    QRect out = widget.adjusted(-2.0, -2.0, 2.0, 2.0)
                    .toAlignedRect()
                    .intersected(rect());
    // The moving image can reach past the ants box (a view preview carries a
    // margin), and the frame it just left must be cleared too: an ants-only
    // repaint that covered only the new place would leave the old one (#1250).
    if (movePreviewActive_) {
        out = out.united(moveImageWidgetRect()).united(lastMoveRect_);
    }
    return out;
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
