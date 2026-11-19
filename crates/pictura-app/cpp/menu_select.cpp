#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "panel_host.h"
#include "tools.h"

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include "panels/layers_panel.h"

#include "color_range_dialog.h"
#include "replace_color_dialog.h"
#include "refine_edge_dialog.h"

#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>

namespace pictura {


// Select menu operations. Parameters come from QInputDialog; a cancelled dialog
// returns before the bridge, so it records nothing. Each applied command records
// exactly one state in the bridge.
void MenuRegistrar::registerSelectHandlers()
{
    commands_->setHandler(command_ids::SelectReselect, [this]() {
        if (PictureView* view = activeView()) {
            view->reselect();
        }
    });
    commands_->setHandler(command_ids::SelectInverse, [this]() {
        if (PictureView* view = activeView()) {
            view->invert_selection();
        }
    });
    const auto modifyOp = [this](const char* op, const QString& title, double initial, double min,
                                 double max, bool decimal) {
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        bool ok = false;
        const double amount = decimal
            ? QInputDialog::getDouble(dialogParent(), title, tr("Radius:"), initial, min, max, 1, &ok)
            : QInputDialog::getInt(dialogParent(), title, tr("Amount:"), int(initial), int(min), int(max), 1, &ok);
        if (!ok) {
            return;
        }
        view->modify_selection(QString::fromLatin1(op), amount);
    };
    commands_->setHandler(command_ids::SelectModifyBorder, [modifyOp]() {
        modifyOp("border", tr("Border Selection"), 1, 1, 200, false);
    });
    commands_->setHandler(command_ids::SelectModifySmooth, [modifyOp]() {
        modifyOp("smooth", tr("Smooth Selection"), 1, 1, 100, false);
    });
    commands_->setHandler(command_ids::SelectModifyExpand, [modifyOp]() {
        modifyOp("expand", tr("Expand Selection"), 1, 1, 100, false);
    });
    commands_->setHandler(command_ids::SelectModifyContract, [modifyOp]() {
        modifyOp("contract", tr("Contract Selection"), 1, 1, 100, false);
    });
    commands_->setHandler(command_ids::SelectModifyFeather, [modifyOp]() {
        modifyOp("feather", tr("Feather Selection"), 1.0, 0.0, 250.0, true);
    });
    commands_->setHandler(command_ids::SelectGrow, [this]() {
        PictureView* view = activeView();
        const SelectionOptions* selection = tools_ ? tools_->optionsFor<SelectionOptions>(ToolId::MagicWand) : nullptr;
        const int tolerance = selection ? selection->tolerance() : 0;
        if (view) {
            view->grow_selection(tolerance);
        }
    });
    commands_->setHandler(command_ids::SelectSimilar, [this]() {
        PictureView* view = activeView();
        const SelectionOptions* selection = tools_ ? tools_->optionsFor<SelectionOptions>(ToolId::MagicWand) : nullptr;
        const int tolerance = selection ? selection->tolerance() : 0;
        if (view) {
            view->similar_selection(tolerance);
        }
    });
    commands_->setHandler(command_ids::SelectSave, [this]() {
        if (runtime_.openSaveSelectionDialog) {
            runtime_.openSaveSelectionDialog();
        }
    });
    commands_->setHandler(command_ids::SelectLoad, [this]() {
        if (runtime_.openLoadSelectionDialog) {
            runtime_.openLoadSelectionDialog();
        }
    });

    commands_->setHandler(command_ids::SelectTransformSelection, [this]() {
        if (tools_ && tools_->beginTransformSelection()) {
            refresh();
        }
    });

    commands_->setHandler(command_ids::SelectAllLayers, [this]() {
        PictureView* view = activeView();
        if (!view || !panels_->layersPanel()) {
            return;
        }
        const QStringList paths = view->select_all_layers();
        if (paths.isEmpty()) {
            return;
        }
        set_layer_target(*view, paths, paths.value(0));
        if (panels_->layersPanel()) {
            panels_->layersPanel()->refresh();
        }
        refresh();
    });
    commands_->setHandler(command_ids::SelectDeselectLayers, [this]() {
        if (PictureView* view = activeView()) {
            clear_layer_target(*view);
            if (panels_->layersPanel()) {
                panels_->layersPanel()->refresh();
            }
            refresh();
        }
    });
    commands_->setHandler(command_ids::SelectSimilarLayers, [this]() {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return;
        }
        view->select_similar(path);
        if (panels_->layersPanel()) {
            panels_->layersPanel()->refresh();
        }
        refresh();
    });

    const auto selectHasDoc = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const char* id : {command_ids::SelectReselect, command_ids::SelectInverse,
                           command_ids::SelectModifyBorder, command_ids::SelectModifySmooth,
                           command_ids::SelectModifyExpand, command_ids::SelectModifyContract,
                           command_ids::SelectModifyFeather, command_ids::SelectGrow,
                           command_ids::SelectSimilar, command_ids::SelectSave,
                           command_ids::SelectLoad, command_ids::SelectAllLayers,
                           command_ids::SelectDeselectLayers, command_ids::SelectSimilarLayers}) {
        commands_->setEnabledProvider(id, selectHasDoc);
    }
    for (const char* id : {command_ids::SelectInverse, command_ids::SelectModifyBorder,
                           command_ids::SelectModifySmooth, command_ids::SelectModifyExpand,
                           command_ids::SelectModifyContract, command_ids::SelectModifyFeather,
                           command_ids::SelectGrow, command_ids::SelectSimilar,
                           command_ids::SelectSave}) {
        commands_->setEnabledProvider(id, [this]() {
            return activeView() && activeView()->has_document() && activeView()->has_selection();
        });
    }
    commands_->setEnabledProvider(command_ids::SelectReselect, [this]() {
        return activeView() && activeView()->has_document()
            && activeView()->has_deselected_selection();
    });
    // Select > Load Selection also lists the document's colour rows, so every
    // same-size document has a source; `selectHasDoc` above governs it.
    commands_->setEnabledProvider(command_ids::SelectSimilarLayers, [this]() {
        return activeView() && activeView()->has_document() && panels_->layersPanel()
            && !layerAnchor().isEmpty();
    });
    commands_->setEnabledProvider(command_ids::SelectTransformSelection, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && view->has_selection()
            && view->selection_count() > 0 && !(tools_ && tools_->transformPending());
    });

    // Select > Color Range…: non-modal, so its eyedropper reaches the canvas;
    // OK selects (or narrows a live selection) as one "Color Range" state.
    const QString colorRange = QString::fromLatin1(command_ids::SelectColorRange);
    commands_->setImplemented(colorRange, true);
    commands_->setHandler(colorRange, [this]() {
        PictureView* view = activeView();
        if (!view || !color_range_available(*view)) {
            return;
        }
        if (colorRangeDialog_) {
            colorRangeDialog_->raise();
            colorRangeDialog_->activateWindow();
            return;
        }
        auto* dialog = new ColorRangeDialog(view, tools_, dialogParent());
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        colorRangeDialog_ = dialog;
        connect(dialog, &QDialog::finished, this, [this, dialog, view](int result) {
            if (result == QDialog::Accepted && activeView() == view) {
                color_range_apply(*view, dialog->range(), dialog->sampledColor().rgb() & 0xffffffu,
                                  dialog->fuzziness(), dialog->inverted());
            }
        });
        dialog->show();
    });
    commands_->setEnabledProvider(colorRange, [this]() {
        PictureView* view = activeView();
        return view && color_range_available(*view);
    });

    // Select > Refine Edge…: the dialog previews the refined mask; OK applies
    // the settings once as one "Refine Edge" state.
    const QString refineEdge = QString::fromLatin1(command_ids::SelectRefineEdge);
    commands_->setImplemented(refineEdge, true);
    const auto openRefineEdge = [this]() {
        PictureView* view = activeView();
        if (!view || !refine_edge_available(*view)) {
            return;
        }
        auto* dialog = new RefineEdgeDialog(view, dialogParent());
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        if (runDialog(*dialog, dialogParent()) == QDialog::Accepted && activeView() == view) {
            refine_edge_apply(*view, dialog->smartRadius(), dialog->radius(),
                              dialog->smooth(), dialog->feather(), dialog->contrast(),
                              dialog->shiftEdge(), dialog->decontaminate(), dialog->amount(),
                              dialog->outputTo());
        }
    };
    commands_->setHandler(refineEdge, openRefineEdge);
    commands_->setEnabledProvider(refineEdge, [this]() {
        PictureView* view = activeView();
        return view && refine_edge_available(*view);
    });
}

void MenuRegistrar::cancelDocumentDialogs(PictureView* view)
{
    if (colorRangeDialog_ && colorRangeDialog_->view() != view) {
        colorRangeDialog_->reject();
    }
    if (replaceColorDialog_ && replaceColorDialog_->view() != view) {
        replaceColorDialog_->reject();
    }
}

} // namespace pictura
