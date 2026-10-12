// Content-Aware Move tool: outline a region, then drag the outline (see
// RegionDragToolHandler). Move relocates the pixels and rebuilds the hole from
// its surroundings; Extend copies them and keeps the original
// (`content_aware_move` in `cxxqt_object/healing.rs`).

#include "tool_region_drag.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/healing.cxxqt.h"

#include <memory>

namespace pictura {

namespace {

class ContentAwareMoveToolHandler : public RegionDragToolHandler {
public:
    explicit ContentAwareMoveToolHandler(std::shared_ptr<ContentAwareMoveGroup> contentAware)
        : contentAware_(std::move(contentAware))
    {
        addOptions(contentAware_.get());
    }

protected:
    // ponytail: no Sample All Layers yet; the active layer is both read and
    // written.
    bool apply(ToolServices& ctx, PictureView& view, int dx, int dy) override
    {
        const bool moved = content_aware_move(view, dx, dy, contentAware_->get().extend,
                                              contentAware_->get().adaptation);
        if (moved) {
            ctx.emitSelectionCommitted();
        }
        return moved;
    }

    QString verb() const override { return QStringLiteral("move"); }

private:
    std::shared_ptr<ContentAwareMoveGroup> contentAware_;
};

} // namespace

std::unique_ptr<ToolHandler> makeContentAwareMoveToolHandler(
    std::shared_ptr<ContentAwareMoveGroup> contentAware)
{
    return std::make_unique<ContentAwareMoveToolHandler>(std::move(contentAware));
}

} // namespace pictura
