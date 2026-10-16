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
    const double ppi = measurePpi_ ? measurePpi_() : 72.0;
    const QSize doc = documentSize();
    const auto unit = RulerUnit(measureUnit_);
    return QStringLiteral("ΔX: %1\nΔY: %2")
        .arg(formatRulerLength(delta.x(), unit, ppi, measureTraditional_, doc.width()),
             formatRulerLength(delta.y(), unit, ppi, measureTraditional_, doc.height()));
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
