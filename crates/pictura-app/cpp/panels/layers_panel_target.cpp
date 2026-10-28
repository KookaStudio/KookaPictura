#include "layers_panel.h"

#include "layers_panel_model.h"

#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"

#include <QtCore/QItemSelectionModel>
#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtWidgets/QTreeView>

#include <utility>

namespace pictura {

namespace {

// Whether two path lists name the same set, regardless of order.
bool samePathSet(const QStringList& a, const QStringList& b)
{
    if (a.size() != b.size()) {
        return false;
    }
    QStringList left = a;
    QStringList right = b;
    left.sort();
    right.sort();
    return left == right;
}

} // namespace

void LayersPanel::setTransformPending(bool pending)
{
    if (transformPending_ == pending) {
        return;
    }
    transformPending_ = pending;
    // The layer list stays selectable; the editing controls follow the flag.
    syncControls();
    syncStripButtons();
}

void LayersPanel::setPendingTransformResolver(std::function<bool()> resolver)
{
    pendingTransformResolver_ = std::move(resolver);
}

bool LayersPanel::resolvePendingTransform()
{
    if (!pendingTransformResolver_) {
        return true;
    }
    return pendingTransformResolver_();
}

void LayersPanel::writeTarget()
{
    if (!view_ || applyingTarget_ || syncing_) {
        return;
    }
    const QString current = currentPath();
    if (isSyntheticRowPath(current)) {
        return;
    }
    QStringList paths = selectedPaths();
    if (paths.isEmpty()) {
        if (!targetPaths().isEmpty() && !resolvePendingTransform()) {
            applyTargetSelection();
            return;
        }
        set_layer_target(*view_, {}, {});
        rememberTarget();
        // The tree's current row projects the target; with no target it must
        // not keep an active highlight the document does not have.
        const bool wasApplying = applyingTarget_;
        applyingTarget_ = true;
        tree_->selectionModel()->setCurrentIndex(QModelIndex(), QItemSelectionModel::NoUpdate);
        applyingTarget_ = wasApplying;
        return;
    }
    // A current row outside the selection is not a new active layer: keep the
    // target's existing active path instead of resetting it to the first row.
    const QString anchor = paths.contains(current) ? current : layer_selection_anchor(*view_);
    const bool moved = !samePathSet(paths, targetPaths())
        || anchor != layer_selection_anchor(*view_);
    if (moved && !resolvePendingTransform()) {
        applyTargetSelection();
        return;
    }
    set_layer_target(*view_, paths, anchor);
    rememberTarget();
}

void LayersPanel::writeCurrentAnchor()
{
    if (!view_ || applyingTarget_ || syncing_) {
        return;
    }
    const QString current = currentPath();
    if (current.isEmpty() || isSyntheticRowPath(current)) {
        return;
    }
    // Ctrl+arrow moves the current row without changing the selection. Only the
    // active path moves here; the selected set (which a filter may hide) stays
    // the document's. A row outside the selection is not a new active layer.
    const QStringList visible = selectedPaths();
    if (!visible.contains(current)) {
        return;
    }
    const QStringList target = targetPaths();
    if (!target.contains(current)) {
        return;
    }
    if (current != layer_selection_anchor(*view_) && !resolvePendingTransform()) {
        applyTargetSelection();
        return;
    }
    set_layer_target(*view_, target, current);
    rememberTarget();
}

void LayersPanel::applyTargetSelection()
{
    if (!view_ || !model_ || !tree_) {
        return;
    }
    const bool wasApplying = applyingTarget_;
    applyingTarget_ = true;
    const QStringList target = selected_layer_paths(*view_);
    const QString anchor = layer_selection_anchor(*view_);
    setTreeSelection(target, anchor);
    applyingTarget_ = wasApplying;
}

void LayersPanel::rememberTarget()
{
    if (!view_) {
        rememberedTarget_.clear();
        rememberedAnchor_.clear();
        rememberedGeneration_ = 0;
        return;
    }
    rememberedTarget_ = selected_layer_paths(*view_);
    rememberedAnchor_ = layer_selection_anchor(*view_);
    rememberedGeneration_ = view_->document_generation();
}

void LayersPanel::projectTarget(const QString& selectedPath, bool wasEditing)
{
    if (!view_) {
        return;
    }
    const QStringList target = selected_layer_paths(*view_);
    const QString anchor = layer_selection_anchor(*view_);
    const bool generationChanged = view_->document_generation() != rememberedGeneration_;
    const bool moved =
        generationChanged || target != rememberedTarget_ || anchor != rememberedAnchor_;
    applyTargetSelection();
    // A target change moves the current row onto its active path; otherwise
    // restore the row the user had (which may sit outside the selection).
    const QString row = moved ? anchor : selectedPath;
    if (!row.isEmpty()) {
        const QModelIndex index = proxyIndexForPath(row);
        if (index.isValid()) {
            tree_->selectionModel()->setCurrentIndex(index, QItemSelectionModel::NoUpdate);
        }
    }
    if (wasEditing) {
        const QModelIndex editIndex = proxyIndexForPath(row);
        if (editIndex.isValid()) {
            // The selection model's own move keeps the multi-selection; the
            // view's would clear it for a single row.
            tree_->selectionModel()->setCurrentIndex(editIndex, QItemSelectionModel::NoUpdate);
            tree_->edit(editIndex);
        }
    }
    rememberedTarget_ = target;
    rememberedAnchor_ = anchor;
    rememberedGeneration_ = view_->document_generation();
}

QSet<QString> LayersPanel::expandedPaths() const
{
    QSet<QString> paths;
    if (!view_) {
        return paths;
    }
    const QStringList stored = expansion_paths(*view_);
    for (const QString& path : stored) {
        paths.insert(path);
    }
    return paths;
}

void LayersPanel::setExpandedPaths(const QSet<QString>& paths)
{
    if (!view_) {
        return;
    }
    QStringList stored;
    stored.reserve(paths.size());
    for (const QString& path : paths) {
        stored.push_back(path);
    }
    expansion_set(*view_, stored);
}

void LayersPanel::expandPath(const QString& path)
{
    QSet<QString> paths = expandedPaths();
    if (!paths.contains(path)) {
        paths.insert(path);
        setExpandedPaths(paths);
    }
}

void LayersPanel::collapsePath(const QString& path)
{
    QSet<QString> paths = expandedPaths();
    if (paths.remove(path)) {
        setExpandedPaths(paths);
    }
}

} // namespace pictura
