// Patch tool: outline a region, then drag the outline (see
// RegionDragToolHandler). Source repairs the selection from the dragged-to
// area, Destination applies the selection there, Content-Aware rebuilds it in
// place (`patch_selection` in `cxxqt_object/healing.rs`).

#include "tool_region_drag.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/healing.cxxqt.h"

#include <memory>

namespace pictura {

namespace {

class PatchToolHandler : public RegionDragToolHandler {
public:
    explicit PatchToolHandler(std::shared_ptr<PatchGroup> patch)
        : patch_(std::move(patch))
    {
        addOptions(patch_.get());
    }

protected:
    // ponytail: no Adaptation, Sample All Layers, or Use Pattern yet.
    bool apply(ToolServices& ctx, PictureView& view, int dx, int dy) override
    {
        return patch_selection(view, dx, dy, patch_->get().contentAware, patch_->get().destination,
                               patch_->get().transparent);
    }

    QString verb() const override { return QStringLiteral("patch"); }

private:
    std::shared_ptr<PatchGroup> patch_;
};

} // namespace

std::unique_ptr<ToolHandler> makePatchToolHandler(std::shared_ptr<PaintOptions> paint,
                                                  std::shared_ptr<PatchGroup> patch)
{
    (void)paint;
    return std::make_unique<PatchToolHandler>(std::move(patch));
}

} // namespace pictura
