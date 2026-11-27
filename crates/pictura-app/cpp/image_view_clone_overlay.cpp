#include "image_view.h"

#include <QtGui/QPainter>

namespace pictura {

void ImageView::setCloneOverlay(const CloneOverlay& overlay)
{
    cloneOverlay_ = overlay;
    update();
}

void ImageView::clearCloneOverlay()
{
    if (!cloneOverlay_.source.isNull()) {
        cloneOverlay_ = {};
        update();
    }
}

void ImageView::paintCloneOverlay(QPainter& painter)
{
    const auto& overlay = cloneOverlay_;
    if (overlay.source.isNull() || overlay.opacity <= 0) return;
    painter.save();
    if (overlay.clipped) {
        QPainterPath shape;
        QTransform brush;
        brush.translate(overlay.pointer.x(), overlay.pointer.y());
        brush.rotate(overlay.brushAngle);
        const qreal radius = overlay.brushSize / 2.0;
        shape.addEllipse(QRectF(-radius, -radius * overlay.brushRoundness,
                                radius * 2, radius * 2 * overlay.brushRoundness));
        painter.setClipPath(brush.map(shape), Qt::IntersectClip);
    }
    static constexpr QPainter::CompositionMode modes[] = {
        QPainter::CompositionMode_SourceOver, QPainter::CompositionMode_Darken,
        QPainter::CompositionMode_Lighten, QPainter::CompositionMode_Difference};
    painter.setCompositionMode(modes[std::clamp(overlay.mode, 0, 3)]);
    painter.setOpacity(overlay.opacity / 100.0);
    painter.translate(overlay.destinationAnchor);
    painter.rotate(-overlay.angle);
    painter.scale(overlay.scaleX, overlay.scaleY);
    painter.translate(-overlay.sourceAnchor);
    painter.drawImage(QRectF(QPointF(overlay.origin), QSizeF(overlay.source.size()) * overlay.sourceStep),
                      overlay.source);
    painter.restore();
}

} // namespace pictura
