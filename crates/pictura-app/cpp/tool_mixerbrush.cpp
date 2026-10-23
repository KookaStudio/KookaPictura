// Mixer Brush: wet paint that mixes the brush's reservoir with the canvas under
// it (Wet / Load / Mix / Flow). The stroke is the Brush's live stroke begun
// with `begin_mixer_brush` (`cxxqt_object/paint_tools.rs`); the reservoir lives
// on the controller so it carries from one stroke to the next unless the bar's
// Load / Clean after-stroke toggles say otherwise. Alt-click loads the brush
// from the image. Ported from photorust's mixer stroke handling.

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

class MixerBrushToolHandler : public ToolHandler {
public:
    MixerBrushToolHandler(std::shared_ptr<PaintOptions> paint, std::shared_ptr<MixerGroup> mixer)
        : paint_(std::move(paint))
        , mixer_(std::move(mixer))
    {
        addOptions(paint_.get());
        addOptions(mixer_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        if (mods.testFlag(Qt::AltModifier)) {
            // ponytail: loads one solid colour; CS6's sampled-variation load
            // (and Load Solid Colors Only) is not modelled.
            setReservoir(QColor::fromRgba(
                v->sample_argb(qRound(imagePos.x()), qRound(imagePos.y()))));
            return true;
        }
        const MixerOptions o = mixer_->get();
        // ponytail: Sample All Layers is not wired; the active layer is sampled.
        {
            pictura::ScopedTimer beginTimer("cxx_begin_mixer_brush (call)");
            stroke_.emplace(*v, [&] {
                return begin_mixer_brush(*v, mixer_->get().reservoir.rgba(), paintTip(*paint_),
                                         o.wet, o.load, o.mix, o.flow, paint_->opacity(),
                                         paint_->mode());
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
        PictureView* v = ctx.view();
        if (!v || !stroke_) {
            return;
        }
        const QColor carried = QColor::fromRgba(mixer_reservoir(*v));
        {
            pictura::ScopedTimer t("cxx_end_paint (call)");
            stroke_->finish();
            stroke_.reset();
        }
        // Clean wins over Load when both are on: a cleaned brush is not then
        // reloaded.
        const MixerOptions o = mixer_->get();
        if (o.cleanAfterStroke) {
            setReservoir(QColor(Qt::transparent));
        } else if (o.loadAfterStroke) {
            setReservoir(ctx.foreground());
        } else {
            setReservoir(carried);
        }
    }

private:
    void setReservoir(const QColor& color)
    {
        MixerOptions options = mixer_->get();
        options.reservoir = color;
        mixer_->set(options);
    }

    std::optional<paint_api::Stroke> stroke_;
    std::shared_ptr<PaintOptions> paint_;
    std::shared_ptr<MixerGroup> mixer_;
};

} // namespace

std::unique_ptr<ToolHandler> makeMixerBrushToolHandler(std::shared_ptr<PaintOptions> paint,
                                                       std::shared_ptr<MixerGroup> mixer)
{
    return std::make_unique<MixerBrushToolHandler>(std::move(paint), std::move(mixer));
}

} // namespace pictura
