#include "tool_handler.h"

#include "api/paint_api.h"
#include "paint_tip.h"
#include "pictura_debug_timing.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QObject>
#include <QtGui/QColor>

#include <memory>
#include <optional>

namespace pictura {

namespace {

// Brush and Pencil share this handler; `aliased_` selects the hard-edged
// Pencil rasterisation.
class BrushToolHandler : public ToolHandler {
public:
    BrushToolHandler(bool aliased, std::shared_ptr<PaintOptions> paint)
        : aliased_(aliased)
        , paint_(std::move(paint))
    {
        addOptions(paint_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        // Transient Alt eyedropper: sample the pointer without starting a
        // stroke, leaving the active tool unchanged.
        if (mods.testFlag(Qt::AltModifier)) {
            const quint32 argb = v->sample_argb(qRound(imagePos.x()), qRound(imagePos.y()));
            if (argb != 0) {
                ctx.sampledForeground(QColor::fromRgb(argb));
            }
            return true;
        }
        {
            pictura::ScopedTimer t("cxx_begin_brush (call)");
            stroke_ = paint_api::beginBrush(*v, quint32(ctx.foreground().rgba()),
                                            quint32(ctx.background().rgba()), paintTip(*paint_),
                                            paint_->opacity(), paint_->flow(), paint_->mode(),
                                            aliased_, paint_->autoErase());
        }
        if (!stroke_) {
            if (activePixelLocked(v)) {
                ctx.refused(QObject::tr("Could not paint: the layer's pixels are locked."));
            } else if (!v->active_layer_visible()) {
                ctx.refused(QObject::tr("Could not paint: the active layer is invisible."));
            } else if (v->active_layer_path().isEmpty()) {
                ctx.refused(QObject::tr("Could not paint: select a single layer first."));
            }
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        {
            pictura::ScopedTimer t("cxx_paint_dab (call)");
            stroke_->dab(imagePos.x(), imagePos.y());
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging() || !stroke_) {
            return;
        }
        pictura::ScopedTimer t("cxx_paint_dab (call)");
        stroke_->dab(imagePos.x(), imagePos.y());
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        if (stroke_) {
            pictura::ScopedTimer t("cxx_end_paint (call)");
            stroke_->finish();
            stroke_.reset();
        }
    }

    // The paint tool's canvas menu is the options-bar Brush Preset picker; the
    // frame owns it.
    bool hasCanvasContextMenu() const override { return true; }
    void onContextMenu(ToolServices& ctx, const QPointF& imagePos,
                       const QPoint& globalPos) override
    {
        (void)imagePos;
        ctx.requestBrushPicker(globalPos);
    }

private:
    bool aliased_;
    std::optional<paint_api::Stroke> stroke_;
    std::shared_ptr<PaintOptions> paint_;
};

}

std::unique_ptr<ToolHandler> makeBrushToolHandler(bool aliased, std::shared_ptr<PaintOptions> paint)
{
    return std::make_unique<BrushToolHandler>(aliased, std::move(paint));
}

} // namespace pictura
