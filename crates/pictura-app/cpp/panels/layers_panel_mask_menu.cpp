// The Layers panel's mask and trash buttons at the foot of the panel, the
// vector-mask thumbnail menu, and the Refine Mask and Layer Mask Display
// Options dialogs the layer-mask thumbnail menu opens (that menu itself is in
// layers_panel_menu.cpp).

#include "layers_panel.h"

#include "dialogs.h"
#include "mask_display_options_dialog.h"
#include "refine_edge_dialog.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/refine.cxxqt.h"

#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>

namespace pictura {

void LayersPanel::syncStripButtons()
{
    if (!maskButton_ || !deleteButton_) {
        return;
    }
    // CS6 adds a layer mask first, then a vector mask; with both there is
    // nothing left to add.
    const bool layerMask = view_ && layer_mask_present(*view_);
    const bool vectorMask = view_ && vector_mask_present(*view_);
    maskButton_->setEnabled(!(layerMask && vectorMask));
    maskButton_->setToolTip(!layerMask    ? tr("Add layer mask")
                            : !vectorMask ? tr("Add vector mask")
                                          : tr("Add a mask"));
    // The trash deletes the vector mask of a doubly-masked layer, else the
    // selection, which must leave the document a layer.
    deleteButton_->setEnabled(view_
                              && (!trashVectorMaskPath().isEmpty()
                                  || layers_can_delete(*view_, selectedPaths())));
}

void LayersPanel::addMaskFromButton()
{
    if (!view_) {
        return;
    }
    const bool alt = QApplication::keyboardModifiers().testFlag(Qt::AltModifier);
    if (!layer_mask_present(*view_)) {
        layer_mask_add(*view_, alt                      ? QStringLiteral("hide-all")
                               : view_->has_selection() ? QStringLiteral("reveal-selection")
                                                        : QStringLiteral("reveal-all"));
    } else if (!vector_mask_present(*view_)) {
        vector_mask_add(*view_, alt ? QStringLiteral("hide-all") : QStringLiteral("reveal-all"));
    }
    syncStripButtons();
}

void LayersPanel::populateVectorMaskMenu(QMenu& menu, const QString& path)
{
    const auto add = [this, &menu, path](const char* id, const QString& label) {
        QAction* action = menu.addAction(label);
        connect(action, &QAction::triggered, this,
                [this, id, path] { performMaskAction(QLatin1String(id), path); });
    };
    const bool disabled = view_ && vector_mask_disabled_path(*view_, path);
    add("vectorToggleEnabled", disabled ? tr("Enable Vector Mask") : tr("Disable Vector Mask"));
    menu.addSeparator();
    add("vectorDelete", tr("Delete Vector Mask"));
    add("vectorRasterize", tr("Rasterize Vector Mask"));
}

QString LayersPanel::trashVectorMaskPath() const
{
    // Read the selected row, not the active layer: the current row can sit
    // outside the selection (Ctrl+arrow), and the trash acts on the selection.
    const QStringList paths = selectedPaths();
    if (!view_ || paths.size() != 1) {
        return {};
    }
    const QString& path = paths.first();
    return layer_mask_present_path(*view_, path) && vector_mask_present_path(*view_, path)
               ? path
               : QString();
}

void LayersPanel::deleteFromTrash()
{
    if (!view_) {
        return;
    }
    const QString vectorMask = trashVectorMaskPath();
    if (!vectorMask.isEmpty()) {
        if (confirmDeleteVectorMask()) {
            vector_mask_delete_path(*view_, vectorMask);
        }
        return;
    }
    deleteSelection();
}

bool LayersPanel::confirmDeleteVectorMask()
{
    SessionState session = loadSession();
    if (!session.confirmDeleteVectorMask) {
        return true;
    }
    QMessageBox box(QMessageBox::Warning, QStringLiteral("Kooka Pictura"),
                    tr("Delete Vector Mask?"), QMessageBox::Ok | QMessageBox::Cancel, this);
    box.setObjectName(QStringLiteral("deleteVectorMaskPrompt"));
    box.setDefaultButton(QMessageBox::Ok);
    auto* dontShow = new QCheckBox(tr("Don't show again"), &box);
    dontShow->setObjectName(QStringLiteral("deleteVectorMaskDontShowAgain"));
    box.setCheckBox(dontShow);
    const bool ok = dialogDriver_ ? dialogDriver_(box) : box.exec() == QMessageBox::Ok;
    if (ok && dontShow->isChecked()) {
        session = loadSession();
        session.confirmDeleteVectorMask = false;
        saveSession(session);
    }
    return ok;
}

bool LayersPanel::runPanelDialog(QDialog& dialog)
{
    return dialogDriver_ ? dialogDriver_(dialog) : runDialog(dialog, this) == QDialog::Accepted;
}

void LayersPanel::openRefineMask(const QString& path)
{
    if (!view_ || !refine_mask_available(*view_, path)) {
        return;
    }
    RefineEdgeDialog dialog(view_, this, path);
    if (runPanelDialog(dialog) && view_) {
        refine_mask_apply(*view_, path, dialog.smartRadius(), dialog.radius(), dialog.smooth(),
                          dialog.feather(), dialog.contrast(), dialog.shiftEdge(),
                          dialog.decontaminate(), dialog.amount(), dialog.outputTo());
    }
}

void LayersPanel::openMaskOptions()
{
    if (!view_) {
        return;
    }
    MaskDisplayOptionsDialog dialog(QColor::fromRgba(mask_display_color(*view_)),
                                    mask_display_opacity(*view_), this);
    if (runPanelDialog(dialog) && view_) {
        set_mask_display_options(*view_, dialog.color().rgba(), dialog.opacity());
        emit maskDisplayChanged();
    }
}

QString LayersPanel::maskButtonToolTipForTest() const
{
    return maskButton_ ? maskButton_->toolTip() : QString();
}

bool LayersPanel::maskButtonEnabledForTest() const
{
    return maskButton_ && maskButton_->isEnabled();
}

void LayersPanel::clickMaskButtonForTest()
{
    if (maskButton_) {
        maskButton_->click();
    }
}

bool LayersPanel::deleteButtonEnabledForTest() const
{
    return deleteButton_ && deleteButton_->isEnabled();
}

void LayersPanel::clickDeleteButtonForTest()
{
    if (deleteButton_) {
        deleteButton_->click();
    }
}

QStringList LayersPanel::vectorMenuTextsForTest(const QString& path)
{
    QMenu menu;
    populateVectorMaskMenu(menu, path);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

void LayersPanel::setDialogDriverForTest(std::function<bool(QDialog&)> driver)
{
    dialogDriver_ = std::move(driver);
}

} // namespace pictura
