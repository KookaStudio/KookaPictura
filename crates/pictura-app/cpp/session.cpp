#include "session.h"

#include <QtCore/QByteArray>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonParseError>
#include <QtCore/QSaveFile>
#include <QtCore/QVariantMap>
#include <QtCore/QUuid>

namespace pictura {

namespace {

// The state file is tiny JSON; anything larger is corrupt or hostile, so it is
// not read into memory.
constexpr qint64 kMaxSessionBytes = 1024 * 1024;

// This launch's identity in the store. A store another token wrote at or after
// the launch start is newer than the state this launch loaded.
QString instanceToken()
{
    static const QString token = QUuid::createUuid().toString(QUuid::WithoutBraces);
    return token;
}

// Captured at process start, not on first save: a store written before the
// first save but after the launch must still be refused.
const qint64 kInstanceStartMs = QDateTime::currentMSecsSinceEpoch();

QMap<QString, QString> readStringMap(const QJsonObject& map)
{
    QMap<QString, QString> values;
    for (auto it = map.constBegin(); it != map.constEnd(); ++it) {
        if (it.value().isString()) {
            values.insert(it.key(), it.value().toString());
        }
    }
    return values;
}

QVariantMap toVariantMap(const QMap<QString, QString>& values)
{
    QVariantMap map;
    for (auto it = values.constBegin(); it != values.constEnd(); ++it) {
        map.insert(it.key(), it.value());
    }
    return map;
}

// The existing store's top-level object, or empty when missing/corrupt. Used by
// the load-then-write save so keys this build does not know survive a rewrite.
QJsonObject readStoreObject()
{
    QFile file(sessionFilePath());
    if (!file.open(QIODevice::ReadOnly) || file.size() > kMaxSessionBytes) {
        return {};
    }
    QJsonParseError error{};
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !doc.isObject()) {
        return {};
    }
    return doc.object();
}

} // namespace

// ponytail: stopgap for the deferred typed pictura-prefs / prefs.toml store
// (XC-002); keeps the C++ session file so M16 needs no new dependency.
QString sessionFilePath()
{
    const QString base = qEnvironmentVariable("XDG_STATE_HOME");
    const QString root =
        base.isEmpty() ? QDir::homePath() + QStringLiteral("/.local/state") : base;
    return root + QStringLiteral("/kooka-pictura/state.json");
}

SessionState loadSession()
{
    SessionState state;

    QFile file(sessionFilePath());
    if (!file.open(QIODevice::ReadOnly) || file.size() > kMaxSessionBytes) {
        return state;
    }

    QJsonParseError error{};
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !doc.isObject()) {
        return SessionState{};
    }

    const QJsonObject obj = doc.object();
    state.schemaVersion = obj.value(QStringLiteral("schemaVersion")).toInt(1);
    state.brightnessLevel = obj.value(QStringLiteral("brightnessLevel")).toInt(1);
    state.themeFamily = obj.value(QStringLiteral("themeFamily")).toInt(0);
    state.gpuCompute = obj.value(QStringLiteral("gpuCompute")).toBool(true);
    const int colorPolicy = obj.value(QStringLiteral("colorPolicy")).toInt(0);
    state.colorPolicy = (colorPolicy == 1 || colorPolicy == 2) ? colorPolicy : 0;
    state.textGammaEnabled = obj.value(QStringLiteral("textGammaEnabled")).toBool(true);
    state.textGamma = qBound(1.0, obj.value(QStringLiteral("textGamma")).toDouble(1.45), 2.2);
    state.layersThumbSize = obj.value(QStringLiteral("layersThumbSize")).toInt(2);
    state.layersThumbContents = obj.value(QStringLiteral("layersThumbContents")).toInt(0);
    state.layersExpandNewEffects =
        obj.value(QStringLiteral("layersExpandNewEffects")).toBool(true);
    state.layersAddCopyOnDuplicate =
        obj.value(QStringLiteral("layersAddCopyOnDuplicate")).toBool(true);
    state.layersUseDefaultMasksOnFill =
        obj.value(QStringLiteral("layersUseDefaultMasksOnFill")).toBool(true);
    state.toolsColumns =
        obj.value(QStringLiteral("toolsColumns")).toInt(1) == 2 ? 2 : 1;
    state.useShiftKeyForToolSwitch =
        obj.value(QStringLiteral("useShiftKeyForToolSwitch")).toBool(true);
    state.typePlaceholderText = obj.value(QStringLiteral("typePlaceholderText")).toBool(true);
    state.confirmLiveShapeToPath =
        obj.value(QStringLiteral("confirmLiveShapeToPath")).toBool(true);
    state.confirmDeleteVectorMask =
        obj.value(QStringLiteral("confirmDeleteVectorMask")).toBool(true);
    state.showFullScreenMessage =
        obj.value(QStringLiteral("showFullScreenMessage")).toBool(true);
    state.autoSaveRecovery = obj.value(QStringLiteral("autoSaveRecovery")).toBool(true);
    const int autoSaveMinutes = obj.value(QStringLiteral("autoSaveMinutes")).toInt(10);
    state.autoSaveMinutes = kAutoSaveMinuteChoices.contains(autoSaveMinutes) ? autoSaveMinutes : 10;
    state.rulersVisible = obj.value(QStringLiteral("rulersVisible")).toBool(false);
    state.rulerUnit = obj.value(QStringLiteral("rulerUnit")).toInt(1);
    state.traditionalPoints = obj.value(QStringLiteral("traditionalPoints")).toBool(false);
    state.guidesVisible = obj.value(QStringLiteral("guidesVisible")).toBool(true);
    state.guidesLocked = obj.value(QStringLiteral("guidesLocked")).toBool(false);
    state.smartGuidesVisible = obj.value(QStringLiteral("smartGuidesVisible")).toBool(true);
    state.freePan = obj.value(QStringLiteral("freePan")).toBool(true);
    state.snapCanvasToWorkspace =
        obj.value(QStringLiteral("snapCanvasToWorkspace")).toBool(false);
    state.guideColor = obj.value(QStringLiteral("guideColor")).toString(state.guideColor);
    state.pdfOptions = obj.value(QStringLiteral("pdfOptions")).toString();
    state.guideDashed = obj.value(QStringLiteral("guideDashed")).toBool(false);
    state.cropClassicMode = obj.value(QStringLiteral("cropClassicMode")).toBool(true);
    state.cropDeletePixels = obj.value(QStringLiteral("cropDeletePixels")).toBool(true);
    state.cropRatio = obj.value(QStringLiteral("cropRatio")).toDouble(0.0);
    state.cropGridOverlay = obj.value(QStringLiteral("cropGridOverlay")).toInt(0);
    state.toolOptions = obj.value(QStringLiteral("toolOptions")).toObject();
    if (obj.contains(QStringLiteral("swatches"))
        && obj.value(QStringLiteral("swatches")).isArray()) {
        state.swatches = obj.value(QStringLiteral("swatches")).toArray();
        state.swatchesLoaded = true;
    }
    state.actionsButtonMode = obj.value(QStringLiteral("actionsButtonMode")).toBool(false);
    state.actionsAllowToolRecording =
        obj.value(QStringLiteral("actionsAllowToolRecording")).toBool(false);
    const QJsonArray actionsExpanded = obj.value(QStringLiteral("actionsExpanded")).toArray();
    for (const QJsonValue& entry : actionsExpanded) {
        if (entry.isString()) {
            state.actionsExpanded << entry.toString();
        }
    }
    const QJsonArray actionsSelection = obj.value(QStringLiteral("actionsSelection")).toArray();
    for (const QJsonValue& entry : actionsSelection) {
        if (entry.isString()) {
            state.actionsSelection << entry.toString();
        }
    }
    const int actionsPlaybackMode =
        obj.value(QStringLiteral("actionsPlaybackMode")).toInt(0);
    state.actionsPlaybackMode = (actionsPlaybackMode >= 0 && actionsPlaybackMode <= 2)
                                    ? actionsPlaybackMode
                                    : 0;
    state.actionsPlaybackPause =
        qMax(0.0, obj.value(QStringLiteral("actionsPlaybackPause")).toDouble(0.0));
    state.panelRailMode = obj.value(QStringLiteral("panelRailMode")).toString(
        QStringLiteral("normal"));
    if (state.panelRailMode != QStringLiteral("iconic")) {
        state.panelRailMode = QStringLiteral("normal");
    }
    state.railWidth = obj.value(QStringLiteral("railWidth")).toInt(0);
    state.autoCollapseIconic =
        obj.value(QStringLiteral("autoCollapseIconic")).toBool(false);
    state.autoShowHidden = obj.value(QStringLiteral("autoShowHidden")).toBool(false);
    state.panelGroups = obj.value(QStringLiteral("panelGroups")).toArray();
    state.panelColumns = obj.value(QStringLiteral("panelColumns")).toArray();
    // v5 (or older) stores have no per-column layout: synthesise one right-hand
    // column from the legacy flat per-group state so they open unchanged. A
    // store with no legacy groups is a fresh/default store, not a one-column
    // empty workspace.
    if (!obj.contains(QStringLiteral("panelColumns"))
        && !state.panelGroups.isEmpty()) {
        QJsonObject column;
        column.insert(QStringLiteral("side"), QStringLiteral("right"));
        column.insert(QStringLiteral("order"), 0);
        column.insert(QStringLiteral("groups"), state.panelGroups);
        state.panelColumns = QJsonArray{column};
    }
    state.layout =
        QByteArray::fromBase64(obj.value(QStringLiteral("layout")).toString().toLatin1());
    state.layoutRevision = obj.value(QStringLiteral("layoutRevision")).toInt(0);
    state.windowGeometry = QByteArray::fromBase64(
        obj.value(QStringLiteral("windowGeometry")).toString().toLatin1());
    state.windowMaximized =
        obj.value(QStringLiteral("windowMaximized")).toBool(false);
    const QJsonObject shortcuts = obj.value(QStringLiteral("shortcuts")).toObject();
    for (auto it = shortcuts.constBegin(); it != shortcuts.constEnd(); ++it) {
        QStringList bindings;
        for (const QJsonValue& binding : it.value().toArray()) {
            if (binding.isString()) {
                bindings << binding.toString();
            }
        }
        state.shortcuts.insert(it.key(), bindings);
    }
    state.toolShortcuts = readStringMap(obj.value(QStringLiteral("toolShortcuts")).toObject());
    const QJsonArray recent = obj.value(QStringLiteral("recent")).toArray();
    for (const QJsonValue& entry : recent) {
        if (entry.isString()) {
            state.recent.append(entry.toString());
        }
    }
    return state;
}

bool saveSession(const SessionState& state)
{
    const QString path = sessionFilePath();
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        return false;
    }

    QJsonObject obj = readStoreObject();
    // A store another instance wrote after this one launched is newer than the
    // state this instance loaded; refuse rather than silently clobber it.
    const QString writer = obj.value(QStringLiteral("writerToken")).toString();
    const qint64 writtenAt =
        obj.value(QStringLiteral("writtenAt")).toVariant().toLongLong();
    if (!writer.isEmpty() && writer != instanceToken() && writtenAt >= kInstanceStartMs) {
        return false;
    }
    obj.insert(QStringLiteral("writerToken"), instanceToken());
    obj.insert(QStringLiteral("writtenAt"), QDateTime::currentMSecsSinceEpoch());
    obj.insert(QStringLiteral("schemaVersion"), state.schemaVersion);
    obj.insert(QStringLiteral("brightnessLevel"), state.brightnessLevel);
    obj.insert(QStringLiteral("themeFamily"), state.themeFamily);
    obj.insert(QStringLiteral("gpuCompute"), state.gpuCompute);
    obj.insert(QStringLiteral("colorPolicy"), state.colorPolicy);
    obj.insert(QStringLiteral("textGammaEnabled"), state.textGammaEnabled);
    obj.insert(QStringLiteral("textGamma"), state.textGamma);
    obj.insert(QStringLiteral("layersThumbSize"), state.layersThumbSize);
    obj.insert(QStringLiteral("layersThumbContents"), state.layersThumbContents);
    obj.insert(QStringLiteral("layersExpandNewEffects"), state.layersExpandNewEffects);
    obj.insert(QStringLiteral("layersAddCopyOnDuplicate"), state.layersAddCopyOnDuplicate);
    obj.insert(QStringLiteral("layersUseDefaultMasksOnFill"), state.layersUseDefaultMasksOnFill);
    obj.insert(QStringLiteral("toolsColumns"), state.toolsColumns);
    obj.insert(QStringLiteral("useShiftKeyForToolSwitch"), state.useShiftKeyForToolSwitch);
    obj.insert(QStringLiteral("typePlaceholderText"), state.typePlaceholderText);
    obj.insert(QStringLiteral("confirmLiveShapeToPath"), state.confirmLiveShapeToPath);
    obj.insert(QStringLiteral("confirmDeleteVectorMask"), state.confirmDeleteVectorMask);
    obj.insert(QStringLiteral("showFullScreenMessage"), state.showFullScreenMessage);
    obj.insert(QStringLiteral("autoSaveRecovery"), state.autoSaveRecovery);
    obj.insert(QStringLiteral("autoSaveMinutes"), state.autoSaveMinutes);
    obj.insert(QStringLiteral("rulersVisible"), state.rulersVisible);
    obj.insert(QStringLiteral("rulerUnit"), state.rulerUnit);
    obj.insert(QStringLiteral("traditionalPoints"), state.traditionalPoints);
    obj.insert(QStringLiteral("guidesVisible"), state.guidesVisible);
    obj.insert(QStringLiteral("guidesLocked"), state.guidesLocked);
    obj.insert(QStringLiteral("smartGuidesVisible"), state.smartGuidesVisible);
    obj.insert(QStringLiteral("freePan"), state.freePan);
    obj.insert(QStringLiteral("snapCanvasToWorkspace"), state.snapCanvasToWorkspace);
    obj.insert(QStringLiteral("guideColor"), state.guideColor);
    obj.insert(QStringLiteral("pdfOptions"), state.pdfOptions);
    obj.insert(QStringLiteral("guideDashed"), state.guideDashed);
    obj.insert(QStringLiteral("cropClassicMode"), state.cropClassicMode);
    obj.insert(QStringLiteral("cropDeletePixels"), state.cropDeletePixels);
    obj.insert(QStringLiteral("cropRatio"), state.cropRatio);
    obj.insert(QStringLiteral("cropGridOverlay"), state.cropGridOverlay);
    obj.insert(QStringLiteral("toolOptions"), state.toolOptions);
    obj.insert(QStringLiteral("actionsButtonMode"), state.actionsButtonMode);
    obj.insert(QStringLiteral("actionsAllowToolRecording"), state.actionsAllowToolRecording);
    obj.insert(QStringLiteral("actionsExpanded"),
               QJsonArray::fromStringList(state.actionsExpanded));
    obj.insert(QStringLiteral("actionsSelection"),
               QJsonArray::fromStringList(state.actionsSelection));
    obj.insert(QStringLiteral("actionsPlaybackMode"), state.actionsPlaybackMode);
    obj.insert(QStringLiteral("actionsPlaybackPause"), state.actionsPlaybackPause);
    if (state.swatchesLoaded) {
        obj.insert(QStringLiteral("swatches"), state.swatches);
    }
    obj.insert(QStringLiteral("panelRailMode"), state.panelRailMode);
    obj.insert(QStringLiteral("railWidth"), state.railWidth);
    obj.insert(QStringLiteral("autoCollapseIconic"), state.autoCollapseIconic);
    obj.insert(QStringLiteral("autoShowHidden"), state.autoShowHidden);
    obj.insert(QStringLiteral("panelGroups"), state.panelGroups);
    obj.insert(QStringLiteral("panelColumns"), state.panelColumns);
    obj.insert(QStringLiteral("layout"), QString::fromLatin1(state.layout.toBase64()));
    obj.insert(QStringLiteral("layoutRevision"), state.layoutRevision);
    obj.insert(QStringLiteral("windowGeometry"),
               QString::fromLatin1(state.windowGeometry.toBase64()));
    obj.insert(QStringLiteral("windowMaximized"), state.windowMaximized);
    QJsonObject shortcuts;
    for (auto it = state.shortcuts.constBegin(); it != state.shortcuts.constEnd(); ++it) {
        shortcuts.insert(it.key(), QJsonArray::fromStringList(it.value()));
    }
    obj.insert(QStringLiteral("shortcuts"), shortcuts);
    obj.insert(QStringLiteral("toolShortcuts"),
               QJsonObject::fromVariantMap(toVariantMap(state.toolShortcuts)));
    QJsonArray recent;
    for (const QString& path : state.recent) {
        recent.append(path);
    }
    obj.insert(QStringLiteral("recent"), recent);

    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Compact));
    return file.commit();
}

} // namespace pictura
