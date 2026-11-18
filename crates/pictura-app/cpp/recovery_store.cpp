#include "recovery_store.h"

#include "session.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLockFile>
#include <QtCore/QUuid>

#include <optional>

namespace pictura {

namespace {

constexpr qint64 kMaxManifestBytes = 64 * 1024;

QString lockPath(const QString& dir)
{
    return dir + QStringLiteral("/lock");
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

void removeLockedDirectory(const QString& path)
{
    QDir dir(path);
    for (const QFileInfo& entry : dir.entryInfoList(QDir::AllEntries | QDir::Hidden
                                                   | QDir::System | QDir::NoDotAndDotDot)) {
        if (entry.fileName() == QStringLiteral("lock")) {
            continue;
        }
        if (entry.isDir() && !entry.isSymLink()) {
            if (!QDir(entry.absoluteFilePath()).removeRecursively()) {
                return;
            }
        } else if (!QFile::remove(entry.absoluteFilePath())) {
            return;
        }
    }
    QFile::remove(lockPath(path));
    QDir().rmdir(path);
}

bool validSessionDirectory(const QString& dir)
{
    const QFileInfo entry(dir);
    return entry.isDir() && !entry.isSymLink()
        && entry.absoluteFilePath() != RecoveryStore::quarantineRoot()
        && (entry.absolutePath() == RecoveryStore::rootPath()
            || entry.absolutePath() == RecoveryStore::versionedRootPath());
}

QString localFile(const QString& dir, const QString& filename)
{
    if (filename.isEmpty() || filename == QStringLiteral(".") || filename == QStringLiteral("..")
        || filename.contains(QLatin1Char('/')) || filename.contains(QLatin1Char('\\'))) {
        return {};
    }
    const QFileInfo info(QDir(dir).filePath(filename));
    return info.isFile() && !info.isSymLink() ? info.absoluteFilePath() : QString();
}

std::optional<RecoverableDocument> readManifest(const QString& dir, const QString& file,
                                                 bool versioned)
{
    const QString manifestFile = localFile(dir, file);
    QFile manifest(manifestFile);
    if (!manifest.open(QIODevice::ReadOnly) || manifest.size() > kMaxManifestBytes) {
        return std::nullopt;
    }
    const QJsonObject obj = QJsonDocument::fromJson(manifest.readAll()).object();
    RecoverableDocument doc;
    doc.manifest = manifestFile;
    doc.versioned = versioned;
    if (versioned) {
        if (obj.value(QStringLiteral("version")).toInt() != 2) {
            return std::nullopt;
        }
        doc.snapshot = localFile(dir, obj.value(QStringLiteral("snapshot")).toString());
        for (const auto& key : {QStringLiteral("resources"), QStringLiteral("journal")}) {
            const QString companion = localFile(dir, obj.value(key).toString());
            if (!companion.isEmpty()) {
                doc.companions.append(companion);
            }
        }
        if (doc.snapshot.isEmpty()) {
            // Keep the manifest discoverable. Its chain may still be repairable.
            doc.snapshot = manifestFile;
        }
    } else {
        doc.snapshot = localFile(dir, QFileInfo(file).completeBaseName() + QStringLiteral(".psd"));
        if (doc.snapshot.isEmpty()) {
            return std::nullopt;
        }
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

QString RecoveryStore::versionedRootPath()
{
    return QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/recovery-v2");
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
    const QString dir = versionedRootPath() + QStringLiteral("/%1-%2")
        .arg(QCoreApplication::applicationPid())
        .arg(QUuid::createUuid().toString(QUuid::WithoutBraces));
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

void RecoveryStore::close()
{
    if (!isOpen()) {
        return;
    }
    removeLockedDirectory(dir_);
    lock_.reset();
    dir_.clear();
}

std::unique_ptr<QLockFile> RecoveryStore::lockOrphan(const QString& dir)
{
    return validSessionDirectory(dir) ? tryLock(dir) : nullptr;
}

OrphanSession RecoveryStore::readLocked(const QString& dir)
{
    OrphanSession session;
    session.dir = dir;
    const bool versioned = QFileInfo(dir).absolutePath() == versionedRootPath();
    const auto manifests = QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files, QDir::Name);
    for (const QString& file : manifests) {
        if (const auto doc = readManifest(dir, file, versioned)) {
            session.documents.append(*doc);
        } else if (versioned) {
            // An older reader or a broken manifest must not trigger empty-session cleanup.
            session.preserve = true;
        }
    }
    return session;
}

QList<OrphanSession> RecoveryStore::orphans(const QString& exceptDir)
{
    QList<OrphanSession> out;
    const QString except = QFileInfo(exceptDir).absoluteFilePath();
    for (const QString& root : {rootPath(), versionedRootPath()}) {
        for (const QFileInfo& entry : QDir(root).entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot)) {
            const QString dir = entry.absoluteFilePath();
            if (dir != except) {
                if (auto lock = lockOrphan(dir)) {
                    out.append(readLocked(dir));
                }
            }
        }
    }
    return out;
}

void RecoveryStore::discard(const QString& dir)
{
    if (auto lock = lockOrphan(dir); lock && !readLocked(dir).preserve) {
        discardLocked(dir);
    }
}

void RecoveryStore::discardLocked(const QString& dir)
{
    if (validSessionDirectory(dir)) {
        removeLockedDirectory(dir);
    }
}

bool RecoveryStore::retire(const RecoverableDocument& doc)
{
    if (!QFile::remove(doc.manifest)) {
        return false;
    }
    QFile::remove(doc.snapshot);
    for (const QString& companion : doc.companions) {
        QFile::remove(companion);
    }
    return true;
}

bool RecoveryStore::quarantine(const QString& sessionDir, const RecoverableDocument& doc)
{
    if (!validSessionDirectory(sessionDir)) {
        return false;
    }
    const QString destDir = QDir(quarantineRoot()).filePath(QFileInfo(sessionDir).fileName());
    if (!QDir().mkpath(destDir)) {
        return false;
    }
    QStringList files{doc.snapshot, doc.manifest};
    files.append(doc.companions);
    QStringList copied;
    for (const QString& path : files) {
        const QFileInfo entry(path);
        const QString target = QDir(destDir).filePath(entry.fileName());
        if (entry.absolutePath() == sessionDir && !entry.isSymLink()) {
            QFile::remove(target);
            if (QFile::copy(path, target)) {
                copied.append(target);
                continue;
            }
        }
        break;
    }
    if (copied.size() == files.size() && retire(doc)) {
        return true;
    }
    for (const QString& path : copied) {
        QFile::remove(path);
    }
    return false;
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
