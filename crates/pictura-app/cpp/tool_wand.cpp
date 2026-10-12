#include "tool_handler.h"

#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

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
        ctx.setDragMode(ctx.resolveSelectionMode(mods, v->has_selection()));
        ctx.setDragging(true);
        ctx.setDragCommitted(v->quick_select(qRound(imagePos.x()), qRound(imagePos.y()),
                                             selection_->tolerance(),
                                             selectionModeString(ctx.dragMode())));
        return true;
    }

    void onMove(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        PictureView* v = ctx.view();
        if (!v) {
            return;
        }
        if (v->quick_select(qRound(imagePos.x()), qRound(imagePos.y()), selection_->tolerance(),
                            selectionModeString(ctx.dragMode()))) {
            ctx.setDragCommitted(true);
        }
    }

    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (!ctx.dragging()) {
            return;
        }
        ctx.setDragging(false);
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearSelectionPreview();
        }
        if (ctx.dragCommitted()) {
            ctx.emitSelectionCommitted();
        }
    }

private:
    std::shared_ptr<SelectionOptions> selection_;
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
