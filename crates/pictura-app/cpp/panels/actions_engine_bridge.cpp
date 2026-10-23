#include "actions_engine_bridge.h"

#include "actions_tool_recording_bridge.h"

#include "pictura_app/src/cxxqt_object/actions_engine.cxxqt.h"

#include "session.h"

#include <QtCore/QByteArray>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonParseError>
#include <QtCore/QSaveFile>
#include <QtCore/QSet>
#include <QtCore/QStandardPaths>
#include <QtCore/QTextStream>

#include <algorithm>
#include <cstdint>

namespace pictura {

namespace {

ActionsEngineBridge* g_activeActionsEngine = nullptr;

::rust::Slice<const std::uint8_t> byteSlice(const QByteArray& bytes)
{
    return ::rust::Slice<const std::uint8_t>(
        reinterpret_cast<const std::uint8_t*>(bytes.constData()),
        static_cast<std::size_t>(bytes.size()));
}

int stepCountIn(const QList<ActionNodeRow>& rows, const QString& actionId)
{
    int count = 0;
    for (const ActionNodeRow& row : rows) {
        if (row.kind == actionKindStep() && row.parentId == actionId) {
            ++count;
        }
    }
    return count;
}

int actionCountIn(const QList<ActionNodeRow>& rows, const QString& setId)
{
    int count = 0;
    for (const ActionNodeRow& row : rows) {
        if (row.kind == actionKindAction() && row.parentId == setId) {
            ++count;
        }
    }
    return count;
}

bool tool_recording_active_hook()
{
    return g_activeActionsEngine && g_activeActionsEngine->isRecording()
           && g_activeActionsEngine->allowToolRecording();
}

bool record_tool_stroke_hook(const QString& payload)
{
    return g_activeActionsEngine && g_activeActionsEngine->recordToolStrokePayload(payload);
}

void tool_stroke_refused_hook(const QString& message)
{
    if (g_activeActionsEngine) {
        g_activeActionsEngine->toolStrokeRefused(message);
    }
}

} // namespace

ActionsEngineBridge::ActionsEngineBridge(ActionsEngine* engine, QObject* parent)
    : ActionsEngineApi(parent)
    , engine_(engine)
{
    set_tool_recording_hooks(&tool_recording_active_hook, &record_tool_stroke_hook,
                             &tool_stroke_refused_hook);
    if (engine_) {
        QObject::connect(engine_, &ActionsEngine::changed, this, [this] {
            refreshSignals();
            emit changed();
        });
        QObject::connect(engine_, &QObject::destroyed, this, [this] { engine_ = nullptr; });
#ifdef Q_OS_WIN
        engine_->set_windows_platform(true);
#endif
        recordingState_ = isRecording();
        playbackState_ = isPlaying();
        lastEngineError_ = engine_->last_error();
    }
}

ActionsEngineBridge::~ActionsEngineBridge()
{
    if (g_activeActionsEngine == this) {
        g_activeActionsEngine = nullptr;
        set_tool_recording_hooks(nullptr, nullptr, nullptr);
    }
}

ActionsEngineBridge::Ref ActionsEngineBridge::parseId(const QString& id)
{
    Ref ref;
    if (!id.startsWith(QLatin1Char('s'))) {
        return ref;
    }
    int position = 1;
    int set = 0;
    while (position < id.size() && id.at(position).isDigit()) {
        set = set * 10 + id.at(position).digitValue();
        ++position;
    }
    if (position == 1) {
        return ref;
    }
    ref.set = set;
    if (position >= id.size()) {
        return ref;
    }
    if (id.at(position++) != QLatin1Char('a')) {
        return Ref{};
    }
    int action = 0;
    const int actionStart = position;
    while (position < id.size() && id.at(position).isDigit()) {
        action = action * 10 + id.at(position).digitValue();
        ++position;
    }
    if (position == actionStart) {
        return Ref{};
    }
    ref.action = action;
    if (position >= id.size()) {
        return ref;
    }
    if (id.at(position++) != QLatin1Char('t')) {
        return Ref{};
    }
    int step = 0;
    const int stepStart = position;
    while (position < id.size() && id.at(position).isDigit()) {
        step = step * 10 + id.at(position).digitValue();
        ++position;
    }
    if (position == stepStart || position != id.size()) {
        return Ref{};
    }
    ref.step = step;
    return ref;
}

QString ActionsEngineBridge::makeId(const Ref& ref)
{
    if (ref.set < 0) {
        return QString();
    }
    QString id = QStringLiteral("s%1").arg(ref.set);
    if (ref.action < 0) {
        return id;
    }
    id += QStringLiteral("a%1").arg(ref.action);
    if (ref.step < 0) {
        return id;
    }
    return id + QStringLiteral("t%1").arg(ref.step);
}

int ActionsEngineBridge::keyNumber(int key)
{
    if (key < Qt::Key_F1 || key > Qt::Key_F12) {
        return 0;
    }
    return key - Qt::Key_F1 + 1;
}

QList<ActionNodeRow> ActionsEngineBridge::snapshot() const
{
    QList<ActionNodeRow> rows;
    if (!engine_) {
        return rows;
    }
    for (const ScriptActionNodeRow& source : engine_->snapshot()) {
        ActionNodeRow row;
        row.id = source.id;
        row.parentId = source.parent_id;
        row.kind = source.kind;
        row.name = source.name;
        row.enabled = source.enabled;
        const QString modal = source.modal;
        row.modal = modal == QLatin1String("on") || modal == QLatin1String("required");
        if (row.kind == actionKindStep()) {
            if (modal == QLatin1String("required")) {
                row.dialogOptions = 1;
            } else if (modal == QLatin1String("none")) {
                row.dialogOptions = 2;
            } else if (modal == QLatin1String("specific")) {
                row.dialogOptions = 3;
            }
        }
        row.color = source.color;
        row.functionShift = source.function_shift;
        row.functionCtrl = source.function_ctrl;
        row.functionKey = source.function_key_number >= 1
                              ? Qt::Key_F1 + source.function_key_number - 1
                              : 0;
        row.stepIndex = source.step_index;
        row.canExpand = source.can_expand;
        row.stepKind = source.step_kind;
        row.commandId = source.command_id;
        row.message = source.message;
        row.allowContinue = source.allow_continue;
        row.condition = source.condition;
        row.thenActionId = source.then_action;
        row.elseActionId = source.else_action;
        row.path = source.path;
        row.params = QJsonDocument::fromJson(source.params_json.toUtf8()).object();
        rows << row;
    }
    return rows;
}

QString ActionsEngineBridge::lastError() const
{
    const QString engineError = engine_ ? engine_->last_error() : QString();
    return engineError.isEmpty() ? refusedError_ : engineError;
}

void ActionsEngineBridge::toolStrokeRefused(const QString& message)
{
    refusedError_ = message;
    refreshSignals();
}

void ActionsEngineBridge::refreshSignals()
{
    const bool recording = isRecording();
    if (recording != recordingState_) {
        recordingState_ = recording;
        emit recordingChanged(recording);
    }
    const bool playing = isPlaying();
    if (playing != playbackState_) {
        playbackState_ = playing;
        emit playbackChanged(playing);
    }
    const QString engineError = engine_ ? engine_->last_error() : QString();
    if (engineError != lastEngineError_) {
        lastEngineError_ = engineError;
        if (engineError.isEmpty()) {
            refusedError_.clear();
        }
    }
    const QString message = lastError();
    if (message.isEmpty()) {
        emittedError_.clear();
        return;
    }
    if (message != emittedError_) {
        emittedError_ = message;
        emit error(message);
    }
}

void ActionsEngineBridge::setDocumentFacts(bool present, bool square, const QString& mode,
                                           bool hasSelection)
{
    if (engine_) {
        engine_->set_document_facts(present, square, mode, hasSelection);
    }
}

QString ActionsEngineBridge::createSet(const QString& name)
{
    if (!engine_) {
        return QString();
    }
    const int index = engine_->create_set(name);
    return index >= 0 ? makeId(Ref{index, -1, -1}) : QString();
}

QString ActionsEngineBridge::createAction(const QString& setId, const QString& name,
                                          int functionKey, bool shift, bool ctrl, int color)
{
    const Ref set = parseId(setId);
    if (!engine_ || set.set < 0) {
        return QString();
    }
    const int index = engine_->create_action(set.set, name);
    if (index < 0) {
        refreshSignals();
        return QString();
    }
    const Ref action{set.set, index, -1};
    engine_->set_action_color(set.set, index, color);
    if (functionKey != 0) {
        engine_->assign_function_key(set.set, index, keyNumber(functionKey), shift, ctrl);
    }
    refreshSignals();
    return makeId(action);
}

bool ActionsEngineBridge::renameNode(const QString& id, const QString& name)
{
    const Ref ref = parseId(id);
    if (!engine_ || ref.set < 0) {
        return false;
    }
    if (ref.step >= 0) {
        return false;
    }
    if (ref.action >= 0) {
        return engine_->rename_action(ref.set, ref.action, name);
    }
    return engine_->rename_set(ref.set, name);
}

bool ActionsEngineBridge::deleteNodes(const QStringList& ids)
{
    if (!engine_) {
        return false;
    }
    QSet<QString> selected;
    for (const QString& id : ids) {
        selected.insert(id);
    }
    QList<Ref> refs;
    for (const QString& id : ids) {
        const Ref ref = parseId(id);
        if (ref.set < 0) {
            continue;
        }
        bool ancestorSelected = false;
        for (Ref ancestor = ref; ancestor.action >= 0 || ancestor.step >= 0;) {
            if (ancestor.step >= 0) {
                ancestor.step = -1;
            } else if (ancestor.action >= 0) {
                ancestor.action = -1;
            }
            if (selected.contains(makeId(ancestor))) {
                ancestorSelected = true;
                break;
            }
        }
        if (!ancestorSelected) {
            refs << ref;
        }
    }
    bool ok = true;
    std::sort(refs.begin(), refs.end(), [](const Ref& a, const Ref& b) {
        return a.step > b.step;
    });
    for (const Ref& ref : refs) {
        if (ref.step >= 0) {
            ok = engine_->delete_step(ref.set, ref.action, ref.step) && ok;
        }
    }
    std::sort(refs.begin(), refs.end(), [](const Ref& a, const Ref& b) {
        return a.action > b.action;
    });
    for (const Ref& ref : refs) {
        if (ref.action >= 0 && ref.step < 0) {
            ok = engine_->delete_action(ref.set, ref.action) && ok;
        }
    }
    std::sort(refs.begin(), refs.end(),
              [](const Ref& a, const Ref& b) { return a.set > b.set; });
    for (const Ref& ref : refs) {
        if (ref.action < 0) {
            ok = engine_->delete_set(ref.set) && ok;
        }
    }
    refreshSignals();
    return ok;
}

bool ActionsEngineBridge::duplicateNodes(const QStringList& ids)
{
    if (!engine_) {
        return false;
    }
    QSet<QString> selected;
    for (const QString& id : ids) {
        selected.insert(id);
    }
    QList<Ref> refs;
    for (const QString& id : ids) {
        const Ref ref = parseId(id);
        if (ref.set < 0) {
            continue;
        }
        bool ancestorSelected = false;
        for (Ref ancestor = ref; ancestor.action >= 0 || ancestor.step >= 0;) {
            if (ancestor.step >= 0) {
                ancestor.step = -1;
            } else if (ancestor.action >= 0) {
                ancestor.action = -1;
            }
            if (selected.contains(makeId(ancestor))) {
                ancestorSelected = true;
                break;
            }
        }
        if (!ancestorSelected) {
            refs << ref;
        }
    }
    std::sort(refs.begin(), refs.end(), [](const Ref& a, const Ref& b) {
        return a.step > b.step;
    });
    for (const Ref& ref : refs) {
        if (ref.step >= 0) {
            engine_->duplicate_step(ref.set, ref.action, ref.step);
        }
    }
    std::sort(refs.begin(), refs.end(), [](const Ref& a, const Ref& b) {
        return a.action > b.action;
    });
    for (const Ref& ref : refs) {
        if (ref.action >= 0 && ref.step < 0) {
            engine_->duplicate_action(ref.set, ref.action);
        }
    }
    std::sort(refs.begin(), refs.end(),
              [](const Ref& a, const Ref& b) { return a.set > b.set; });
    for (const Ref& ref : refs) {
        if (ref.action < 0) {
            engine_->duplicate_set(ref.set);
        }
    }
    refreshSignals();
    return true;
}

bool ActionsEngineBridge::moveNode(const QString& id, const QString& parentId, int row, bool copy)
{
    if (!engine_) {
        return false;
    }
    const Ref source = parseId(id);
    const Ref target = parseId(parentId);
    if (source.set < 0) {
        return false;
    }
    QList<ActionNodeRow> rows = snapshot();
    if (copy) {
        if (source.step >= 0) {
            if (target.action < 0 || target.set < 0) {
                return false;
            }
            if (!engine_->duplicate_step(source.set, source.action, source.step)) {
                return false;
            }
            const int copyIndex = source.step + 1;
            if (target.set == source.set && target.action == source.action) {
                const QString actionId = makeId(Ref{source.set, source.action, -1});
                const int count = stepCountIn(rows, actionId);
                const int to = row < 0 ? count : qBound(0, row, count);
                return engine_->move_step(source.set, source.action, copyIndex, to);
            }
            const QString targetActionId = makeId(Ref{target.set, target.action, -1});
            const int to = row < 0 ? stepCountIn(rows, targetActionId) : row;
            return engine_->move_step_to(source.set, source.action, copyIndex, target.set,
                                         target.action, to);
        }
        if (source.action >= 0) {
            const int duplicate = engine_->duplicate_action(source.set, source.action);
            if (duplicate < 0 || target.set < 0) {
                return duplicate >= 0;
            }
            const QString targetSetId = makeId(Ref{target.set, -1, -1});
            if (target.set == source.set) {
                const int count = actionCountIn(rows, targetSetId);
                const int to = row < 0 ? count : qBound(0, row, count);
                return engine_->move_action(source.set, duplicate, target.set, to);
            }
            const int to = row >= 0 ? row : actionCountIn(rows, targetSetId);
            return engine_->move_action(source.set, duplicate, target.set, to);
        }
        return engine_->duplicate_set(source.set) >= 0;
    }
    if (source.step >= 0) {
        if (target.action < 0 || target.set < 0) {
            return false;
        }
        if (target.set == source.set && target.action == source.action) {
            const QString actionId = makeId(Ref{source.set, source.action, -1});
            const int count = stepCountIn(rows, actionId);
            int to = row >= 0 ? row : count;
            if (to > source.step) {
                --to;
            }
            to = qBound(0, to, count - 1);
            if (to == source.step) {
                return true;
            }
            return engine_->move_step(source.set, source.action, source.step, to);
        }
        const QString targetActionId = makeId(Ref{target.set, target.action, -1});
        const int to = row < 0 ? stepCountIn(rows, targetActionId) : row;
        return engine_->move_step_to(source.set, source.action, source.step, target.set,
                                     target.action, to);
    }
    if (source.action >= 0) {
        if (target.set < 0) {
            return false;
        }
        const QString targetSetId = makeId(Ref{target.set, -1, -1});
        int to = row >= 0 ? row : actionCountIn(rows, targetSetId);
        if (target.set == source.set) {
            const int count = actionCountIn(rows, targetSetId);
            if (to > source.action) {
                --to;
            }
            to = qBound(0, to, count - 1);
            if (to == source.action) {
                return true;
            }
        }
        return engine_->move_action(source.set, source.action, target.set, to);
    }
    int setCount = 0;
    for (const ActionNodeRow& row : rows) {
        if (row.kind == actionKindSet()) {
            ++setCount;
        }
    }
    int to = row >= 0 ? row : setCount;
    if (to > source.set) {
        --to;
    }
    to = qBound(0, to, setCount - 1);
    if (to == source.set) {
        return true;
    }
    return engine_->move_set(source.set, to);
}

bool ActionsEngineBridge::setIncluded(const QString& id, bool included)
{
    if (!engine_) {
        return false;
    }
    const Ref ref = parseId(id);
    if (ref.set < 0) {
        return false;
    }
    if (ref.step >= 0) {
        return engine_->set_step_enabled(ref.set, ref.action, ref.step, included);
    }
    const QList<ActionNodeRow> rows = snapshot();
    for (const ActionNodeRow& row : rows) {
        if (row.kind != actionKindStep()) {
            continue;
        }
        const Ref step = parseId(row.id);
        const bool inTarget = ref.action >= 0
            ? (step.set == ref.set && step.action == ref.action)
            : step.set == ref.set;
        if (inTarget) {
            engine_->set_step_enabled(step.set, step.action, step.step, included);
        }
    }
    return true;
}

bool ActionsEngineBridge::toggleSiblingInclusion(const QString& id)
{
    if (!engine_) {
        return false;
    }
    const Ref clicked = parseId(id);
    if (clicked.step < 0) {
        return false;
    }
    bool included = true;
    const QList<ActionNodeRow> rows = snapshot();
    for (const ActionNodeRow& row : rows) {
        if (row.id == id) {
            included = row.enabled;
        }
    }
    for (const ActionNodeRow& row : rows) {
        if (row.kind != actionKindStep() || row.id == id) {
            continue;
        }
        const Ref sibling = parseId(row.id);
        if (sibling.set == clicked.set && sibling.action == clicked.action) {
            engine_->set_step_enabled(sibling.set, sibling.action, sibling.step, !included);
        }
    }
    return true;
}

bool ActionsEngineBridge::setModal(const QString& id, bool modal)
{
    if (!engine_) {
        return false;
    }
    const Ref ref = parseId(id);
    if (ref.set < 0) {
        return false;
    }
    if (ref.step >= 0) {
        return engine_->set_step_with_dialog(ref.set, ref.action, ref.step, modal);
    }
    const QList<ActionNodeRow> rows = snapshot();
    for (const ActionNodeRow& row : rows) {
        if (row.kind != actionKindStep()) {
            continue;
        }
        const Ref step = parseId(row.id);
        const bool inTarget = ref.action >= 0
            ? (step.set == ref.set && step.action == ref.action)
            : step.set == ref.set;
        if (inTarget) {
            engine_->set_step_with_dialog(step.set, step.action, step.step, modal);
        }
    }
    return true;
}

bool ActionsEngineBridge::setActionOptions(const QString& id, const QString& name,
                                           const QString& targetSetId, int functionKey,
                                           bool shift, bool ctrl, int color)
{
    const Ref action = parseId(id);
    const Ref target = parseId(targetSetId);
    if (!engine_ || action.action < 0) {
        return false;
    }
    int targetSet = action.set;
    int targetAction = action.action;
    if (target.set >= 0 && target.set != action.set) {
        const QString targetId = makeId(Ref{target.set, -1, -1});
        const int to = actionCountIn(snapshot(), targetId);
        if (!engine_->move_action(action.set, action.action, target.set, to)) {
            refreshSignals();
            return false;
        }
        targetSet = target.set;
        targetAction = to;
    }
    bool ok = engine_->rename_action(targetSet, targetAction, name);
    ok = engine_->set_action_color(targetSet, targetAction, color) && ok;
    if (functionKey != 0) {
        ok = engine_->assign_function_key(targetSet, targetAction, keyNumber(functionKey), shift,
                                          ctrl)
             && ok;
    }
    if (ok) {
        refusedError_.clear();
    }
    refreshSignals();
    return ok;
}

bool ActionsEngineBridge::setSetName(const QString& id, const QString& name)
{
    const Ref ref = parseId(id);
    return engine_ && ref.set >= 0 && engine_->rename_set(ref.set, name);
}

bool ActionsEngineBridge::assignFunctionKey(const QString& actionId, int functionKey, bool shift,
                                            bool ctrl, QString* error)
{
    const Ref action = parseId(actionId);
    if (!engine_ || action.action < 0) {
        return false;
    }
    const bool ok = functionKey == 0
        ? engine_->clear_function_key(action.set, action.action)
        : engine_->assign_function_key(action.set, action.action, keyNumber(functionKey), shift,
                                       ctrl);
    refreshSignals();
    if (!ok && error) {
        *error = engine_->last_error();
    }
    return ok;
}

bool ActionsEngineBridge::editStep(const QString& stepId, const QJsonObject& params)
{
    const Ref ref = parseId(stepId);
    if (!engine_ || ref.step < 0) {
        return false;
    }
    const QString payload =
        QString::fromUtf8(QJsonDocument(params).toJson(QJsonDocument::Compact));
    const bool edited = engine_->edit_step(ref.set, ref.action, ref.step, payload);
    refreshSignals();
    return edited;
}

bool ActionsEngineBridge::startRecording(const QString& actionId, int stepIndex)
{
    const Ref action = parseId(actionId);
    if (!engine_ || action.action < 0) {
        return false;
    }
    const int step = stepIndex >= 0 ? stepIndex : 0;
    const bool started =
        engine_->record_start(stepIndex >= 0 ? 3 : 2, action.set, action.action, step);
    if (started) {
        refusedError_.clear();
    }
    refreshSignals();
    return started;
}

void ActionsEngineBridge::stopRecording()
{
    refusedError_.clear();
    if (engine_) {
        engine_->record_stop();
    }
    refreshSignals();
}

bool ActionsEngineBridge::isRecording() const
{
    return engine_ && engine_->recording();
}

bool ActionsEngineBridge::recordAgain(const QString& actionId)
{
    const Ref action = parseId(actionId);
    if (!engine_ || action.action < 0) {
        return false;
    }
    recordAgainAction_ = action;
    recordAgainAction_.step = -1;
    recordAgainStep_ = 0;
    return true;
}

QString ActionsEngineBridge::recordAgainNext(QJsonObject* params)
{
    if (!engine_ || recordAgainAction_.action < 0) {
        return QString();
    }
    QList<ActionNodeRow> steps;
    const QString actionId = makeId(recordAgainAction_);
    for (const ActionNodeRow& row : snapshot()) {
        if (row.kind == actionKindStep() && row.parentId == actionId) {
            steps << row;
        }
    }
    if (recordAgainStep_ >= steps.size()) {
        recordAgainAction_ = Ref{};
        return QString();
    }
    const ActionNodeRow row = steps.at(recordAgainStep_++);
    if (params) {
        *params = row.params;
    }
    return row.id;
}

bool ActionsEngineBridge::recordToolStroke(const QString& toolId, const QJsonObject& params,
                                           int payloadBytes)
{
    Q_UNUSED(toolId)
    Q_UNUSED(params)
    Q_UNUSED(payloadBytes)
    refusedError_ = QStringLiteral(
        "recordToolStroke is a test seam; play back tool strokes through "
        "recordToolStrokePayload");
    refreshSignals();
    return false;
}

bool ActionsEngineBridge::recordToolStrokePayload(const QString& payload)
{
    const bool recorded = engine_ && engine_->record_tool_stroke(payload);
    if (recorded) {
        refusedError_.clear();
    }
    refreshSignals();
    return recorded;
}

void set_active_actions_engine(ActionsEngineBridge* bridge)
{
    g_activeActionsEngine = bridge;
}

void ActionsEngineBridge::recordCommand(const QString& commandId, const QJsonObject& params)
{
    if (engine_) {
        const bool recorded = engine_->record_command(
            commandId,
            QString::fromUtf8(QJsonDocument(params).toJson(QJsonDocument::Compact)));
        if (recorded) {
            refusedError_.clear();
        }
        refreshSignals();
    }
}

void ActionsEngineBridge::setAllowToolRecording(bool on)
{
    if (engine_) {
        engine_->set_allow_tool_recording(on);
    }
}

bool ActionsEngineBridge::allowToolRecording() const
{
    return engine_ && engine_->allow_tool_recording();
}

bool ActionsEngineBridge::startPlayback(const QStringList& stepIds)
{
    if (!engine_ || stepIds.isEmpty()) {
        return false;
    }
    QList<Ref> refs;
    for (const QString& id : stepIds) {
        const Ref ref = parseId(id);
        if (ref.step < 0) {
            return false;
        }
        refs << ref;
    }
    const Ref first = refs.first();
    bool sameSet = true;
    bool sameAction = true;
    for (const Ref& ref : refs) {
        sameSet = sameSet && ref.set == first.set;
        sameAction = sameAction && ref.action == first.action;
    }
    if (!sameSet) {
        return false;
    }
    const int scope = sameAction ? (refs.size() == 1 ? 4 : (first.step == 0 ? 2 : 3)) : 1;
    const bool started = engine_->play_start(scope, first.set, first.action, first.step);
    if (started) {
        refusedError_.clear();
    }
    refreshSignals();
    return started;
}

ActionNodeRow ActionsEngineBridge::playbackNext()
{
    ActionNodeRow row;
    if (!engine_) {
        row.stepKind = QStringLiteral("finished");
        return row;
    }
    const ScriptPlaybackRow source = engine_->play_next();
    const Ref ref{source.set, source.action, source.step};
    row.id = makeId(ref);
    row.commandId = source.command;
    row.modal = source.modal;
    row.message = source.message;
    row.allowContinue = source.allow_continue;
    row.path = source.path;
    const QString kind = source.kind;
    if (kind == QLatin1String("execute")) {
        row.stepKind = QStringLiteral("command");
    } else if (kind == QLatin1String("tool")) {
        row.stepKind = QStringLiteral("tool");
        row.commandId = QStringLiteral("pictura.toolStroke");
    } else if (kind == QLatin1String("stop")) {
        row.stepKind = QStringLiteral("stop");
        row.name = QStringLiteral("Stop");
    } else if (kind == QLatin1String("path")) {
        row.stepKind = QStringLiteral("path");
    } else if (kind == QLatin1String("waiting")) {
        row.stepKind = QStringLiteral("waiting");
    } else if (kind == QLatin1String("error")) {
        row.stepKind = QStringLiteral("error");
        row.message = source.error;
    } else {
        row.stepKind = QStringLiteral("finished");
        return row;
    }
    QJsonParseError error{};
    const QJsonDocument params =
        QJsonDocument::fromJson(source.params_json.toUtf8(), &error);
    if (error.error == QJsonParseError::NoError && params.isObject()) {
        row.params = params.object();
    }
    return row;
}

void ActionsEngineBridge::playbackContinue()
{
    if (engine_) {
        engine_->play_continue();
    }
}

void ActionsEngineBridge::cancelPlayback()
{
    if (engine_) {
        engine_->play_cancel();
    }
    refreshSignals();
}

bool ActionsEngineBridge::isPlaying() const
{
    return engine_ && engine_->playback_active();
}

QString ActionsEngineBridge::conditionalBranch(const QString& stepId)
{
    Q_UNUSED(stepId)
    return QString();
}

bool ActionsEngineBridge::applyPathStep(const QString& stepId)
{
    Q_UNUSED(stepId)
    // TODO(actions-bridge): route an Insert Path playback row to the path tool.
    return false;
}

bool ActionsEngineBridge::createStopStep(const QString& anchorId, const QString& message,
                                         bool allowContinue)
{
    const Ref anchor = parseId(anchorId);
    if (!engine_ || anchor.action < 0) {
        return false;
    }
    const int index = anchor.step >= 0 ? anchor.step + 1 : -1;
    const bool inserted =
        engine_->insert_stop(anchor.set, anchor.action, index, message, allowContinue) >= 0;
    refreshSignals();
    return inserted;
}

bool ActionsEngineBridge::createMenuItemStep(const QString& anchorId, const QString& commandId)
{
    const Ref anchor = parseId(anchorId);
    if (!engine_ || anchor.action < 0) {
        return false;
    }
    const int index = anchor.step >= 0 ? anchor.step + 1 : -1;
    const bool inserted = engine_->insert_menu_item(anchor.set, anchor.action, index, commandId,
                                                    QStringLiteral("{}"))
        >= 0;
    refreshSignals();
    return inserted;
}

bool ActionsEngineBridge::createPathStep(const QString& anchorId, const QString& path)
{
    const Ref anchor = parseId(anchorId);
    if (!engine_ || anchor.action < 0) {
        return false;
    }
    const int index = anchor.step >= 0 ? anchor.step + 1 : -1;
    const bool inserted = engine_->insert_path(anchor.set, anchor.action, index, path) >= 0;
    refreshSignals();
    return inserted;
}

bool ActionsEngineBridge::createConditionalStep(const QString& anchorId,
                                                const QString& condition,
                                                const QString& thenActionId,
                                                const QString& elseActionId, QString* error)
{
    const Ref anchor = parseId(anchorId);
    if (!engine_ || anchor.action < 0) {
        return false;
    }
    const QString thenName = actionNameFor(thenActionId);
    const QString elseName = actionNameFor(elseActionId);
    QString kind = condition;
    QString mode;
    const int colon = condition.indexOf(QLatin1Char(':'));
    if (colon >= 0) {
        kind = condition.left(colon);
        mode = condition.mid(colon + 1);
    }
    const int index = engine_->insert_conditional(anchor.set, anchor.action,
                                                  anchor.step >= 0 ? anchor.step + 1 : -1,
                                                  kind, mode, thenName, elseName);
    refreshSignals();
    if (index < 0) {
        if (error) {
            *error = engine_->last_error();
        }
        return false;
    }
    return true;
}

QStringList ActionsEngineBridge::actionIdsInSet(const QString& setId) const
{
    QStringList ids;
    for (const ActionNodeRow& row : snapshot()) {
        if (row.kind == actionKindAction() && row.parentId == setId) {
            ids << row.id;
        }
    }
    return ids;
}

bool ActionsEngineBridge::loadSetFile(const QString& path, bool replace)
{
    if (!engine_) {
        return false;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return false;
    }
    const bool loaded = engine_->load_palette_bytes(byteSlice(file.readAll()), path, replace);
    refreshSignals();
    return loaded;
}

bool ActionsEngineBridge::saveSetFile(const QString& setId, const QString& path)
{
    const Ref ref = parseId(setId);
    const bool saved = engine_ && ref.set >= 0 && engine_->save_set(path, ref.set);
    refreshSignals();
    return saved;
}

bool ActionsEngineBridge::writeTextDump(const QString& setId, const QString& path)
{
    const Ref ref = parseId(setId);
    if (ref.set < 0) {
        return false;
    }
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    QTextStream out(&file);
    out << "Kooka Pictura Actions\n";
    for (const ActionNodeRow& row : snapshot()) {
        const Ref node = parseId(row.id);
        if (node.set != ref.set) {
            continue;
        }
        out << (node.step >= 0 ? "    " : (node.action >= 0 ? "  " : "")) << row.name
            << '\n';
    }
    return true;
}

bool ActionsEngineBridge::loadLibrary()
{
    if (!engine_) {
        return false;
    }
    const QString path = palettePath();
    if (!QFileInfo::exists(path)) {
        return true;
    }
    const bool loaded = engine_->load_palette(path);
    refreshSignals();
    return loaded;
}

bool ActionsEngineBridge::saveLibrary()
{
    if (!engine_) {
        return false;
    }
    const QString path = palettePath();
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        return false;
    }
    const ::rust::Vec<uint8_t> bytes = engine_->palette_bytes();
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    file.write(reinterpret_cast<const char*>(bytes.data()), qint64(bytes.size()));
    return file.commit();
}

bool ActionsEngineBridge::reset(bool append)
{
    if (!engine_) {
        return false;
    }
    engine_->reset_actions(append);
    return true;
}

bool ActionsEngineBridge::clearAll()
{
    if (!engine_) {
        return false;
    }
    engine_->clear_actions();
    return true;
}

int ActionsEngineBridge::playbackMode() const
{
    return engine_ ? engine_->playback_speed() : 0;
}

double ActionsEngineBridge::playbackPauseSeconds() const
{
    return engine_ ? engine_->playback_pause_seconds() : 0.0;
}

void ActionsEngineBridge::setPlaybackOptions(int mode, double pauseSeconds)
{
    if (engine_) {
        engine_->set_playback_options(mode, pauseSeconds);
    }
}

void ActionsEngineBridge::setWindowsPlatform(bool windows)
{
    if (engine_) {
        engine_->set_windows_platform(windows);
    }
}

QString ActionsEngineBridge::actionNameFor(const QString& idOrName) const
{
    if (idOrName.isEmpty()) {
        return QString();
    }
    for (const ActionNodeRow& row : snapshot()) {
        if (row.kind == actionKindAction()
            && (row.id == idOrName || row.name == idOrName)) {
            return row.name;
        }
    }
    return QString();
}

QString ActionsEngineBridge::palettePath() const
{
    return QFileInfo(sessionFilePath()).absolutePath()
           + QStringLiteral("/Actions Palette.psp");
}

} // namespace pictura
