#pragma once

#include <QtCore/QAbstractItemModel>
#include <QtCore/QModelIndex>
#include <QtCore/QSet>
#include <QtCore/QSortFilterProxyModel>
#include <QtCore/QString>

namespace pictura {

struct LayerFilter {
    bool enabled = false;
    QString name;
    QSet<QString> kinds;
    QString mode;
    int color = -1;
    QString attribute;
    QString effect;
    /// The Selected dimension: match only the rows whose path is in
    /// `selectedPaths` (the panel's current selection).
    bool selected = false;
    QSet<QString> selectedPaths;
};

class LayersFilterProxyModel : public QSortFilterProxyModel {
    Q_OBJECT

public:
    explicit LayersFilterProxyModel(QAbstractItemModel* source = nullptr,
                                    QObject* parent = nullptr);

    void setFilter(const LayerFilter& filter);
    const LayerFilter& filter() const { return filter_; }
    bool hasActiveCriteria() const;
    bool hasMatchingDescendant(const QModelIndex& sourceIndex) const;

    void setSourceModel(QAbstractItemModel* sourceModel) override;

protected:
    bool filterAcceptsRow(int sourceRow, const QModelIndex& sourceParent) const override;

private:
    bool rowMatches(const QModelIndex& sourceIndex) const;

    LayerFilter filter_;
};

} // namespace pictura
