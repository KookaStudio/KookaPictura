#include "tool_handler.h"

#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QRectF>
#include <QtGui/QPolygonF>

#include <algorithm>
#include <memory>

namespace pictura {

namespace {

// The Hand tool pans through ImageView's pan-enabled mode; no pointer hook is
// needed here.
class HandToolHandler : public ToolHandler {
};

// A drag shorter than this many device pixels in both axes is a click, not a
// marquee; the click keeps the in/out step behaviour.
constexpr double kClickSlopPx = 4.0;

class ZoomToolHandler : public ToolHandler {
public:
    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ImageView* canvas = ctx.canvas();
        PictureView* view = ctx.view();
        if (!canvas || !view || !view->has_document()) {
            return true;
        }
        anchor_ = imagePos;
        last_ = imagePos;
        marquee_ = QRectF(imagePos, imagePos);
        // Ctrl or Alt switches to zoom-out; the step is deferred to release so
        // a drag can draw a marquee instead.
        out_ = mods.testFlag(Qt::ControlModifier) || mods.testFlag(Qt::AltModifier);
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        if (canvas->spacePanForTest()) {
            // Space held mid-marquee repositions it: translate the whole
            // rectangle instead of resizing it from the anchor.
            marquee_.translate(imagePos - last_);
        } else {
            marquee_ = QRectF(anchor_, imagePos).normalized();
        }
        last_ = imagePos;
        canvas->setSelectionPreview({QPolygonF(marquee_)}, true, /*solid=*/true);
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        Q_UNUSED(imagePos);
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return;
        }
        canvas->clearSelectionPreview();

        const double screenW = marquee_.width() * canvas->zoom();
        const double screenH = marquee_.height() * canvas->zoom();
        if (screenW >= kClickSlopPx || screenH >= kClickSlopPx) {
            zoomToMarquee(*canvas);
            return;
        }
        // A plain click (no drag) keeps the in/out step: zoom-in stays anchored
        // on the clicked image point, zoom-out steps at the canvas centre.
        const QPointF anchor = out_
            ? QPointF(canvas->width() / 2.0, canvas->height() / 2.0)
            : canvas->imageToWidget(anchor_);
        canvas->setZoom(canvas->zoom() * (out_ ? 1.0 / 1.2 : 1.2), anchor);
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
    }

    // The Zoom tool's canvas menu is the toolbox preset menu; the frame owns it.
    bool hasCanvasContextMenu() const override { return true; }
    void onContextMenu(ToolServices& ctx, const QPointF& imagePos,
                       const QPoint& globalPos) override
    {
        (void)imagePos;
        ctx.requestZoomMenu(globalPos);
    }

private:
    // Display the marquee area at the highest magnification that still fits it
    // in the viewport, centred.
    void zoomToMarquee(ImageView& canvas) const
    {
        if (marquee_.width() <= 0.0 || marquee_.height() <= 0.0) {
            return;
        }
        const double fit = std::min(canvas.width() / marquee_.width(),
                                    canvas.height() / marquee_.height());
        canvas.setZoom(fit, QPointF(canvas.width() / 2.0, canvas.height() / 2.0));
        const QPointF centre = marquee_.center();
        canvas.setOffset(QPointF(canvas.width() / 2.0 - centre.x() * canvas.zoom(),
                                 canvas.height() / 2.0 - centre.y() * canvas.zoom()));
    }

    QPointF anchor_;
    QPointF last_;
    QRectF marquee_;
    bool out_ = false;
};

} // namespace

std::unique_ptr<ToolHandler> makeHandToolHandler()
{
    return std::make_unique<HandToolHandler>();
}

std::unique_ptr<ToolHandler> makeZoomToolHandler()
{
    return std::make_unique<ZoomToolHandler>();
}

} // namespace pictura
