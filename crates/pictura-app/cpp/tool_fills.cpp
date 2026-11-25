// The fill tools (`pictura_paint::gradient`, `pictura_paint::bucket`). The
// Gradient drags out an axis and draws the chosen gradient along it on
// release; the Paint Bucket fills the colours matching a click with the
// foreground colour or a pattern. Both fill the active layer through the
// selection. Ported from photorust's CanvasView gradient drag and fillBucket.

#include "tool_handler.h"

#include "document_command.h"
#include "gradient_editor_dialog.h"
#include "image_view.h"
#include "memory_refusal.h"
#include "tools.h"

#include "api/paint_bridge.h"

#include <QtCore/QLineF>
#include <QtCore/QObject>
#include <QtCore/QtMath>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

// The window a fill's job progress is shown over.
QWidget* jobParent(ToolServices& ctx)
{
    return ctx.canvas() ? ctx.canvas()->window() : nullptr;
}

void reportRefusal(ToolServices& ctx, PictureView* v)
{
    // The fill's memory message covers a memory or a scratch-full refusal; an
    // unreadable-tile refusal falls through to the layer checks, as before.
    const pictura::Refusal reason = last_refusal();
    if (reason == pictura::Refusal::Memory || reason == pictura::Refusal::ScratchFull) {
        ctx.refused(fillMemoryRefusalMessage());
    } else if (activePixelLocked(v)) {
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
        const GradientSpec spec = o.gradient.isEmpty() ? GradientSpec{} : parseGradientSpec(o.gradient);
        const QString stops = spec.stops.isEmpty() ? QString() : formatGradientSpecStops(spec);
        const QRgb foreground = ctx.foreground().rgba();
        const QRgb background = ctx.background().rgba();
        // A large document's gradient runs as a job behind a progress window
        // (#1300).
        const bool drawn = runDocumentCommand(jobParent(ctx), *v, QObject::tr("Gradient"), [&] {
            draw_gradient(*v, o.preset, stops, foreground, background, o.style, o.mode, o.opacity,
                          o.reverse, o.dither, o.transparency, start_.x(), start_.y(), end.x(),
                          end.y());
        });
        if (!drawn) {
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
        const QRgb foreground = ctx.foreground().rgba();
        const bool filled = runDocumentCommand(jobParent(ctx), *v, QObject::tr("Paint Bucket"), [&] {
            bucket_fill_at(*v, qFloor(imagePos.x()), qFloor(imagePos.y()), foreground,
                           o.fill == 1 ? o.pattern : -1, o.mode, o.opacity, o.tolerance,
                           o.antialias, o.contiguous, o.allLayers);
        });
        if (!filled) {
            reportRefusal(ctx, v);
        }
        return true;
    }

private:
    std::shared_ptr<BucketGroup> bucket_;
};

} // namespace

QString fillMemoryRefusalMessage()
{
    // The reason comes from the seam; only the wording is the fill's.
    const QString memory = QObject::tr("Could not fill: not enough memory. Make a smaller "
                                       "selection, close other documents, or free memory and try "
                                       "again.");
    const QString scratchFull = QObject::tr("Could not fill: the scratch disks are full. Free "
                                            "space on the scratch disks, then try again.");
    const QString unreadable = QObject::tr("Could not fill: image data could not be read back "
                                           "from the scratch disk. Check the scratch disks, then "
                                           "try again.");
    const Refusal reason = last_refusal();
    const QString message =
        refusalMessage(reason, memory, scratchFull, unreadable);
    // Callers only ask after a refusal; a missing reason keeps the memory copy.
    return message.isEmpty() ? memory : message;
}

std::unique_ptr<ToolHandler> makeGradientToolHandler(std::shared_ptr<GradientGroup> gradient)
{
    return std::make_unique<GradientToolHandler>(std::move(gradient));
}

std::unique_ptr<ToolHandler> makePaintBucketToolHandler(std::shared_ptr<BucketGroup> bucket)
{
    return std::make_unique<PaintBucketToolHandler>(std::move(bucket));
}

} // namespace pictura
