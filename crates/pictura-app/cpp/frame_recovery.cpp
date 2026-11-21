// PicturaMainWindow crash-recovery entry points: thin forwarders to the
// ImportSaveCoordinator. The store, timer, snapshots, and autosave settings
// live in import_save_recovery.cpp; the preferences wiring stays here.

#include "frame_includes.h"

#include "import_save_coordinator.h"
#include "preferences_dialog.h"

namespace pictura {

void PicturaMainWindow::enableRecovery()
{
    if (importSave_) {
        importSave_->enableRecovery();
    }
}

void PicturaMainWindow::setAutoSaveRecovery(bool on, int minutes)
{
    if (importSave_) {
        importSave_->setAutoSaveRecovery(on, minutes);
    }
}

bool PicturaMainWindow::autoSaveRecovery() const
{
    return importSave_ && importSave_->autoSaveRecovery();
}

int PicturaMainWindow::autoSaveMinutes() const
{
    return importSave_ ? importSave_->autoSaveMinutes() : 10;
}

void PicturaMainWindow::offerRecovery()
{
    if (importSave_) {
        importSave_->offerRecovery();
    }
}

int PicturaMainWindow::recoverOrphans()
{
    return importSave_ ? importSave_->recoverOrphans() : 0;
}

void PicturaMainWindow::autosaveRecovery()
{
    if (importSave_) {
        importSave_->autosaveRecovery();
    }
}

void PicturaMainWindow::shutdownRecovery()
{
    if (importSave_) {
        importSave_->shutdownRecovery();
    }
}

QString PicturaMainWindow::recoverySessionDirForTest() const
{
    return importSave_ ? importSave_->recoverySessionDirForTest() : QString();
}

void PicturaMainWindow::connectRecoveryPreferences(PreferencesDialog* dialog)
{
    if (!dialog || !importSave_) {
        return;
    }
    dialog->setSaveInBackground(importSave_->saveInBackground());
    connect(dialog, &PreferencesDialog::saveInBackgroundChanged, this, [this](bool on) {
        importSave_->setSaveInBackground(on);
        saveSession();
    });
    dialog->setAutoSaveRecovery(importSave_->autoSaveRecovery(),
                                importSave_->autoSaveMinutes());
    dialog->setNativeSaveCompatibility(importSave_->nativeSaveCompatibility());
    dialog->setRawSettingsStore(importSave_->rawSettingsStore());
    connect(dialog, &PreferencesDialog::rawSettingsStoreChanged, this, [this](int store) {
        importSave_->setRawSettingsStore(store);
        saveSession();
    });
    connect(dialog, &PreferencesDialog::nativeSaveCompatibilityChanged, this,
            [this](int choice) {
                importSave_->setNativeSaveCompatibility(choice);
                saveSession();
            });
    connect(dialog, &PreferencesDialog::autoSaveRecoveryChanged, this,
            [this](bool on, int minutes) {
                importSave_->setAutoSaveRecovery(on, minutes);
                saveSession();
            });
}

} // namespace pictura
