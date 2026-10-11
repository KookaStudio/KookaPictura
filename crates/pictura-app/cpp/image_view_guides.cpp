// ImageView guides: the document's guides and the one a ruler drag is placing.

#include "image_view.h"

#include <QtGui/QPainter>

#include <cmath>

namespace pictura {

void ImageView::setGuides(const QList<GuideLine>& guides)
{
    if (guides == guides_) {
        return;
    }
    guides_ = guides;
    update();
}

void ImageView::setGuidesVisible(bool on)
{
    if (guidesVisible_ != on) {
        guidesVisible_ = on;
        update();
    }
}

void ImageView::setGuideAppearance(const QColor& color, bool dashed)
{
    if (guideColor_ != color || guidesDashed_ != dashed) {
        guideColor_ = color;
        guidesDashed_ = dashed;
        update();
    }
}

void ImageView::setGuidePreview(const GuideLine& guide)
{
    guidePreview_ = guide;
    guidePreviewActive_ = true;
    update();
}

void ImageView::clearGuidePreview()
{
    if (guidePreviewActive_) {
        guidePreviewActive_ = false;
        update();
    }
}

// Drawn in the view frame so a guide stays one screen pixel wide at any zoom
// and runs past the canvas edge across the whole view, as in CS6.
void ImageView::paintGuides(QPainter& painter)
{
    const bool shown = guidesVisible_ && !guides_.isEmpty();
    if (docSize_.isEmpty() || (!shown && !guidePreviewActive_)) {
        return;
    }
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, false);
    QPen pen(guideColor_, 0, guidesDashed_ ? Qt::DashLine : Qt::SolidLine);
    painter.setPen(pen);
    const QRectF frame = viewRect();
    const auto draw = [&](const GuideLine& guide) {
        if (guide.vertical) {
            const double x = std::floor(guide.position * zoom_ + offset_.x()) + 0.5;
            painter.drawLine(QPointF(x, frame.top()), QPointF(x, frame.bottom()));
        } else {
            const double y = std::floor(guide.position * zoom_ + offset_.y()) + 0.5;
            painter.drawLine(QPointF(frame.left(), y), QPointF(frame.right(), y));
        }
    };
    if (shown) {
        for (const GuideLine& guide : guides_) {
            draw(guide);
        }
    }
    if (guidePreviewActive_) {
        draw(guidePreview_);
    }
    painter.restore();
}

} // namespace pictura
