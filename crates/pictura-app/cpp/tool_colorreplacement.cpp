// Color Replacement tool: paint the foreground over pixels that match the
// sampled colour, keeping what the Mode leaves alone (under Color, each
// pixel's luminosity). The stroke is the Brush's live stroke begun with
// `begin_color_replacement` (`cxxqt_object/paint_tools.rs`). Ported from
// photorust's colour-replacement stroke handling.

#include "tool_handler.h"

#include "api/paint_api.h"
#include "paint_tip.h"
#include "pictura_debug_timing.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QObject>
#include <QtGui/QColor>

#include <memory>
#include <optional>

namespace pictura {

namespace {

class ColorReplacementToolHandler : public ToolHandler {
public:
    bool onPress(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        // Alt samples the replacement colour, as with the Brush.
        if (mods.testFlag(Qt::AltModifier)) {
            const quint32 argb = v->sample_argb(qRound(imagePos.x()), qRound(imagePos.y()));
            if (argb != 0) {
                ctx.sampledForeground(QColor::fromRgb(argb));
            }
            return true;
        }
        const ColorReplaceOptions o = ctx.colorReplaceOptions();
        {
            pictura::ScopedTimer beginTimer("cxx_begin_color_replacement (call)");
            stroke_.emplace(*v, [&] {
                return begin_color_replacement(*v, ctx.foreground().rgba(), ctx.background().rgba(),
                                               paintTip(ctx), o.mode, o.sampling, o.limits,
                                               o.tolerance, o.antialias);
            });
        }
        if (!stroke_->open()) {
            stroke_.reset();
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

    void onMove(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging() || !stroke_) {
            return;
        }
        pictura::ScopedTimer t("cxx_paint_dab (call)");
        stroke_->dab(imagePos.x(), imagePos.y());
    }

    void onRelease(ToolContext& ctx, const QPointF&, Qt::KeyboardModifiers) override
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

private:
    std::optional<paint_api::Stroke> stroke_;
};

} // namespace

std::unique_ptr<ToolHandler> makeColorReplacementToolHandler()
{
    return std::make_unique<ColorReplacementToolHandler>();
}

} // namespace pictura
