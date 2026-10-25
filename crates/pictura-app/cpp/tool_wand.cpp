#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/bridge.cxxqt.h"

#include <memory>

namespace pictura {

namespace {

class MagicWandHandler : public ToolHandler {
public:
    explicit MagicWandHandler(std::shared_ptr<SelectionOptions> selection)
        : selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        const bool committed = v->magic_wand(qRound(imagePos.x()), qRound(imagePos.y()),
                                             selection_->tolerance(), selection_->contiguous(),
                                             selectionModeString(ctx.dragMode()));
        if (committed) {
            ctx.emitSelectionCommitted();
        }
        return true;
    }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

private:
    std::shared_ptr<SelectionOptions> selection_;
};

class QuickSelectionHandler : public ToolHandler {
public:
    explicit QuickSelectionHandler(std::shared_ptr<SelectionOptions> selection)
        : selection_(std::move(selection))
    {
        addOptions(selection_.get());
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        PictureView* v = ctx.view();
        if (!v) {
            return true;
        }
        ctx_ = &ctx;
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        ctx.setDragging(true);
        ctx.setDragCommitted(false);
        if (!pictura::quick_select_begin(*v)) {
            ctx.setDragging(false);
            return true;
        }
        applyDab(ctx, imagePos);
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        applyDab(ctx, imagePos);
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        PictureView* v = ctx.view();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        if (!v) {
            return;
        }
        if (ctx.dragCommitted()) {
            if (pictura::quick_select_commit(*v)) {
                ctx.emitSelectionCommitted();
            }
        } else {
            pictura::quick_select_cancel(*v);
        }
    }

    bool onEscape() override
    {
        if (!ctx_ || !ctx_->dragging()) {
            return false;
        }
        onSessionCancelled(*ctx_);
        return true;
    }

    void onSessionCancelled(ToolServices& ctx) override
    {
        if (PictureView* v = ctx.view()) {
            pictura::quick_select_cancel(*v);
        }
        ctx.setDragging(false);
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        onSessionCancelled(ctx);
        ctx_ = nullptr;
    }

    bool hasCanvasContextMenu() const override { return true; }
    QMenu* buildContextMenu(ToolServices& ctx, const QPointF& imagePos) override
    {
        return selectionCanvasMenu(ctx, imagePos);
    }

private:
    // One dab of the stroke. A New-mode dab replaces; the tool then switches
    // to Add so the rest of the stroke accumulates on top of it.
    bool applyDab(ToolServices& ctx, const QPointF& imagePos)
    {
        PictureView* v = ctx.view();
        if (!v) {
            return false;
        }
        if (!pictura::quick_select_preview(*v, qRound(imagePos.x()), qRound(imagePos.y()),
                                           selection_->tolerance(),
                                           selectionModeString(ctx.dragMode()),
                                           ctx.dragCommitted())) {
            return false;
        }
        ctx.setDragCommitted(true);
        if (ctx.dragMode() == SelectionMode::New) {
            ctx.setDragMode(SelectionMode::Add);
            selection_->setCombineMode(SelectionMode::Add);
        }
        return true;
    }

    std::shared_ptr<SelectionOptions> selection_;
    ToolServices* ctx_ = nullptr;
};

} // namespace

std::unique_ptr<ToolHandler> makeMagicWandToolHandler(std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<MagicWandHandler>(std::move(selection));
}

std::unique_ptr<ToolHandler> makeQuickSelectionToolHandler(
    std::shared_ptr<SelectionOptions> selection)
{
    return std::make_unique<QuickSelectionHandler>(std::move(selection));
}

} // namespace pictura
