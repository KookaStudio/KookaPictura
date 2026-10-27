#include "layers_panel.h"

#include "layers_panel_internal.h"
#include "dialogs.h"
#include "layer_new_dialog.h"
#include "layer_style_dialog.h"

#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_arrange.cxxqt.h"

#include <QtCore/QHash>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

void LayersPanel::syncEditTarget()
{
    // Mirrors the bridge's `active_mask_target`: a target only routes strokes
    // while its layer is the active one.
    const QString target = view_ ? mask_edit_target_get(*view_) : QString();
    const bool mask = !target.isEmpty() && target == layer_selection_anchor(*view_);
    if (mask != maskEditing_) {
        maskEditing_ = mask;
        emit editTargetChanged(mask);
    }
}

void LayersPanel::syncMaskSolo()
{
    // A solo tied to another layer ends when the current layer changes (a layer
    // thumbnail/row click, another layer's mask thumbnail, a menu, anything):
    // the mask plane belongs to its own layer, so showing the composite again.
    if (!view_) {
        return;
    }
    const QString active = layer_selection_anchor(*view_);
    const QString soloed = mask_solo_get(*view_);
    if (!soloed.isEmpty() && soloed != active) {
        mask_solo_set(*view_, QString());
    }
    // The rubylith overlay follows the same rule; its path is positional, so
    // a structural edit that moves the layer ends it rather than letting it
    // land on another layer's mask.
    const QString overlay = mask_overlay_path(*view_);
    if (!overlay.isEmpty() && overlay != active) {
        mask_overlay_toggle(*view_, overlay);
        emit maskDisplayChanged();
    }
}

void LayersPanel::addLayerAt(const QString& path)
{
    if (!view_) {
        return;
    }
    clearSolo();
    const QString created = view_->add_layer_in(path);
    if (!created.isEmpty()) {
        refresh();
    }
}

void LayersPanel::addGroupAt(const QString& path)
{
    if (!view_) {
        return;
    }
    clearSolo();
    const QString created = view_->add_group_in(path);
    if (!created.isEmpty()) {
        expandPath(created);
        refresh();
    }
}

void LayersPanel::openNewLayerDialog()
{
    if (!view_) {
        return;
    }
    LayerNewSpec spec;
    if (!LayerNewDialog::get(false, this, &spec)) {
        return;
    }
    clearSolo();
    const QString created = view_->new_layer_dialog(layer_selection_anchor(*view_), spec.name, spec.color, spec.blend,
                                                    spec.opacity, 255, spec.neutralFill,
                                                    spec.clipping);
    if (!created.isEmpty()) {
        refresh();
    }
}

void LayersPanel::openNewGroupDialog()
{
    if (!view_) {
        return;
    }
    LayerNewSpec spec;
    if (!LayerNewDialog::get(true, this, &spec)) {
        return;
    }
    clearSolo();
    const QString created =
        view_->new_group_dialog(layer_selection_anchor(*view_), spec.name, spec.color, spec.blend, spec.opacity);
    if (!created.isEmpty()) {
        expandPath(created);
        refresh();
    }
}

void LayersPanel::openGroupFromLayersDialog()
{
    if (!view_) {
        return;
    }
    const QStringList paths = view_ ? selected_layer_paths(*view_) : QStringList{};
    if (paths.isEmpty()) {
        return;
    }
    LayerNewSpec spec;
    if (!LayerNewDialog::get(true, this, &spec)) {
        return;
    }
    clearSolo();
    const QString created = view_->group_from_layers_dialog(paths, spec.name, spec.color, spec.blend,
                                                            spec.opacity);
    if (!created.isEmpty()) {
        expandPath(created);
        refresh();
    }
}

void LayersPanel::duplicateSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = view_ ? selected_layer_paths(*view_) : QStringList{};
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    const QStringList created = view_->duplicate_layers(paths);
    if (!created.isEmpty()) {
        refresh();
    }
}

void LayersPanel::deleteSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = view_ ? selected_layer_paths(*view_) : QStringList{};
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    view_->delete_layers(paths);
}

void LayersPanel::deleteActiveMask()
{
    if (!view_) {
        return;
    }
    const QString target = mask_edit_target_get(*view_);
    if (target.isEmpty()) {
        return;
    }
    // No confirmation dialog: Delete removes the mask without applying it, as
    // the mask menu's Delete Layer Mask does.
    layer_mask_delete_path(*view_, target);
    syncEditTarget();
}

void LayersPanel::groupSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = view_ ? selected_layer_paths(*view_) : QStringList{};
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    const QString created = view_->group_layers(paths);
    if (!created.isEmpty()) {
        expandPath(created);
        refresh();
    }
}

void LayersPanel::ungroupSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = view_ ? selected_layer_paths(*view_) : QStringList{};
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    view_->ungroup_layers(paths);
}

bool LayersPanel::canMoveCurrent(int delta) const
{
    if (!view_) {
        return false;
    }
    const QString path = view_ ? layer_selection_anchor(*view_) : QString();
    return !path.isEmpty() && canMovePath(path, delta);
}

bool LayersPanel::canMovePath(const QString& path, int delta) const
{
    return view_ && !path.isEmpty() && pictura::can_move_layer_path(*view_, path, delta);
}

void LayersPanel::moveCurrent(int delta)
{
    if (!view_) {
        return;
    }
    const QString path = view_ ? layer_selection_anchor(*view_) : QString();
    if (path.isEmpty() || !canMovePath(path, delta)) {
        return;
    }
    clearSolo();
    view_->move_layer_path(path, delta);
}

void LayersPanel::toggleSolo(const QString& path)
{
    if (path.isEmpty() || !view_) {
        return;
    }
    if (solo_active(*view_)) {
        restoreSolo();
        return;
    }
    solo_set(*view_, path);
}

void LayersPanel::restoreSolo()
{
    if (view_) {
        solo_restore(*view_);
    }
}

void LayersPanel::clearSolo()
{
    // End any mask solo view too: every shared reset (add/duplicate/group/
    // ungroup/delete) restores the composite.
    endMaskSoloView();
    if (view_) {
        solo_clear(*view_);
    }
}

void LayersPanel::endMaskSoloView()
{
    if (!view_) {
        return;
    }
    mask_solo_set(*view_, QString());
    const QString overlay = mask_overlay_path(*view_);
    if (!overlay.isEmpty()) {
        mask_overlay_toggle(*view_, overlay);
        emit maskDisplayChanged();
    }
}

void LayersPanel::openBackgroundConversion(const QString& path)
{
    if (!view_ || path.isEmpty()) {
        return;
    }
    LayerNewSpec spec;
    if (bgConvertArmed_) {
        if (!bgConvertAccept_) {
            return;
        }
        spec.name = bgConvertName_;
        spec.color = bgConvertColor_;
    } else {
        const QString defaultName = view_->next_layer_name(QStringLiteral("Layer"));
        if (!LayerNewDialog::getNameColor(this, defaultName, &spec)) {
            return;
        }
    }
    if (view_->convert_background(path, spec.name, spec.color)) {
        refresh();
    }
}

void LayersPanel::applyFxButton(bool alt)
{
    if (!view_) {
        return;
    }
    if (alt) {
        layer_style_set_all_visible(*view_, !layer_style_any_visible(*view_, true));
    } else {
        openLayerStyle(layer_selection_anchor(*view_));
    }
}

void LayersPanel::openLayerStyle(const QString& path, const QString& effect)
{
    if (!view_ || path.isEmpty() || !layer_style_can_edit(*view_, path)) {
        return;
    }
    LayerStyleDialog dialog(view_, path, effect, this);
    if (!runPanelDialog(dialog)) {
        // A rejected dialog drops its live edits; the driver path skips exec.
        dialog.reject();
    }
    refresh();
}

void LayersPanel::toggleEffectVisibility(const QModelIndex& index, const QString& kind)
{
    if (!view_) {
        return;
    }
    const QString path = pathForProxyIndex(index);
    const int at = path.indexOf(QStringLiteral("/@fx"));
    if (at < 0) {
        return;
    }
    const QString base = path.left(at);
    const bool on = !index.data(VisibleRole).toBool();
    if (kind == QLatin1String("layer-effects")) {
        if (layer_style_set(*view_, base, QStringLiteral("fx.visible"), on ? 1.0 : 0.0)) {
            layer_style_commit(*view_,
                               on ? tr("Enable Layer Effects") : tr("Disable Layer Effects"));
        }
    } else {
        const QString effect = path.mid(at + 5);
        if (layer_style_set(*view_, base, effect + QStringLiteral(".on"), on ? 1.0 : 0.0)) {
            layer_style_commit(*view_, (on ? tr("Show %1") : tr("Hide %1"))
                                            .arg(LayerStyleDialog::effectTitle(effect)));
        }
    }
    refresh();
}

} // namespace pictura
