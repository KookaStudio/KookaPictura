#include "layers_filter_proxy.h"

#include "layers_panel_internal.h"

namespace pictura {

LayersFilterProxyModel::LayersFilterProxyModel(QAbstractItemModel* source, QObject* parent)
    : QSortFilterProxyModel(parent)
{
    if (source) {
        setSourceModel(source);
    }
}

void LayersFilterProxyModel::setFilter(const LayerFilter& filter)
{
    filter_ = filter;
    invalidate();
}

bool LayersFilterProxyModel::hasActiveCriteria() const
{
    return !filter_.name.isEmpty() || !filter_.kinds.isEmpty() || !filter_.mode.isEmpty()
        || filter_.color >= 0 || !filter_.attribute.isEmpty() || !filter_.effect.isEmpty()
        || filter_.selected;
}

bool LayersFilterProxyModel::rowMatches(const QModelIndex& index) const
{
    if (filter_.selected && !filter_.selectedPaths.contains(index.data(PathRole).toString())) {
        return false;
    }
    if (!filter_.name.isEmpty()
        && !index.data(Qt::DisplayRole)
                .toString()
                .contains(filter_.name, Qt::CaseInsensitive)) {
        return false;
    }
    if (!filter_.kinds.isEmpty() && !filter_.kinds.contains(index.data(KindRole).toString())) {
        return false;
    }
    if (!filter_.mode.isEmpty() && index.data(BlendRole).toString() != filter_.mode) {
        return false;
    }
    if (filter_.color >= 0 && index.data(ColorRole).toInt() != filter_.color) {
        return false;
    }
    if (!filter_.attribute.isEmpty()) {
        const QString& attribute = filter_.attribute;
        bool matches = false;
        if (attribute == QLatin1String("visible")) {
            matches = index.data(VisibleRole).toBool();
        } else if (attribute == QLatin1String("hidden")) {
            matches = !index.data(VisibleRole).toBool();
        } else if (attribute == QLatin1String("locked")) {
            matches = (index.data(LockRole).toInt() & 0x0F) == 0x0F;
        } else if (attribute == QLatin1String("mask")) {
            matches = index.data(HasMaskRole).toBool();
        } else if (attribute == QLatin1String("clipped")) {
            matches = index.data(ClippingRole).toBool();
        }
        if (!matches) {
            return false;
        }
    }
    if (!filter_.effect.isEmpty()
        && !index.data(StyleEffectsRole).toStringList().contains(filter_.effect)) {
        return false;
    }
    return true;
}

// ponytail: O(subtree) recursion per row, so a full filter pass is O(rows^2);
// fine for panel-scale layer counts. Cache a match set if a huge document makes
// it hot.
bool LayersFilterProxyModel::hasMatchingDescendant(const QModelIndex& sourceIndex) const
{
    const QAbstractItemModel* source = sourceModel();
    if (!source) {
        return false;
    }
    const int children = source->rowCount(sourceIndex);
    for (int row = 0; row < children; ++row) {
        const QModelIndex child = source->index(row, 0, sourceIndex);
        if (rowMatches(child) || hasMatchingDescendant(child)) {
            return true;
        }
    }
    return false;
}

bool LayersFilterProxyModel::filterAcceptsRow(int sourceRow,
                                              const QModelIndex& sourceParent) const
{
    if (!filter_.enabled || !hasActiveCriteria()) {
        return true;
    }
    const QAbstractItemModel* source = sourceModel();
    if (!source) {
        return true;
    }
    const QModelIndex index = source->index(sourceRow, 0, sourceParent);
    return rowMatches(index) || hasMatchingDescendant(index);
}

} // namespace pictura

#include "moc_layers_filter_proxy.cpp"
