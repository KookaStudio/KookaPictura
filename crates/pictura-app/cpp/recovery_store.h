#pragma once

#include <QtCore/QDateTime>
#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <memory>

class QLockFile;

namespace pictura {

// One document a crashed session left a snapshot for.
struct RecoverableDocument {
    QString manifest;
    QString snapshot;      // the PSD/PSB checkpoint
    QStringList companions;
    bool versioned = false;
    QString name;          // the tab name when it was written
    QString originalPath;  // the file it was opened from or saved to; empty if untitled
    QDateTime savedAt;
};

// A session directory whose owning process is gone.
struct OrphanSession {
    QString dir;
    QList<RecoverableDocument> documents;
    bool preserve = false;
};

// Owns the locked per-instance recovery directory and discovers both the
// current store and legacy checkpoint-only sessions.
class RecoveryStore {
public:
    RecoveryStore();
    ~RecoveryStore();

    static QString rootPath(); // legacy scanner root
    static QString versionedRootPath();

    // Create and lock this instance's session directory. False when the state
    // directory is not writable; the store then stays closed.
    bool open();
    bool isOpen() const { return lock_ != nullptr; }
    QString sessionDir() const { return dir_; }

    // Clean shutdown: delete the session directory and release the lock.
    void close();

    static QList<OrphanSession> orphans(const QString& exceptDir);
    static std::unique_ptr<QLockFile> lockOrphan(const QString& dir);
    static OrphanSession readLocked(const QString& dir);
    static void discard(const QString& dir);
    static void discardLocked(const QString& dir);
    static bool retire(const RecoverableDocument& doc);
    static bool quarantine(const QString& sessionDir, const RecoverableDocument& doc);
    static QString quarantineRoot();

private:
    QString dir_;
    std::unique_ptr<QLockFile> lock_;
};

// The tab name of a recovered document: CS6 appends "-Recovered" to the name,
// before the extension.
QString recoveredDocumentName(const QString& name);

} // namespace pictura
