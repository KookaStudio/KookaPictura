// Crash recovery session lifecycle and startup recovery UI.

#include "import_save_coordinator.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/paint_bridge.h"

#include "document_registry.h"
#include "preferences.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QLocale>
#include <QtCore/QLockFile>
#include <QtCore/QTimer>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {

void ImportSaveCoordinator::enableRecovery()
{
    if (recovery_) {
        return;
    }
    recovery_ = std::make_unique<RecoveryStore>();
    if (!recovery_->open()) {
        recovery_.reset();
        if (autoSaveRecovery_ && hooks_.showStatus) {
            hooks_.showStatus(tr("Could not open the crash recovery store."), 8000);
        }
        return;
    }
    if (!autosaveTimer_) {
        autosaveTimer_ = new QTimer(this);
        connect(autosaveTimer_, &QTimer::timeout, this, &ImportSaveCoordinator::autosaveRecovery);
        recoveryStatusTimer_ = new QTimer(this);
        connect(recoveryStatusTimer_, &QTimer::timeout, this, &ImportSaveCoordinator::pollRecoveryStatus);
    }
    setAutoSaveRecovery(autoSaveRecovery_, autoSaveMinutes_);
}

void ImportSaveCoordinator::setAutoSaveRecovery(bool on, int minutes)
{
    autoSaveRecovery_ = on;
    autoSaveMinutes_ = kAutoSaveMinuteChoices.contains(minutes) ? minutes : 10;
    if (!recovery_) {
        return;
    }
    if (on) {
        autosaveTimer_->start(autoSaveMinutes_ * 60 * 1000);
        recoveryStatusTimer_->start(250);
        autosaveRecovery();
        return;
    }
    autosaveTimer_->stop();
    recoveryStatusTimer_->stop();
    for (const DocEntry* entry : documents_.docs()) {
        if (entry->view && !recovery_discard(*entry->view, true) && hooks_.showStatus) {
            hooks_.showStatus(tr("Could not remove crash recovery data."), 8000);
        }
    }
    recoveryWarned_.clear();
}

void ImportSaveCoordinator::trackRecovery(PictureView* view)
{
    if (!recoveryIds_.contains(view)) {
        recoveryIds_.insert(view, ++recoveryCounter_);
    }
    attachRecovery(view);
}

void ImportSaveCoordinator::attachRecovery(PictureView* view)
{
    if (!view || !recovery_ || !autoSaveRecovery_) {
        return;
    }
    const int index = indexOfView(view);
    const DocEntry* entry = documents_.docAt(index);
    if (!entry || entry->loading) {
        return;
    }
    if (!recoveryIds_.contains(view)) {
        recoveryIds_.insert(view, ++recoveryCounter_);
    }
    recovery_track(*view, recovery_->sessionDir(), static_cast<quint32>(recoveryIds_.value(view)),
                   documents_.documentName(index), entry->path, static_cast<quint32>(autoSaveMinutes_));
}

void ImportSaveCoordinator::pollRecoveryStatus()
{
    for (const DocEntry* entry : documents_.docs()) {
        PictureView* view = entry->view;
        if (!view) {
            continue;
        }
        if (recovery_status(*view) == 0) {
            recoveryWarned_.remove(view);
        } else if (!recoveryWarned_.contains(view)) {
            recoveryWarned_.insert(view);
            if (hooks_.showStatus) {
                hooks_.showStatus(tr("Crash recovery protection is behind. Save your work."), 8000);
            }
        }
    }
}

void ImportSaveCoordinator::autosaveRecovery()
{
    if (!recovery_ || !autoSaveRecovery_) {
        return;
    }
    for (const DocEntry* entry : documents_.docs()) {
        if (entry->view && !entry->loading) {
            attachRecovery(entry->view);
            recovery_checkpoint(*entry->view);
        }
    }
}

void ImportSaveCoordinator::dropRecoverySnapshot(PictureView* view)
{
    if (view && !recovery_discard(*view, false) && hooks_.showStatus) {
        hooks_.showStatus(tr("Could not remove superseded crash recovery data."), 8000);
    }
}

void ImportSaveCoordinator::forgetRecovery(PictureView* view)
{
    if (view && !recovery_discard(*view, true) && hooks_.showStatus) {
        hooks_.showStatus(tr("Could not remove crash recovery data."), 8000);
    }
    recoveryIds_.remove(view);
    recoveryWarned_.remove(view);
}

void ImportSaveCoordinator::shutdownRecovery()
{
    if (!recovery_) {
        return;
    }
    autosaveTimer_->stop();
    recoveryStatusTimer_->stop();
    for (const DocEntry* entry : documents_.docs()) {
        if (entry->view) {
            recovery_discard(*entry->view, true);
        }
    }
    recoveryWarned_.clear();
    recovery_->close();
    recovery_.reset();
}

QString ImportSaveCoordinator::recoverySessionDirForTest() const
{
    return recovery_ ? recovery_->sessionDir() : QString();
}

int ImportSaveCoordinator::recoverOrphans()
{
    const QString own = recovery_ ? recovery_->sessionDir() : QString();
    int recovered = 0;
    QStringList failures;
    const QList<OrphanSession> orphans = RecoveryStore::orphans(own);
    for (const OrphanSession& orphan : orphans) {
        auto lock = RecoveryStore::lockOrphan(orphan.dir);
        if (!lock) {
            continue;
        }
        const OrphanSession session = RecoveryStore::readLocked(orphan.dir);
        bool retain = session.preserve;
        for (const RecoverableDocument& doc : session.documents) {
            auto* view = new PictureView(dialogParent_);
            if (hooks_.applyColorSettings) {
                hooks_.applyColorSettings(view);
            }
            const int outcome = doc.versioned ? recovery_open_chain(*view, doc.manifest)
                                              : (recovery_open(*view, doc.snapshot) ? 1 : 0);
            if (outcome == 0 || outcome == 3) {
                delete view;
                if (outcome == 3) {
                    retain = true;
                    failures.append(tr("%1: its recovery data could not be read. It is kept at %2.")
                                        .arg(doc.name, QDir::toNativeSeparators(doc.manifest)));
                    continue;
                }
                if (open_refused_for_memory()) {
                    retain = true;
                    failures.append(tr("%1: not enough memory to open it. Its snapshot is kept "
                                       "at %2 and will be offered again at the next launch.")
                                        .arg(doc.name, QDir::toNativeSeparators(doc.snapshot)));
                    continue;
                }
                const QString quarantined = QDir(RecoveryStore::quarantineRoot())
                    .filePath(QFileInfo(session.dir).fileName());
                if (RecoveryStore::quarantine(session.dir, doc)) {
                    failures.append(tr("%1: the snapshot could not be read. It was moved to %2.")
                                        .arg(doc.name, QDir::toNativeSeparators(quarantined)));
                } else {
                    retain = true;
                    failures.append(tr("%1: the snapshot could not be read. It is kept at %2.")
                                        .arg(doc.name, QDir::toNativeSeparators(doc.snapshot)));
                }
                continue;
            }
            if (outcome == 2) {
                failures.append(tr("%1: part of the journal could not be read. "
                                   "Recovered up to the last valid edit.").arg(doc.name));
            }
            const int index = documents_.addDocument(view, QString(), true);
            documents_.docs()[index]->displayName = recoveredDocumentName(doc.name);
            documents_.updateTabTitle(index);
            if (hooks_.updateWindowTitle) {
                hooks_.updateWindowTitle();
            }
            ++recovered;
            if (recovery_ && autoSaveRecovery_) {
                attachRecovery(view);
                recovery_checkpoint(*view);
                if (recovery_wait(*view) && RecoveryStore::retire(doc)) {
                    continue;
                }
                failures.append(tr("%1: the recovered copy could not be protected. "
                                   "Its original recovery data was kept.").arg(doc.name));
            }
            retain = true;
        }
        if (!retain) {
            RecoveryStore::discardLocked(session.dir);
        }
    }
    if (!failures.isEmpty()) {
        // Window-modal without a nested event loop, so it neither blocks the
        // caller nor outlives the window.
        auto* box = new QMessageBox(QMessageBox::Warning, tr("Recover Documents"),
                                    tr("Some recovery data needs attention."),
                                    QMessageBox::Ok, dialogParent_);
        box->setObjectName(QStringLiteral("recoveryFailures"));
        box->setAttribute(Qt::WA_DeleteOnClose);
        box->setInformativeText(failures.join(QLatin1Char('\n')));
        box->open();
    }
    return recovered;
}

void ImportSaveCoordinator::offerRecovery()
{
    const QString own = recovery_ ? recovery_->sessionDir() : QString();
    const QList<OrphanSession> orphans = RecoveryStore::orphans(own);
    QStringList rows;
    for (const OrphanSession& session : orphans) {
        if (session.documents.isEmpty()) {
            if (!session.preserve) {
                RecoveryStore::discard(session.dir);
            }
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
                    QMessageBox::NoButton, dialogParent_);
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
