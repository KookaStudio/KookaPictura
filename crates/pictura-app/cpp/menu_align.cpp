// Layer > Align / Align Layers To Selection / Distribute and the Move tool's
// matching options-bar buttons, over the Layers panel's selected layers.

#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "tools.h"

#include "auto_align_dialog.h"
#include "auto_blend_dialog.h"
#include "busy_job.h"
#include "dialogs.h"
#include "options_bar.h"

#include "panels/layers_panel.h"

#include "api/document_api.h"
#include "api/filters_bridge.h"
#include "api/selection_bridge.h"
#include "api/transform_bridge.h"
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

bool MenuRegistrar::runAutoAlignJob(int projection, bool vignette, bool geometric)
{
    PictureView* view = activeView();
    LayersPanel* panel = panels_ ? panels_->layersPanel() : nullptr;
    QWidget* parent = dialogParent();
    if (!view || !panel || !parent) {
        return false;
    }
    const QStringList paths = selectedLayerPaths();
    QStringList moved;
    const bool committed = runCancellableJob(
        parent, tr("Auto-Align Layers"), tr("Aligning layers…"),
        [&] { return auto_align_start(*view, paths, projection, vignette, geometric); },
        [&] { return auto_align_running(*view); },
        [&] { return auto_align_progress(*view); },
        [&] { auto_align_cancel(*view); },
        [&] {
            moved = auto_align_finish(*view);
            return !moved.isEmpty();
        });
    if (committed) {
        selectLayerPath(moved.last());
        panel->selectPaths(moved, moved.last());
    }
    if (const QString notice = auto_align_notice(*view); !notice.isEmpty()) {
        showStatus(notice, 8000);
    }
    return committed;
}

bool MenuRegistrar::runAutoBlendJob(bool stackImages, bool seamless, bool fillTransparent)
{
    PictureView* view = activeView();
    LayersPanel* panel = panels_ ? panels_->layersPanel() : nullptr;
    QWidget* parent = dialogParent();
    if (!view || !panel || !parent) {
        return false;
    }
    const QStringList paths = selectedLayerPaths();
    QStringList blended;
    const bool committed = runCancellableJob(
        parent, tr("Auto-Blend Layers"), tr("Blending layers…"),
        [&] { return auto_blend_start(*view, paths, stackImages, seamless, fillTransparent); },
        [&] { return auto_blend_running(*view); },
        [&] { return auto_blend_progress(*view); },
        [&] { auto_blend_cancel(*view); },
        [&] {
            blended = auto_blend_finish(*view);
            return !blended.isEmpty();
        });
    if (committed) {
        selectLayerPath(blended.last());
        panel->selectPaths(blended, blended.last());
    }
    if (const QString notice = auto_blend_error(*view); !notice.isEmpty()) {
        showStatus(notice, 8000);
    }
    return committed;
}

void MenuRegistrar::wireAlignMenu()
{
    const auto selectedPaths = [this]() { return selectedLayerPaths(); };
    for (int edge = 0; edge < 6; ++edge) {
        const QString align = QString::fromLatin1(kAlignIds[edge]);
        commands_->setImplemented(align, true);
        commands_->setHandler(align, [this, edge]() { alignSelectedLayers(edge, false); });
        commands_->setEnabledProvider(align, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && align_can(*view, selectedPaths(), false, 0);
        });

        const QString toSelection = QString::fromLatin1(kAlignToSelectionIds[edge]);
        commands_->setImplemented(toSelection, true);
        commands_->setHandler(toSelection, [this, edge]() { alignSelectedLayers(edge, true); });
        commands_->setEnabledProvider(toSelection, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && view->has_selection() && align_can(*view, selectedPaths(), true, 0);
        });

        const QString distribute = QString::fromLatin1(kDistributeIds[edge]);
        commands_->setImplemented(distribute, true);
        commands_->setHandler(distribute, [this, edge]() { distributeSelectedLayers(edge); });
        commands_->setEnabledProvider(distribute, [this, selectedPaths]() {
            PictureView* view = activeView();
            return view && distribute_can(*view, selectedPaths());
        });
    }

    const QString autoBlend = QString::fromLatin1(command_ids::EditAutoBlendLayers);
    commands_->setImplemented(autoBlend, true);
    commands_->setHandler(autoBlend, [this, last = AutoBlendChoice{}]() mutable {
        PictureView* view = activeView();
        AutoBlendDialog dialog(view && document_api::documentDepthBits(*view) == 8,
                               dialogParent(), last);
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            last = dialog.choice();
            runAutoBlendJob(dialog.stackImages(), dialog.seamless(), dialog.fillTransparent());
        }
    });
    commands_->setEnabledProvider(autoBlend, [this, selectedPaths]() {
        PictureView* view = activeView();
        return view && auto_blend_can(*view, selectedPaths());
    });

    const QString autoAlign = QString::fromLatin1(command_ids::EditAutoAlignLayers);
    commands_->setImplemented(autoAlign, true);
    commands_->setHandler(autoAlign, [this, last = AutoAlignChoice{}]() mutable {
        AutoAlignDialog dialog(dialogParent(), last);
        if (runDialog(dialog, dialogParent()) == QDialog::Accepted) {
            last = dialog.choice();
            runAutoAlignJob(last.projection, last.vignette, last.geometric);
        }
    });
    commands_->setEnabledProvider(autoAlign, [this, selectedPaths]() {
        PictureView* view = activeView();
        return view && auto_align_can(*view, selectedPaths());
    });
}

void MenuRegistrar::alignSelectedLayers(int edge, bool toSelection, int alignTo)
{
    PictureView* view = activeView();
    if (view && panels_ && panels_->layersPanel()) {
        align_apply(*view, selectedLayerPaths(), edge, toSelection, alignTo);
    }
}

void MenuRegistrar::distributeSelectedLayers(int edge)
{
    PictureView* view = activeView();
    if (view && panels_ && panels_->layersPanel()) {
        distribute_apply(*view, selectedLayerPaths(), edge);
    }
}

void MenuRegistrar::updateAlignControls()
{
    OptionsBar* bar = optionsBar();
    if (!bar) {
        return;
    }
    PictureView* view = activeView();
    const QStringList paths = selectedLayerPaths();
    const MoveGroup* move = tools_ ? tools_->optionsFor<MoveGroup>(ToolId::Move) : nullptr;
    const int alignTo = move ? move->get().alignTo : 0;
    bar->setAlignEnabled(view && align_can(*view, paths, alignTo == 0, alignTo),
                         view && distribute_can(*view, paths));
    // The selection tools' Select and Mask button mirrors the Refine Edge
    // command's enablement, so a click is never a silent no-op.
    bar->setRefineEdgeEnabled(view && refine_edge_available(*view));
    const QString layer = layerAnchor();
    bar->setWarpTextEnabled(view && !layer.isEmpty() && type_api::readLayer(*view, layer).isType);
}

} // namespace pictura
