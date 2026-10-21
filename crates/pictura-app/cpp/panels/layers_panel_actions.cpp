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
        expandedPaths_.insert(created);
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
        expandedPaths_.insert(created);
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
        expandedPaths_.insert(created);
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
        selectPath(created.last());
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
        expandedPaths_.insert(created);
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
    if (soloActive_) {
        restoreSolo();
        return;
    }
    const QHash<QString, bool> snapshot = model_->visibilityByPath();
    if (snapshot.isEmpty()) {
        return;
    }
    soloSnapshot_ = snapshot;
    soloActive_ = true;
    soloPath_ = path;
    view_->apply_visibility(soloPaths(path, snapshot), QStringLiteral("Solo Visibility"));
}

void LayersPanel::restoreSolo()
{
    if (!view_) {
        clearSolo();
        return;
    }
    QStringList visible;
    for (auto it = soloSnapshot_.cbegin(); it != soloSnapshot_.cend(); ++it) {
        if (it.value()) {
            visible.push_back(it.key());
        }
    }
    clearSolo();
    view_->apply_visibility(visible, QStringLiteral("Restore Visibility"));
}

void LayersPanel::clearSolo()
{
    // End any mask solo view too: every shared reset (add/duplicate/group/
    // ungroup/delete, document switch) restores the composite.
    if (view_) {
        mask_solo_set(*view_, QString());
        const QString overlay = mask_overlay_path(*view_);
        if (!overlay.isEmpty()) {
            mask_overlay_toggle(*view_, overlay);
            emit maskDisplayChanged();
        }
    }
    soloActive_ = false;
    soloPath_.clear();
    soloSnapshot_.clear();
}

QStringList LayersPanel::soloPaths(const QString& path, const QHash<QString, bool>& snapshot) const
{
    QStringList result{path};
    int slash = path.lastIndexOf(QLatin1Char('/'));
    while (slash > 0) {
        const QString ancestor = path.left(slash);
        result.push_back(ancestor);
        slash = ancestor.lastIndexOf(QLatin1Char('/'));
    }
    if (model_->indexForPath(path).data(KindRole).toString() == QLatin1String("group")) {
        const QString prefix = path + QLatin1Char('/');
        for (auto it = snapshot.cbegin(); it != snapshot.cend(); ++it) {
            if (it.value() && it.key().startsWith(prefix)) {
                result.push_back(it.key());
            }
        }
    }
    result.removeDuplicates();
    return result;
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
        selectPath(path);
    }
}

void LayersPanel::openLayerStyle(const QString& path)
{
    if (!view_ || path.isEmpty() || !layer_style_can_edit(*view_, path)) {
        return;
    }
    LayerStyleDialog dialog(view_, path, QString(), this);
    runDialog(dialog, this);
    refresh();
}

} // namespace pictura
