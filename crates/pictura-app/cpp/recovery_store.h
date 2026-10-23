#pragma once

#include <QtCore/QDateTime>
#include <QtCore/QList>
#include <QtCore/QString>

#include <memory>

class QLockFile;

namespace pictura {

// One document a crashed session left a snapshot for.
struct RecoverableDocument {
    QString snapshot;      // the PSD/PSB snapshot file
    QString name;          // the tab name when it was written
    QString originalPath;  // the file it was opened from or saved to; empty if untitled
    QDateTime savedAt;
};

// A session directory whose owning process is gone.
struct OrphanSession {
    QString dir;
    QList<RecoverableDocument> documents;
};

// The crash-recovery store (docs/11-cross-cutting/crash-recovery-and-autosave.md):
// one directory per running instance under
// $XDG_STATE_HOME/kooka-pictura/recovery/, held by a QLockFile so a later
// launch can tell a live instance from a crashed one. Each document writes
// `<id>.psd` (the snapshot) and `<id>.json` (its manifest). A clean exit
// removes the directory; one left behind with a dead owner is an orphan.
//
// ponytail: no journal and no crash handler. Work since the last snapshot is
// lost, bounded by the autosave interval.
class RecoveryStore {
public:
    RecoveryStore();
    ~RecoveryStore();

    static QString rootPath();

    // Create and lock this instance's session directory. False when the state
    // directory is not writable; the store then stays closed.
    bool open();
    bool isOpen() const { return lock_ != nullptr; }
    QString sessionDir() const { return dir_; }

    QString snapshotPath(int id) const;
    bool writeManifest(int id, const QString& name, const QString& originalPath);
    // Delete document `id`'s snapshot and manifest.
    void remove(int id);
    // Clean shutdown: delete the session directory and release the lock.
    void close();

    // Sessions under the root, other than `exceptDir`, whose owner is no longer
    // running; each lists the documents whose snapshot and manifest both exist.
    static QList<OrphanSession> orphans(const QString& exceptDir);
    static void discard(const QString& dir);
    // Keep a document that could not be recovered under
    // `<root>/quarantine/<session>/`, outside the orphan scan. Copies both
    // files first and removes the originals only after both copies succeed.
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
