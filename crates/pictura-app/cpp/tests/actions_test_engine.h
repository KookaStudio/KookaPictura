#pragma once

#include "panels/actions_tree_model.h"

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QStringList>

using pictura::ActionNodeRow;

class FakeActionsEngine : public pictura::ActionsEngineApi {
public:
    explicit FakeActionsEngine(QObject* parent = nullptr)
        : ActionsEngineApi(parent)
    {
        resetFixture();
    }

    QList<ActionNodeRow> snapshot() const override { return rows_; }
    QString lastError() const override { return error_; }

    QString createSet(const QString& name) override
    {
        error_.clear();
        ActionNodeRow row;
        row.kind = pictura::actionKindSet();
        row.name = name;
        row.id = nextId();
        rows_ << row;
        emit changed();
        return row.id;
    }

    QString createAction(const QString& setId, const QString& name, int functionKey, bool shift,
                         bool ctrl, int color) override
    {
        error_.clear();
        ActionNodeRow row;
        row.kind = pictura::actionKindAction();
        row.parentId = setId;
        row.name = name;
        row.functionKey = functionKey;
        row.functionShift = shift;
        row.functionCtrl = ctrl;
        row.color = color;
        row.id = nextId();
        rows_ << row;
        emit changed();
        return row.id;
    }

    bool renameNode(const QString& id, const QString& name) override
    {
        ActionNodeRow* row = find(id);
        if (!row) {
            return fail(QStringLiteral("unknown node"));
        }
        row->name = name;
        emit changed();
        return true;
    }

    bool deleteNodes(const QStringList& ids) override
    {
        if (failDelete_) {
            return fail(QStringLiteral("delete refused"));
        }
        for (const QString& id : ids) {
            removeSubtree(id);
        }
        emit changed();
        return true;
    }

    bool duplicateNodes(const QStringList& ids) override
    {
        for (const QString& id : ids) {
            ActionNodeRow* row = find(id);
            if (row) {
                cloneNode(id, row->parentId);
            }
        }
        emit changed();
        return true;
    }

    bool moveNode(const QString& id, const QString& parentId, int row, bool copy) override
    {
        if (failMove_) {
            return fail(QStringLiteral("move refused"));
        }
        lastMoveParentId_ = parentId;
        lastMoveRow_ = row;
        ActionNodeRow* moving = find(id);
        if (!moving) {
            return fail(QStringLiteral("unknown node"));
        }
        if (copy) {
            cloneNode(id, parentId);
        } else {
            QList<ActionNodeRow> subtree;
            collectSubtree(id, subtree);
            for (const ActionNodeRow& entry : subtree) {
                removeNodeOnly(entry.id);
            }
            if (!subtree.isEmpty()) {
                subtree.first().parentId = parentId;
                rows_ << subtree;
            }
        }
        emit changed();
        return true;
    }

    bool setIncluded(const QString& id, bool included) override
    {
        ActionNodeRow* row = find(id);
        if (!row) {
            return false;
        }
        row->enabled = included;
        for (const ActionNodeRow& child : QList<ActionNodeRow>(rows_)) {
            if (isDescendant(child.id, id)) {
                if (ActionNodeRow* target = find(child.id)) {
                    target->enabled = included;
                }
            }
        }
        emit changed();
        return true;
    }

    bool toggleSiblingInclusion(const QString& id) override
    {
        ActionNodeRow* row = find(id);
        if (!row) {
            return false;
        }
        const bool siblingsIncluded = !row->enabled;
        const QString parentId = row->parentId;
        for (ActionNodeRow& sibling : rows_) {
            if (sibling.parentId == parentId && sibling.id != id
                && sibling.kind == pictura::actionKindStep()) {
                sibling.enabled = siblingsIncluded;
            }
        }
        emit changed();
        return true;
    }

    bool setModal(const QString& id, bool modal) override
    {
        ActionNodeRow* row = find(id);
        if (!row) {
            return false;
        }
        if (row->kind == pictura::actionKindStep()) {
            if (row->dialogOptions != 0 && row->dialogOptions != 4) {
                return false;
            }
            row->modal = modal;
        } else {
            for (ActionNodeRow& child : rows_) {
                if (isDescendant(child.id, id) && child.kind == pictura::actionKindStep()
                    && (child.dialogOptions == 0 || child.dialogOptions == 4)) {
                    child.modal = modal;
                }
            }
        }
        emit changed();
        return true;
    }

    bool setActionOptions(const QString& id, const QString& name, const QString& targetSetId,
                          int functionKey, bool shift, bool ctrl, int color) override
    {
        if (failOptions_) {
            return fail(QStringLiteral("options refused"));
        }
        ActionNodeRow* row = find(id);
        if (!row) {
            return false;
        }
        row->name = name;
        row->parentId = targetSetId;
        row->functionKey = functionKey;
        row->functionShift = shift;
        row->functionCtrl = ctrl;
        row->color = color;
        emit changed();
        return true;
    }

    bool setSetName(const QString& id, const QString& name) override
    {
        ActionNodeRow* row = find(id);
        if (!row) {
            return false;
        }
        row->name = name;
        emit changed();
        return true;
    }

    bool assignFunctionKey(const QString& actionId, int functionKey, bool shift, bool ctrl,
                           QString* error) override
    {
        if (functionKey != 0 && (functionKey < Qt::Key_F1 || functionKey > Qt::Key_F12)) {
            if (error) {
                *error = QStringLiteral("invalid function key");
            }
            return false;
        }
        for (const ActionNodeRow& row : rows_) {
            if (row.id != actionId && row.kind == pictura::actionKindAction()
                && row.functionKey == functionKey && row.functionShift == shift
                && row.functionCtrl == ctrl && functionKey != 0) {
                if (error) {
                    *error = QStringLiteral("function key already assigned");
                }
                return false;
            }
        }
        ActionNodeRow* row = find(actionId);
        if (!row) {
            return false;
        }
        row->functionKey = functionKey;
        row->functionShift = shift;
        row->functionCtrl = ctrl;
        emit changed();
        return true;
    }

    bool editStep(const QString& stepId, const QJsonObject& params) override
    {
        ActionNodeRow* row = find(stepId);
        if (!row) {
            return false;
        }
        row->params = params;
        emit changed();
        return true;
    }

    bool startRecording(const QString& actionId, int stepIndex) override
    {
        Q_UNUSED(stepIndex)
        if (recording_) {
            return false;
        }
        recording_ = true;
        recordingAction_ = actionId;
        emit recordingChanged(true);
        return true;
    }

    void stopRecording() override
    {
        if (!recording_) {
            return;
        }
        recording_ = false;
        recordingAction_.clear();
        emit recordingChanged(false);
    }

    bool isRecording() const override { return recording_; }

    bool recordAgain(const QString& actionId) override
    {
        recordAgainQueue_.clear();
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == actionId && row.kind == pictura::actionKindStep()) {
                recordAgainQueue_ << row.id;
            }
        }
        recordAgainIndex_ = 0;
        return !recordAgainQueue_.isEmpty();
    }

    QString recordAgainNext(QJsonObject* params) override
    {
        if (recordAgainIndex_ >= recordAgainQueue_.size()) {
            return QString();
        }
        const QString id = recordAgainQueue_.at(recordAgainIndex_++);
        if (params) {
            *params = find(id)->params;
        }
        return id;
    }

    bool recordToolStroke(const QString& toolId, const QJsonObject& params,
                          int payloadBytes) override
    {
        if (rejectTool_) {
            return fail(QStringLiteral("tool stroke refused"));
        }
        if (!allowToolRecording_) {
            return fail(QStringLiteral("tool recording is off"));
        }
        if (!recording_) {
            return fail(QStringLiteral("not recording"));
        }
        if (payloadBytes > 1000000) {
            return fail(QStringLiteral("tool stroke exceeds the payload bound"));
        }
        appendStep(recordingAction_, QStringLiteral("Tool Stroke"),
                   QStringLiteral("tool.") + toolId, params, QStringLiteral("command"));
        emit changed();
        return true;
    }

    void recordCommand(const QString& commandId, const QJsonObject& params) override
    {
        if (!recording_ || recordingAction_.isEmpty()) {
            return;
        }
        appendStep(recordingAction_, commandId, commandId, params, QStringLiteral("command"));
        emit changed();
    }

    void setAllowToolRecording(bool on) override { allowToolRecording_ = on; }
    bool allowToolRecording() const override { return allowToolRecording_; }

    bool startPlayback(const QStringList& stepIds) override
    {
        if (playing_) {
            return false;
        }
        playback_ = stepIds;
        playbackIndex_ = 0;
        playing_ = true;
        emit playbackChanged(true);
        return true;
    }

    void setDocumentFacts(bool present, bool square, const QString& mode,
                          bool hasSelection) override
    {
        factsPresent_ = present;
        factsSquare_ = square;
        factsMode_ = mode;
        factsHasSelection_ = hasSelection;
        ++factsCalls_;
    }

    bool factsPresent() const { return factsPresent_; }
    bool factsSquare() const { return factsSquare_; }
    QString factsMode() const { return factsMode_; }
    bool factsHasSelection() const { return factsHasSelection_; }
    int factsCalls() const { return factsCalls_; }
    void clearDocumentFacts()
    {
        factsPresent_ = false;
        factsSquare_ = false;
        factsMode_.clear();
        factsHasSelection_ = false;
        factsCalls_ = 0;
    }

    ActionNodeRow playbackNext() override
    {
        if (playbackIndex_ >= playback_.size()) {
            ActionNodeRow row;
            row.stepKind = QStringLiteral("finished");
            return row;
        }
        const ActionNodeRow* row = find(playback_.at(playbackIndex_++));
        if (!row) {
            ActionNodeRow finished;
            finished.stepKind = QStringLiteral("finished");
            return finished;
        }
        return *row;
    }

    void playbackContinue() override {}

    void cancelPlayback() override
    {
        if (!playing_) {
            return;
        }
        playing_ = false;
        playback_.clear();
        playbackIndex_ = 0;
        emit playbackChanged(false);
    }

    bool isPlaying() const override { return playing_; }

    QString conditionalBranch(const QString& stepId) override
    {
        const ActionNodeRow* row = find(stepId);
        if (!row) {
            return QString();
        }
        return conditionTrue_ ? row->thenActionId : row->elseActionId;
    }

    bool applyPathStep(const QString& stepId) override
    {
        const ActionNodeRow* row = find(stepId);
        if (!row) {
            return false;
        }
        appliedPath_ = row->path;
        return true;
    }

    bool createStopStep(const QString& anchorId, const QString& message,
                        bool allowContinue) override
    {
        lastInsertAnchor_ = anchorId;
        const QString actionId = actionForAnchor(anchorId);
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionId;
        row.name = QStringLiteral("Stop");
        row.message = message;
        row.allowContinue = allowContinue;
        row.stepKind = QStringLiteral("stop");
        appendStepRow(row);
        emit changed();
        return true;
    }

    bool createMenuItemStep(const QString& anchorId, const QString& commandId) override
    {
        lastInsertAnchor_ = anchorId;
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionForAnchor(anchorId);
        row.name = commandId;
        row.commandId = commandId;
        row.stepKind = QStringLiteral("command");
        row.dialogOptions = 3;
        appendStepRow(row);
        emit changed();
        return true;
    }

    bool createPathStep(const QString& anchorId, const QString& path) override
    {
        lastInsertAnchor_ = anchorId;
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionForAnchor(anchorId);
        row.name = QStringLiteral("Insert Path");
        row.path = path;
        row.stepKind = QStringLiteral("path");
        appendStepRow(row);
        emit changed();
        return true;
    }

    bool createConditionalStep(const QString& anchorId, const QString& condition,
                               const QString& thenActionId, const QString& elseActionId,
                               QString* error) override
    {
        lastInsertAnchor_ = anchorId;
        const QString actionId = actionForAnchor(anchorId);
        if (thenActionId.isEmpty() && elseActionId.isEmpty()) {
            if (error) {
                *error = QStringLiteral("choose a branch");
            }
            return false;
        }
        const QString setId = setForAction(actionId);
        for (const QString& branch : {thenActionId, elseActionId}) {
            if (!branch.isEmpty() && setForAction(branch) != setId) {
                if (error) {
                    *error = QStringLiteral("branch action is outside the set");
                }
                return false;
            }
        }
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionId;
        row.name = QStringLiteral("Conditional");
        row.condition = condition;
        row.thenActionId = thenActionId;
        row.elseActionId = elseActionId;
        row.stepKind = QStringLiteral("conditional");
        appendStepRow(row);
        emit changed();
        return true;
    }

    QStringList actionIdsInSet(const QString& setId) const override
    {
        QStringList ids;
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == setId && row.kind == pictura::actionKindAction()) {
                ids << row.id;
            }
        }
        return ids;
    }

    bool loadSetFile(const QString& path, bool replace) override
    {
        if (!path.endsWith(QStringLiteral(".atn"))) {
            return fail(QStringLiteral("%1 is not an action set").arg(path));
        }
        if (replace) {
            rows_.clear();
        }
        if (savedSets_.contains(path)) {
            rows_ << savedSets_.value(path);
        } else {
            ActionNodeRow set;
            set.kind = pictura::actionKindSet();
            set.name = QFileInfo(path).completeBaseName();
            set.id = nextId();
            rows_ << set;
        }
        loadedPath_ = path;
        if (path.contains(QStringLiteral("partial"))) {
            error_ = QStringLiteral("parse error at offset 12");
            emit error(error_);
        }
        emit changed();
        return true;
    }

    bool saveSetFile(const QString& setId, const QString& path) override
    {
        QList<ActionNodeRow> set;
        collectSubtree(setId, set);
        savedSets_.insert(path, set);
        QFile file(path);
        if (file.open(QIODevice::WriteOnly)) {
            file.write("atn");
        }
        return true;
    }

    bool writeTextDump(const QString& setId, const QString& path) override
    {
        Q_UNUSED(setId)
        QFile file(path);
        if (!file.open(QIODevice::WriteOnly)) {
            return false;
        }
        file.write("Kooka Actions\n");
        return true;
    }

    bool loadLibrary() override
    {
        rows_ = savedLibrary_;
        emit changed();
        return true;
    }

    bool saveLibrary() override
    {
        savedLibrary_ = rows_;
        return true;
    }

    bool reset(bool append) override
    {
        if (!append) {
            rows_.clear();
        }
        bool exists = false;
        for (const ActionNodeRow& row : rows_) {
            if (row.kind == pictura::actionKindSet()
                && row.name == QStringLiteral("Default Actions")) {
                exists = true;
            }
        }
        if (!exists) {
            ActionNodeRow set;
            set.kind = pictura::actionKindSet();
            set.name = QStringLiteral("Default Actions");
            set.id = nextId();
            rows_ << set;
        }
        emit changed();
        return true;
    }

    bool clearAll() override
    {
        rows_.clear();
        emit changed();
        return true;
    }

    int playbackMode() const override { return playbackMode_; }
    double playbackPauseSeconds() const override { return playbackPause_; }

    void setPlaybackOptions(int mode, double pauseSeconds) override
    {
        playbackMode_ = mode;
        playbackPause_ = pauseSeconds;
    }

    void setWindowsPlatform(bool windows) override { windowsPlatform_ = windows; }

    void resetFixture()
    {
        rows_.clear();
        savedSets_.clear();
        savedLibrary_.clear();
        error_.clear();
        nextId_ = 1;
        recording_ = false;
        playing_ = false;
        allowToolRecording_ = false;
        playbackMode_ = 0;
        playbackPause_ = 0.0;
        appliedPath_.clear();
        loadedPath_.clear();
        conditionTrue_ = true;
        failDelete_ = false;
        failMove_ = false;
        failOptions_ = false;
        rejectTool_ = false;
        lastMoveParentId_.clear();
        lastMoveRow_ = -1;
        lastInsertAnchor_.clear();
        clearDocumentFacts();
        const QString setA = addSet(QStringLiteral("Set A"));
        const QString actionOne = addAction(setA, QStringLiteral("Action One"), 0, false, false, 4);
        appendStep(actionOne, QStringLiteral("Step One"), QStringLiteral("edit.fill"),
                   QJsonObject{{QStringLiteral("value"), 1}}, QStringLiteral("command"));
        appendStep(actionOne, QStringLiteral("Step Two"),
                   QStringLiteral("layer.flatten.image"), QJsonObject(),
                   QStringLiteral("command"));
        const QString actionTwo = addAction(setA, QStringLiteral("Action Two"), 0, false, false, 5);
        appendStep(actionTwo, QStringLiteral("Step Three"),
                   QStringLiteral("image.rotate90cw"), QJsonObject(),
                   QStringLiteral("command"));
        const QString setB = addSet(QStringLiteral("Set B"));
        const QString actionThree =
            addAction(setB, QStringLiteral("Action Three"), 0, false, false, 6);
        appendStep(actionThree, QStringLiteral("Step Four"), QStringLiteral("image.rotate180"),
                   QJsonObject(), QStringLiteral("command"));
        emit changed();
    }

    QString idForName(const QString& name) const
    {
        for (const ActionNodeRow& row : rows_) {
            if (row.name == name) {
                return row.id;
            }
        }
        return QString();
    }

    QStringList stepIdsUnder(const QString& actionName) const
    {
        QStringList ids;
        const QString actionId = idForName(actionName);
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == actionId && row.kind == pictura::actionKindStep()) {
                ids << row.id;
            }
        }
        return ids;
    }

    void setDialogOptions(const QString& stepName, int options, bool modal)
    {
        if (ActionNodeRow* row = find(idForName(stepName))) {
            row->dialogOptions = options;
            row->modal = modal;
        }
        emit changed();
    }

    void setStepParams(const QString& stepName, const QJsonObject& params)
    {
        if (ActionNodeRow* row = find(idForName(stepName))) {
            row->params = params;
        }
        emit changed();
    }

    void setConditionResult(bool value) { conditionTrue_ = value; }
    QString appliedPath() const { return appliedPath_; }
    QString loadedPath() const { return loadedPath_; }
    void setFailDelete(bool fail) { failDelete_ = fail; }
    void setFailMove(bool fail) { failMove_ = fail; }
    void setFailOptions(bool fail) { failOptions_ = fail; }
    void setRejectToolStroke(bool reject) { rejectTool_ = reject; }
    QString lastMoveParentId() const { return lastMoveParentId_; }
    int lastMoveRow() const { return lastMoveRow_; }
    QString lastInsertAnchor() const { return lastInsertAnchor_; }
    ActionNodeRow rowForId(const QString& id) const
    {
        const ActionNodeRow* row = find(id);
        return row ? *row : ActionNodeRow{};
    }
    QString appendStepForTest(const QString& actionId, const QString& name,
                              const QString& commandId, const QJsonObject& params)
    {
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionId;
        row.name = name;
        row.commandId = commandId;
        row.params = params;
        row.stepKind = QStringLiteral("command");
        const QString id = appendStepRow(row);
        emit changed();
        return id;
    }
    QString appendToolStepForTest(const QString& actionId, const QJsonObject& params)
    {
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionId;
        row.name = QStringLiteral("Tool Stroke");
        row.commandId = QStringLiteral("pictura.toolStroke");
        row.params = params;
        row.stepKind = QStringLiteral("tool");
        const QString id = appendStepRow(row);
        emit changed();
        return id;
    }
    void clearForLoadTest()
    {
        rows_.clear();
        emit changed();
    }
    int stepCount() const
    {
        int count = 0;
        for (const ActionNodeRow& row : rows_) {
            count += row.kind == pictura::actionKindStep() ? 1 : 0;
        }
        return count;
    }

private:
    ActionNodeRow* find(const QString& id)
    {
        for (ActionNodeRow& row : rows_) {
            if (row.id == id) {
                return &row;
            }
        }
        return nullptr;
    }
    const ActionNodeRow* find(const QString& id) const
    {
        for (const ActionNodeRow& row : rows_) {
            if (row.id == id) {
                return &row;
            }
        }
        return nullptr;
    }
    QString nextId() { return QStringLiteral("n%1").arg(nextId_++); }
    bool fail(const QString& message)
    {
        error_ = message;
        emit error(message);
        return false;
    }
    QString addSet(const QString& name)
    {
        ActionNodeRow row;
        row.kind = pictura::actionKindSet();
        row.name = name;
        row.id = nextId();
        rows_ << row;
        return row.id;
    }
    QString addAction(const QString& setId, const QString& name, int functionKey, bool shift,
                      bool ctrl, int color)
    {
        ActionNodeRow row;
        row.kind = pictura::actionKindAction();
        row.parentId = setId;
        row.name = name;
        row.functionKey = functionKey;
        row.functionShift = shift;
        row.functionCtrl = ctrl;
        row.color = color;
        row.id = nextId();
        rows_ << row;
        return row.id;
    }
    void appendStep(const QString& actionId, const QString& name, const QString& commandId,
                    const QJsonObject& params, const QString& stepKind)
    {
        ActionNodeRow row;
        row.kind = pictura::actionKindStep();
        row.parentId = actionId;
        row.name = name;
        row.commandId = commandId;
        row.params = params;
        row.stepKind = stepKind;
        appendStepRow(row);
    }
    QString appendStepRow(ActionNodeRow row)
    {
        int index = 0;
        for (const ActionNodeRow& existing : rows_) {
            if (existing.parentId == row.parentId
                && existing.kind == pictura::actionKindStep()) {
                ++index;
            }
        }
        row.stepIndex = index;
        row.id = nextId();
        rows_ << row;
        return row.id;
    }
    bool isDescendant(const QString& id, const QString& ancestor) const
    {
        const ActionNodeRow* row = find(id);
        while (row && !row->parentId.isEmpty()) {
            if (row->parentId == ancestor) {
                return true;
            }
            row = find(row->parentId);
        }
        return false;
    }
    void collectSubtree(const QString& id, QList<ActionNodeRow>& out) const
    {
        for (const ActionNodeRow& row : rows_) {
            if (row.id == id) {
                out << row;
            }
        }
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == id) {
                collectSubtree(row.id, out);
            }
        }
    }
    void removeNodeOnly(const QString& id)
    {
        for (int i = rows_.size() - 1; i >= 0; --i) {
            if (rows_.at(i).id == id) {
                rows_.removeAt(i);
            }
        }
    }
    void removeSubtree(const QString& id)
    {
        QStringList children;
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == id) {
                children << row.id;
            }
        }
        for (const QString& child : children) {
            removeSubtree(child);
        }
        removeNodeOnly(id);
    }
    void cloneNode(const QString& id, const QString& parentId)
    {
        const ActionNodeRow* source = find(id);
        if (!source) {
            return;
        }
        ActionNodeRow copy = *source;
        copy.id = nextId();
        copy.parentId = parentId;
        rows_ << copy;
        QStringList children;
        for (const ActionNodeRow& row : rows_) {
            if (row.parentId == id && row.id != copy.id) {
                children << row.id;
            }
        }
        for (const QString& child : children) {
            cloneNode(child, copy.id);
        }
    }
    QString actionForAnchor(const QString& anchorId) const
    {
        const ActionNodeRow* row = find(anchorId);
        if (!row) {
            return QString();
        }
        if (row->kind == pictura::actionKindAction()) {
            return row->id;
        }
        if (row->kind == pictura::actionKindStep()) {
            return row->parentId;
        }
        return QString();
    }
    QString setForAction(const QString& actionId) const
    {
        const ActionNodeRow* row = find(actionId);
        return row ? row->parentId : QString();
    }

    QList<ActionNodeRow> rows_;
    QHash<QString, QList<ActionNodeRow>> savedSets_;
    QList<ActionNodeRow> savedLibrary_;
    QString error_;
    QString loadedPath_;
    QString appliedPath_;
    QString recordingAction_;
    QStringList recordAgainQueue_;
    int recordAgainIndex_ = 0;
    int nextId_ = 1;
    int playbackMode_ = 0;
    double playbackPause_ = 0.0;
    int playbackIndex_ = 0;
    bool recording_ = false;
    bool playing_ = false;
    bool allowToolRecording_ = false;
    bool conditionTrue_ = true;
    bool windowsPlatform_ = false;
    bool failDelete_ = false;
    bool failMove_ = false;
    bool failOptions_ = false;
    bool rejectTool_ = false;
    bool factsPresent_ = false;
    bool factsSquare_ = false;
    QString factsMode_;
    bool factsHasSelection_ = false;
    int factsCalls_ = 0;
    QString lastMoveParentId_;
    int lastMoveRow_ = -1;
    QString lastInsertAnchor_;
    QStringList playback_;
};


