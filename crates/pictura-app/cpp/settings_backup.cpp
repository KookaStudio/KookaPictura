#include "settings_backup.h"

#include "dialogs.h"
#include "session.h"
#include "workspace_store.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QFileInfoList>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonParseError>
#include <QtCore/QRect>
#include <QtCore/QSaveFile>
#include <QtCore/QSet>
#include <QtCore/QStandardPaths>
#include <QtCore/QStringList>
#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QWidget>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

// A backup is mostly settings plus small preset libraries. Anything larger is
// corrupt or hostile, so it is not read into memory.
constexpr qint64 kMaxBackupBytes = 256 * 1024 * 1024;
constexpr qint64 kMaxStoreBytes = 16 * 1024 * 1024;
constexpr qint64 kMaxPresetBytes = 256 * 1024 * 1024;
constexpr int kMaxPresetFiles = 10000;
constexpr int kMaxBackupsKept = 10;

QString stateDirectory()
{
    return QFileInfo(sessionFilePath()).absolutePath();
}

QString xdgDataHome()
{
    const QString base = qEnvironmentVariable("XDG_DATA_HOME");
    return base.isEmpty() ? QDir::homePath() + QStringLiteral("/.local/share") : base;
}

QString adjustmentPresetsDirectory()
{
    return xdgDataHome() + QStringLiteral("/kooka-pictura/Presets/Adjustments");
}

QString actionsPresetsDirectory()
{
    QString base = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    if (base.isEmpty()) {
        base = stateDirectory();
    }
    return base + QStringLiteral("/Presets/Actions");
}

QString pdfPresetsDirectory()
{
    return stateDirectory() + QStringLiteral("/PDF Presets");
}

QString actionsPalettePath()
{
    return stateDirectory() + QStringLiteral("/Actions Palette.psp");
}

QString message(const char* text)
{
    return QCoreApplication::translate("PicturaMainWindow", text);
}

void setError(QString* error, const QString& text)
{
    if (error != nullptr) {
        *error = text;
    }
}

bool readFileBytes(const QString& path, QByteArray* out, qint64 maxBytes)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return false;
    }
    *out = file.read(maxBytes + 1);
    return out->size() <= maxBytes;
}

QJsonObject readJsonObject(const QString& path, qint64 maxBytes)
{
    QByteArray bytes;
    if (!readFileBytes(path, &bytes, maxBytes)) {
        return {};
    }
    QJsonParseError error{};
    const QJsonDocument doc = QJsonDocument::fromJson(bytes, &error);
    if (error.error != QJsonParseError::NoError || !doc.isObject()) {
        return {};
    }
    return doc.object();
}

bool writeJsonObject(const QString& path, const QJsonObject& object, bool pretty,
                     QString* error)
{
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        setError(error, message("The settings folder could not be created."));
        return false;
    }
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        setError(error, message("The settings file could not be opened for writing."));
        return false;
    }
    const QByteArray bytes =
        QJsonDocument(object).toJson(pretty ? QJsonDocument::Indented : QJsonDocument::Compact);
    if (file.write(bytes) != bytes.size()) {
        file.cancelWriting();
        setError(error, message("The settings file could not be written."));
        return false;
    }
    if (!file.commit()) {
        setError(error, message("The settings file could not be saved."));
        return false;
    }
    return true;
}

bool writeFileBytes(const QString& path, const QByteArray& bytes, QString* error)
{
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        setError(error, message("The settings folder could not be created."));
        return false;
    }
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size()) {
        file.cancelWriting();
        setError(error, message("A preset file could not be written."));
        return false;
    }
    if (!file.commit()) {
        setError(error, message("A preset file could not be saved."));
        return false;
    }
    return true;
}

QJsonObject encodeEntry(const QString& name, const QByteArray& bytes)
{
    QJsonObject entry;
    entry.insert(QStringLiteral("file"), name);
    const QString text = QString::fromUtf8(bytes);
    if (!bytes.contains('\0') && text.toUtf8() == bytes) {
        entry.insert(QStringLiteral("text"), text);
    } else {
        entry.insert(QStringLiteral("base64"), QString::fromLatin1(bytes.toBase64()));
    }
    return entry;
}

QJsonArray collectFileEntries(const QString& directory, const QString& suffix)
{
    QJsonArray entries;
    const QDir dir(directory);
    if (!dir.exists()) {
        return entries;
    }
    const QFileInfoList files = dir.entryInfoList({suffix}, QDir::Files, QDir::Name);
    for (const QFileInfo& info : files) {
        QByteArray bytes;
        if (!readFileBytes(info.absoluteFilePath(), &bytes, kMaxPresetBytes)) {
            continue;
        }
        entries.append(encodeEntry(info.fileName(), bytes));
    }
    return entries;
}

QJsonObject collectWorkspaces()
{
    QJsonObject out;
    const QString directory = workspaceStoreDir();
    out.insert(QStringLiteral("index"),
               readJsonObject(directory + QStringLiteral("/index.json"), kMaxStoreBytes));

    QJsonObject files;
    const QDir dir(directory);
    if (dir.exists()) {
        const QFileInfoList entries =
            dir.entryInfoList({QStringLiteral("*.json")}, QDir::Files, QDir::Name);
        for (const QFileInfo& info : entries) {
            if (info.fileName() == QStringLiteral("index.json")) {
                continue;
            }
            const QJsonObject object = readJsonObject(info.absoluteFilePath(), kMaxStoreBytes);
            if (!object.isEmpty()) {
                files.insert(info.fileName(), object);
            }
        }
    }
    out.insert(QStringLiteral("files"), files);
    return out;
}

QJsonObject collectPresets()
{
    QJsonObject presets;
    presets.insert(QStringLiteral("adjustments"),
                   collectFileEntries(adjustmentPresetsDirectory(),
                                      QStringLiteral("*.presets")));
    presets.insert(QStringLiteral("actions"),
                   collectFileEntries(actionsPresetsDirectory(), QStringLiteral("*.atn")));
    presets.insert(QStringLiteral("pdf"),
                   collectFileEntries(pdfPresetsDirectory(), QStringLiteral("*.joboptions")));

    QJsonObject palette;
    QByteArray bytes;
    if (readFileBytes(actionsPalettePath(), &bytes, kMaxPresetBytes)) {
        palette = encodeEntry(QStringLiteral("Actions Palette.psp"), bytes);
    }
    presets.insert(QStringLiteral("actionsPalette"), palette);
    return presets;
}

bool isSafeFileName(const QString& name, const QString& suffix)
{
    if (name.isEmpty() || name == QStringLiteral(".") || name == QStringLiteral("..")) {
        return false;
    }
    if (name.contains(QLatin1Char('/')) || name.contains(QLatin1Char('\\'))
        || name.contains(QLatin1Char(':'))) {
        return false;
    }
    if (QFileInfo(name).fileName() != name) {
        return false;
    }
    return suffix.isEmpty() || name.endsWith(suffix, Qt::CaseInsensitive);
}

bool validPayload(const QJsonObject& entry, QString* error)
{
    const QJsonValue text = entry.value(QStringLiteral("text"));
    if (text.isString()) {
        if (text.toString().toUtf8().size() > kMaxPresetBytes) {
            setError(error, message("A preset file is too large."));
            return false;
        }
        return true;
    }
    const QJsonValue base64 = entry.value(QStringLiteral("base64"));
    if (!base64.isString()) {
        setError(error, message("A preset entry has no text or base64 payload."));
        return false;
    }
    const QByteArray input = base64.toString().toLatin1();
    const QByteArray decoded = QByteArray::fromBase64(input);
    if (decoded.size() > kMaxPresetBytes || decoded.toBase64() != input) {
        setError(error, message("A preset entry has an invalid base64 payload."));
        return false;
    }
    return true;
}

bool decodePayload(const QJsonObject& entry, QByteArray* out, QString* error)
{
    const QJsonValue text = entry.value(QStringLiteral("text"));
    if (text.isString()) {
        *out = text.toString().toUtf8();
        return true;
    }
    const QJsonValue base64 = entry.value(QStringLiteral("base64"));
    if (!base64.isString()) {
        setError(error, message("A preset entry has no text or base64 payload."));
        return false;
    }
    *out = QByteArray::fromBase64(base64.toString().toLatin1());
    return true;
}

bool geometryIsOnScreen(const QByteArray& blob)
{
    if (blob.isEmpty()) {
        return false;
    }
    QWidget probe;
    if (!probe.restoreGeometry(blob)) {
        return false;
    }
    const QRect rect = probe.geometry();
    if (!rect.isValid()) {
        return false;
    }
    for (const QScreen* screen : QGuiApplication::screens()) {
        if (screen->availableGeometry().intersects(rect)) {
            return true;
        }
    }
    return false;
}

bool sanitizeSession(QJsonObject* session, QString* error)
{
    session->remove(QStringLiteral("writerToken"));
    session->remove(QStringLiteral("writtenAt"));

    if (session->contains(QStringLiteral("recent"))) {
        if (!session->value(QStringLiteral("recent")).isArray()) {
            setError(error, message("The session recent-file list is not an array."));
            return false;
        }
        QJsonArray recent;
        for (const QJsonValue& value : session->value(QStringLiteral("recent")).toArray()) {
            if (!value.isString()) {
                continue;
            }
            const QString path = value.toString();
            if (QFileInfo(path).isAbsolute() && QFileInfo::exists(path)) {
                recent.append(path);
            }
        }
        session->insert(QStringLiteral("recent"), recent);
    }

    if (session->contains(QStringLiteral("windowGeometry"))) {
        const QString encoded = session->value(QStringLiteral("windowGeometry")).toString();
        const QByteArray blob = QByteArray::fromBase64(encoded.toLatin1());
        if (!geometryIsOnScreen(blob)) {
            session->remove(QStringLiteral("windowGeometry"));
            session->remove(QStringLiteral("windowMaximized"));
        }
    }
    return true;
}

bool sanitizePresetCategory(QJsonObject* presets, const QString& key,
                            const QString& suffix, QString* error)
{
    if (!presets->contains(key)) {
        return true;
    }
    if (!presets->value(key).isArray()) {
        setError(error, message("A presets section is not an array."));
        return false;
    }
    const QJsonArray entries = presets->value(key).toArray();
    if (entries.size() > kMaxPresetFiles) {
        setError(error, message("The settings file contains too many presets."));
        return false;
    }
    QJsonArray out;
    for (const QJsonValue& value : entries) {
        if (!value.isObject()) {
            setError(error, message("A preset entry is not an object."));
            return false;
        }
        const QJsonObject entry = value.toObject();
        const QString name = entry.value(QStringLiteral("file")).toString();
        if (!isSafeFileName(name, suffix)) {
            setError(error, message("A preset file name is not safe: %1").arg(name));
            return false;
        }
        if (!validPayload(entry, error)) {
            return false;
        }
        out.append(entry);
    }
    presets->insert(key, out);
    return true;
}

bool sanitizeWorkspaces(QJsonObject* workspaces, QString* error)
{
    if (!workspaces->value(QStringLiteral("index")).isObject()) {
        setError(error, message("The workspace index is missing."));
        return false;
    }
    if (!workspaces->value(QStringLiteral("files")).isObject()) {
        setError(error, message("The workspace file list is missing."));
        return false;
    }
    const QJsonObject files = workspaces->value(QStringLiteral("files")).toObject();
    if (files.size() > kMaxPresetFiles) {
        setError(error, message("The settings file contains too many workspaces."));
        return false;
    }
    QJsonObject out;
    for (auto it = files.constBegin(); it != files.constEnd(); ++it) {
        if (it.key() == QStringLiteral("index.json")
            || !isSafeFileName(it.key(), QStringLiteral(".json"))) {
            setError(error, message("A workspace file name is not safe: %1").arg(it.key()));
            return false;
        }
        if (!it.value().isObject()) {
            setError(error, message("A workspace entry is not an object."));
            return false;
        }
        out.insert(it.key(), it.value());
    }
    workspaces->insert(QStringLiteral("files"), out);
    return true;
}

bool sanitizePresets(QJsonObject* presets, QString* error)
{
    if (!sanitizePresetCategory(presets, QStringLiteral("adjustments"),
                                QStringLiteral(".presets"), error)
        || !sanitizePresetCategory(presets, QStringLiteral("actions"),
                                   QStringLiteral(".atn"), error)
        || !sanitizePresetCategory(presets, QStringLiteral("pdf"),
                                   QStringLiteral(".joboptions"), error)) {
        return false;
    }
    if (presets->contains(QStringLiteral("actionsPalette"))) {
        if (!presets->value(QStringLiteral("actionsPalette")).isObject()) {
            setError(error, message("The Actions palette entry is not an object."));
            return false;
        }
        const QJsonObject entry = presets->value(QStringLiteral("actionsPalette")).toObject();
        if (!entry.isEmpty() && !validPayload(entry, error)) {
            return false;
        }
    }
    return true;
}

bool sanitizeBackup(const QJsonObject& source, QJsonObject* out, QString* error)
{
    *out = source;
    out->insert(QStringLiteral("format"), QString::fromLatin1(kSettingsBackupFormat));
    out->insert(QStringLiteral("formatVersion"), kSettingsBackupFormatVersion);

    if (out->contains(QStringLiteral("session"))) {
        if (!out->value(QStringLiteral("session")).isObject()) {
            setError(error, message("The session section is not an object."));
            return false;
        }
        QJsonObject session = out->value(QStringLiteral("session")).toObject();
        if (!sanitizeSession(&session, error)) {
            return false;
        }
        out->insert(QStringLiteral("session"), session);
    }

    if (out->contains(QStringLiteral("workspaces"))) {
        if (!out->value(QStringLiteral("workspaces")).isObject()) {
            setError(error, message("The workspaces section is not an object."));
            return false;
        }
        QJsonObject workspaces = out->value(QStringLiteral("workspaces")).toObject();
        if (!sanitizeWorkspaces(&workspaces, error)) {
            return false;
        }
        out->insert(QStringLiteral("workspaces"), workspaces);
    }

    if (out->contains(QStringLiteral("presets"))) {
        if (!out->value(QStringLiteral("presets")).isObject()) {
            setError(error, message("The presets section is not an object."));
            return false;
        }
        QJsonObject presets = out->value(QStringLiteral("presets")).toObject();
        if (!sanitizePresets(&presets, error)) {
            return false;
        }
        out->insert(QStringLiteral("presets"), presets);
    }
    return true;
}

bool applyWorkspaces(const QJsonObject& workspaces, QString* error)
{
    const QString directory = workspaceStoreDir();
    if (!QDir().mkpath(directory)) {
        setError(error, message("The workspace folder could not be created."));
        return false;
    }
    const QJsonObject files = workspaces.value(QStringLiteral("files")).toObject();
    for (auto it = files.constBegin(); it != files.constEnd(); ++it) {
        if (!writeJsonObject(directory + QLatin1Char('/') + it.key(), it.value().toObject(),
                             false, error)) {
            return false;
        }
    }
    const QJsonObject index = workspaces.value(QStringLiteral("index")).toObject();
    if (!writeJsonObject(directory + QStringLiteral("/index.json"), index, false, error)) {
        return false;
    }

    QSet<QString> keep = {QStringLiteral("index.json")};
    for (auto it = files.constBegin(); it != files.constEnd(); ++it) {
        keep.insert(it.key());
    }
    const QFileInfoList entries =
        QDir(directory).entryInfoList({QStringLiteral("*.json")}, QDir::Files);
    for (const QFileInfo& info : entries) {
        if (!keep.contains(info.fileName())) {
            QFile::remove(info.absoluteFilePath());
        }
    }
    return true;
}

bool replacePresetCategory(const QJsonObject& presets, const QString& key,
                           const QString& directory, const QString& suffix, QString* error)
{
    if (!presets.contains(key)) {
        return true;
    }
    if (!QDir().mkpath(directory)) {
        setError(error, message("A preset folder could not be created."));
        return false;
    }
    const QFileInfoList existing =
        QDir(directory).entryInfoList({suffix}, QDir::Files);
    for (const QFileInfo& info : existing) {
        QFile::remove(info.absoluteFilePath());
    }
    for (const QJsonValue& value : presets.value(key).toArray()) {
        const QJsonObject entry = value.toObject();
        QByteArray bytes;
        if (!decodePayload(entry, &bytes, error)) {
            return false;
        }
        const QString name = entry.value(QStringLiteral("file")).toString();
        if (!writeFileBytes(directory + QLatin1Char('/') + name, bytes, error)) {
            return false;
        }
    }
    return true;
}

bool applyPresets(const QJsonObject& presets, QString* error)
{
    if (!replacePresetCategory(presets, QStringLiteral("adjustments"),
                               adjustmentPresetsDirectory(), QStringLiteral("*.presets"), error)
        || !replacePresetCategory(presets, QStringLiteral("actions"),
                                  actionsPresetsDirectory(), QStringLiteral("*.atn"), error)
        || !replacePresetCategory(presets, QStringLiteral("pdf"), pdfPresetsDirectory(),
                                  QStringLiteral("*.joboptions"), error)) {
        return false;
    }
    if (presets.contains(QStringLiteral("actionsPalette"))) {
        const QJsonObject entry = presets.value(QStringLiteral("actionsPalette")).toObject();
        if (entry.isEmpty()) {
            QFile::remove(actionsPalettePath());
            return true;
        }
        QByteArray bytes;
        if (!decodePayload(entry, &bytes, error)) {
            return false;
        }
        return writeFileBytes(actionsPalettePath(), bytes, error);
    }
    return true;
}

bool applyBackup(const QJsonObject& backup, QString* error)
{
    if (backup.contains(QStringLiteral("session"))) {
        QJsonObject session = backup.value(QStringLiteral("session")).toObject();
        session.remove(QStringLiteral("writerToken"));
        session.remove(QStringLiteral("writtenAt"));
        if (!writeJsonObject(sessionFilePath(), session, false, error)) {
            return false;
        }
    }
    if (backup.contains(QStringLiteral("workspaces"))
        && !applyWorkspaces(backup.value(QStringLiteral("workspaces")).toObject(), error)) {
        return false;
    }
    if (backup.contains(QStringLiteral("presets"))
        && !applyPresets(backup.value(QStringLiteral("presets")).toObject(), error)) {
        return false;
    }
    return true;
}

} // namespace

QString pendingSettingsImportPath()
{
    return stateDirectory() + QStringLiteral("/pending-settings.json");
}

QString settingsBackupDirectory()
{
    return stateDirectory() + QStringLiteral("/settings-backups");
}

bool hasPendingSettingsImport()
{
    return QFileInfo::exists(pendingSettingsImportPath());
}

QJsonObject collectSettingsBackup()
{
    QJsonObject backup;
    backup.insert(QStringLiteral("format"), QString::fromLatin1(kSettingsBackupFormat));
    backup.insert(QStringLiteral("formatVersion"), kSettingsBackupFormatVersion);
    backup.insert(QStringLiteral("app"), QStringLiteral("Kooka Pictura"));
#ifdef PICTURA_VERSION
    backup.insert(QStringLiteral("appVersion"), QString::fromLatin1(PICTURA_VERSION));
#else
    backup.insert(QStringLiteral("appVersion"), QStringLiteral("unknown"));
#endif
    backup.insert(QStringLiteral("exportedAt"),
                  QDateTime::currentDateTimeUtc().toString(Qt::ISODate));
    backup.insert(QStringLiteral("session"), readJsonObject(sessionFilePath(), kMaxStoreBytes));
    backup.insert(QStringLiteral("workspaces"), collectWorkspaces());
    backup.insert(QStringLiteral("presets"), collectPresets());
    return backup;
}

bool writeSettingsBackup(const QString& path, const QJsonObject& backup, QString* error)
{
    return writeJsonObject(path, backup, true, error);
}

bool readSettingsBackup(const QString& path, QJsonObject* backup, QString* error)
{
    if (backup == nullptr) {
        setError(error, message("No output object was supplied."));
        return false;
    }
    *backup = {};
    QByteArray bytes;
    if (!readFileBytes(path, &bytes, kMaxBackupBytes)) {
        setError(error, message("The settings file could not be read."));
        return false;
    }
    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(bytes, &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        setError(error, message("The settings file is not valid JSON."));
        return false;
    }
    const QJsonObject object = doc.object();
    if (object.value(QStringLiteral("format")).toString()
        != QString::fromLatin1(kSettingsBackupFormat)) {
        setError(error, message("The file is not a Kooka Pictura settings backup."));
        return false;
    }
    const QJsonValue versionValue = object.value(QStringLiteral("formatVersion"));
    const double version = versionValue.toDouble(0.0);
    if (!versionValue.isDouble() || version < 1.0 || version != std::floor(version)) {
        setError(error, message("The settings file has no valid format version."));
        return false;
    }
    if (version > kSettingsBackupFormatVersion) {
        setError(error,
                 message("The settings file was written by a newer version of "
                         "Kooka Pictura."));
        return false;
    }
    *backup = object;
    return true;
}

bool validateSettingsImport(const QString& path, QJsonObject* sanitized, QString* error)
{
    if (sanitized == nullptr) {
        setError(error, message("No output object was supplied."));
        return false;
    }
    *sanitized = {};
    QJsonObject backup;
    if (!readSettingsBackup(path, &backup, error)) {
        return false;
    }
    return sanitizeBackup(backup, sanitized, error);
}

bool writePendingSettingsImport(const QJsonObject& sanitized, QString* error)
{
    return writeJsonObject(pendingSettingsImportPath(), sanitized, true, error);
}

bool stageSettingsImport(const QString& path, QString* error)
{
    QJsonObject sanitized;
    if (!validateSettingsImport(path, &sanitized, error)) {
        return false;
    }
    return writePendingSettingsImport(sanitized, error);
}

bool applyPendingSettingsImport(QString* error)
{
    const QString pending = pendingSettingsImportPath();
    if (!QFileInfo::exists(pending)) {
        return true;
    }
    QJsonObject backup;
    if (!readSettingsBackup(pending, &backup, error)) {
        const QString quarantine =
            pending + QStringLiteral(".corrupt-")
            + QString::number(QDateTime::currentSecsSinceEpoch());
        QFile::rename(pending, quarantine);
        return false;
    }
    QJsonObject sanitized;
    if (!sanitizeBackup(backup, &sanitized, error)) {
        const QString quarantine =
            pending + QStringLiteral(".corrupt-")
            + QString::number(QDateTime::currentSecsSinceEpoch());
        QFile::rename(pending, quarantine);
        return false;
    }
    if (!applyBackup(sanitized, error)) {
        return false;
    }
    if (!QFile::remove(pending)) {
        setError(error, message("The pending settings import could not be removed."));
        return false;
    }
    return true;
}

namespace {

bool gSettingsRestartPromptInteractive = true;
int gSettingsRestartPromptCount = 0;

} // namespace

void setSettingsRestartPromptInteractive(bool interactive)
{
    gSettingsRestartPromptInteractive = interactive;
}

int settingsRestartPromptCountForTest()
{
    return gSettingsRestartPromptCount;
}

bool exportSettingsFile(const QString& path, QString* error)
{
    return writeSettingsBackup(path, collectSettingsBackup(), error);
}

bool importSettingsFile(const QString& path, QWidget* parent, QString* error)
{
    QJsonObject sanitized;
    if (!validateSettingsImport(path, &sanitized, error)) {
        return false;
    }
    if (backupCurrentSettings(error).isEmpty()) {
        return false;
    }
    if (!writePendingSettingsImport(sanitized, error)) {
        return false;
    }
    ++gSettingsRestartPromptCount;
    if (!gSettingsRestartPromptInteractive) {
        return true;
    }
    QMessageBox box(parent);
    box.setWindowTitle(QCoreApplication::translate("PicturaMainWindow", "Import Settings"));
    box.setIcon(QMessageBox::Information);
    box.setText(QCoreApplication::translate(
        "PicturaMainWindow", "Settings imported. Restart Kooka Pictura to apply them."));
    auto* quit = box.addButton(
        QCoreApplication::translate("PicturaMainWindow", "Quit Now"),
        QMessageBox::AcceptRole);
    box.addButton(QCoreApplication::translate("PicturaMainWindow", "Later"),
                  QMessageBox::RejectRole);
    runDialog(box, parent);
    if (box.clickedButton() == quit) {
        QCoreApplication::quit();
    }
    return true;
}

QString backupCurrentSettings(QString* error)
{
    const QString directory = settingsBackupDirectory();
    if (!QDir().mkpath(directory)) {
        setError(error, message("The settings backup folder could not be created."));
        return {};
    }
    const QString stamp = QDateTime::currentDateTime().toString(
        QStringLiteral("yyyyMMdd-HHmmss-zzz"));
    const QString path = directory + QStringLiteral("/pictura-") + stamp
        + QStringLiteral(".json");
    if (!writeSettingsBackup(path, collectSettingsBackup(), error)) {
        return {};
    }

    QFileInfoList backups =
        QDir(directory).entryInfoList({QStringLiteral("pictura-*.json")}, QDir::Files);
    std::sort(backups.begin(), backups.end(), [](const QFileInfo& a, const QFileInfo& b) {
        return a.lastModified() > b.lastModified();
    });
    for (int i = kMaxBackupsKept; i < backups.size(); ++i) {
        QFile::remove(backups.at(i).absoluteFilePath());
    }
    return path;
}

} // namespace pictura
