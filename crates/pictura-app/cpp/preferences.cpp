#include "preferences.h"
#include "profile_startup.h"
#include "session.h"
#include "settings_backup.h"

#include <utility>

#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QLockFile>
#include <QtCore/QSet>
#include <QtCore/QStandardPaths>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QMessageBox>

#include "api/preferences_bridge.h"

namespace pictura {
namespace {

rust::Str bytes(const QByteArray& utf8)
{
    return {utf8.constData(), static_cast<size_t>(utf8.size())};
}

QString fromRust(const rust::String& value)
{
    return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
}

QByteArray json(const QJsonObject& values)
{
    return QJsonDocument(values).toJson(QJsonDocument::Compact);
}

QString stateDirectory()
{
    return QFileInfo(sessionFilePath()).absolutePath();
}

QByteArray systemFiles()
{
#ifdef Q_OS_LINUX
    QStringList roots = qEnvironmentVariable("XDG_CONFIG_DIRS").split(QLatin1Char(':'), Qt::SkipEmptyParts);
    if (roots.isEmpty()) {
        roots << QStringLiteral("/etc/xdg");
    }
    QJsonArray paths;
    for (const auto& root : roots) {
        if (QDir::isAbsolutePath(root)) {
            paths.append(QDir(root).filePath(QStringLiteral("kooka-pictura/prefs.toml")));
        }
    }
    return QJsonDocument(paths).toJson(QJsonDocument::Compact);
#else
    return QByteArrayLiteral("[]");
#endif
}

QStringList decodeList(const QJsonValue& value)
{
    QStringList result;
    for (const auto& item : value.toArray()) {
        result.append(item.toString());
    }
    return result;
}

void decode(bool& result, const QJsonValue& value) { result = value.toBool(); }
void decode(int& result, const QJsonValue& value) { result = value.toInt(); }
void decode(double& result, const QJsonValue& value) { result = value.toDouble(); }
void decode(QString& result, const QJsonValue& value) { result = value.toString(); }
void decode(QStringList& result, const QJsonValue& value) { result = decodeList(value); }

void decode(CommandShortcutOverrides& result, const QJsonValue& value)
{
    const auto entries = value.toObject();
    for (auto it = entries.constBegin(); it != entries.constEnd(); ++it) {
        result.insert(it.key(), decodeList(it.value()));
    }
}

void decode(ToolShortcutOverrides& result, const QJsonValue& value)
{
    const auto entries = value.toObject();
    for (auto it = entries.constBegin(); it != entries.constEnd(); ++it) {
        result.insert(it.key(), it.value().toString());
    }
}

QJsonValue encode(const QStringList& values) { return QJsonArray::fromStringList(values); }
QJsonValue encode(bool value) { return value; }
QJsonValue encode(int value) { return value; }
QJsonValue encode(double value) { return value; }
QJsonValue encode(const QString& value) { return value; }

template<typename Value>
QJsonObject shortcutChanges(const QMap<QString, Value>& before,
                            const QMap<QString, Value>& after)
{
    QJsonObject changes;
    for (auto it = before.cbegin(); it != before.cend(); ++it) {
        if (!after.contains(it.key())) {
            changes.insert(it.key(), QJsonValue::Null);
        }
    }
    for (auto it = after.cbegin(); it != after.cend(); ++it) {
        const auto previous = before.constFind(it.key());
        if (previous == before.cend() || previous.value() != it.value()) {
            changes.insert(it.key(), encode(it.value()));
        }
    }
    return changes;
}

void mergeFields(QJsonObject& values, const QJsonObject& patch, bool removeNulls)
{
    for (auto it = patch.constBegin(); it != patch.constEnd(); ++it) {
        if (it.key() == QStringLiteral("shortcuts") || it.key() == QStringLiteral("toolShortcuts")) {
            auto map = values.value(it.key()).toObject();
            const auto updates = it->toObject();
            for (auto binding = updates.constBegin(); binding != updates.constEnd(); ++binding) {
                if (removeNulls && binding->isNull()) {
                    map.remove(binding.key());
                } else {
                    map.insert(binding.key(), binding.value());
                }
            }
            values.insert(it.key(), map);
        } else {
            values.insert(it.key(), it.value());
        }
    }
}

QHash<QString, QJsonObject>& unsaved()
{
    static QHash<QString, QJsonObject> edits;
    return edits;
}

void setError(QString* error, const rust::Error& failure)
{
    if (error) {
        *error = QString::fromUtf8(failure.what());
    }
}

const QStringList& preferenceKeys()
{
    static const QStringList keys = [] {
        QStringList result;
        for (const auto& key : prefs_legacy_keys()) {
            result.append(fromRust(key));
        }
        return result;
    }();
    return keys;
}

Preferences decodePreferences(const QJsonObject& values)
{
    Preferences result;
#define PICTURA_PREF(Type, name) decode(result.name, values.value(QStringLiteral(#name)));
#include "preference_fields.inc"
#undef PICTURA_PREF
    return result;
}

Preferences readPreferences(bool* clean, QString* notice)
{
    const auto root = configurationDirectory();
    QLockFile stateLock(sessionFilePath() + QStringLiteral(".lock"));
    const bool stateExists = QFileInfo::exists(sessionFilePath());
    const bool stateLocked = stateExists && stateLock.tryLock(1000);

    auto state = readSessionObject();
    const auto migrated = preferenceMigrationMarker();
    const auto legacy = state.value(migrated).toBool() ? QJsonObject{} : state;
    const auto reply = prefs_load(bytes(root.toUtf8()), bytes(systemFiles()), bytes(json(legacy)),
                                 stateExists && !stateLocked);

    bool ready = reply.can_clean_legacy;
    QString message = fromRust(reply.notice);
    if (ready && !state.isEmpty()) {
        auto cleaned = state;
        removeLegacyPreferenceKeys(cleaned);
        cleaned.insert(migrated, true);
        if (cleaned != state && (!stateLocked || !writeSessionObject(cleaned))) {
            ready = false;
            if (message.isEmpty()) {
                message = QCoreApplication::translate(
                    "PreferencesStore", "Preferences were saved, but legacy session cleanup must be retried.");
            }
        }
    }
    if (!message.isEmpty()) {
        static QSet<QString> warnings;
        const auto identity = root + QLatin1Char('\n') + message;
        if (!warnings.contains(identity)) {
            warnings.insert(identity);
            qWarning().noquote() << "pictura:" << message;
        }
    }

    if (clean) {
        *clean = ready;
    }
    if (notice) {
        *notice = message;
    }
    auto values = QJsonDocument::fromJson(QByteArray(reply.values.data(),
        static_cast<qsizetype>(reply.values.size()))).object();
    mergeFields(values, unsaved().value(root), true);
    auto result = decodePreferences(values);
    result.agentRecoveryGuard = reply.agent_guard;
    return result;
}

template<typename Value>
void diff(QJsonObject& patch, const char* key, const Value& before, const Value& after)
{
    if (before != after) {
        patch.insert(QString::fromLatin1(key), encode(after));
    }
}

template<typename Value>
void diff(QJsonObject& patch, const char* key, const QMap<QString, Value>& before,
          const QMap<QString, Value>& after)
{
    const auto changes = shortcutChanges(before, after);
    if (!changes.isEmpty()) {
        patch.insert(QString::fromLatin1(key), changes);
    }
}

bool wantsReset(const QStringList& arguments)
{
    constexpr auto chord = Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier;
    return arguments.contains(QStringLiteral("--reset-preferences"))
        || (!QGuiApplication::platformName().startsWith(QStringLiteral("wayland"))
            && (QGuiApplication::queryKeyboardModifiers() & chord) == chord);
}

} // namespace

QString configurationDirectory()
{
    QString base = qEnvironmentVariable("XDG_CONFIG_HOME");
    if (!QDir::isAbsolutePath(base)) {
#ifdef Q_OS_LINUX
        base = QDir::home().filePath(QStringLiteral(".config"));
#else
        base = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
#endif
    }
    return QDir(base).absoluteFilePath(QStringLiteral("kooka-pictura"));
}

QString preferencesFilePath()
{
    return QDir(configurationDirectory()).filePath(QStringLiteral("prefs.toml"));
}

QString preferenceMigrationMarker()
{
    static const auto marker = fromRust(prefs_migration_marker());
    return marker;
}

void removeLegacyPreferenceKeys(QJsonObject& state)
{
    for (const auto& key : preferenceKeys()) {
        state.remove(key);
    }
}

bool initializePreferences(QString* notice)
{
    bool ready = false;
    readPreferences(&ready, notice);
    return ready;
}

Preferences defaultPreferences()
{
    const auto defaults = prefs_defaults();
    const auto data = QByteArray(defaults.data(), static_cast<qsizetype>(defaults.size()));
    return decodePreferences(QJsonDocument::fromJson(data).object());
}

Preferences loadPreferences()
{
    return readPreferences(nullptr, nullptr);
}

bool savePreferences(const Preferences& before, const Preferences& after, bool explicitAgentGrant)
{
    QJsonObject changes;
#define PICTURA_PREF(Type, name) diff(changes, #name, before.name, after.name);
#include "preference_fields.inc"
#undef PICTURA_PREF

    const auto root = configurationDirectory();
    auto pending = unsaved().value(root);
    mergeFields(pending, changes, false);
    if (pending.isEmpty()) {
        return true;
    }
    initializePreferences();
    try {
        prefs_update(bytes(root.toUtf8()), bytes(systemFiles()), bytes(json(pending)),
                     explicitAgentGrant, true);
        unsaved().remove(root);
        return true;
    } catch (const rust::Error& error) {
        // These selectors restore the last saved choice on failure.
        pending.remove(QStringLiteral("uiLanguage"));
        pending.remove(QStringLiteral("imageInterpolation"));
        for (const auto& key : preferenceKeys()) {
            if (key.startsWith(QStringLiteral("agent"))) {
                pending.remove(key);
            }
        }
        if (!pending.isEmpty()) {
            unsaved().insert(root, pending);
        } else {
            unsaved().remove(root);
        }
        qWarning().noquote() << "pictura: preferences could not be saved:" << error.what();
        return false;
    }
}

bool validatePortablePreferences(const QString& toml, QString* error)
{
    try {
        prefs_validate_portable(bytes(toml.toUtf8()));
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

bool exportPortablePreferences(QString* toml, QString* error)
{
    try {
        *toml = fromRust(prefs_export(bytes(configurationDirectory().toUtf8())));
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

bool applyImportedPreferences(const QJsonObject& legacy, const QString* toml,
                              const QString& id, QString* error)
{
    const auto root = configurationDirectory();
    if (!toml && !legacy.isEmpty()) {
        initializePreferences();
    }
    try {
        prefs_apply_import(bytes(root.toUtf8()), bytes(json(legacy)),
                           bytes(toml ? toml->toUtf8() : QByteArray{}), toml != nullptr,
                           bytes(id.toUtf8()), toml == nullptr);
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

void retirePreferencesReceipt(const QString& id)
{
    try {
        prefs_retire_receipt(bytes(configurationDirectory().toUtf8()), bytes(id.toUtf8()));
    } catch (const rust::Error& error) {
        qWarning().noquote() << "pictura: preferences import receipt could not be removed:" << error.what();
    }
}

struct ProfilePresence::Private {
    rust::Box<ProfileLease> lease;
};

ProfilePresence::ProfilePresence(std::unique_ptr<Private> data) : data_(std::move(data)) {}
ProfilePresence::~ProfilePresence() = default;

std::unique_ptr<ProfilePresence> prepareProfile(const QStringList& arguments, bool interactive,
                                               QString* warning)
{
    if (interactive && wantsReset(arguments)) {
        QString body = QCoreApplication::translate(
            "PreferencesStore",
            "Reset preferences, color settings, custom shortcuts, and workspaces? A backup will be kept.");
        if (hasPendingSettingsImport()) {
            body += QStringLiteral("\n\n") + QCoreApplication::translate(
                "PreferencesStore", "The pending settings import will be moved to the backup.");
        }
        if (QMessageBox::question(nullptr,
                QCoreApplication::translate("PreferencesStore", "Reset Preferences"),
                body, QMessageBox::Ok | QMessageBox::Cancel, QMessageBox::Cancel) == QMessageBox::Ok) {
            resetProfile(warning);
        }
    }
    try {
        auto lease = profile_open(bytes(configurationDirectory().toUtf8()),
                                  bytes(stateDirectory().toUtf8()));
        return std::unique_ptr<ProfilePresence>(
            new ProfilePresence(std::make_unique<ProfilePresence::Private>(ProfilePresence::Private{std::move(lease)})));
    } catch (const rust::Error& failure) {
        setError(warning, failure);
        return {};
    }
}

bool resetProfile(QString* error)
{
    try {
        profile_reset(bytes(configurationDirectory().toUtf8()), bytes(stateDirectory().toUtf8()));
        unsaved().remove(configurationDirectory());
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

bool writeProfileFileDurably(const QString& path, const QByteArray& data, QString* error)
{
    try {
        profile_write(bytes(path.toUtf8()), {reinterpret_cast<const uint8_t*>(data.constData()),
                                             static_cast<size_t>(data.size())});
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

bool retireProfileFileDurably(const QString& path, QString* error)
{
    try {
        profile_retire(bytes(path.toUtf8()));
        return true;
    } catch (const rust::Error& failure) {
        setError(error, failure);
        return false;
    }
}

} // namespace pictura
