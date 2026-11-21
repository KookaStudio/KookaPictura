#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QMap>
#include <QtCore/QList>
#include <QtCore/QStringList>

namespace pictura {

inline const QList<int> kAutoSaveMinuteChoices = {5, 10, 15, 30, 60};

using CommandShortcutOverrides = QMap<QString, QStringList>;
using ToolShortcutOverrides = QMap<QString, QString>;

struct Preferences {
#define PICTURA_PREF(Type, name) Type name{};
#include "preference_fields.inc"
#undef PICTURA_PREF
    bool agentRecoveryGuard = false;
};

QString configurationDirectory();
QString preferencesFilePath();
bool initializePreferences(QString* notice = nullptr);
Preferences defaultPreferences();
Preferences loadPreferences();
bool savePreferences(const Preferences& before, const Preferences& after,
                     bool explicitAgentGrant = false);

QString preferenceMigrationMarker();
void removeLegacyPreferenceKeys(QJsonObject& state);

bool validatePortablePreferences(const QString& toml, QString* error);
bool exportPortablePreferences(QString* toml, QString* error);
bool applyImportedPreferences(const QJsonObject& legacy, const QString* toml,
                              const QString& id, QString* error);
void retirePreferencesReceipt(const QString& id);

} // namespace pictura
