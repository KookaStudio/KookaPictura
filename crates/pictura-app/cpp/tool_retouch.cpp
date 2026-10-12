// The retouch tools: Blur and Sharpen (`pictura_paint::focus`), Smudge
// (`pictura_paint::smudge`), and Dodge, Burn, and Sponge (`pictura_paint::tone`). Each works on
// the pixels already there, more with every pass. It begins a per-dab stroke
// (`cxxqt_object/paint_tools.rs`) and then shares the Brush's live stroke:
// `paint_dab` per move, `end_paint` on release (one history state). Ported
// from photorust's retouch stroke handling.

#include "tool_handler.h"

#include "api/paint_api.h"
#include "paint_tip.h"

#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QObject>

#include <memory>
#include <optional>

namespace pictura {

namespace {

class RetouchToolHandler : public ToolHandler {
public:
    explicit RetouchToolHandler(ToolId id)
        : id_(id)
    {
    }

    bool onPress(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        stroke_.emplace(*v, [&] { return begin(ctx, *v); });
        if (!stroke_->open()) {
            stroke_.reset();
            if (activePixelLocked(v)) {
                ctx.refused(QObject::tr("Could not retouch: the layer's pixels are locked."));
            } else if (!v->active_layer_visible()) {
                ctx.refused(QObject::tr("Could not retouch: the active layer is invisible."));
            } else if (v->active_layer_path().isEmpty()) {
                ctx.refused(QObject::tr("Could not retouch: select a single layer first."));
            }
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        stroke_->dab(imagePos.x(), imagePos.y());
        return true;
    }

    void onMove(ToolContext& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging() || !stroke_) {
            return;
        }
        stroke_->dab(imagePos.x(), imagePos.y());
    }

    void onRelease(ToolContext& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        if (stroke_) {
            stroke_->finish();
            stroke_.reset();
        }
    }

private:
    bool begin(ToolContext& ctx, PictureView& v) const
    {
        if (id_ == ToolId::Dodge || id_ == ToolId::Burn || id_ == ToolId::Sponge) {
            const ToneOptions o = ctx.toneOptions(id_);
            const int tool = id_ == ToolId::Burn ? 1 : id_ == ToolId::Sponge ? 2 : 0;
            return begin_tone(v, paintTip(ctx), tool, o.amount, o.range, o.protectTones,
                              o.spongeMode, o.vibrance);
        }
        const RetouchOptions o = ctx.retouchOptions(id_);
        if (id_ == ToolId::Smudge) {
            return begin_smudge(v, ctx.foreground().rgba(), paintTip(ctx), o.strength, o.mode,
                                o.sampleAllLayers, o.fingerPainting);
        }
        return begin_focus(v, paintTip(ctx), id_ == ToolId::Sharpen, o.strength, o.mode,
                           o.sampleAllLayers, o.protectDetail);
    }

    ToolId id_;
    std::optional<paint_api::Stroke> stroke_;
};

} // namespace

std::unique_ptr<ToolHandler> makeRetouchToolHandler(ToolId id)
{
    return std::make_unique<RetouchToolHandler>(id);
}

} // namespace pictura
