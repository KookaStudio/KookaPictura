#include "tools.h"
#include "image_view.h"
#include "api/paint_bridge.h"

#include <QtGui/QTransform>
#include <QtCore/QTimer>
#include <cstdint>

namespace pictura {

bool ToolController::cloneOverlayPendingForTest() const
{
    return cloneOverlayTimer_ && cloneOverlayTimer_->isActive();
}

void ToolController::refreshCloneOverlay()
{
    if (!canvas_) return;
    PictureView* v = view();
    const auto clear = [this, v] {
        canvas_->clearCloneOverlay();
        cloneOverlayTimer_->stop();
        cloneOverlayImage_ = {};
        cloneOverlayInverted_ = {};
        cloneOverlayImageId_ = 0;
        if (v) clone_overlay_cancel(*v);
    };
    if (!v || !v->has_document() || !clone_ || !clonePointerValid_
        || (active_ != ToolId::CloneStamp && active_ != ToolId::HealingBrush)
        || !QRectF(0, 0, v->document_width(), v->document_height()).contains(clonePointer_)) {
        clear();
        return;
    }
    const CloneSource& slot = clone_->active();
    if (!slot.hasSource || (active_ == ToolId::HealingBrush && !slot.healingSourceValid)
        || !QRect(0, 0, v->document_width(), v->document_height()).contains(slot.source)
        || !slot.overlayShow || slot.overlayOpacity == 0
        || (slot.overlayAutoHide && dragging_)) {
        clear();
        return;
    }
    const auto* stamp = optionsFor<StampGroup>(ToolId::CloneStamp);
    const auto* healing = optionsFor<HealingGroup>(ToolId::HealingBrush);
    const bool aligned = active_ == ToolId::CloneStamp ? stamp->get().cloneAligned
                                                       : healing->get().aligned;
    const bool anchored = slot.hasOffset && (aligned || dragging_);
    const QPointF destination = anchored ? QPointF(slot.anchor) + QPointF(.5, .5)
                                         : QPointF(qRound(clonePointer_.x()) + .5,
                                                   qRound(clonePointer_.y()) + .5);
    const QPointF source = anchored ? QPointF(slot.anchor + slot.offset) + QPointF(.5, .5)
                                    : QPointF(slot.source) + QPointF(.5, .5);
    const double sx = slot.width / 100.0 * (slot.flipH ? -1.0 : 1.0);
    const double sy = slot.height / 100.0 * (slot.flipV ? -1.0 : 1.0);
    if (sx == 0 || sy == 0) { clear(); return; }
    QTransform mapping;
    mapping.translate(destination.x(), destination.y());
    mapping.rotate(-slot.angle);
    mapping.scale(sx, sy);
    mapping.translate(-source.x(), -source.y());
    bool invertible = false;
    const QTransform inverse = mapping.inverted(&invertible);
    if (!invertible) { clear(); return; }
    QRectF visible = canvas_->visibleDocumentRect();
    if (slot.overlayClipped) {
        const double r = paint_->size() / 2.0 + 2.0;
        visible = visible.intersected(QRectF(clonePointer_.x() - r, clonePointer_.y() - r, 2*r, 2*r));
    }
    const QRect sourceRect = inverse.mapRect(visible).adjusted(-2, -2, 2, 2).toAlignedRect();
    int left = 0, top = 0, step = 1;
    std::uint64_t imageId = 0;
    bool busy = false;
    const StampOptions stampOptions = stamp->get();
    const int scope = active_ == ToolId::CloneStamp ? stampOptions.cloneSample : 0;
    QImage fresh = clone_overlay_source(*v, sourceRect.x(), sourceRect.y(), sourceRect.width(),
                                        sourceRect.height(), scope, stampOptions.ignoreAdjustments,
                                        active_ == ToolId::CloneStamp && dragging_, left, top, step,
                                        imageId, busy);
    if (busy && !cloneOverlayTimer_->isActive()) cloneOverlayTimer_->start();
    if (!busy) cloneOverlayTimer_->stop();
    if (!fresh.isNull()) {
        cloneOverlayImage_ = fresh;
        cloneOverlayInverted_ = {};
        cloneOverlayImageId_ = imageId;
    }
    if (imageId == 0 || imageId != cloneOverlayImageId_ || cloneOverlayImage_.isNull()) {
        canvas_->clearCloneOverlay();
        return;
    }
    if (slot.overlayInvert && cloneOverlayInverted_.isNull()) {
        cloneOverlayInverted_ = cloneOverlayImage_.convertToFormat(QImage::Format_RGBA8888);
        cloneOverlayInverted_.invertPixels(QImage::InvertRgb);
    }
    const QImage& image = slot.overlayInvert ? cloneOverlayInverted_ : cloneOverlayImage_;
    ImageView::CloneOverlay overlay;
    overlay.source = image;
    overlay.origin = QPoint(left, top);
    overlay.sourceStep = step;
    overlay.sourceAnchor = source;
    overlay.destinationAnchor = destination;
    overlay.pointer = clonePointer_;
    overlay.scaleX = sx;
    overlay.scaleY = sy;
    overlay.angle = slot.angle;
    overlay.brushSize = paint_->size();
    overlay.brushRoundness = active_ == ToolId::HealingBrush || paint_->roundness() == 0
        ? 1.0 : paint_->roundness() / 100.0;
    overlay.brushAngle = active_ == ToolId::HealingBrush ? 0 : paint_->angle();
    overlay.opacity = slot.overlayOpacity;
    overlay.mode = slot.overlayMode;
    overlay.clipped = slot.overlayClipped;
    canvas_->setCloneOverlay(overlay);
}

} // namespace pictura
