#include "layers_panel.h"

#include "layers_panel_internal.h"
#include "percent_field.h"

#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/adjustment_edit.cxxqt.h"

#include <QtCore/QModelIndex>
#include <QtCore/QStringList>
#include <QtCore/QTimer>

namespace pictura {

QStringList LayersPanel::propertyPaths() const
{
    QStringList paths;
    if (!model_) {
        return paths;
    }
    const QStringList targets = view_ ? selected_layer_paths(*view_) : QStringList{};
    for (const QString& path : targets) {
        const QModelIndex index = model_->indexForPath(path);
        if (index.isValid() && !index.data(SyntheticRole).toBool()) {
            paths.push_back(path);
        }
    }
    return paths;
}

QStringList LayersPanel::namePathsFor(const QStringList& paths) const
{
    QStringList names;
    if (!model_) {
        return names;
    }
    for (const QString& path : paths) {
        const QString name = model_->namePathForPath(path);
        if (name.isEmpty()) {
            return {};
        }
        names.push_back(name);
    }
    return names;
}

QStringList LayersPanel::gestureTargets() const
{
    if (!gesture_.valid || !model_) {
        return {};
    }
    QStringList paths;
    for (int i = 0; i < gesture_.paths.size(); ++i) {
        const QString expected = gesture_.namePaths.value(i);
        QModelIndex index = model_->indexForPath(gesture_.paths.at(i));
        if (!index.isValid()
            || model_->namePathForPath(model_->pathForIndex(index)) != expected) {
            index = model_->indexForNamePath(expected);
        }
        if (!index.isValid()) {
            return {};
        }
        paths.push_back(model_->pathForIndex(index));
    }
    return paths;
}

void LayersPanel::schedulePreview(bool fill, int value)
{
    if (syncing_ || !view_) {
        return;
    }
    const QStringList paths = propertyPaths();
    if (paths.isEmpty()) {
        return;
    }
    const quint64 generation = view_->document_generation();
    const QStringList names = namePathsFor(paths);
    if (gesture_.valid) {
        if (gesture_.fill != fill || gesture_.generation != generation
            || names != gesture_.namePaths) {
            cancelPendingPreview();
            return;
        }
        gesture_.paths = paths;
    } else {
        gesture_.paths = paths;
        gesture_.namePaths = names;
        gesture_.fill = fill;
        gesture_.generation = generation;
        gesture_.valid = true;
    }
    pendingPreview_.paths = paths;
    pendingPreview_.namePaths = names;
    pendingPreview_.fill = fill;
    pendingPreview_.value = value;
    pendingPreview_.generation = generation;
    pendingPreview_.valid = true;
    if (!previewTimer_->isActive()) {
        previewTimer_->start();
    }
}

void LayersPanel::applyPendingPreview()
{
    if (!pendingPreview_.valid) {
        return;
    }
    const PendingPreview pending = pendingPreview_;
    pendingPreview_ = PendingPreview{};
    if (!gesture_.valid || !view_ || pending.fill != gesture_.fill
        || pending.generation != gesture_.generation
        || view_->document_generation() != gesture_.generation) {
        cancelPendingPreview();
        return;
    }
    const QStringList paths = gestureTargets();
    if (paths.isEmpty()) {
        cancelPendingPreview();
        return;
    }
    if (pending.fill) {
        view_->preview_layers_fill(paths, pending.value);
    } else {
        view_->preview_layers_opacity(paths, pending.value);
    }
}

void LayersPanel::clearPendingPreview()
{
    if (previewTimer_) {
        previewTimer_->stop();
    }
    pendingPreview_ = PendingPreview{};
    gesture_ = PendingPreview{};
}

void LayersPanel::cancelPendingPreview()
{
    clearPendingPreview();
    if (view_) {
        adjustment_cancel(*view_);
    }
    if (opacity_) {
        opacity_->cancelPending();
    }
    if (fill_) {
        fill_->cancelPending();
    }
}

void LayersPanel::reconcileGestureTarget()
{
    if (!gesture_.valid) {
        return;
    }
    if (!view_ || view_->document_generation() != gesture_.generation) {
        cancelPendingPreview();
        return;
    }
    const QStringList targets = propertyPaths();
    if (targets.isEmpty()) {
        // An empty selection is the transient model-reset state; a present
        // selection with no real target (a synthetic row) is a real change.
        if (!targetPaths().isEmpty()) {
            cancelPendingPreview();
        }
        return;
    }
    if (namePathsFor(targets) != gesture_.namePaths) {
        cancelPendingPreview();
        return;
    }
    bool eligible = false;
    for (const QString& path : targets) {
        const QModelIndex index = model_->indexForPath(path);
        if (!index.isValid()) {
            continue;
        }
        const QString kind = index.data(KindRole).toString();
        const bool background = kind == QLatin1String("background");
        const bool group = kind == QLatin1String("group");
        const bool fullLock = (index.data(LockRole).toInt() & 0x0F) == 0x0F;
        if (!background && !fullLock && (!gesture_.fill || !group)) {
            eligible = true;
            break;
        }
    }
    if (!eligible) {
        cancelPendingPreview();
        return;
    }
    gesture_.paths = targets;
    if (pendingPreview_.valid) {
        pendingPreview_.paths = targets;
    }
}

} // namespace pictura
