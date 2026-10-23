#pragma once

#include "actions_panel.h"

#include <QtCore/QByteArray>
#include <QtCore/QDataStream>
#include <QtCore/QIODevice>
#include <QtCore/QMimeData>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>

namespace pictura::actions_internal {

inline const QString kMimeType = QStringLiteral("application/x-pictura-actions");

inline QAction* menuAction(QMenu* menu, const QString& id)
{
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == id) {
            return action;
        }
    }
    return nullptr;
}

inline QString nodeSetId(ActionsTreeModel* model, const QString& nodeId)
{
    ActionNodeRow row = model->rowForId(nodeId);
    while (!row.id.isEmpty() && row.kind != actionKindSet()) {
        row = model->rowForId(row.parentId);
    }
    return row.kind == actionKindSet() ? row.id : QString();
}

inline QString nodeActionId(ActionsTreeModel* model, const QString& nodeId)
{
    ActionNodeRow row = model->rowForId(nodeId);
    while (!row.id.isEmpty() && row.kind != actionKindAction()) {
        row = model->rowForId(row.parentId);
    }
    return row.kind == actionKindAction() ? row.id : QString();
}

inline QStringList decodeMimeIds(const QMimeData* mime)
{
    if (!mime || !mime->hasFormat(kMimeType)) {
        return {};
    }
    QByteArray bytes = mime->data(kMimeType);
    QDataStream stream(&bytes, QIODevice::ReadOnly);
    QStringList ids;
    stream >> ids;
    return ids;
}

} // namespace pictura::actions_internal
