#include "tool_handler.h"

#include "image_view.h"
#include "preferences.h"

#include <QtCore/QPointer>
#include <QtCore/QTimer>

#include "api/view_bridge.h"

#include <QtCore/QRectF>
#include <QtGui/QPolygonF>

#include <algorithm>
#include <cmath>
#include <memory>
#include <utility>

namespace pictura {

namespace {

// The Hand tool pans through ImageView's pan-enabled mode; no pointer hook is
// needed here. The frame reads Scroll All Windows when it fans a pan out.
class HandToolHandler : public ToolHandler {
public:
    explicit HandToolHandler(std::shared_ptr<HandGroup> hand)
        : hand_(std::move(hand))
    {
        addOptions(hand_.get());
    }

private:
    std::shared_ptr<HandGroup> hand_;
};

// A drag shorter than this many device pixels in both axes is a click, not a
// marquee; the click keeps the in/out step behaviour.
constexpr double kClickSlopPx = 4.0;

class ZoomToolHandler : public ToolHandler {
public:
    explicit ZoomToolHandler(std::shared_ptr<ZoomGroup> zoom)
        : zoom_(std::move(zoom))
    {
        addOptions(zoom_.get());
    }

    ~ZoomToolHandler() override { stop(); }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        stop();
        ImageView* canvas = ctx.canvas();
        if (!canvas || !ctx.view() || !ctx.view()->has_document()) {
            return true;
        }
        context_ = &ctx;
        canvas_ = canvas;
        anchor_ = imagePos;
        lastImage_ = imagePos;
        pressWidget_ = canvas->imageToWidget(imagePos);
        lastWidget_ = pressWidget_;
        marquee_ = QRectF(imagePos, imagePos);
        out_ = mods.testFlag(Qt::ControlModifier) || mods.testFlag(Qt::AltModifier);
        allWindows_ = mods.testFlag(Qt::ShiftModifier);
        moved_ = false;
        animated_ = false;
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        if (animatedZoomAvailable() && loadPreferences().animatedZoom) {
            timer_ = new QTimer(canvas);
            timer_->setObjectName(QStringLiteral("zoomHoldTimer"));
            timer_->setSingleShot(true);
            QObject::connect(timer_, &QTimer::timeout, timer_, [this] { tick(); });
            viewConnection_ = QObject::connect(canvas, &ImageView::viewChanged, timer_, [this] {
                if (!updating_) {
                    moved_ = true;
                    stopTimer();
                }
            });
            timer_->start(350);
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        ImageView* canvas = ctx.canvas();
        if (!ctx.dragging() || !canvas || canvas != canvas_) {
            return;
        }
        const QPointF widget = canvas->imageToWidget(imagePos);
        if (!moved_ && (std::abs(widget.x() - pressWidget_.x()) >= kClickSlopPx
                        || std::abs(widget.y() - pressWidget_.y()) >= kClickSlopPx)) {
            moved_ = true;
            stopTimer();
        }
        if (moved_ && !animated_) {
            if (zoom_->get().scrubby) {
                applyFactor(std::exp((widget.x() - lastWidget_.x()) * 0.01), pressWidget_);
            } else {
                if (canvas->spacePanForTest()) {
                    marquee_.translate(imagePos - lastImage_);
                } else {
                    marquee_ = QRectF(anchor_, imagePos).normalized();
                }
                canvas->setSelectionPreview({QPolygonF(marquee_)}, true, true);
            }
        }
        lastWidget_ = widget;
        lastImage_ = imagePos;
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            stop();
            return;
        }
        ctx.setDragging(false);
        ImageView* canvas = ctx.canvas();
        stopTimer();
        if (canvas && canvas == canvas_) {
            canvas->clearSelectionPreview();
            if (!animated_ && !moved_) {
                applyFactor(out_ ? 1.0 / 1.2 : 1.2,
                            out_ ? QPointF(canvas->width() / 2.0, canvas->height() / 2.0)
                                 : pressWidget_);
            } else if (!animated_ && moved_ && !zoom_->get().scrubby) {
                const double before = canvas->zoom();
                if (marquee_.width() > 0.0 && marquee_.height() > 0.0) {
                    const double fit = std::min(canvas->width() / marquee_.width(),
                                                canvas->height() / marquee_.height());
                    canvas->setZoom(fit, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
                    const QPointF centre = marquee_.center();
                    canvas->setOffset(QPointF(canvas->width() / 2.0 - centre.x() * canvas->zoom(),
                                              canvas->height() / 2.0 - centre.y() * canvas->zoom()));
                    notify(before);
                }
            }
        }
        stop();
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ImageView* canvas = ctx.canvas()) canvas->clearSelectionPreview();
        ctx.setDragging(false);
        stop();
    }
    void onSessionCancelled(ToolServices& ctx) override
    {
        if (context_) onDeactivate(ctx);
    }
    bool onEscape() override
    {
        if (!context_) return false;
        onDeactivate(*context_);
        return true;
    }
    bool hasCanvasContextMenu() const override { return true; }
    void onContextMenu(ToolServices& ctx, const QPointF&, const QPoint& globalPos) override
    {
        ctx.requestZoomMenu(globalPos);
    }

private:
    void notify(double before)
    {
        if (!canvas_ || !context_ || before <= 0.0) return;
        const auto options = zoom_->get();
        context_->notifyToolZoom(canvas_->zoom() / before, allWindows_ || options.zoomAll,
                                 options.resizeToFit);
    }
    void applyFactor(double factor, const QPointF& anchor)
    {
        if (!canvas_ || !context_) return;
        const double before = canvas_->zoom();
        updating_ = true;
        canvas_->setZoom(before * factor, canvas_->viewRotation().inverted().map(anchor));
        notify(before);
        updating_ = false;
        if (animated_ && canvas_ && canvas_->zoom() == before) stopTimer();
    }
    void tick()
    {
        if (!timer_ || !canvas_ || !context_ || context_->canvas() != canvas_
            || !context_->view() || !context_->view()->has_document() || moved_) {
            stop();
            return;
        }
        animated_ = true;
        const QPointF anchor = out_ ? QPointF(canvas_->width() / 2.0, canvas_->height() / 2.0)
                                    : pressWidget_;
        applyFactor(out_ ? 1.0 / 1.04 : 1.04, anchor);
        if (timer_) {
            timer_->setSingleShot(false);
            timer_->start(16);
        }
    }
    void stopTimer()
    {
        QObject::disconnect(viewConnection_);
        if (timer_) {
            timer_->stop();
            timer_->setObjectName(QString());
            timer_->disconnect();
            timer_->deleteLater();
            timer_ = nullptr;
        }
    }
    void stop()
    {
        stopTimer();
        context_ = nullptr;
        canvas_.clear();
    }
    QPointF anchor_, lastImage_, pressWidget_, lastWidget_;
    QRectF marquee_;
    bool out_ = false, allWindows_ = false, moved_ = false, animated_ = false, updating_ = false;
    ToolServices* context_ = nullptr;
    QPointer<ImageView> canvas_;
    QPointer<QTimer> timer_;
    QMetaObject::Connection viewConnection_;
    std::shared_ptr<ZoomGroup> zoom_;
};

} // namespace

std::unique_ptr<ToolHandler> makeHandToolHandler(std::shared_ptr<HandGroup> hand)
{
    return std::make_unique<HandToolHandler>(std::move(hand));
}

std::unique_ptr<ToolHandler> makeZoomToolHandler(std::shared_ptr<ZoomGroup> zoom)
{
    return std::make_unique<ZoomToolHandler>(std::move(zoom));
}

} // namespace pictura
