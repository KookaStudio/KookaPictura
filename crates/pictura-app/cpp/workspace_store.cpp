#include "workspace_store.h"

#include "session.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QHash>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonParseError>
#include "protected_output.h"
#include <QtCore/QSet>
#include <QtCore/QStringList>

namespace pictura {

namespace {

// Workspaces are small JSON; anything larger is corrupt or hostile.
constexpr qint64 kMaxStoreBytes = 4 * 1024 * 1024;
const QString kIndexName = QStringLiteral("index.json");

// The file's top-level object, or empty when missing/corrupt. Used so keys this
// build does not know survive a load-then-write.
QJsonObject readObject(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() > kMaxStoreBytes) {
        return {};
    }
    QJsonParseError error{};
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !doc.isObject()) {
        return {};
    }
    return doc.object();
}

bool writeObject(const QString& path, const QJsonObject& obj)
{
    pictura::ProtectedOutput file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    if (file.write(QJsonDocument(obj).toJson(QJsonDocument::Compact)) < 0) {
        return false;
    }
    return file.commit();
}

QString workspaceFilePath(const QString& id)
{
    return workspaceStoreDir() + QLatin1Char('/') + id + QStringLiteral(".json");
}

} // namespace

QString workspaceStoreDir()
{
    return QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/workspaces");
}

bool loadWorkspaceStore(WorkspaceStore* out)
{
    if (out == nullptr) {
        return false;
    }
    *out = WorkspaceStore{};

    const QString indexPath = workspaceStoreDir() + QLatin1Char('/') + kIndexName;
    QFile indexFile(indexPath);
    if (!indexFile.open(QIODevice::ReadOnly) || indexFile.size() > kMaxStoreBytes) {
        return false;
    }
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(indexFile.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        return false;
    }
    const QJsonObject index = doc.object();
    out->schemaVersion = index.value(QStringLiteral("schemaVersion")).toInt(1);
    out->activeWorkspace = index.value(QStringLiteral("activeWorkspace")).toString();

    QHash<QString, QJsonObject> entries;
    QStringList entryOrder;
    for (const QJsonValue& value : index.value(QStringLiteral("entries")).toArray()) {
        const QJsonObject entry = value.toObject();
        const QString id = entry.value(QStringLiteral("id")).toString();
        if (!id.isEmpty() && !entries.contains(id)) {
            entries.insert(id, entry);
            entryOrder.append(id);
        }
    }

    QStringList order;
    for (const QJsonValue& value : index.value(QStringLiteral("order")).toArray()) {
        if (value.isString()) {
            order.append(value.toString());
        }
    }
    if (order.isEmpty()) {
        order = entryOrder;
    }

    for (const QString& id : order) {
        const QJsonObject file = readObject(workspaceFilePath(id));
        if (file.isEmpty()) {
            continue;
        }
        const QJsonObject entry = entries.value(id);
        WorkspaceRecord record;
        record.id = id;
        record.name = file.value(QStringLiteral("name"))
                          .toString(entry.value(QStringLiteral("name")).toString());
        record.kind = file.value(QStringLiteral("kind"))
                          .toString(entry.value(QStringLiteral("kind")).toString());
        record.layout = file.value(QStringLiteral("layout")).toArray();
        record.factory = file.value(QStringLiteral("factory")).toArray();
        record.hasLayout = file.contains(QStringLiteral("layout"));
        out->workspaces.append(record);
    }
    return true;
}

bool saveWorkspaceStore(const WorkspaceStore& store)
{
    const QString dir = workspaceStoreDir();
    if (!QDir().mkpath(dir)) {
        return false;
    }

    const QString indexPath = dir + QLatin1Char('/') + kIndexName;
    const QJsonObject existingIndex = readObject(indexPath);

    // Merge each record's entry onto the entry already on disk for that id so
    // keys this build does not know survive the rewrite.
    QHash<QString, QJsonObject> existingEntries;
    for (const QJsonValue& value :
         existingIndex.value(QStringLiteral("entries")).toArray()) {
        const QJsonObject entry = value.toObject();
        const QString id = entry.value(QStringLiteral("id")).toString();
        if (!id.isEmpty()) {
            existingEntries.insert(id, entry);
        }
    }

    QJsonArray order;
    QJsonArray entries;
    QSet<QString> ids;
    for (const WorkspaceRecord& record : store.workspaces) {
        order.append(record.id);
        ids.insert(record.id);
        QJsonObject entry = existingEntries.value(record.id);
        entry.insert(QStringLiteral("id"), record.id);
        entry.insert(QStringLiteral("name"), record.name);
        entry.insert(QStringLiteral("kind"), record.kind);
        entries.append(entry);
    }

    // Write every workspace file before the index so the index never references
    // a file that failed to write.
    bool ok = true;
    for (const WorkspaceRecord& record : store.workspaces) {
        QJsonObject obj = readObject(workspaceFilePath(record.id));
        obj.insert(QStringLiteral("id"), record.id);
        obj.insert(QStringLiteral("name"), record.name);
        obj.insert(QStringLiteral("kind"), record.kind);
        obj.insert(QStringLiteral("factory"), record.factory);
        if (record.hasLayout) {
            obj.insert(QStringLiteral("layout"), record.layout);
        } else {
            obj.remove(QStringLiteral("layout"));
        }
        ok = writeObject(workspaceFilePath(record.id), obj) && ok;
    }
    if (!ok) {
        return false;
    }

    QJsonObject index = existingIndex;
    index.insert(QStringLiteral("schemaVersion"), store.schemaVersion);
    index.insert(QStringLiteral("activeWorkspace"), store.activeWorkspace);
    index.insert(QStringLiteral("order"), order);
    index.insert(QStringLiteral("entries"), entries);
    if (!writeObject(indexPath, index)) {
        return false;
    }

    // Garbage-collect per-id files no longer referenced by the store.
    const QStringList files = QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files);
    for (const QString& fileName : files) {
        if (fileName == kIndexName) {
            continue;
        }
        const QString id = fileName.left(fileName.size() - 5);
        if (!ids.contains(id)) {
            QFile::remove(dir + QLatin1Char('/') + fileName);
        }
    }
    return true;
}

} // namespace pictura
