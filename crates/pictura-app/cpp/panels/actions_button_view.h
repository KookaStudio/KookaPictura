#pragma once

#include "actions_tree_model.h"

#include <QtCore/QAbstractListModel>
#include <QtCore/QList>
#include <QtWidgets/QListView>

namespace pictura {

class ActionsButtonModel : public QAbstractListModel {
    Q_OBJECT

public:
    explicit ActionsButtonModel(QObject* parent = nullptr);

    void setActions(const QList<ActionNodeRow>& actions);
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role = Qt::DisplayRole) const override;
    Qt::ItemFlags flags(const QModelIndex& index) const override;
    QString idAt(int row) const;
    int rowForId(const QString& id) const;
    QList<ActionNodeRow> actions() const { return actions_; }

private:
    QList<ActionNodeRow> actions_;
};

class ActionsButtonView : public QListView {
    Q_OBJECT

public:
    explicit ActionsButtonView(QWidget* parent = nullptr);

    void setActions(const QList<ActionNodeRow>& actions);
    ActionsButtonModel* actionsModel() const { return model_; }
    QStringList actionIdsForTest() const;
    QStringList actionNamesForTest() const;
    QList<QColor> actionColorsForTest() const;
    void activateActionForTest(int row);

signals:
    void actionActivated(const QString& actionId);

private:
    void activateRow(int row);
    ActionsButtonModel* model_ = nullptr;
};

} // namespace pictura
