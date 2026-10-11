// Layer > Align / Align Layers To Selection / Distribute and the Move tool's
// matching options-bar buttons, over the Layers panel's selected layers.

#include "frame_includes.h"

#include "pictura_app/src/cxxqt_object/align.cxxqt.h"

namespace pictura {

namespace {

// Menu order, which is also the engine's edge index (0 Top ... 5 Right).
const char* const kEdgeLeaves[] = {"Top",  "Vertical Center",   "Bottom",
                                   "Left", "Horizontal Center", "Right"};

} // namespace

void PicturaMainWindow::wireAlignMenu()
{
    const auto selectedPaths = [this]() {
        return layersPanel_ ? layersPanel_->selectedPaths() : QStringList();
    };
    for (int edge = 0; edge < 6; ++edge) {
        const QString leaf = QLatin1String(kEdgeLeaves[edge]);

        const QString align = commandIdForPath({QStringLiteral("Layer"), QStringLiteral("Align"), leaf});
        registry_->setImplemented(align, true);
        registry_->setHandler(align, [this, edge]() { alignSelectedLayers(edge, false); });
        registry_->setEnabledProvider(align, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && align_can(*view, selectedPaths(), false, 0);
        });

        const QString toSelection = commandIdForPath(
            {QStringLiteral("Layer"), QStringLiteral("Align Layers To Selection"), leaf});
        registry_->setImplemented(toSelection, true);
        registry_->setHandler(toSelection, [this, edge]() { alignSelectedLayers(edge, true); });
        registry_->setEnabledProvider(toSelection, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && view->has_selection() && align_can(*view, selectedPaths(), true, 0);
        });

        const QString distribute =
            commandIdForPath({QStringLiteral("Layer"), QStringLiteral("Distribute"), leaf});
        registry_->setImplemented(distribute, true);
        registry_->setHandler(distribute, [this, edge]() { distributeSelectedLayers(edge); });
        registry_->setEnabledProvider(distribute, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && distribute_can(*view, selectedPaths());
        });
    }
}

void PicturaMainWindow::alignSelectedLayers(int edge, bool toSelection, int alignTo)
{
    PictureView* view = activeView();
    if (view && layersPanel_
        && align_apply(*view, layersPanel_->selectedPaths(), edge, toSelection, alignTo) > 0) {
        refresh();
    }
}

void PicturaMainWindow::distributeSelectedLayers(int edge)
{
    PictureView* view = activeView();
    if (view && layersPanel_ && distribute_apply(*view, layersPanel_->selectedPaths(), edge) > 0) {
        refresh();
    }
}

void PicturaMainWindow::updateAlignControls()
{
    if (!optionsBar_) {
        return;
    }
    PictureView* view = activeView();
    const QStringList paths = layersPanel_ ? layersPanel_->selectedPaths() : QStringList();
    const int alignTo = tools_ ? tools_->moveAlignTo() : 0;
    optionsBar_->setAlignEnabled(view && align_can(*view, paths, alignTo == 0, alignTo),
                                 view && distribute_can(*view, paths));
}

} // namespace pictura
