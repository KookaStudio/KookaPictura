#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>

class QWidget;

namespace pictura {

// The `pictura.json` backup format this build writes.
inline constexpr int kSettingsBackupFormatVersion = 1;
inline constexpr char kSettingsBackupFormat[] = "pictura-settings";

// $XDG_STATE_HOME/kooka-pictura/pending-settings.json. Written by an import and
// applied before the next launch loads the session.
QString pendingSettingsImportPath();

// $XDG_STATE_HOME/kooka-pictura/settings-backups, the pre-import safety copies.
QString settingsBackupDirectory();

// Whether an import is waiting for the next launch.
bool hasPendingSettingsImport();

// Build the complete backup from the stores on disk. The frame saves the live
// session first, so the exported object reflects the current window.
QJsonObject collectSettingsBackup();

// Write `backup` atomically to `path`. The file is pretty-printed.
bool writeSettingsBackup(const QString& path, const QJsonObject& backup,
                         QString* error = nullptr);

// Parse and validate a backup file without touching any store.
bool readSettingsBackup(const QString& path, QJsonObject* backup, QString* error = nullptr);

// Validate and sanitize a backup without touching any store.
bool validateSettingsImport(const QString& path, QJsonObject* sanitized,
                            QString* error = nullptr);

// Write an already-validated backup as the pending import.
bool writePendingSettingsImport(const QJsonObject& sanitized, QString* error = nullptr);

// Validate, sanitize, and write the pending import. The live stores are left
// alone until the next launch.
bool stageSettingsImport(const QString& path, QString* error = nullptr);

// Apply the pending import before the session loads. A corrupt pending file is
// quarantined and the launch continues. True when there was nothing to apply.
bool applyPendingSettingsImport(QString* error = nullptr);

// Write a timestamped backup of the current settings and return its path, or an
// empty string on failure. Keeps the ten most recent backups.
QString backupCurrentSettings(QString* error = nullptr);

// The frame's export/import handlers: collect the current stores, write the
// file, and (for import) back up, stage, and offer the restart prompt.
bool exportSettingsFile(const QString& path, QString* error = nullptr);
bool importSettingsFile(const QString& path, QWidget* parent = nullptr,
                        QString* error = nullptr);

// Test hooks for the restart prompt. The prompt is non-interactive under Qt
// Test and still counts as shown.
void setSettingsRestartPromptInteractive(bool interactive);
int settingsRestartPromptCountForTest();

} // namespace pictura
