// The Layers panel's mask button at the foot of the panel, and the Refine Mask
// and Layer Mask Display Options dialogs the mask-thumbnail menu opens (the
// menu itself is in layers_panel_menu.cpp).

#include "layers_panel.h"

#include "dialogs.h"
#include "mask_display_options_dialog.h"
#include "refine_edge_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/refine.cxxqt.h"

#include <QtWidgets/QApplication>
#include <QtWidgets/QToolButton>

namespace pictura {

void LayersPanel::syncStripButtons()
{
    if (!maskButton_) {
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

void LayersPanel::setDialogDriverForTest(std::function<bool(QDialog&)> driver)
{
    dialogDriver_ = std::move(driver);
}

} // namespace pictura
