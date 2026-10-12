// The erasers (`pictura_paint::eraser`). The Eraser is a Brush stroke that
// erases the active layer to transparency, or to the background colour on the
// Background and transparency-locked layers; Erase To History, or Alt held at
// the press, paints the History Brush's source state back instead. The
// Background Eraser erases the colour under its crosshair within each dab; the
// Magic Eraser erases the Magic Wand's flood from a click. Ported from
// photorust's erase mode and `core/src/erase.rs`.

#include "tool_handler.h"

#include "api/paint_api.h"
#include "paint_tip.h"

#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QObject>
#include <QtCore/QtMath>

#include <memory>
#include <optional>

namespace pictura {

namespace {

// The press / drag / release the two brush erasers share; `begin` starts the
// tool's stroke and reports whether it did.
class EraseStrokeHandler : public ToolHandler {
public:
    explicit EraseStrokeHandler(std::shared_ptr<PaintOptions> paint)
        : paint_(std::move(paint))
    {
        addOptions(paint_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        const bool toHistory = eraseToHistory(ctx, mods);
        stroke_.emplace(*v, [&] { return begin(ctx, *v, toHistory); });
        if (!stroke_->open()) {
            stroke_.reset();
            if (activePixelLocked(v)) {
                ctx.refused(QObject::tr("Could not erase: the layer's pixels are locked."));
            } else if (!v->active_layer_visible()) {
                ctx.refused(QObject::tr("Could not erase: the active layer is invisible."));
            } else if (v->active_layer_path().isEmpty()) {
                ctx.refused(QObject::tr("Could not erase: select a single layer first."));
            } else if (toHistory) {
                ctx.refused(
                    QObject::tr("Erase to History: the source state has no matching pixel layer."));
            }
            return true;
        }
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        stroke_->dab(imagePos.x(), imagePos.y());
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
        if (stroke_) {
            stroke_->finish();
            stroke_.reset();
        }
    }

protected:
    virtual bool begin(ToolServices& ctx, PictureView& v, bool toHistory) = 0;
    virtual bool eraseToHistory(ToolServices&, Qt::KeyboardModifiers) { return false; }

    std::shared_ptr<PaintOptions> paint_;

private:
    std::optional<paint_api::Stroke> stroke_;
};

class EraserToolHandler : public EraseStrokeHandler {
public:
    EraserToolHandler(std::shared_ptr<PaintOptions> paint, std::shared_ptr<EraserGroup> eraser)
        : EraseStrokeHandler(std::move(paint))
        , eraser_(std::move(eraser))
    {
        addOptions(eraser_.get());
    }

protected:
    bool eraseToHistory(ToolServices& ctx, Qt::KeyboardModifiers mods) override
    {
        return eraser_->get().toHistory || mods.testFlag(Qt::AltModifier);
    }

    bool begin(ToolServices& ctx, PictureView& v, bool toHistory) override
    {
        const EraserOptions o = eraser_->get();
        return begin_eraser(v, ctx.background().rgba(), paintTip(*paint_), o.mode, paint_->opacity(),
                            paint_->flow(), toHistory);
    }

private:
    std::shared_ptr<EraserGroup> eraser_;
};

// The Background Eraser has no Opacity or Flow; it always erases at full
// strength where the colour matches.
class BackgroundEraserToolHandler : public EraseStrokeHandler {
public:
    BackgroundEraserToolHandler(std::shared_ptr<PaintOptions> paint,
                                std::shared_ptr<BackgroundEraseGroup> backgroundErase)
        : EraseStrokeHandler(std::move(paint))
        , backgroundErase_(std::move(backgroundErase))
    {
        addOptions(backgroundErase_.get());
    }

protected:
    bool begin(ToolServices& ctx, PictureView& v, bool) override
    {
        const BackgroundEraseOptions o = backgroundErase_->get();
        return begin_background_eraser(v, ctx.foreground().rgba(), ctx.background().rgba(),
                                       paintTip(*paint_), o.sampling, o.limits, o.tolerance,
                                       o.protectForeground);
    }

private:
    std::shared_ptr<BackgroundEraseGroup> backgroundErase_;
};

// One click erases the region the Magic Wand would select there.
// ponytail: the active selection does not limit the erase.
class MagicEraserToolHandler : public ToolHandler {
public:
    explicit MagicEraserToolHandler(std::shared_ptr<MagicEraseGroup> magicErase)
        : magicErase_(std::move(magicErase))
    {
        addOptions(magicErase_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        PictureView* v = ctx.view();
        if (!v || !v->has_document()) {
            return true;
        }
        const MagicEraseOptions o = magicErase_->get();
        if (!magic_erase_at(*v, qFloor(imagePos.x()), qFloor(imagePos.y()), o.tolerance,
                            o.antialias, o.contiguous, o.sampleAllLayers, o.opacity,
                            ctx.background().rgba())) {
            if (activePixelLocked(v)) {
                ctx.refused(QObject::tr("Could not erase: the layer's pixels are locked."));
            } else if (!v->active_layer_visible()) {
                ctx.refused(QObject::tr("Could not erase: the active layer is invisible."));
            } else if (v->active_layer_path().isEmpty()) {
                ctx.refused(QObject::tr("Could not erase: select a single layer first."));
            }
        }
        return true;
    }

private:
    std::shared_ptr<MagicEraseGroup> magicErase_;
};

} // namespace

std::unique_ptr<ToolHandler> makeEraserToolHandler(std::shared_ptr<PaintOptions> paint,
                                                   std::shared_ptr<EraserGroup> eraser)
{
    return std::make_unique<EraserToolHandler>(std::move(paint), std::move(eraser));
}

std::unique_ptr<ToolHandler> makeBackgroundEraserToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<BackgroundEraseGroup> backgroundErase)
{
    return std::make_unique<BackgroundEraserToolHandler>(std::move(paint),
                                                         std::move(backgroundErase));
}

std::unique_ptr<ToolHandler> makeMagicEraserToolHandler(
    std::shared_ptr<PaintOptions> paint, std::shared_ptr<MagicEraseGroup> magicErase)
{
    (void)paint;
    return std::make_unique<MagicEraserToolHandler>(std::move(magicErase));
}

} // namespace pictura
