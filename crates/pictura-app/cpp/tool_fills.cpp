// The fill tools (`pictura_paint::gradient`, `pictura_paint::bucket`). The
// Gradient drags out an axis and draws the chosen gradient along it on
// release; the Paint Bucket fills the colours matching a click with the
// foreground colour or a pattern. Both fill the active layer through the
// selection. Ported from photorust's CanvasView gradient drag and fillBucket.

#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include <QtCore/QLineF>
#include <QtCore/QObject>
#include <QtCore/QtMath>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

void reportRefusal(ToolServices& ctx, PictureView* v)
{
    if (activePixelLocked(v)) {
        ctx.refused(QObject::tr("Could not fill: the layer's pixels are locked."));
    } else if (!v->active_layer_visible()) {
        ctx.refused(QObject::tr("Could not fill: the active layer is invisible."));
    } else if (v->active_layer_path().isEmpty()) {
        ctx.refused(QObject::tr("Could not fill: select a single layer first."));
    }
}

// Nothing is drawn until the release: only the axis follows the cursor. A
// click without a drag draws nothing.
class GradientToolHandler : public ToolHandler {
public:
    explicit GradientToolHandler(std::shared_ptr<GradientGroup> gradient)
        : gradient_(std::move(gradient))
    {
        addOptions(gradient_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        start_ = imagePos;
        ctx.setDragging(true);
        showAxis(ctx, imagePos);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (ctx.dragging()) {
            showAxis(ctx, constrained(imagePos, mods));
        }
    }

    void onRelease(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        hideAxis(ctx);
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        const QPointF end = constrained(imagePos, mods);
        const GradientOptions o = gradient_->get();
        if (!draw_gradient(*v, o.preset, ctx.foreground().rgba(), ctx.background().rgba(), o.style,
                           o.mode, o.opacity, o.reverse, o.dither, o.transparency, start_.x(),
                           start_.y(), end.x(), end.y())) {
            reportRefusal(ctx, v);
        }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (ctx.dragging()) {
            ctx.setDragging(false);
            hideAxis(ctx);
        }
    }

private:
    // Shift snaps the axis to 45° steps, keeping its length.
    QPointF constrained(const QPointF& pos, Qt::KeyboardModifiers mods) const
    {
        const QLineF axis(start_, pos);
        if (!mods.testFlag(Qt::ShiftModifier) || axis.length() < 1e-6) {
            return pos;
        }
        const double step = M_PI / 4.0;
        const double angle = std::round(std::atan2(axis.dy(), axis.dx()) / step) * step;
        return start_ + QPointF(std::cos(angle), std::sin(angle)) * axis.length();
    }

    // The canvas's measuring line doubles as the drag axis: the Ruler shows its
    // own only while it is the active tool.
    void showAxis(ToolServices& ctx, const QPointF& end) const
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->setRulerLine(QLineF(start_, end));
        }
    }

    void hideAxis(ToolServices& ctx) const
    {
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearRulerLine();
        }
    }

    QPointF start_;
    std::shared_ptr<GradientGroup> gradient_;
};

class PaintBucketToolHandler : public ToolHandler {
public:
    explicit PaintBucketToolHandler(std::shared_ptr<BucketGroup> bucket)
        : bucket_(std::move(bucket))
    {
        addOptions(bucket_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        const BucketOptions o = bucket_->get();
        if (!bucket_fill_at(*v, qFloor(imagePos.x()), qFloor(imagePos.y()),
                            ctx.foreground().rgba(), o.fill == 1 ? o.pattern : -1, o.mode,
                            o.opacity, o.tolerance, o.antialias, o.contiguous, o.allLayers)) {
            reportRefusal(ctx, v);
        }
        return true;
    }

private:
    std::shared_ptr<BucketGroup> bucket_;
};

} // namespace

std::unique_ptr<ToolHandler> makeGradientToolHandler(std::shared_ptr<GradientGroup> gradient)
{
    return std::make_unique<GradientToolHandler>(std::move(gradient));
}

std::unique_ptr<ToolHandler> makePaintBucketToolHandler(std::shared_ptr<BucketGroup> bucket)
{
    return std::make_unique<PaintBucketToolHandler>(std::move(bucket));
}

} // namespace pictura
