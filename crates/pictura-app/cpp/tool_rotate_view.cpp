// The Rotate View tool (CS6 TOOL-043, `docs/03-tools/rotate-view.md`): a drag
// turns the canvas about its centre without touching the document, with a
// compass whose red needle points to the document's top; Shift snaps to 15°
// steps and Esc resets the view. The options bar's Rotation Angle field, dial,
// and Reset View set the same angle through the controller. No photorust
// source; implemented fresh.
//
// ponytail: no Rotate All Windows or trackpad rotate gestures; the angle
// belongs to the canvas widget, so it is shared by the document tabs.

#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include <QtCore/QtMath>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

class RotateViewToolHandler : public ToolHandler {
public:
    void onActivate(ToolServices& ctx) override { ctx_ = &ctx; }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setCompassVisible(false);
        }
        dragging_ = false;
        ctx_ = nullptr;
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        ctx_ = &ctx;
        ImageView* canvas = ctx.canvas();
        if (!canvas) {
            return true;
        }
        startRotation_ = canvas->rotation();
        pressAngle_ = angleAt(*canvas, imagePos);
        dragging_ = true;
        canvas->setCompassVisible(true);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ImageView* canvas = ctx.canvas();
        if (!dragging_ || !canvas) {
            return;
        }
        double angle = startRotation_ + angleAt(*canvas, imagePos) - pressAngle_;
        if (mods.testFlag(Qt::ShiftModifier)) {
            angle = std::round(angle / 15.0) * 15.0;
        }
        ctx.setViewRotation(angle);
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        dragging_ = false;
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setCompassVisible(false);
        }
    }

    // Esc: Reset View.
    bool onEscape() override
    {
        if (!ctx_) {
            return false;
        }
        ctx_->setViewRotation(0.0);
        return true;
    }

private:
    // The pointer's direction from the canvas centre, in widget degrees. The
    // image point maps back to the exact widget point under the rotation it
    // was taken at, before this move changes it.
    static double angleAt(const ImageView& canvas, const QPointF& imagePos)
    {
        const QPointF w = canvas.imageToWidget(imagePos);
        return qRadiansToDegrees(
            std::atan2(w.y() - canvas.height() / 2.0, w.x() - canvas.width() / 2.0));
    }

    ToolServices* ctx_ = nullptr;
    bool dragging_ = false;
    double startRotation_ = 0.0;
    double pressAngle_ = 0.0;
};

} // namespace

std::unique_ptr<ToolHandler> makeRotateViewToolHandler()
{
    return std::make_unique<RotateViewToolHandler>();
}

} // namespace pictura
