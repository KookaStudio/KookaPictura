// The stamp tools: Clone Stamp, Pattern Stamp, History Brush, and Art History
// Brush. Each begins a Brush stroke whose colour comes from a source
// (`cxxqt_object/paint_tools.rs`) and then shares the Brush's live stroke:
// `paint_dab` per move, `end_paint` on release (one history state). Ported
// from photorust's clone / pattern stroke handling; the History Brush and Art
// History Brush have no photorust source.

#include "tool_handler.h"

#include "api/paint_api.h"
#include "paint_tip.h"

#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QGuiApplication>

#include <memory>
#include <optional>

namespace pictura {

namespace {

// The time between dabs while the button is held still: 20 dabs per second.
// CS6's rate is undocumented, so this approximates it; spacing is a distance
// and cannot supply one.
constexpr int kHoldDabMs = 50;

// The press / drag / release shared by the stamps; `begin` starts the tool's
// stroke and reports whether it did. A tool whose `holdPaints` is on keeps
// dabbing from a timer until the stroke ends.
class StampToolHandler : public ToolHandler {
public:
    explicit StampToolHandler(std::shared_ptr<PaintOptions> paint)
        : paint_(std::move(paint))
    {
        addOptions(paint_.get());
        holdTimer_.setInterval(kHoldDabMs);
        QObject::connect(&holdTimer_, &QTimer::timeout, &holdTimer_, [this] {
            if (stroke_) {
                stroke_->hold();
            }
        });
        // A release can be lost with the focus; the stroke then waits for the
        // next release or tool switch, but stops painting now.
        QObject::connect(qGuiApp, &QGuiApplication::applicationStateChanged, &holdTimer_,
                         [this](Qt::ApplicationState state) {
                             if (state != Qt::ApplicationActive) {
                                 holdTimer_.stop();
                             }
                         });
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        if (mods.testFlag(Qt::AltModifier)) {
            onAltPress(ctx, imagePos);
            return true;
        }
        stroke_.emplace(*v, [&] { return begin(ctx, *v, imagePos); });
        if (!stroke_->open()) {
            stroke_.reset();
            if (activePixelLocked(v)) {
                ctx.refused(QObject::tr("Could not paint: the layer's pixels are locked."));
            } else if (!v->active_layer_visible()) {
                ctx.refused(QObject::tr("Could not paint: the active layer is invisible."));
            } else if (v->active_layer_path().isEmpty()) {
                ctx.refused(QObject::tr("Could not paint: select a single layer first."));
            } else {
                refuseSource(ctx);
            }
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        stroke_->dab(imagePos.x(), imagePos.y());
        if (holdPaints()) {
            holdTimer_.start();
        }
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging() || !stroke_) {
            return;
        }
        stroke_->dab(imagePos.x(), imagePos.y());
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        holdTimer_.stop();
        if (stroke_) {
            stroke_->finish();
            stroke_.reset();
        }
    }

    // As with the Brush: a tool switch commits an open stroke, so the Stroke
    // never outlives the view it calls back into.
    void onDeactivate(ToolServices& ctx) override
    {
        holdTimer_.stop();
        if (stroke_) {
            stroke_->finish();
            stroke_.reset();
        }
        ctx.setDragging(false);
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        holdTimer_.stop();
        if (stroke_) {
            stroke_->abandon();
            stroke_.reset();
        }
        ctx.setDragging(false);
    }

protected:
    virtual bool begin(ToolServices& ctx, PictureView& v, const QPointF& imagePos) = 0;
    // A refusal the layer checks do not explain, e.g. a missing source.
    virtual void refuseSource(ToolServices&) {}
    // Whether a stroke keeps dabbing while the pointer is still.
    virtual bool holdPaints() const = 0;

    // Alt samples the foreground, as with the Brush.
    virtual void onAltPress(ToolServices& ctx, const QPointF& imagePos)
    {
        PictureView* v = ctx.view();
        const quint32 argb = v->sample_argb(qRound(imagePos.x()), qRound(imagePos.y()));
        if (argb != 0) {
            ctx.sampledForeground(QColor::fromRgb(argb));
        }
    }

protected:
    std::shared_ptr<PaintOptions> paint_;

private:
    std::optional<paint_api::Stroke> stroke_;
    QTimer holdTimer_;
};

// Clone Stamp: Alt-click sets the active Clone Source slot's source point;
// each stroke paints what lies at the source offset, through the slot's
// transform about the point the offset was measured at. Aligned keeps the
// offset across strokes; unchecked, every stroke copies from the source point
// again.
// ponytail: no source overlay and no on-canvas source crosshair.
class CloneStampToolHandler : public StampToolHandler {
public:
    CloneStampToolHandler(std::shared_ptr<PaintOptions> paint, std::shared_ptr<StampGroup> stamp,
                          std::shared_ptr<CloneSourceOptions> clone)
        : StampToolHandler(std::move(paint))
        , stamp_(std::move(stamp))
        , clone_(std::move(clone))
    {
        addOptions(stamp_.get());
        addOptions(clone_.get());
    }

protected:
    void onAltPress(ToolServices& ctx, const QPointF& imagePos) override
    {
        CloneSource slot = clone_->active();
        slot.source = QPoint(qRound(imagePos.x()), qRound(imagePos.y()));
        slot.hasSource = true;
        slot.hasOffset = false;
        clone_->setActive(slot);
    }

    bool begin(ToolServices& ctx, PictureView& v, const QPointF& imagePos) override
    {
        CloneSource slot = clone_->active();
        if (!slot.hasSource) {
            return false;
        }
        const StampOptions o = stamp_->get();
        if (!o.cloneAligned || !slot.hasOffset) {
            slot.anchor = QPoint(qRound(imagePos.x()), qRound(imagePos.y()));
            slot.offset = slot.source - slot.anchor;
            slot.hasOffset = true;
            clone_->setActive(slot);
        }
        const double sx = slot.width / 100.0 * (slot.flipH ? -1.0 : 1.0);
        const double sy = slot.height / 100.0 * (slot.flipV ? -1.0 : 1.0);
        return begin_clone_stamp(v, paintTip(*paint_), paint_->opacity(), paint_->flow(),
                                 paint_->mode(), slot.offset.x(), slot.offset.y(),
                                 slot.anchor.x() + 0.5, slot.anchor.y() + 0.5, sx, sy,
                                 slot.angle, o.cloneSample, o.ignoreAdjustments);
    }

    bool holdPaints() const override { return stamp_->get().cloneAirbrush; }

    void refuseSource(ToolServices& ctx) override
    {
        ctx.refused(clone_->active().hasSource
                        ? QObject::tr("Clone Stamp: paint away from the source point.")
                        : QObject::tr("Clone Stamp: Alt-click to set a source point first."));
    }

private:
    std::shared_ptr<StampGroup> stamp_;
    std::shared_ptr<CloneSourceOptions> clone_;
};

// Pattern Stamp: paints the chosen pattern, pinned to the document when
// Aligned and to each stroke's start otherwise.
// ponytail: built-in generated patterns only; Impressionist is not wired.
class PatternStampToolHandler : public StampToolHandler {
public:
    PatternStampToolHandler(std::shared_ptr<PaintOptions> paint, std::shared_ptr<StampGroup> stamp)
        : StampToolHandler(std::move(paint))
        , stamp_(std::move(stamp))
    {
        addOptions(stamp_.get());
    }

protected:
    bool begin(ToolServices& ctx, PictureView& v, const QPointF& imagePos) override
    {
        const StampOptions o = stamp_->get();
        const QPoint origin =
            o.patternAligned ? QPoint() : QPoint(qRound(imagePos.x()), qRound(imagePos.y()));
        return begin_pattern_stamp(v, paintTip(*paint_), paint_->opacity(), paint_->flow(),
                                   paint_->mode(), o.pattern, origin.x(), origin.y());
    }

    bool holdPaints() const override { return stamp_->get().patternAirbrush; }

private:
    std::shared_ptr<StampGroup> stamp_;
};

// History Brush: paints the active layer as it was in the History panel's
// source state (the oldest state unless another is chosen), in place.
class HistoryBrushToolHandler : public StampToolHandler {
public:
    HistoryBrushToolHandler(std::shared_ptr<PaintOptions> paint, std::shared_ptr<StampGroup> stamp)
        : StampToolHandler(std::move(paint))
        , stamp_(std::move(stamp))
    {
        addOptions(stamp_.get());
    }

protected:
    bool begin(ToolServices& ctx, PictureView& v, const QPointF&) override
    {
        return begin_history_brush(v, paintTip(*paint_), paint_->opacity(), paint_->flow(),
                                   paint_->mode());
    }

    void refuseSource(ToolServices& ctx) override
    {
        ctx.refused(QObject::tr("History Brush: the source state has no matching pixel layer."));
    }

    bool holdPaints() const override { return stamp_->get().historyAirbrush; }

private:
    std::shared_ptr<StampGroup> stamp_;
};

// Art History Brush: stylized strokes coloured from the History Brush's source
// state, shaped by the bar's Style, Area, and Tolerance. It has no Airbrush
// option: it always keeps painting while the button is held.
class ArtHistoryBrushToolHandler : public StampToolHandler {
public:
    ArtHistoryBrushToolHandler(std::shared_ptr<PaintOptions> paint,
                               std::shared_ptr<ArtHistoryGroup> artHistory)
        : StampToolHandler(std::move(paint))
        , artHistory_(std::move(artHistory))
    {
        addOptions(artHistory_.get());
    }

protected:
    bool begin(ToolServices& ctx, PictureView& v, const QPointF&) override
    {
        const ArtHistoryOptions o = artHistory_->get();
        return begin_art_history_brush(v, paintTip(*paint_), paint_->opacity(), o.style, o.area,
                                       o.tolerance);
    }

    bool holdPaints() const override { return true; }

    void refuseSource(ToolServices& ctx) override
    {
        ctx.refused(QObject::tr("Art History Brush: needs an 8-bit document whose source state "
                                "has a matching pixel layer."));
    }

private:
    std::shared_ptr<ArtHistoryGroup> artHistory_;
};

} // namespace

std::unique_ptr<ToolHandler> makeCloneStampToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<StampGroup> stamp,
    std::shared_ptr<CloneSourceOptions> clone)
{
    return std::make_unique<CloneStampToolHandler>(std::move(paint), std::move(stamp),
                                                   std::move(clone));
}

std::unique_ptr<ToolHandler> makePatternStampToolHandler(std::shared_ptr<PaintOptions> paint,
                                                         std::shared_ptr<StampGroup> stamp)
{
    return std::make_unique<PatternStampToolHandler>(std::move(paint), std::move(stamp));
}

std::unique_ptr<ToolHandler> makeHistoryBrushToolHandler(std::shared_ptr<PaintOptions> paint,
                                                         std::shared_ptr<StampGroup> stamp)
{
    return std::make_unique<HistoryBrushToolHandler>(std::move(paint), std::move(stamp));
}

std::unique_ptr<ToolHandler> makeArtHistoryBrushToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<ArtHistoryGroup> artHistory)
{
    return std::make_unique<ArtHistoryBrushToolHandler>(std::move(paint),
                                                        std::move(artHistory));
}

} // namespace pictura
