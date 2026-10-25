// Crash recovery and autosave (docs/11-cross-cutting/crash-recovery-and-autosave.md).
// While enabled, a timer snapshots every document with unsaved changes into
// this instance's RecoveryStore session; a save or close drops the snapshot,
// and a clean exit deletes the session. A launch that finds a session whose
// owner died offers its documents back as unsaved "-Recovered" tabs.

#include "frame_includes.h"

#include "api/document_api.h"
#include "pictura_app/src/cxxqt_object/impl_core/recovery.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QLocale>
#include <QtWidgets/QMessageBox>

namespace pictura {

void PicturaMainWindow::enableRecovery()
{
    if (recovery_) {
        return;
    }
    recovery_ = std::make_unique<RecoveryStore>();
    if (!recovery_->open()) {
        recovery_.reset();
        return;
    }
    autosaveTimer_ = new QTimer(this);
    connect(autosaveTimer_, &QTimer::timeout, this, &PicturaMainWindow::autosaveRecovery);
    setAutoSaveRecovery(autoSaveRecovery_, autoSaveMinutes_);
}

void PicturaMainWindow::setAutoSaveRecovery(bool on, int minutes)
{
    autoSaveRecovery_ = on;
    autoSaveMinutes_ = kAutoSaveMinuteChoices.contains(minutes) ? minutes : 10;
    if (!autosaveTimer_) {
        return;
    }
    if (on) {
        autosaveTimer_->start(autoSaveMinutes_ * 60 * 1000);
        return;
    }
    autosaveTimer_->stop();
    for (const DocEntry* entry : docs_) {
        dropRecoverySnapshot(entry->view);
    }
}

void PicturaMainWindow::connectRecoveryPreferences(PreferencesDialog* dialog)
{
    dialog->setAutoSaveRecovery(autoSaveRecovery_, autoSaveMinutes_);
    connect(dialog, &PreferencesDialog::autoSaveRecoveryChanged, this,
            [this](bool on, int minutes) {
                setAutoSaveRecovery(on, minutes);
                saveSession();
            });
}

void PicturaMainWindow::trackRecovery(PictureView* view)
{
    recoveryIds_.insert(view, ++recoveryCounter_);
    recoveryStale_.insert(view);
    connect(view, &PictureView::changed, this, [this, view] { recoveryStale_.insert(view); });
}

void PicturaMainWindow::autosaveRecovery()
{
    if (!recovery_ || !autoSaveRecovery_) {
        return;
    }
    for (int i = 0; i < docs_.size(); ++i) {
        PictureView* view = docs_.at(i)->view;
        if (!document_api::isDirty(*view)) {
            dropRecoverySnapshot(view);
            continue;
        }
        // Mid-stroke the document is half painted; the next tick catches it.
        if (!recoveryStale_.contains(view) || view->is_painting()) {
            continue;
        }
        const int id = recoveryIds_.value(view);
        if (!recovery_->writeManifest(id, documentName(i), docs_.at(i)->path)) {
            continue;
        }
        if (recovery_write(*view, recovery_->snapshotPath(id))) {
            recoveryStale_.remove(view);
            recoverySnapshots_.insert(view);
        }
    }
}

void PicturaMainWindow::dropRecoverySnapshot(PictureView* view)
{
    if (!recoverySnapshots_.remove(view)) {
        return;
    }
    // The write may still be in flight; it must land before its file is deleted.
    recovery_wait(*view);
    if (recovery_) {
        recovery_->remove(recoveryIds_.value(view));
    }
}

void PicturaMainWindow::forgetRecovery(PictureView* view)
{
    dropRecoverySnapshot(view);
    recoveryIds_.remove(view);
    recoveryStale_.remove(view);
}

void PicturaMainWindow::shutdownRecovery()
{
    for (PictureView* view : std::as_const(recoverySnapshots_)) {
        recovery_wait(*view);
    }
    recoverySnapshots_.clear();
    if (recovery_) {
        recovery_->close();
        recovery_.reset();
    }
}

QString PicturaMainWindow::recoverySessionDirForTest() const
{
    return recovery_ ? recovery_->sessionDir() : QString();
}

int PicturaMainWindow::recoverOrphans()
{
    const QString own = recovery_ ? recovery_->sessionDir() : QString();
    int recovered = 0;
    QStringList failed;
    struct RecoveredDocument {
        QString session;
        QString snapshot;
        QString manifest;
        PictureView* view = nullptr;
    };
    QList<RecoveredDocument> orphanCopies;
    QSet<QString> keep;
    const QList<OrphanSession> orphans = RecoveryStore::orphans(own);
    for (const OrphanSession& session : orphans) {
        bool quarantineFailed = false;
        for (const RecoverableDocument& doc : session.documents) {
            auto* view = new PictureView(this);
            applyColorSettings(view);
            if (!recovery_open(*view, doc.snapshot)) {
                delete view;
                failed.append(doc.name);
                if (!RecoveryStore::quarantine(session.dir, doc)) {
                    quarantineFailed = true;
                }
                continue;
            }
            // Recovery reopens as tabs whatever Open Documents As Tabs says.
            const int index = addDocument(view, QString(), true);
            docs_[index]->displayName = recoveredDocumentName(doc.name);
            updateTabTitle(index);
            updateWindowTitle();
            ++recovered;
            orphanCopies.append(
                {session.dir, doc.snapshot,
                 session.dir + QLatin1Char('/')
                     + QFileInfo(doc.snapshot).completeBaseName() + QStringLiteral(".json"),
                 view});
        }
        if (quarantineFailed) {
            // The failed snapshot could not be moved out; keep the whole session
            // so neither it nor the recovered copies are lost.
            keep.insert(session.dir);
        }
    }
    // Snapshot the recovered documents into this session before the orphaned
    // copies go, so a second crash before the first tick still has them.
    autosaveRecovery();
    QSet<PictureView*> snapshotted;
    for (const DocEntry* entry : docs_) {
        if (recovery_wait(*entry->view)) {
            snapshotted.insert(entry->view);
        }
    }
    // A snapshot that was never started (autosave off) or did not land must not
    // cost the only copy: keep its session.
    for (const RecoveredDocument& doc : orphanCopies) {
        if (!snapshotted.contains(doc.view) || !recoverySnapshots_.contains(doc.view)) {
            keep.insert(doc.session);
        }
    }
    const auto kept = [&keep](const QString& dir) { return keep.contains(dir); };
    for (const RecoveredDocument& doc : orphanCopies) {
        if (kept(doc.session)) {
            continue;
        }
        QFile::remove(doc.snapshot);
        QFile::remove(doc.manifest);
    }
    for (const OrphanSession& session : orphans) {
        if (!kept(session.dir)) {
            RecoveryStore::discard(session.dir);
        }
    }
    if (!failed.isEmpty()) {
        statusBar()->showMessage(tr("Could not recover: %1").arg(failed.join(QStringLiteral(", "))));
    }
    return recovered;
}

void PicturaMainWindow::offerRecovery()
{
    const QString own = recovery_ ? recovery_->sessionDir() : QString();
    const QList<OrphanSession> orphans = RecoveryStore::orphans(own);
    QStringList rows;
    for (const OrphanSession& session : orphans) {
        if (session.documents.isEmpty()) {
            RecoveryStore::discard(session.dir);
            continue;
        }
        for (const RecoverableDocument& doc : session.documents) {
            rows.append(tr("%1 (saved %2)").arg(doc.name,
                                                QLocale().toString(doc.savedAt, QLocale::ShortFormat)));
        }
    }
    if (rows.isEmpty()) {
        return;
    }
    QMessageBox box(QMessageBox::Question, tr("Recover Documents"),
                    tr("Kooka Pictura did not shut down normally. Recover the unsaved "
                       "documents it was editing?"),
                    QMessageBox::NoButton, this);
    box.setInformativeText(rows.join(QLatin1Char('\n')));
    QPushButton* recover = box.addButton(tr("Recover"), QMessageBox::AcceptRole);
    QPushButton* discard = box.addButton(tr("Discard"), QMessageBox::DestructiveRole);
    box.addButton(tr("Later"), QMessageBox::RejectRole);
    box.setDefaultButton(recover);
    box.exec();
    if (box.clickedButton() == recover) {
        recoverOrphans();
    } else if (box.clickedButton() == discard) {
        for (const OrphanSession& session : orphans) {
            RecoveryStore::discard(session.dir);
        }
    }
}

} // namespace pictura
