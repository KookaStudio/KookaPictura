#include "actions_panel.h"
#include "actions_panel_internal.h"

#include "actions_button_view.h"
#include "actions_dialogs.h"
#include "commands.h"
#include "icons.h"
#include "session.h"

#include <QtCore/QDataStream>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QJsonValue>
#include <QtCore/QMimeData>
#include <QtCore/QScopeGuard>
#include <QtCore/QStandardPaths>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QShortcut>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QStyle>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>


namespace pictura {

using namespace actions_internal;

bool ActionsPanel::playSteps(const QStringList& stepIds)
{
    if (!engine_ || playing_ || engine_->isRecording() || stepIds.isEmpty()) {
        return stepIds.isEmpty() && engine_ != nullptr;
    }
    if (engine_->isPlaying()) {
        return false;
    }
    updateDocumentFacts();
    if (!engine_->startPlayback(stepIds)) {
        reportError(engine_->lastError());
        return false;
    }
    playing_ = true;
    playQueue_ = stepIds;
    playIndex_ = 0;
    playDepth_ = 0;
    lastError_.clear();
    updateToolbar();
    schedulePlayback();
    return true;
}

bool ActionsPanel::playSelection(bool singleStep)
{
    if (!engine_ || engine_->isRecording() || playing_ || engine_->isPlaying()) {
        return false;
    }
    const QStringList ids = selectedNodeIds();
    if (ids.isEmpty()) {
        return false;
    }
    bool hasSteps = false;
    QString error;
    const QStringList queue = buildPlayQueue(ids, singleStep, &hasSteps, &error);
    if (!error.isEmpty()) {
        reportError(error);
        return false;
    }
    if (!hasSteps) {
        return true;
    }
    return playSteps(queue);
}

bool ActionsPanel::playAction(const QString& actionId, const QString& fromStepId,
                              bool singleStep)
{
    if (!engine_ || engine_->isRecording() || playing_ || engine_->isPlaying()) {
        return false;
    }
    QStringList steps = includedStepsForAction(actionId);
    int start = fromStepId.isEmpty() ? 0 : steps.indexOf(fromStepId);
    if (!fromStepId.isEmpty() && start < 0) {
        const QStringList all = model_->stepIdsForAction(actionId);
        const int position = all.indexOf(fromStepId);
        for (int i = position + 1; i >= 0 && i < all.size(); ++i) {
            const int candidate = steps.indexOf(all.at(i));
            if (candidate >= 0) {
                start = candidate;
                break;
            }
        }
    }
    if (start < 0) {
        start = 0;
    }
    steps = steps.mid(start);
    if (singleStep && !steps.isEmpty()) {
        steps = steps.mid(0, 1);
    }
    if (steps.isEmpty()) {
        return true;
    }
    return playSteps(steps);
}

bool ActionsPanel::playActionForTest(const QString& actionId)
{
    return playAction(actionId, QString(), false);
}

void ActionsPanel::schedulePlayback()
{
    if (!playing_ || !engine_) {
        return;
    }
    int interval = 0;
    if (engine_->playbackMode() == 1) {
        interval = 300;
    } else if (engine_->playbackMode() == 2) {
        interval = qMax(1, qRound(engine_->playbackPauseSeconds() * 1000.0));
    }
    playbackTimer_->start(interval);
}

bool ActionsPanel::effectiveModal(const ActionNodeRow& step)
{
    if (step.dialogOptions == 1) {
        return true;
    }
    if (step.dialogOptions == 2) {
        return false;
    }
    if (step.dialogOptions == 3) {
        return true;
    }
    return step.modal;
}

bool ActionsPanel::executeStep(const ActionNodeRow& step, QJsonObject* params, bool modal)
{
    if (!step.commandId.isEmpty()) {
        if (stepExecutor_) {
            return stepExecutor_(step, params, modal);
        }
        if (registry_) {
            CommandParams commandParams;
            commandParams.values = params ? *params : step.params;
            const bool ok = registry_->executeRecorded(
                step.commandId, &commandParams,
                modal ? CommandMode::Modal : CommandMode::Replay);
            if (ok && params) {
                *params = commandParams.values;
            }
            return ok;
        }
    }
    return false;
}

void ActionsPanel::advancePlayback()
{
    if (!playing_ || !engine_) {
        return;
    }
    advancingPlayback_ = true;
    const auto done = qScopeGuard([this] { advancingPlayback_ = false; });
    const ActionNodeRow step = engine_->playbackNext();
    if (step.stepKind == QLatin1String("finished")
        || (step.id.isEmpty() && step.commandId.isEmpty() && step.stepKind.isEmpty())) {
        finishPlayback();
        return;
    }
    if (step.stepKind == QLatin1String("error")) {
        reportError(step.message.isEmpty() ? tr("Playback failed.") : step.message);
        finishPlayback();
        return;
    }
    if (!step.id.isEmpty()) {
        const QModelIndex current = model_->indexForId(step.id);
        if (current.isValid()) {
            tree_->setCurrentIndex(current);
        }
    }
    bool ok = true;
    if (step.stepKind == QLatin1String("stop") && batchPlayback_) {
        if (!step.allowContinue) {
            reportError(step.message.isEmpty() ? tr("Stop step.") : step.message);
            finishPlayback();
            return;
        }
        engine_->playbackContinue();
    } else if (step.stepKind == QLatin1String("stop")) {
        if (stopHandler_) {
            ok = stopHandler_(step.message, step.allowContinue);
        } else {
            QMessageBox::information(this, tr("Stop"), step.message);
            ok = step.allowContinue;
        }
        if (ok && step.allowContinue) {
            engine_->playbackContinue();
        } else if (!ok) {
            finishPlayback();
            return;
        }
    } else if (step.stepKind == QLatin1String("waiting")) {
        return;
    } else if (step.stepKind == QLatin1String("path")) {
        ok = pathStepExecutor_ ? pathStepExecutor_(step.path)
                               : engine_->applyPathStep(step.id);
    } else if (step.stepKind == QLatin1String("menuItem")) {
        ok = registry_ && registry_->dispatch(step.commandId);
    } else if (step.stepKind == QLatin1String("tool")) {
        if (!toolStepExecutor_) {
            reportError(tr("Tool step %1 has no executor.").arg(step.id));
            finishPlayback();
            return;
        }
        ok = toolStepExecutor_(step.params);
    } else if (step.stepKind == QLatin1String("conditional")) {
        const QString branch = engine_->conditionalBranch(step.id);
        if (branch.isEmpty()) {
            reportError(tr("Conditional step %1 has no branch.").arg(step.name));
            finishPlayback();
            return;
        }
    } else if (batchPlayback_ && batch_.overrideOpen
               && step.commandId == QLatin1String("file.open")) {
        ok = true;
    } else if (!step.commandId.isEmpty()) {
        QJsonObject params = step.params;
        const bool modal = !batchPlayback_ && effectiveModal(step);
        ok = executeStep(step, &params, modal);
        if (ok && modal && params != step.params) {
            engine_->editStep(step.id, params);
        }
    }
    if (!ok) {
        const QString detail = registry_ ? registry_->lastError() : QString();
        reportError(detail.isEmpty()
                        ? tr("Step %1 (%2) failed.").arg(step.name, step.commandId)
                        : tr("Step %1 (%2) failed: %3").arg(step.name, step.commandId, detail));
        finishPlayback();
        return;
    }
    schedulePlayback();
}

void ActionsPanel::advancePlaybackForTest()
{
    advancePlayback();
}

void ActionsPanel::finishPlayback()
{
    if (!playing_) {
        return;
    }
    playing_ = false;
    batchPlayback_ = false;
    playbackTimer_->stop();
    playQueue_.clear();
    playIndex_ = 0;
    playDepth_ = 0;
    if (engine_ && engine_->isPlaying()) {
        engine_->cancelPlayback();
    }
    updateToolbar();
    emit changed();
    emit playbackFinished(lastError_.isEmpty(), lastError_);
}

void ActionsPanel::cancelPlayback()
{
    finishPlayback();
}

QString ActionsPanel::actionIdFor(const QString& setName, const QString& actionName) const
{
    if (!engine_) {
        return {};
    }
    const QList<ActionNodeRow> rows = engine_->snapshot();
    QString setId;
    for (const ActionNodeRow& row : rows) {
        if (row.kind == actionKindSet() && row.name == setName) {
            setId = row.id;
            break;
        }
    }
    for (const ActionNodeRow& row : rows) {
        if (!setId.isEmpty() && row.kind == actionKindAction() && row.parentId == setId
            && row.name == actionName) {
            return row.id;
        }
    }
    return {};
}

QList<QPair<QString, QStringList>> ActionsPanel::actionLibraryNames() const
{
    QList<QPair<QString, QStringList>> out;
    if (!engine_) {
        return out;
    }
    const QList<ActionNodeRow> rows = engine_->snapshot();
    for (const ActionNodeRow& set : rows) {
        if (set.kind != actionKindSet()) {
            continue;
        }
        QStringList actions;
        for (const ActionNodeRow& row : rows) {
            if (row.kind == actionKindAction() && row.parentId == set.id) {
                actions << row.name;
            }
        }
        out << qMakePair(set.name, actions);
    }
    return out;
}

QPair<QString, QString> ActionsPanel::selectedActionNames() const
{
    if (!engine_) {
        return {};
    }
    QHash<QString, ActionNodeRow> byId;
    for (const ActionNodeRow& row : engine_->snapshot()) {
        byId.insert(row.id, row);
    }
    for (const QString& id : selectedNodeIds()) {
        ActionNodeRow row = byId.value(id);
        while (!row.id.isEmpty()
               && (row.kind == actionKindStep() || row.kind == actionKindParam())) {
            row = byId.value(row.parentId);
        }
        if (row.kind == actionKindAction()) {
            return qMakePair(byId.value(row.parentId).name, row.name);
        }
        if (row.kind == actionKindSet()) {
            return qMakePair(row.name, QString());
        }
    }
    return {};
}

QString ActionsPanel::validateBatch(const QJsonObject& options) const
{
    if (!engine_) {
        return tr("The action engine is not available.");
    }
    return engine_->validateBatch(
        QString::fromUtf8(QJsonDocument(options).toJson(QJsonDocument::Compact)));
}

bool ActionsPanel::actionReachesCommand(const QString& set, const QString& action,
                                        const QString& command) const
{
    return engine_ && engine_->actionReachesCommand(set, action, command);
}

bool ActionsPanel::playActionForBatch(const QString& setName, const QString& actionName,
                                      const BatchPlayback& batch)
{
    const QString actionId = actionIdFor(setName, actionName);
    if (actionId.isEmpty()) {
        lastError_ = tr("The action \u201c%1\u201d is not loaded.").arg(actionName);
        return false;
    }
    lastError_.clear();
    batchPlayback_ = true;
    batch_ = batch;
    if (!playAction(actionId, QString(), false)) {
        batchPlayback_ = false;
        if (lastError_.isEmpty()) {
            lastError_ = tr("The action \u201c%1\u201d could not start.").arg(actionName);
        }
        return false;
    }
    if (!playing_) {
        // An action with no included steps finishes at once.
        batchPlayback_ = false;
        QTimer::singleShot(0, this, [this] { emit playbackFinished(true, QString()); });
    }
    return true;
}

void ActionsPanel::cancelPlaybackForTest()
{
    finishPlayback();
}

void ActionsPanel::setButtonMode(bool on)
{
    setButtonModeInternal(on);
}

void ActionsPanel::setButtonModeInternal(bool on)
{
    const bool modeChanged = buttonMode_ != on;
    buttonMode_ = on;
    stack_->setCurrentWidget(on ? static_cast<QWidget*>(buttonView_)
                                : static_cast<QWidget*>(tree_));
    if (!modeChanged) {
        return;
    }
    if (on) {
        updateButtonModel();
    }
    updateToolbar();
    schedulePersist();
    emit changed();
}

void ActionsPanel::updateDocumentFacts()
{
    if (!engine_) {
        return;
    }
    DocumentFacts facts;
    if (documentFactsProvider_) {
        facts = documentFactsProvider_();
    }
    engine_->setDocumentFacts(facts.present, facts.square, facts.mode, facts.hasSelection);
}

void ActionsPanel::setAllowToolRecording(bool on)
{
    allowToolRecording_ = on;
    if (engine_) {
        engine_->setAllowToolRecording(on);
    }
    schedulePersist();
    emit changed();
}


} // namespace pictura
