// Healing tools: Spot Healing Brush and Healing Brush. Both accumulate a
// coverage mask over the active layer while the pointer is down and run the
// heal once, on release, so the region is rebuilt from the pixels around it
// (Spot) or from an offset source (Healing). Ported from photorust's healing
// tool handling onto the cxx-qt bridge in `cxxqt_object/healing.rs`.

#include "tool_handler.h"

#include "tools.h"

#include "api/layers_bridge.h"
#include "api/paint_bridge.h"

#include <QtCore/QObject>
#include <QtCore/QPoint>

#include <memory>

namespace pictura {

namespace {

// The refusals shared by both healing tools, matching the Brush press path.
void refuseHeal(ToolServices& ctx, PictureView* v)
{
    if (!pictura::mask_edit_target_get(*v).isEmpty()) {
        ctx.refused(QObject::tr("Could not heal: a layer mask is the active target."));
    } else if (activePixelLocked(v)) {
        ctx.refused(QObject::tr("Could not heal: the layer's pixels are locked."));
    } else if (v->active_layer_path().isEmpty()) {
        ctx.refused(QObject::tr("Could not heal: select a single layer first."));
    } else {
        ctx.refused(QObject::tr("Could not heal: select a single pixel layer first."));
    }
}

// Spot Healing Brush: click or drag over a defect; the engine fills the region
// from its surroundings (Proximity Match / Create Texture / Content-Aware).
class SpotHealingToolHandler : public ToolHandler {
public:
    SpotHealingToolHandler(std::shared_ptr<PaintOptions> paint,
                           std::shared_ptr<HealingGroup> healing)
        : paint_(std::move(paint))
        , healing_(std::move(healing))
    {
        addOptions(paint_.get());
        addOptions(healing_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        if (!healing_begin(*v, paint_->size(), paint_->hardness())) {
            refuseHeal(ctx, v);
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        healing_dab(*v, imagePos.x(), imagePos.y());
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        if (PictureView* v = ctx.view()) {
            healing_dab(*v, imagePos.x(), imagePos.y());
        }
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        if (!healing_commit(*v, healing_->get().spotType, false, 0, 0, false)) {
            healing_cancel(*v);
        }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            healing_cancel(*v);
        }
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            healing_cancel(*v);
        }
    }

private:
    std::shared_ptr<PaintOptions> paint_;
    std::shared_ptr<HealingGroup> healing_;
};

// Healing Brush: Alt-click sets a sample point, then paint to transplant that
// source's texture with the destination's lighting. `Aligned` keeps the sample
// offset across strokes; unchecked, each stroke re-anchors it to its own start.
class HealingBrushToolHandler : public ToolHandler {
public:
    HealingBrushToolHandler(std::shared_ptr<PaintOptions> paint,
                            std::shared_ptr<HealingGroup> healing,
                            std::shared_ptr<CloneSourceOptions> clone)
        : paint_(std::move(paint))
        , healing_(std::move(healing))
        , clone_(std::move(clone))
    {
        addOptions(paint_.get());
        addOptions(healing_.get());
        addOptions(clone_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        if (mods.testFlag(Qt::AltModifier)) {
            CloneSource slot = clone_->active();
            slot.source = QPoint(qRound(imagePos.x()), qRound(imagePos.y()));
            slot.hasSource = true;
            slot.healingSourceValid = true;
            slot.hasOffset = false;
            clone_->setActive(slot);
            return true;
        }
        CloneSource slot = clone_->active();
        if (!slot.hasSource || !slot.healingSourceValid) {
            ctx.refused(QObject::tr("Healing Brush: Alt-click to set a sample point first."));
            return true;
        }
        if (!healing_->get().aligned || !slot.hasOffset) {
            slot.anchor = QPoint(qRound(imagePos.x()), qRound(imagePos.y()));
            slot.offset = slot.source - slot.anchor;
            slot.hasOffset = true;
            clone_->setActive(slot);
        }
        strokeSource_ = slot;
        if (!healing_begin(*v, paint_->size(), paint_->hardness())) {
            refuseHeal(ctx, v);
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        healing_dab(*v, imagePos.x(), imagePos.y());
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        if (PictureView* v = ctx.view()) {
            healing_dab(*v, imagePos.x(), imagePos.y());
        }
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        // ponytail: Sampled source only; Pattern is not wired yet.
        const CloneSource& slot = strokeSource_;
        const double sx = slot.width / 100.0 * (slot.flipH ? -1.0 : 1.0);
        const double sy = slot.height / 100.0 * (slot.flipV ? -1.0 : 1.0);
        if (!healing_commit_transformed(*v, slot.offset.x(), slot.offset.y(),
                                        slot.anchor.x() + 0.5, slot.anchor.y() + 0.5,
                                        sx, sy, slot.angle)) {
            healing_cancel(*v);
        }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            healing_cancel(*v);
        }
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            healing_cancel(*v);
        }
        clone_->invalidateHealingSources();
    }

private:
    CloneSource strokeSource_;
    std::shared_ptr<PaintOptions> paint_;
    std::shared_ptr<HealingGroup> healing_;
    std::shared_ptr<CloneSourceOptions> clone_;
};

} // namespace

std::unique_ptr<ToolHandler> makeSpotHealingToolHandler(std::shared_ptr<PaintOptions> paint,
                                                        std::shared_ptr<HealingGroup> healing)
{
    return std::make_unique<SpotHealingToolHandler>(std::move(paint), std::move(healing));
}

std::unique_ptr<ToolHandler> makeHealingToolHandler(std::shared_ptr<PaintOptions> paint,
                                                    std::shared_ptr<HealingGroup> healing,
                                                    std::shared_ptr<CloneSourceOptions> clone)
{
    return std::make_unique<HealingBrushToolHandler>(std::move(paint), std::move(healing), std::move(clone));
}

} // namespace pictura
