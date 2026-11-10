// Layer > Align / Align Layers To Selection / Distribute and the Move tool's
// matching options-bar buttons, over the Layers panel's selected layers.

#include "frame_includes.h"

#include "auto_align_dialog.h"
#include "auto_blend_dialog.h"
#include "busy_job.h"
#include "dialogs.h"
#include "api/filters_bridge.h"
#include "api/selection_bridge.h"
#include "api/transform_bridge.h"

#include "api/document_api.h"
#include "api/type_api.h"

namespace pictura {

namespace {

// Edge order, which is also the engine's edge index (0 Top ... 5 Right).
const char* const kAlignIds[] = {
    command_ids::LayerAlignTop,          command_ids::LayerAlignVerticalCenter,
    command_ids::LayerAlignBottom,       command_ids::LayerAlignLeft,
    command_ids::LayerAlignHorizontalCenter, command_ids::LayerAlignRight};
const char* const kAlignToSelectionIds[] = {
    command_ids::LayerAlignLayersToSelectionTop,
    command_ids::LayerAlignLayersToSelectionVerticalCenter,
    command_ids::LayerAlignLayersToSelectionBottom,
    command_ids::LayerAlignLayersToSelectionLeft,
    command_ids::LayerAlignLayersToSelectionHorizontalCenter,
    command_ids::LayerAlignLayersToSelectionRight};
const char* const kDistributeIds[] = {
    command_ids::LayerDistributeTop,      command_ids::LayerDistributeVerticalCenter,
    command_ids::LayerDistributeBottom,   command_ids::LayerDistributeLeft,
    command_ids::LayerDistributeHorizontalCenter, command_ids::LayerDistributeRight};

} // namespace

void PicturaMainWindow::wireAlignMenu()
{
    const auto selectedPaths = [this]() {
        return selectedLayerPaths();
    };
    for (int edge = 0; edge < 6; ++edge) {
        const QString align = QString::fromLatin1(kAlignIds[edge]);
        registry_->setImplemented(align, true);
        registry_->setHandler(align, [this, edge]() { alignSelectedLayers(edge, false); });
        registry_->setEnabledProvider(align, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && align_can(*view, selectedPaths(), false, 0);
        });

        const QString toSelection = QString::fromLatin1(kAlignToSelectionIds[edge]);
        registry_->setImplemented(toSelection, true);
        registry_->setHandler(toSelection, [this, edge]() { alignSelectedLayers(edge, true); });
        registry_->setEnabledProvider(toSelection, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && view->has_selection() && align_can(*view, selectedPaths(), true, 0);
        });

        const QString distribute = QString::fromLatin1(kDistributeIds[edge]);
        registry_->setImplemented(distribute, true);
        registry_->setHandler(distribute, [this, edge]() { distributeSelectedLayers(edge); });
        registry_->setEnabledProvider(distribute, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && distribute_can(*view, selectedPaths());
        });
    }

    const QString autoBlend = QString::fromLatin1(command_ids::EditAutoBlendLayers);
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

    const QString autoAlign = QString::fromLatin1(command_ids::EditAutoAlignLayers);
    registry_->setImplemented(autoAlign, true);
    registry_->setHandler(autoAlign, [this, last = AutoAlignChoice{}]() mutable {
        AutoAlignDialog dialog(this, last);
        if (runDialog(dialog, this) == QDialog::Accepted) {
            last = dialog.choice();
            runAutoAlign(*this, last.projection, last.vignette, last.geometric);
        }
    });
    registry_->setEnabledProvider(autoAlign, [this, selectedPaths]() {
        PictureView* view = activeView();
        return view && auto_align_can(*view, selectedPaths());
    });
}

bool runAutoAlign(PicturaMainWindow& window, int projection, bool vignette, bool geometric)
{
    PictureView* view = window.activeView();
    auto* panel = window.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view || !panel) {
        return false;
    }
    const QStringList paths = window.selectedLayerPaths();
    QStringList moved;
    const bool committed = runCancellableJob(
        &window, PicturaMainWindow::tr("Auto-Align Layers"),
        PicturaMainWindow::tr("Aligning layers…"),
        [&] { return auto_align_start(*view, paths, projection, vignette, geometric); },
        [&] { return auto_align_running(*view); },
        [&] { return auto_align_progress(*view); },
        [&] { auto_align_cancel(*view); },
        [&] {
            moved = auto_align_finish(*view);
            return !moved.isEmpty();
        });
    if (committed) {
        window.selectLayerPath(moved.last());
        panel->selectPaths(moved, moved.last());
    }
    if (const QString notice = auto_align_notice(*view); !notice.isEmpty()) {
        window.statusBar()->showMessage(notice, 8000);
    }
    return committed;
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
    }
    if (const QString notice = auto_blend_error(*view); !notice.isEmpty()) {
        window.statusBar()->showMessage(notice, 8000);
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
