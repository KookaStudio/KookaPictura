#include "layers_panel.h"

#include "layers_panel_internal.h"
#include "dialogs.h"
#include "layer_new_dialog.h"
#include "layer_style_dialog.h"

#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

#include <QtCore/QHash>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

void LayersPanel::addLayerAt(const QString& path)
{
    if (!view_) {
        return;
    }
    clearSolo();
    const QString created = view_->add_layer_in(path);
    if (!created.isEmpty()) {
        refresh();
        selectPath(created);
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
        selectPath(created);
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
    const QString created = view_->new_layer_dialog(currentPath(), spec.name, spec.color, spec.blend,
                                                    spec.opacity, 255, spec.neutralFill,
                                                    spec.clipping);
    if (!created.isEmpty()) {
        refresh();
        selectPath(created);
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
        view_->new_group_dialog(currentPath(), spec.name, spec.color, spec.blend, spec.opacity);
    if (!created.isEmpty()) {
        expandedPaths_.insert(created);
        refresh();
        selectPath(created);
    }
}

void LayersPanel::openGroupFromLayersDialog()
{
    if (!view_) {
        return;
    }
    const QStringList paths = selectedPaths();
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
        selectPath(created);
    }
}

void LayersPanel::duplicateSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = selectedPaths();
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
    const QStringList paths = selectedPaths();
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    view_->delete_layers(paths);
}

void LayersPanel::groupSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = selectedPaths();
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    const QString created = view_->group_layers(paths);
    if (!created.isEmpty()) {
        expandedPaths_.insert(created);
        refresh();
        selectPath(created);
    }
}

void LayersPanel::ungroupSelection()
{
    if (!view_) {
        return;
    }
    const QStringList paths = selectedPaths();
    if (paths.isEmpty()) {
        return;
    }
    clearSolo();
    view_->ungroup_layers(paths);
}

void LayersPanel::moveCurrent(int delta)
{
    if (!view_) {
        return;
    }
    const QString path = currentPath();
    if (path.isEmpty()) {
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
