#include "session.h"

#include <QtCore/QByteArray>
#include <QtGui/QColor>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonParseError>
#include "protected_output.h"
#include <QtCore/QLockFile>
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

bool gFailSessionCommitForTest = false;

} // namespace

QJsonObject readSessionObject()
{
    QFile file(sessionFilePath());
    if (!file.open(QIODevice::ReadOnly) || file.size() > kMaxSessionBytes) {
        return {};
    }
    const auto doc = QJsonDocument::fromJson(file.readAll());
    return doc.object();
}

void setSessionCommitFailureForTest(bool fail)
{
    gFailSessionCommitForTest = fail;
}

bool writeSessionObject(const QJsonObject& object)
{
    const auto data = QJsonDocument(object).toJson(QJsonDocument::Compact);
    ProtectedOutput file(sessionFilePath());
    if (!file.open(QIODevice::WriteOnly) || file.write(data) != data.size()
        || gFailSessionCommitForTest) {
        return false;
    }
    return file.commit();
}

QString sessionFilePath()
{
    const QString base = qEnvironmentVariable("XDG_STATE_HOME");
    const QString root =
        base.isEmpty() ? QDir::homePath() + QStringLiteral("/.local/state") : base;
    return root + QStringLiteral("/kooka-pictura/state.json");
}

bool sessionHasColumnLayout(const SessionState& state)
{
    for (const QJsonValue& value : state.panelColumns) {
        const QJsonObject entry = value.toObject();
        if (!entry.value(QStringLiteral("tools")).toBool()
            && !entry.value(QStringLiteral("groups")).toArray().isEmpty()) {
            return true;
        }
    }
    return false;
}

bool sessionHasPanelLayout(const SessionState& state)
{
    return !state.panelGroups.isEmpty() || sessionHasColumnLayout(state);
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
    state.layersThumbSize = obj.value(QStringLiteral("layersThumbSize")).toInt(2);
    state.quickMaskSelected=obj.value(QStringLiteral("quickMaskSelected")).toBool(false);
    const QColor quickColor(obj.value(QStringLiteral("quickMaskColor")).toString());
    state.quickMaskColor=quickColor.isValid()?quickColor.name():QStringLiteral("#ff0000");
    state.quickMaskOpacity=qBound(0,obj.value(QStringLiteral("quickMaskOpacity")).toInt(50),100);
    state.layersThumbContents = obj.value(QStringLiteral("layersThumbContents")).toInt(0);
    state.layersExpandNewEffects =
        obj.value(QStringLiteral("layersExpandNewEffects")).toBool(true);
    state.layersAddCopyOnDuplicate =
        obj.value(QStringLiteral("layersAddCopyOnDuplicate")).toBool(true);
    state.layersUseDefaultMasksOnFill =
        obj.value(QStringLiteral("layersUseDefaultMasksOnFill")).toBool(true);
    state.layersAddMaskByDefault =
        obj.value(QStringLiteral("layersAddMaskByDefault")).toBool(true);
    state.historyAutoFirstSnapshot =
        obj.value(QStringLiteral("historyAutoFirstSnapshot")).toBool(true);
    state.historyAutoSnapshotOnSave =
        obj.value(QStringLiteral("historyAutoSnapshotOnSave")).toBool(false);
    state.historyAllowNonLinear =
        obj.value(QStringLiteral("historyAllowNonLinear")).toBool(false);
    state.historyShowSnapshotDialog =
        obj.value(QStringLiteral("historyShowSnapshotDialog")).toBool(false);
    state.historyVisibilityUndoable =
        obj.value(QStringLiteral("historyVisibilityUndoable")).toBool(false);
    state.toolsColumns =
        obj.value(QStringLiteral("toolsColumns")).toInt(1) == 2 ? 2 : 1;
    state.rulersVisible = obj.value(QStringLiteral("rulersVisible")).toBool(false);
    state.guidesVisible = obj.value(QStringLiteral("guidesVisible")).toBool(true);
    state.guidesLocked = obj.value(QStringLiteral("guidesLocked")).toBool(false);
    state.smartGuidesVisible = obj.value(QStringLiteral("smartGuidesVisible")).toBool(true);
    state.selectionEdges = obj.value(QStringLiteral("selectionEdges")).toBool(true);
    state.showTargetPath = obj.value(QStringLiteral("showTargetPath")).toBool(true);
    state.showLayerEdges = obj.value(QStringLiteral("showLayerEdges")).toBool(true);
    state.showGrid = obj.value(QStringLiteral("showGrid")).toBool(false);
    state.showCount = obj.value(QStringLiteral("showCount")).toBool(true);
    state.showSlices = obj.value(QStringLiteral("showSlices")).toBool(true);
    state.showNotes = obj.value(QStringLiteral("showNotes")).toBool(true);
    state.showPixelGrid = obj.value(QStringLiteral("showPixelGrid")).toBool(true);
    state.showColorSamplers = obj.value(QStringLiteral("showColorSamplers")).toBool(true);
    state.extras = obj.value(QStringLiteral("extras")).toBool(true);
    const QJsonValue extrasMembers = obj.value(QStringLiteral("extrasMembers"));
    state.extrasMembersSet = extrasMembers.isArray();
    for (const QJsonValue& id : extrasMembers.toArray()) {
        if (id.isString()) {
            state.extrasMembers.append(id.toString());
        }
    }
    state.lockSlices = obj.value(QStringLiteral("lockSlices")).toBool(false);
    state.gridColor = obj.value(QStringLiteral("gridColor")).toString(state.gridColor);
    state.gridStyle = qBound(0, obj.value(QStringLiteral("gridStyle")).toInt(0), 2);
    state.gridUnit = qBound(0, obj.value(QStringLiteral("gridUnit")).toInt(1), 6);
    state.gridEvery = obj.value(QStringLiteral("gridEvery")).toDouble(1.0);
    if (!(state.gridEvery > 0.0)) {
        state.gridEvery = 1.0;
    }
    state.gridSubdivisions = qMax(1, obj.value(QStringLiteral("gridSubdivisions")).toInt(4));
    state.freePan = obj.value(QStringLiteral("freePan")).toBool(true);
    state.snapCanvasToWorkspace =
        obj.value(QStringLiteral("snapCanvasToWorkspace")).toBool(false);
    state.proofDevice = qBound(0, obj.value(QStringLiteral("proofDevice")).toInt(0), 4);
    state.proofIntent = qBound(0, obj.value(QStringLiteral("proofIntent")).toInt(1), 3);
    state.proofBpc = obj.value(QStringLiteral("proofBpc")).toBool(true);
    state.proofSimulatePaper = obj.value(QStringLiteral("proofSimulatePaper")).toBool(false);
    state.proofSimulateBlack = obj.value(QStringLiteral("proofSimulateBlack")).toBool(false);
    state.proofColors = obj.value(QStringLiteral("proofColors")).toBool(false);
    state.pdfOptions = obj.value(QStringLiteral("pdfOptions")).toString();
    state.saveFormatOptions = obj.value(QStringLiteral("saveFormatOptions")).toObject();
    state.batchOptions = obj.value(QStringLiteral("batchOptions")).toObject();
    state.statisticsOptions = obj.value(QStringLiteral("statisticsOptions")).toObject();
    state.cropClassicMode = obj.value(QStringLiteral("cropClassicMode")).toBool(false);
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

    QLockFile lock(path + QStringLiteral(".lock"));
    if (!lock.tryLock(1000)) {
        return false;
    }
    QJsonObject obj = readSessionObject();
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
    obj.insert(QStringLiteral("layersThumbSize"), state.layersThumbSize);
    obj.insert(QStringLiteral("quickMaskSelected"),state.quickMaskSelected);
    obj.insert(QStringLiteral("quickMaskColor"),state.quickMaskColor);
    obj.insert(QStringLiteral("quickMaskOpacity"),state.quickMaskOpacity);
    obj.insert(QStringLiteral("layersThumbContents"), state.layersThumbContents);
    obj.insert(QStringLiteral("layersExpandNewEffects"), state.layersExpandNewEffects);
    obj.insert(QStringLiteral("layersAddCopyOnDuplicate"), state.layersAddCopyOnDuplicate);
    obj.insert(QStringLiteral("layersUseDefaultMasksOnFill"), state.layersUseDefaultMasksOnFill);
    obj.insert(QStringLiteral("layersAddMaskByDefault"), state.layersAddMaskByDefault);
    obj.insert(QStringLiteral("historyAutoFirstSnapshot"), state.historyAutoFirstSnapshot);
    obj.insert(QStringLiteral("historyAutoSnapshotOnSave"), state.historyAutoSnapshotOnSave);
    obj.insert(QStringLiteral("historyAllowNonLinear"), state.historyAllowNonLinear);
    obj.insert(QStringLiteral("historyShowSnapshotDialog"), state.historyShowSnapshotDialog);
    obj.insert(QStringLiteral("historyVisibilityUndoable"), state.historyVisibilityUndoable);
    obj.insert(QStringLiteral("toolsColumns"), state.toolsColumns);
    obj.insert(QStringLiteral("rulersVisible"), state.rulersVisible);
    obj.insert(QStringLiteral("guidesVisible"), state.guidesVisible);
    obj.insert(QStringLiteral("guidesLocked"), state.guidesLocked);
    obj.insert(QStringLiteral("smartGuidesVisible"), state.smartGuidesVisible);
    obj.insert(QStringLiteral("selectionEdges"), state.selectionEdges);
    obj.insert(QStringLiteral("showTargetPath"), state.showTargetPath);
    obj.insert(QStringLiteral("showLayerEdges"), state.showLayerEdges);
    obj.insert(QStringLiteral("showGrid"), state.showGrid);
    obj.insert(QStringLiteral("showCount"), state.showCount);
    obj.insert(QStringLiteral("showSlices"), state.showSlices);
    obj.insert(QStringLiteral("showNotes"), state.showNotes);
    obj.insert(QStringLiteral("showPixelGrid"), state.showPixelGrid);
    obj.insert(QStringLiteral("showColorSamplers"), state.showColorSamplers);
    obj.insert(QStringLiteral("extras"), state.extras);
    if (state.extrasMembersSet) {
        obj.insert(QStringLiteral("extrasMembers"), QJsonArray::fromStringList(state.extrasMembers));
    }
    obj.insert(QStringLiteral("lockSlices"), state.lockSlices);
    obj.insert(QStringLiteral("gridColor"), state.gridColor);
    obj.insert(QStringLiteral("gridStyle"), state.gridStyle);
    obj.insert(QStringLiteral("gridUnit"), state.gridUnit);
    obj.insert(QStringLiteral("gridEvery"), state.gridEvery);
    obj.insert(QStringLiteral("gridSubdivisions"), state.gridSubdivisions);
    obj.insert(QStringLiteral("freePan"), state.freePan);
    obj.insert(QStringLiteral("snapCanvasToWorkspace"), state.snapCanvasToWorkspace);
    obj.insert(QStringLiteral("proofDevice"), state.proofDevice);
    obj.insert(QStringLiteral("proofIntent"), state.proofIntent);
    obj.insert(QStringLiteral("proofBpc"), state.proofBpc);
    obj.insert(QStringLiteral("proofSimulatePaper"), state.proofSimulatePaper);
    obj.insert(QStringLiteral("proofSimulateBlack"), state.proofSimulateBlack);
    obj.insert(QStringLiteral("proofColors"), state.proofColors);
    obj.insert(QStringLiteral("pdfOptions"), state.pdfOptions);
    obj.insert(QStringLiteral("saveFormatOptions"), state.saveFormatOptions);
    obj.insert(QStringLiteral("batchOptions"), state.batchOptions);
    obj.insert(QStringLiteral("statisticsOptions"), state.statisticsOptions);
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
    QJsonArray recent;
    for (const QString& path : state.recent) {
        recent.append(path);
    }
    obj.insert(QStringLiteral("recent"), recent);

    return writeSessionObject(obj);
}

} // namespace pictura
