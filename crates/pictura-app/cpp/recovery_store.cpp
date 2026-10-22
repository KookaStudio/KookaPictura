#include "recovery_store.h"

#include "session.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLockFile>
#include <QtCore/QSaveFile>

#include <optional>

namespace pictura {

namespace {

constexpr qint64 kMaxManifestBytes = 64 * 1024;

QString lockPath(const QString& dir)
{
    return dir + QStringLiteral("/lock");
}

QString manifestPath(const QString& dir, int id)
{
    return dir + QStringLiteral("/%1.json").arg(id);
}

// A lock is stale only when its owner is gone; age alone must not let a later
// launch steal a long-running instance's session.
std::unique_ptr<QLockFile> tryLock(const QString& dir)
{
    auto lock = std::make_unique<QLockFile>(lockPath(dir));
    lock->setStaleLockTime(0);
    if (!lock->tryLock(0)) {
        return nullptr;
    }
    return lock;
}

std::optional<RecoverableDocument> readManifest(const QString& dir, const QString& file)
{
    QFile manifest(dir + QLatin1Char('/') + file);
    if (!manifest.open(QIODevice::ReadOnly) || manifest.size() > kMaxManifestBytes) {
        return std::nullopt;
    }
    const QJsonObject obj = QJsonDocument::fromJson(manifest.readAll()).object();
    RecoverableDocument doc;
    doc.snapshot = dir + QLatin1Char('/') + QFileInfo(file).completeBaseName()
                   + QStringLiteral(".psd");
    if (!QFileInfo::exists(doc.snapshot)) {
        return std::nullopt;
    }
    doc.name = obj.value(QStringLiteral("name")).toString();
    doc.originalPath = obj.value(QStringLiteral("originalPath")).toString();
    doc.savedAt = QFileInfo(doc.snapshot).lastModified();
    if (doc.name.isEmpty()) {
        doc.name = QFileInfo(doc.snapshot).fileName();
    }
    return doc;
}

} // namespace

RecoveryStore::RecoveryStore() = default;

RecoveryStore::~RecoveryStore() = default;

QString RecoveryStore::rootPath()
{
    return QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/recovery");
}

QString RecoveryStore::quarantineRoot()
{
    return rootPath() + QStringLiteral("/quarantine");
}

bool RecoveryStore::open()
{
    if (isOpen()) {
        return true;
    }
    const QString dir = rootPath() + QStringLiteral("/%1-%2")
                                         .arg(QCoreApplication::applicationPid())
                                         .arg(QDateTime::currentMSecsSinceEpoch());
    if (!QDir().mkpath(dir)) {
        return false;
    }
    lock_ = tryLock(dir);
    if (!lock_) {
        QDir(dir).removeRecursively();
        return false;
    }
    dir_ = dir;
    return true;
}

QString RecoveryStore::snapshotPath(int id) const
{
    return dir_ + QStringLiteral("/%1.psd").arg(id);
}

bool RecoveryStore::writeManifest(int id, const QString& name, const QString& originalPath)
{
    if (!isOpen()) {
        return false;
    }
    QJsonObject obj;
    obj.insert(QStringLiteral("name"), name);
    obj.insert(QStringLiteral("originalPath"), originalPath);
    QSaveFile file(manifestPath(dir_, id));
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Compact));
    return file.commit();
}

void RecoveryStore::remove(int id)
{
    if (!isOpen()) {
        return;
    }
    QFile::remove(snapshotPath(id));
    QFile::remove(manifestPath(dir_, id));
}

void RecoveryStore::close()
{
    if (!isOpen()) {
        return;
    }
    lock_->unlock();
    lock_.reset();
    QDir(dir_).removeRecursively();
    dir_.clear();
}

QList<OrphanSession> RecoveryStore::orphans(const QString& exceptDir)
{
    QList<OrphanSession> out;
    const QDir root(rootPath());
    const QString except = QFileInfo(exceptDir).absoluteFilePath();
    const QString quarantine = QFileInfo(quarantineRoot()).absoluteFilePath();
    for (const QFileInfo& entry : root.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        const QString dir = entry.absoluteFilePath();
        if (dir == except || dir == quarantine) {
            continue;
        }
        // Taking the lock proves the owner is gone; release it straight away so
        // the session stays discoverable until it is recovered or discarded.
        if (!tryLock(dir)) {
            continue;
        }
        OrphanSession session;
        session.dir = dir;
        const QStringList manifests =
            QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files, QDir::Name);
        for (const QString& file : manifests) {
            if (auto doc = readManifest(dir, file)) {
                session.documents.append(*doc);
            }
        }
        out.append(session);
    }
    return out;
}

void RecoveryStore::discard(const QString& dir)
{
    const QString absolute = QFileInfo(dir).absoluteFilePath();
    if (QFileInfo(dir).absolutePath() != QFileInfo(rootPath()).absoluteFilePath()
        || absolute == QFileInfo(quarantineRoot()).absoluteFilePath()) {
        return;
    }
    QDir(dir).removeRecursively();
}

bool RecoveryStore::quarantine(const QString& sessionDir, const RecoverableDocument& doc)
{
    const QString root = QFileInfo(rootPath()).absoluteFilePath();
    const QString session = QFileInfo(sessionDir).absoluteFilePath();
    if (QFileInfo(session).absolutePath() != root
        || QFileInfo(doc.snapshot).absolutePath() != session) {
        return false;
    }
    const QString manifest = session + QLatin1Char('/')
                             + QFileInfo(doc.snapshot).completeBaseName()
                             + QStringLiteral(".json");
    if (!QFileInfo::exists(manifest)) {
        return false;
    }
    const QString destDir = quarantineRoot() + QLatin1Char('/') + QFileInfo(session).fileName();
    if (!QFileInfo(destDir).absoluteFilePath().startsWith(
            QFileInfo(quarantineRoot()).absoluteFilePath() + QLatin1Char('/'))) {
        return false;
    }
    if (!QDir().mkpath(destDir)) {
        return false;
    }
    const QString snapshotDest = destDir + QLatin1Char('/') + QFileInfo(doc.snapshot).fileName();
    const QString manifestDest = destDir + QLatin1Char('/') + QFileInfo(manifest).fileName();
    // A crash after an earlier copy but before the originals were removed
    // leaves a stale quarantine copy; the originals are still intact here, so
    // replacing the stale copy is safe and lets the retry proceed.
    QFile::remove(snapshotDest);
    QFile::remove(manifestDest);
    if (!QFile::copy(doc.snapshot, snapshotDest)) {
        return false;
    }
    if (!QFile::copy(manifest, manifestDest)) {
        QFile::remove(snapshotDest);
        return false;
    }
    // Drop the manifest first: if that fails, both originals are intact. A
    // failed snapshot removal leaves the quarantine copy as the survivor.
    if (!QFile::remove(manifest)) {
        QFile::remove(snapshotDest);
        QFile::remove(manifestDest);
        return false;
    }
    QFile::remove(doc.snapshot);
    return true;
}

QString recoveredDocumentName(const QString& name)
{
    const QFileInfo info(name);
    const QString suffix = info.suffix();
    if (suffix.isEmpty() || suffix.contains(QLatin1Char(' '))) {
        return name + QStringLiteral("-Recovered");
    }
    return info.completeBaseName() + QStringLiteral("-Recovered.") + suffix;
}

} // namespace pictura
