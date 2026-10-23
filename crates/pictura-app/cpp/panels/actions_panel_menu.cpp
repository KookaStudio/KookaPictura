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
#include <QtCore/QItemSelectionModel>
#include <QtCore/QJsonValue>
#include <QtCore/QMimeData>
#include <QtCore/QStandardPaths>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QShortcut>
#include <QtWidgets/QApplication>
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

bool ActionsPanel::createSetForTest(const QString& name)
{
    if (!engine_ || name.trimmed().isEmpty()) {
        return false;
    }
    const QString id = engine_->createSet(name.trimmed());
    if (id.isEmpty()) {
        reportError(engine_->lastError());
        return false;
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::renameSetForTest(const QString& setId, const QString& name)
{
    if (!engine_ || setId.isEmpty() || name.trimmed().isEmpty()) {
        return false;
    }
    const bool ok = engine_->setSetName(setId, name.trimmed());
    if (!ok) {
        reportError(engine_->lastError());
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return ok;
}

bool ActionsPanel::createActionForTest(const QString& name, const QString& setId, int functionKey,
                                       bool shift, bool ctrl, int color, bool record)
{
    if (!engine_ || setId.isEmpty() || name.trimmed().isEmpty()) {
        return false;
    }
    if (functionKey != 0 && windowsPlatform_
        && windowsKeyRefused(functionKey, (ctrl ? Qt::ControlModifier : 0)
                                            | (shift ? Qt::ShiftModifier : 0),
                             true)) {
        reportError(tr("That function key is reserved on Windows."));
        return false;
    }
    const QString id = engine_->createAction(setId, name.trimmed(), 0, false, false, color);
    if (id.isEmpty()) {
        reportError(engine_->lastError());
        return false;
    }
    if (functionKey != 0) {
        QString error;
        if (!applyFunctionKey(id, functionKey, shift, ctrl, &error)) {
            reportError(error);
        }
    }
    if (record) {
        startRecordingOnAction(id, -1);
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::applyActionOptionsForTest(const QString& actionId, const QString& name,
                                             const QString& setId, int functionKey, bool shift,
                                             bool ctrl, int color)
{
    if (!engine_ || actionId.isEmpty() || setId.isEmpty() || name.trimmed().isEmpty()) {
        return false;
    }
    const int previousSequence = actionKeySequence(actionId);
    if (!engine_->setActionOptions(actionId, name.trimmed(), setId, 0, false, false, color)) {
        const QString error = engine_->lastError();
        reportError(error.isEmpty() ? tr("The action could not be updated.") : error);
        return false;
    }
    QString resolvedId;
    QString fallbackId;
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.kind != actionKindAction() || row.parentId != setId
            || row.name != name.trimmed()) {
            continue;
        }
        if (row.id == actionId) {
            resolvedId = row.id;
        } else if (fallbackId.isEmpty()) {
            fallbackId = row.id;
        }
    }
    if (resolvedId.isEmpty()) {
        resolvedId = fallbackId;
    }
    if (resolvedId.isEmpty()) {
        reportError(tr("The action could not be moved to the selected set."));
        return false;
    }
    if (functionKey != 0) {
        QString error;
        if (!applyFunctionKey(resolvedId, functionKey, shift, ctrl, &error, previousSequence)) {
            restoreShadowedSequence(previousSequence);
            reportError(error);
            return false;
        }
    } else {
        restoreShadowedSequence(previousSequence);
        QString error;
        if (!engine_->assignFunctionKey(resolvedId, 0, false, false, &error)) {
            const QString message = error.isEmpty() ? engine_->lastError() : error;
            reportError(message.isEmpty() ? tr("The function key could not be cleared.")
                                          : message);
            return false;
        }
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::setPlaybackOptionsForTest(int mode, double pauseSeconds)
{
    if (!engine_) {
        return false;
    }
    engine_->setPlaybackOptions(mode, qMax(0.0, pauseSeconds));
    schedulePersist();
    return true;
}

bool ActionsPanel::insertStopForTest(const QString& message, bool allowContinue)
{
    if (!engine_) {
        return false;
    }
    const QString anchor = !selectedStepId().isEmpty() ? selectedStepId() : selectedActionId();
    if (anchor.isEmpty() || !engine_->createStopStep(anchor, message, allowContinue)) {
        reportError(engine_->lastError());
        return false;
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::insertMenuItemForTest(const QString& commandId)
{
    if (!engine_ || commandId.isEmpty()) {
        return false;
    }
    const QString anchor = !selectedStepId().isEmpty() ? selectedStepId() : selectedActionId();
    if (anchor.isEmpty() || !engine_->createMenuItemStep(anchor, commandId)) {
        reportError(engine_->lastError());
        return false;
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::insertPathForTest(const QString& path)
{
    if (!engine_ || path.isEmpty()) {
        return false;
    }
    const QString anchor = !selectedStepId().isEmpty() ? selectedStepId() : selectedActionId();
    if (anchor.isEmpty() || !engine_->createPathStep(anchor, path)) {
        reportError(engine_->lastError());
        return false;
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::insertConditionalForTest(const QString& condition,
                                            const QString& thenActionId,
                                            const QString& elseActionId, QString* error)
{
    if (!engine_) {
        return false;
    }
    if (thenActionId.isEmpty() && elseActionId.isEmpty()) {
        if (error) {
            *error = tr("Choose a Then or Else action.");
        }
        return false;
    }
    const QString anchor = !selectedStepId().isEmpty() ? selectedStepId() : selectedActionId();
    if (anchor.isEmpty()) {
        if (error) {
            *error = tr("Select an action or step first.");
        }
        return false;
    }
    if (!engine_->createConditionalStep(anchor, condition, thenActionId, elseActionId, error)) {
        if (error && error->isEmpty()) {
            *error = engine_->lastError();
        }
        reportError(error ? *error : engine_->lastError());
        return false;
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return true;
}

bool ActionsPanel::deleteSelectionForTest(bool skipConfirmation)
{
    if (!engine_) {
        return false;
    }
    const QStringList ids = selectedNodeIds();
    if (ids.isEmpty()) {
        return false;
    }
    if (!skipConfirmation
        && !confirm(tr("Delete"), tr("Delete the selected action items?"))) {
        return false;
    }
    const bool ok = engine_->deleteNodes(ids);
    if (!ok) {
        const QString error = engine_->lastError();
        reportError(error.isEmpty() ? tr("The selected items could not be deleted.") : error);
    }
    return ok;
}

bool ActionsPanel::moveNodeForTest(const QString& id, const QString& parentId, int row, bool copy)
{
    if (!engine_) {
        return false;
    }
    const bool ok = engine_->moveNode(id, parentId, row, copy);
    if (!ok) {
        const QString error = engine_->lastError();
        reportError(error.isEmpty() ? tr("The item could not be moved.") : error);
    }
    return ok;
}

bool ActionsPanel::saveSetForTest(const QString& setId, const QString& path, bool textDump)
{
    if (!engine_ || setId.isEmpty() || path.isEmpty()) {
        return false;
    }
    const bool ok = textDump ? engine_->writeTextDump(setId, path)
                             : engine_->saveSetFile(setId, path);
    if (!ok) {
        reportError(engine_->lastError());
    }
    return ok;
}

bool ActionsPanel::loadForTest(const QString& path, bool replace)
{
    if (!engine_ || path.isEmpty()) {
        return false;
    }
    const bool ok = engine_->loadSetFile(path, replace);
    const QString engineError = engine_->lastError();
    if (!ok || !engineError.isEmpty()) {
        reportError(engineError.isEmpty() ? tr("Could not load actions.") : engineError);
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return ok;
}

bool ActionsPanel::resetForTest(bool append)
{
    if (!engine_) {
        return false;
    }
    const bool ok = engine_->reset(append);
    if (!ok) {
        reportError(engine_->lastError());
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return ok;
}

bool ActionsPanel::clearAllForTest()
{
    if (!engine_) {
        return false;
    }
    const bool ok = engine_->clearAll();
    if (!ok) {
        reportError(engine_->lastError());
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return ok;
}

bool ActionsPanel::recordToolStrokeForTest(const QString& toolId, const QJsonObject& params,
                                           int payloadBytes)
{
    if (!engine_ || !allowToolRecording_ || !isRecording()) {
        return false;
    }
    const bool ok = engine_->recordToolStroke(toolId, params, payloadBytes);
    if (!ok) {
        const QString error = engine_->lastError();
        reportError(error.isEmpty() ? tr("The tool stroke was refused.") : error);
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
    return ok;
}

bool ActionsPanel::overwriteStepForTest(const QString& stepId)
{
    if (!engine_ || !editingAllowed()) {
        return false;
    }
    const ActionNodeRow step = model_->rowForId(stepId);
    if (step.kind != actionKindStep()) {
        return false;
    }
    QJsonObject params = step.params;
    if (!executeStep(step, &params, true)) {
        reportError(tr("Step %1 could not be edited.").arg(step.name));
        return false;
    }
    const bool ok = engine_->editStep(stepId, params);
    if (ok) {
        QTimer::singleShot(0, this, [this] { refresh(); });
    }
    return ok;
}

bool ActionsPanel::assignFunctionKeyForTest(const QString& actionId, int key, bool shift,
                                            bool ctrl, QString* error)
{
    const bool ok = applyFunctionKey(actionId, key, shift, ctrl, error);
    if (ok) {
        QTimer::singleShot(0, this, [this] { refresh(); });
    }
    return ok;
}

bool ActionsPanel::applyFunctionKey(const QString& actionId, int key, bool shift, bool ctrl,
                                    QString* error, int previousSequence)
{
    if (!engine_ || actionId.isEmpty() || !editingAllowed()) {
        return false;
    }
    if (key != 0 && (key < Qt::Key_F1 || key > Qt::Key_F12)) {
        if (error) {
            *error = tr("Choose a valid function key (F1-F12).");
        }
        return false;
    }
    if (key == 0 && (shift || ctrl)) {
        if (error) {
            *error = tr("Choose a function key for the modifiers.");
        }
        return false;
    }
    int modifiers = (shift ? Qt::ShiftModifier : 0) | (ctrl ? Qt::ControlModifier : 0);
    if (windowsPlatform_ && windowsKeyRefused(key, modifiers, true)) {
        if (error) {
            *error = tr("That function key is reserved on Windows.");
        }
        return false;
    }
    const int sequence = key != 0 ? (key | modifiers) : 0;
    if (previousSequence < 0) {
        previousSequence = actionKeySequence(actionId);
    }
    QString commandId;
    if (key != 0 && shortcutProvider_) {
        commandId = shortcutProvider_(QKeySequence(sequence));
    }
    if (!engine_->assignFunctionKey(actionId, key, shift, ctrl, error)) {
        return false;
    }
    if (previousSequence != sequence) {
        restoreShadowedSequence(previousSequence);
    }
    if (!commandId.isEmpty()) {
        if (registry_) {
            if (!shadowedCommands_.contains(sequence)) {
                const QList<QKeySequence> shortcuts = registry_->shortcuts(commandId);
                if (!shortcuts.isEmpty()) {
                    shadowedCommands_.insert(sequence, {commandId, shortcuts});
                }
            }
            registry_->setShortcuts(commandId, {});
        }
        lastWarning_ =
            tr("The command %1 uses this shortcut; the action takes precedence.").arg(commandId);
        emit functionKeyWarning(lastWarning_);
    }
    schedulePersist();
    return true;
}

int ActionsPanel::actionKeySequence(const QString& actionId) const
{
    if (!engine_) {
        return 0;
    }
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.id != actionId || row.kind != actionKindAction() || row.functionKey == 0) {
            continue;
        }
        int sequence = row.functionKey;
        if (row.functionShift) {
            sequence |= Qt::ShiftModifier;
        }
        if (row.functionCtrl) {
            sequence |= Qt::ControlModifier;
        }
        return sequence;
    }
    return 0;
}

void ActionsPanel::refreshShadowedCommands()
{
    if (!registry_) {
        return;
    }
    QSet<int> owned;
    if (engine_) {
        for (const ActionNodeRow& row : engine_->snapshot()) {
            if (row.kind != actionKindAction() || row.functionKey == 0) {
                continue;
            }
            int sequence = row.functionKey;
            if (row.functionShift) {
                sequence |= Qt::ShiftModifier;
            }
            if (row.functionCtrl) {
                sequence |= Qt::ControlModifier;
            }
            owned.insert(sequence);
        }
    }
    for (auto it = shadowedCommands_.begin(); it != shadowedCommands_.end();) {
        if (!owned.contains(it.key())) {
            registry_->setShortcuts(it.value().first, it.value().second);
            it = shadowedCommands_.erase(it);
        } else {
            ++it;
        }
    }
}

void ActionsPanel::restoreShadowedSequence(int sequence)
{
    if (sequence == 0) {
        return;
    }
    const auto it = shadowedCommands_.find(sequence);
    if (it == shadowedCommands_.end()) {
        return;
    }
    if (registry_) {
        registry_->setShortcuts(it.value().first, it.value().second);
    }
    shadowedCommands_.erase(it);
}

bool ActionsPanel::handleFunctionKeyForTest(int key, int modifiers)
{
    if (!engine_ || isRecording() || playing_) {
        return false;
    }
    const bool shift = modifiers & Qt::ShiftModifier;
    const bool ctrl = modifiers & Qt::ControlModifier;
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.kind == actionKindAction() && row.functionKey == key && row.functionShift == shift
            && row.functionCtrl == ctrl) {
            return playAction(row.id, QString(), false);
        }
    }
    return false;
}

void ActionsPanel::updateActionShortcuts()
{
    qDeleteAll(actionShortcuts_);
    actionShortcuts_.clear();
    if (!engine_) {
        return;
    }
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.kind != actionKindAction() || row.functionKey == 0) {
            continue;
        }
        int sequence = row.functionKey;
        if (row.functionShift) {
            sequence |= Qt::ShiftModifier;
        }
        if (row.functionCtrl) {
            sequence |= Qt::ControlModifier;
        }
        if (registry_ && !shadowedCommands_.contains(sequence)) {
            const QString commandId = registry_->commandForShortcut(QKeySequence(sequence));
            if (!commandId.isEmpty()) {
                const QList<QKeySequence> shortcuts = registry_->shortcuts(commandId);
                if (!shortcuts.isEmpty()) {
                    shadowedCommands_.insert(sequence, {commandId, shortcuts});
                    registry_->setShortcuts(commandId, {});
                }
            }
        }
        auto* shortcut = new QShortcut(QKeySequence(sequence), this);
        shortcut->setContext(Qt::ApplicationShortcut);
        connect(shortcut, &QShortcut::activated, this, [this, id = row.id] {
            if (!isRecording() && !playing_) {
                playAction(id, QString(), false);
            }
        });
        actionShortcuts_ << shortcut;
    }
}

void ActionsPanel::setPresetsRoot(const QString& root)
{
    presetsRoot_ = root;
}

QString ActionsPanel::resolvedPresetsRoot() const
{
    if (!presetsRoot_.isEmpty()) {
        return presetsRoot_;
    }
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
           + QStringLiteral("/Presets/Actions");
}

QStringList ActionsPanel::installedSetPaths() const
{
    QStringList paths;
    const QDir dir(resolvedPresetsRoot());
    for (const QFileInfo& info : dir.entryInfoList({QStringLiteral("*.atn")}, QDir::Files,
                                                   QDir::Name)) {
        paths << info.absoluteFilePath();
    }
    return paths;
}

QStringList ActionsPanel::installedSetNamesForTest() const
{
    QStringList names;
    for (const QString& path : installedSetPaths()) {
        names << QFileInfo(path).completeBaseName();
    }
    return names;
}

QSet<QString> ActionsPanel::shadowedCommandIds() const
{
    QSet<QString> ids;
    for (auto it = shadowedCommands_.cbegin(); it != shadowedCommands_.cend(); ++it) {
        ids.insert(it.value().first);
    }
    return ids;
}

void ActionsPanel::updatePanelMenu(QMenu* menu)
{
    if (!menu) {
        return;
    }
    const bool idle = editingAllowed();
    const bool recording = isRecording();
    const bool hasSelection = !selectedNodeIds().isEmpty();
    bool hasSets = false;
    bool setHasActions = false;
    if (engine_) {
        const QList<ActionNodeRow> rows = engine_->snapshot();
        for (const ActionNodeRow& row : rows) {
            hasSets = hasSets || row.kind == actionKindSet();
        }
        const QString setId = selectedSetId();
        setHasActions = !setId.isEmpty() && !model_->actionIdsForSet(setId).isEmpty();
    }

    if (QAction* action = menuAction(menu, QStringLiteral("actions.buttonMode"))) {
        action->setChecked(buttonMode_);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.allowToolRecording"))) {
        action->setChecked(allowToolRecording_);
        action->setEnabled(idle);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.newSet"))) {
        action->setEnabled(idle);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.newAction"))) {
        action->setEnabled(idle && (selectedIsSet() || hasSets));
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.duplicate"))) {
        action->setEnabled(idle && hasSelection);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.delete"))) {
        action->setEnabled(idle && hasSelection);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.play"))) {
        action->setEnabled(idle && (selectedIsSet() || selectedIsAction() || selectedIsStep()));
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.toggleRecording"))) {
        action->setText(recording ? tr("Stop Recording") : tr("Start Recording"));
        action->setEnabled(recording
                           || (idle
                               && (selectedIsAction() || selectedIsStep() || setHasActions)));
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.recordAgain"))) {
        action->setEnabled(idle && selectedIsAction());
    }
    const bool insertEnabled = idle && (selectedIsAction() || selectedIsStep());
    for (const QString& id : {QStringLiteral("actions.insertMenuItem"),
                              QStringLiteral("actions.insertStop"),
                              QStringLiteral("actions.insertConditional")}) {
        if (QAction* action = menuAction(menu, id)) {
            action->setEnabled(insertEnabled);
        }
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.insertPath"))) {
        action->setEnabled(insertEnabled && pathProvider_ && !pathProvider_().isEmpty());
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.options"))) {
        action->setText(selectedIsSet() ? tr("Set Options…") : tr("Action Options…"));
        action->setEnabled(idle && (selectedIsSet() || selectedIsAction()));
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.playbackOptions"))) {
        action->setEnabled(idle);
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.clear"))) {
        action->setEnabled(idle && hasSets);
    }
    for (const QString& id : {QStringLiteral("actions.reset"), QStringLiteral("actions.load"),
                              QStringLiteral("actions.replace")}) {
        if (QAction* action = menuAction(menu, id)) {
            action->setEnabled(idle);
        }
    }
    if (QAction* action = menuAction(menu, QStringLiteral("actions.save"))) {
        action->setEnabled(idle && selectedIsSet());
    }

    for (QAction* action : menu->actions()) {
        if (action->data().toString().startsWith(QStringLiteral("actions.loadInstalled:"))) {
            menu->removeAction(action);
            action->deleteLater();
        }
    }
    QAction* placeholder = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == QStringLiteral("actions.installedPlaceholder")
            || action->text() == QStringLiteral("(installed sets)")) {
            placeholder = action;
            break;
        }
    }
    const QStringList installed = installedSetPaths();
    if (placeholder) {
        const QList<QAction*> actions = menu->actions();
        const int index = actions.indexOf(placeholder);
        if (installed.isEmpty() && index > 0 && actions.at(index - 1)->isSeparator()) {
            menu->removeAction(actions.at(index - 1));
        }
        menu->removeAction(placeholder);
        placeholder->deleteLater();
    }
    if (!installed.isEmpty()) {
        QAction* closeAction = menuAction(menu, QStringLiteral("close"));
        if (!closeAction) {
            for (QAction* action : menu->actions()) {
                if (action->text() == tr("Close") && !action->isSeparator()) {
                    closeAction = action;
                    break;
                }
            }
        }
        QAction* insertBefore = closeAction;
        if (closeAction) {
            const QList<QAction*> actions = menu->actions();
            const int index = actions.indexOf(closeAction);
            if (index > 0 && actions.at(index - 1)->isSeparator()) {
                insertBefore = actions.at(index - 1);
            }
        }
        for (const QString& path : installed) {
            auto* action = new QAction(QFileInfo(path).completeBaseName(), menu);
            action->setData(QStringLiteral("actions.loadInstalled:") + path);
            action->setEnabled(idle);
            connect(action, &QAction::triggered, this, [this, path] {
                if (editingAllowed()) {
                    loadForTest(path, false);
                }
            });
            if (insertBefore) {
                menu->insertAction(insertBefore, action);
            } else {
                menu->addAction(action);
            }
        }
    }
}

bool ActionsPanel::performPanelMenuAction(const QString& actionId)
{
    if (actionId.startsWith(QStringLiteral("actions.loadInstalled:"))) {
        if (!editingAllowed()) {
            return false;
        }
        return loadForTest(actionId.mid(QStringLiteral("actions.loadInstalled:").size()), false);
    }
    if (actionId == QLatin1String("actions.buttonMode")) {
        setButtonMode(!buttonMode_);
        return true;
    }
    if (actionId == QLatin1String("actions.allowToolRecording")) {
        setAllowToolRecording(!allowToolRecording_);
        return true;
    }
    if (actionId == QLatin1String("actions.newAction")) {
        showNewAction();
        return true;
    }
    if (actionId == QLatin1String("actions.newSet")) {
        showNewSet();
        return true;
    }
    if (actionId == QLatin1String("actions.duplicate")) {
        if (engine_) {
            engine_->duplicateNodes(selectedNodeIds());
        }
        return true;
    }
    if (actionId == QLatin1String("actions.delete")) {
        return deleteSelectionForTest(false);
    }
    if (actionId == QLatin1String("actions.play")) {
        return playSelection(QApplication::keyboardModifiers() & Qt::ControlModifier);
    }
    if (actionId == QLatin1String("actions.toggleRecording")) {
        if (isRecording()) {
            stopRecording();
        } else {
            startRecordingSelection();
        }
        return true;
    }
    if (actionId == QLatin1String("actions.recordAgain")) {
        recordAgain();
        return true;
    }
    if (actionId == QLatin1String("actions.insertMenuItem")) {
        showInsertMenuItem();
        return true;
    }
    if (actionId == QLatin1String("actions.insertStop")) {
        showInsertStop();
        return true;
    }
    if (actionId == QLatin1String("actions.insertConditional")) {
        showInsertConditional();
        return true;
    }
    if (actionId == QLatin1String("actions.insertPath")) {
        showInsertPath();
        return true;
    }
    if (actionId == QLatin1String("actions.options")) {
        showOptions();
        return true;
    }
    if (actionId == QLatin1String("actions.playbackOptions")) {
        showPlaybackOptions();
        return true;
    }
    if (actionId == QLatin1String("actions.clear")) {
        if (confirm(tr("Clear All Actions"), tr("Clear every action set?"))) {
            clearAllForTest();
        }
        return true;
    }
    if (actionId == QLatin1String("actions.reset")) {
        QMessageBox box(this);
        box.setWindowTitle(tr("Reset Actions"));
        box.setText(tr("Reset the action library?"));
        QPushButton* reset = box.addButton(tr("Reset"), QMessageBox::AcceptRole);
        QPushButton* append = box.addButton(tr("Append"), QMessageBox::ActionRole);
        box.addButton(QMessageBox::Cancel);
        box.exec();
        if (box.clickedButton() == reset) {
            resetForTest(false);
        } else if (box.clickedButton() == append) {
            resetForTest(true);
        }
        return true;
    }
    if (actionId == QLatin1String("actions.load")) {
        const QString path = fileDialog(
            false, tr("Load Actions"),
            tr("Action Sets (*.atn);;Text Listings (*.txt);;All Files (*)"));
        return !path.isEmpty() && loadForTest(path, false);
    }
    if (actionId == QLatin1String("actions.replace")) {
        const QString path = fileDialog(
            false, tr("Replace Actions"),
            tr("Action Sets (*.atn);;Text Listings (*.txt);;All Files (*)"));
        if (path.isEmpty() || !confirm(tr("Replace Actions"),
                                       tr("Replace every action set with this file?"))) {
            return false;
        }
        return loadForTest(path, true);
    }
    if (actionId == QLatin1String("actions.save")) {
        const QString setId = selectedSetId();
        if (setId.isEmpty()) {
            return false;
        }
        const Qt::KeyboardModifiers modifiers = QApplication::keyboardModifiers();
        const bool textDump = (modifiers & Qt::AltModifier)
                              && (modifiers & Qt::ControlModifier);
        QString path = fileDialog(true, tr("Save Actions"), tr("Action Sets (*.atn)"));
        if (path.isEmpty()) {
            return false;
        }
        if (QFileInfo(path).suffix().isEmpty()) {
            path += QStringLiteral(".atn");
        }
        return saveSetForTest(setId, path, textDump);
    }
    return false;
}

void ActionsPanel::showNewAction()
{
    if (!engine_) {
        return;
    }
    QList<ActionDialogSet> sets;
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.kind == actionKindSet()) {
            sets << ActionDialogSet{row.id, row.name};
        }
    }
    if (sets.isEmpty()) {
        return;
    }
    QString currentSet = selectedSetId();
    if (currentSet.isEmpty()) {
        const QString actionId = !selectedActionId().isEmpty()
            ? selectedActionId()
            : nodeActionId(model_, selectedStepId());
        currentSet = nodeSetId(model_, actionId);
    }
    NewActionDialog dialog(this);
    dialog.setSets(sets, currentSet);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    createActionForTest(dialog.actionName(), dialog.setId(), dialog.functionKey(),
                        dialog.shift(), dialog.ctrl(), dialog.color(), dialog.record());
}

void ActionsPanel::showNewSet()
{
    if (!engine_) {
        return;
    }
    SetNameDialog dialog(tr("New Set"), tr("Action Set"), this);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    createSetForTest(dialog.name());
}

void ActionsPanel::showOptions()
{
    if (!engine_) {
        return;
    }
    if (const QString setId = selectedSetId(); !setId.isEmpty()) {
        const ActionNodeRow row = model_->rowForId(setId);
        SetNameDialog dialog(tr("Set Options"), row.name, this);
        if (dialog.exec() == QDialog::Accepted) {
            engine_->setSetName(setId, dialog.name());
            QTimer::singleShot(0, this, [this] { refresh(); });
            schedulePersist();
        }
        return;
    }
    const QString actionId = selectedActionId();
    if (actionId.isEmpty()) {
        return;
    }
    const ActionNodeRow action = model_->rowForId(actionId);
    QList<ActionDialogSet> sets;
    QString currentSetId = nodeSetId(model_, actionId);
    for (const ActionNodeRow& row : engine_->snapshot()) {
        if (row.kind == actionKindSet()) {
            sets << ActionDialogSet{row.id, row.name};
        }
    }
    ActionOptionsDialog dialog(this);
    dialog.setSets(sets, currentSetId);
    dialog.setValues(action.name, action.functionKey, action.functionShift, action.functionCtrl,
                     action.color);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    applyActionOptionsForTest(actionId, dialog.actionName(), dialog.setId(),
                              dialog.functionKey(), dialog.shift(), dialog.ctrl(),
                              dialog.color());
}

void ActionsPanel::showPlaybackOptions()
{
    if (!engine_) {
        return;
    }
    PlaybackOptionsDialog dialog(this);
    dialog.setOptions(engine_->playbackMode(), engine_->playbackPauseSeconds());
    if (dialog.exec() == QDialog::Accepted) {
        setPlaybackOptionsForTest(dialog.mode(), dialog.pauseSeconds());
    }
}

void ActionsPanel::showInsertStop()
{
    InsertStopDialog dialog(this);
    if (dialog.exec() == QDialog::Accepted) {
        insertStopForTest(dialog.message(), dialog.allowContinue());
    }
}

void ActionsPanel::showInsertMenuItem()
{
    InsertMenuItemDialog dialog(this);
    QList<QPair<QString, QString>> commands;
    if (registry_) {
        for (const CommandInfo& info : registry_->describe()) {
            commands << qMakePair(info.id, info.path.join(QStringLiteral(" > ")));
        }
    }
    dialog.setCommands(commands);
    if (dialog.exec() == QDialog::Accepted) {
        insertMenuItemForTest(dialog.commandId());
    }
}

void ActionsPanel::showInsertPath()
{
    const QString path = pathProvider_ ? pathProvider_() : QString();
    if (path.isEmpty()) {
        return;
    }
    InsertPathDialog dialog(path, this);
    if (dialog.exec() == QDialog::Accepted) {
        insertPathForTest(path);
    }
}

void ActionsPanel::showInsertConditional()
{
    if (!engine_) {
        return;
    }
    const QString actionId = selectedIsAction() ? selectedActionId()
                                                : nodeActionId(model_, selectedStepId());
    if (actionId.isEmpty()) {
        return;
    }
    const QString setId = nodeSetId(model_, actionId);
    const QStringList allowed = model_->actionIdsForSet(setId);
    QList<ActionDialogSet> actions;
    for (const QString& id : allowed) {
        const ActionNodeRow row = model_->rowForId(id);
        if (row.id != actionId) {
            actions << ActionDialogSet{row.id, row.name};
        }
    }
    InsertConditionalDialog dialog(this);
    dialog.setActions(actions);
    dialog.setAllowedActionIds(allowed);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    QString error;
    insertConditionalForTest(dialog.condition(), dialog.thenActionId(), dialog.elseActionId(),
                             &error);
}

QString ActionsPanel::fileDialog(bool save, const QString& title, const QString& filter)
{
    if (fileDialogProvider_) {
        return fileDialogProvider_(save, title, filter);
    }
    if (save) {
        return QFileDialog::getSaveFileName(this, title, QString(), filter);
    }
    return QFileDialog::getOpenFileName(this, title, QString(), filter);
}

bool ActionsPanel::confirm(const QString& title, const QString& text)
{
    if (confirmHandler_) {
        return confirmHandler_(title, text);
    }
    return QMessageBox::question(this, title, text,
                                 QMessageBox::Yes | QMessageBox::No)
           == QMessageBox::Yes;
}

void ActionsPanel::applySessionState(const SessionState& state)
{
    applyingSession_ = true;
    buttonMode_ = state.actionsButtonMode;
    allowToolRecording_ = state.actionsAllowToolRecording;
    pendingExpanded_ = state.actionsExpanded;
    pendingSelection_ = state.actionsSelection;
    if (engine_) {
        engine_->setAllowToolRecording(allowToolRecording_);
        engine_->setPlaybackOptions(state.actionsPlaybackMode, state.actionsPlaybackPause);
    }
    applyingSession_ = false;
    setButtonModeInternal(buttonMode_);
    refresh();
}

void ActionsPanel::writeSessionState(SessionState* state) const
{
    if (!state) {
        return;
    }
    state->actionsButtonMode = buttonMode_;
    state->actionsAllowToolRecording = allowToolRecording_;
    QStringList expanded;
    for (const QString& id : expandedIds_) {
        const QString identity = model_->identityForId(id);
        if (!identity.isEmpty()) {
            expanded << identity;
        }
    }
    state->actionsExpanded = expanded;
    state->actionsSelection = model_->identitiesForIds(selectedNodeIds());
    if (engine_) {
        state->actionsPlaybackMode = engine_->playbackMode();
        state->actionsPlaybackPause = engine_->playbackPauseSeconds();
    }
}

void ActionsPanel::schedulePersist()
{
    if (!persistTimer_ || !persistEnabled_ || applyingSession_) {
        return;
    }
    persistTimer_->start();
}

void ActionsPanel::persistNow()
{
    if (!persistEnabled_) {
        return;
    }
    if (engine_) {
        engine_->saveLibrary();
    }
    SessionState state = loadSession();
    writeSessionState(&state);
    saveSession(state);
}

void ActionsPanel::persistNowForTest()
{
    persistNow();
}

void ActionsPanel::saveLibraryNowForTest()
{
    if (engine_) {
        engine_->saveLibrary();
    }
}

void ActionsPanel::loadLibraryNowForTest()
{
    if (engine_) {
        engine_->loadLibrary();
        QTimer::singleShot(0, this, [this] { refresh(); });
    }
}

void ActionsPanel::reportError(const QString& message)
{
    if (message.isEmpty()) {
        return;
    }
    lastError_ = message;
    emit errorOccurred(message);
}

bool ActionsPanel::windowsKeyRefused(int key, int modifiers, bool windowsPlatform)
{
    if (!windowsPlatform) {
        return false;
    }
    if (key == Qt::Key_F1) {
        return true;
    }
    return (modifiers & Qt::ControlModifier)
           && (key == Qt::Key_F4 || key == Qt::Key_F6);
}

bool ActionsPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == deleteButton_ || watched == newActionButton_ || watched == newSetButton_) {
        if (event->type() == QEvent::DragEnter) {
            auto* drag = static_cast<QDragEnterEvent*>(event);
            if (drag->mimeData()->hasFormat(kMimeType)) {
                drag->acceptProposedAction();
                return true;
            }
        } else if (event->type() == QEvent::Drop) {
            auto* drop = static_cast<QDropEvent*>(event);
            const QStringList ids = decodeMimeIds(drop->mimeData());
            if (!ids.isEmpty() && engine_ && editingAllowed()) {
                if (watched == deleteButton_) {
                    engine_->deleteNodes(ids);
                } else {
                    engine_->duplicateNodes(ids);
                }
                drop->acceptProposedAction();
                return true;
            }
        }
    }
    if (event->type() == QEvent::KeyPress && isRecording()) {
        auto* key = static_cast<QKeyEvent*>(event);
        if (key->key() == Qt::Key_Escape) {
            stopRecording();
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura
