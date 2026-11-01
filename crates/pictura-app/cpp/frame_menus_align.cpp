// Layer > Align / Align Layers To Selection / Distribute and the Move tool's
// matching options-bar buttons, over the Layers panel's selected layers.

#include "frame_includes.h"

#include "auto_blend_dialog.h"
#include "busy_job.h"
#include "dialogs.h"
#include "pictura_app/src/cxxqt_object/align.cxxqt.h"
#include "pictura_app/src/cxxqt_object/auto_blend.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/refine.cxxqt.h"

#include "api/document_api.h"
#include "api/type_api.h"

namespace pictura {

namespace {

// Menu order, which is also the engine's edge index (0 Top ... 5 Right).
const char* const kEdgeLeaves[] = {"Top",  "Vertical Center",   "Bottom",
                                   "Left", "Horizontal Center", "Right"};

} // namespace

void PicturaMainWindow::wireAlignMenu()
{
    const auto selectedPaths = [this]() {
        return selectedLayerPaths();
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

    const QString autoBlend = commandIdForPath({QStringLiteral("Edit"), QStringLiteral("Auto-Blend Layers…")});
    registry_->setImplemented(autoBlend, true);
    registry_->setHandler(autoBlend, [this, last = AutoBlendChoice{}]() mutable {
        PictureView* view = activeView();
        AutoBlendDialog dialog(view && document_api::documentDepthBits(*view) == 8, this, last);
        if (runDialog(dialog, this) == QDialog::Accepted) {
            last = dialog.choice();
            runAutoBlend(*this, dialog.stackImages(), dialog.seamless(), dialog.fillTransparent());
        }
    });
    registry_->setEnabledProvider(autoBlend, [this, selectedPaths]() {
        PictureView* view = activeView();
        return view && auto_blend_can(*view, selectedPaths());
    });
}

bool runAutoBlend(PicturaMainWindow& window, bool stackImages, bool seamless, bool fillTransparent)
{
    PictureView* view = window.activeView();
    auto* panel = window.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view || !panel) {
        return false;
    }
    const QStringList paths = window.selectedLayerPaths();
    QStringList blended;
    const bool committed = runCancellableJob(
        &window, PicturaMainWindow::tr("Auto-Blend Layers"),
        PicturaMainWindow::tr("Blending layers…"),
        [&] { return auto_blend_start(*view, paths, stackImages, seamless, fillTransparent); },
        [&] { return auto_blend_running(*view); },
        [&] { return auto_blend_progress(*view); },
        [&] { auto_blend_cancel(*view); },
        [&] {
            blended = auto_blend_finish(*view);
            return !blended.isEmpty();
        });
    if (committed) {
        window.selectLayerPath(blended.last());
        panel->selectPaths(blended, blended.last());
    } else if (const QString error = auto_blend_error(*view); !error.isEmpty()) {
        window.statusBar()->showMessage(error, 8000);
    }
    return committed;
}

void PicturaMainWindow::alignSelectedLayers(int edge, bool toSelection, int alignTo)
{
    PictureView* view = activeView();
    if (view && layersPanel_) {
        align_apply(*view, selectedLayerPaths(), edge, toSelection, alignTo);
    }
}

void PicturaMainWindow::distributeSelectedLayers(int edge)
{
    PictureView* view = activeView();
    if (view && layersPanel_) {
        distribute_apply(*view, selectedLayerPaths(), edge);
    }
}

void PicturaMainWindow::updateAlignControls()
{
    if (!optionsBar_) {
        return;
    }
    PictureView* view = activeView();
    const QStringList paths = selectedLayerPaths();
    const MoveGroup* move = tools_ ? tools_->optionsFor<MoveGroup>(ToolId::Move) : nullptr;
    const int alignTo = move ? move->get().alignTo : 0;
    optionsBar_->setAlignEnabled(view && align_can(*view, paths, alignTo == 0, alignTo),
                                 view && distribute_can(*view, paths));
    // The selection tools' Select and Mask button mirrors the Refine Edge
    // command's enablement, so a click is never a silent no-op.
    optionsBar_->setRefineEdgeEnabled(view && refine_edge_available(*view));
    const QString layer = layerAnchor();
    optionsBar_->setWarpTextEnabled(view && !layer.isEmpty() && type_api::readLayer(*view, layer).isType);
}

} // namespace pictura
