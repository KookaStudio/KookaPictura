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

#include <algorithm>

namespace pictura {

using namespace actions_internal;

namespace {

QToolButton* makeToolbarButton(QWidget* parent, const QString& objectName,
                               const QString& iconId, const QString& toolTip)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(objectName);
    button->setIcon(pictura::icon(iconId));
    button->setIconSize(QSize(16, 16));
    button->setAutoRaise(true);
    button->setToolTip(toolTip);
    button->setFixedSize(24, 24);
    return button;
}

} // namespace

ActionsPanel::ActionsPanel(QWidget* parent)
    : QWidget(parent)
{
#ifdef Q_OS_WIN
    windowsPlatform_ = true;
#endif
    buildUi();

    playbackTimer_ = new QTimer(this);
    playbackTimer_->setSingleShot(true);
    connect(playbackTimer_, &QTimer::timeout, this, [this] { advancePlayback(); });

    persistTimer_ = new QTimer(this);
    persistTimer_->setSingleShot(true);
    persistTimer_->setInterval(500);
    connect(persistTimer_, &QTimer::timeout, this, [this] { persistNow(); });

    installEventFilter(this);
    tree_->installEventFilter(this);
    buttonView_->installEventFilter(this);
    deleteButton_->installEventFilter(this);
    deleteButton_->setAcceptDrops(true);
    newActionButton_->installEventFilter(this);
    newActionButton_->setAcceptDrops(true);
    newSetButton_->installEventFilter(this);
    newSetButton_->setAcceptDrops(true);
    updateToolbar();
}

ActionsPanel::~ActionsPanel()
{
    if (qApp) {
        qApp->removeEventFilter(this);
    }
    if (engine_) {
        engine_->saveLibrary();
    }
    engine_ = nullptr;
    persistNow();
}

void ActionsPanel::buildUi()
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("actionsStack"));

    tree_ = new ActionsTreeView(stack_);
    tree_->setObjectName(QStringLiteral("actionsTree"));
    model_ = new ActionsTreeModel(this);
    tree_->setModel(model_);
    tree_->setTreePosition(2);
    tree_->header()->setStretchLastSection(true);
    tree_->setColumnWidth(0, 22);
    tree_->setColumnWidth(1, 22);
    tree_->setHorizontalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    stack_->addWidget(tree_);

    buttonView_ = new ActionsButtonView(stack_);
    buttonView_->setObjectName(QStringLiteral("actionsButtons"));
    stack_->addWidget(buttonView_);

    toolbar_ = new QWidget(this);
    toolbar_->setObjectName(QStringLiteral("actionsToolbar"));
    auto* tools = new QHBoxLayout(toolbar_);
    tools->setContentsMargins(4, 2, 4, 2);
    tools->setSpacing(2);
    stopButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsStopButton"),
                                    QStringLiteral("actions.stop"),
                                    tr("Stop Playing/Recording"));
    recordButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsRecordButton"),
                                      QStringLiteral("actions.record"),
                                      tr("Begin Recording"));
    recordButton_->setCheckable(true);
    playButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsPlayButton"),
                                    QStringLiteral("actions.play"), tr("Play"));
    newSetButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsNewSetButton"),
                                      QStringLiteral("actions.newSet"),
                                      tr("Create New Set"));
    newActionButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsNewActionButton"),
                                         QStringLiteral("actions.newAction"),
                                         tr("Create New Action"));
    deleteButton_ = makeToolbarButton(toolbar_, QStringLiteral("actionsDeleteButton"),
                                      QStringLiteral("actions.delete"), tr("Delete"));
    tools->addWidget(stopButton_);
    tools->addWidget(recordButton_);
    tools->addWidget(playButton_);
    tools->addWidget(newSetButton_);
    tools->addWidget(newActionButton_);
    tools->addWidget(deleteButton_);
    tools->addStretch();

    layout->addWidget(stack_, 1);
    layout->addWidget(toolbar_, 0);

    connect(stopButton_, &QToolButton::clicked, this, [this] {
        if (isRecording()) {
            stopRecording();
        } else {
            finishPlayback();
        }
    });
    connect(recordButton_, &QToolButton::clicked, this, [this] {
        if (isRecording()) {
            stopRecording();
        } else if (!startRecordingSelection()) {
            recordButton_->setChecked(false);
        }
    });
    connect(playButton_, &QToolButton::clicked, this, [this] {
        playSelection(QApplication::keyboardModifiers() & Qt::ControlModifier);
    });
    connect(newSetButton_, &QToolButton::clicked, this, [this] { showNewSet(); });
    connect(newActionButton_, &QToolButton::clicked, this, [this] { showNewAction(); });
    connect(deleteButton_, &QToolButton::clicked, this, [this] {
        deleteSelectionForTest(QApplication::keyboardModifiers() & Qt::AltModifier);
    });
    connect(buttonView_, &ActionsButtonView::actionActivated, this,
            [this](const QString& id) { playAction(id, QString(), false); });
    connect(tree_, &ActionsTreeView::inclusionToggleRequested, this,
            [this](const QModelIndex& index) {
                model_->requestInclusion(index, model_->inclusionState(index) != Qt::Checked);
            });
    connect(tree_, &ActionsTreeView::siblingInclusionToggleRequested, model_,
            &ActionsTreeModel::toggleSiblingsIncluded);
    connect(tree_, &ActionsTreeView::modalToggleRequested, this,
            [this](const QModelIndex& index) {
                model_->requestModal(index, model_->modalState(index) != Qt::Checked);
            });
    connect(tree_, &ActionsTreeView::escapePressed, this, [this] {
        if (isRecording()) {
            stopRecording();
        }
    });
    connect(tree_, &QTreeView::expanded, this, [this](const QModelIndex& index) {
        if (restoringState_) {
            return;
        }
        const QString id = model_->nodeId(index);
        if (!id.isEmpty()) {
            expandedIds_.insert(id);
            schedulePersist();
        }
    });
    connect(tree_, &QTreeView::collapsed, this, [this](const QModelIndex& index) {
        if (restoringState_) {
            return;
        }
        const QString id = model_->nodeId(index);
        if (!id.isEmpty()) {
            expandedIds_.remove(id);
            schedulePersist();
        }
    });
    connect(tree_->selectionModel(), &QItemSelectionModel::selectionChanged, this, [this] {
        if (!restoringState_) {
            schedulePersist();
        }
    });
    connect(tree_, &QTreeView::doubleClicked, this, [this](const QModelIndex& index) {
        if (!editingAllowed()) {
            return;
        }
        const QString id = model_->nodeId(index);
        const ActionNodeRow row = model_->rowForId(id);
        if (row.kind != actionKindStep()) {
            return;
        }
        if (QApplication::keyboardModifiers() & Qt::ControlModifier) {
            playAction(nodeActionId(model_, id), id, true);
        } else {
            overwriteStepForTest(id);
        }
    });
    connectModel();
}

void ActionsPanel::connectModel()
{
    connect(model_, &ActionsTreeModel::inclusionRequested, this,
            [this](const QString& id, bool included) {
                if (engine_) {
                    engine_->setIncluded(id, included);
                }
            });
    connect(model_, &ActionsTreeModel::siblingInclusionRequested, this,
            [this](const QString& id) {
                if (engine_) {
                    engine_->toggleSiblingInclusion(id);
                }
            });
    connect(model_, &ActionsTreeModel::modalRequested, this,
            [this](const QString& id, bool modal) {
                if (engine_) {
                    engine_->setModal(id, modal);
                }
            });
    connect(model_, &ActionsTreeModel::renameRequested, this,
            [this](const QString& id, const QString& name) {
                if (!editingAllowed()) {
                    return;
                }
                if (engine_) {
                    engine_->renameNode(id, name);
                    schedulePersist();
                }
            });
    connect(model_, &ActionsTreeModel::moveRequested, this,
            [this](const QString& id, const QString& parentId, int row, bool copy) {
                if (!editingAllowed()) {
                    return;
                }
                if (engine_ && !engine_->moveNode(id, parentId, row, copy)) {
                    const QString error = engine_->lastError();
                    reportError(error.isEmpty() ? tr("The item could not be moved.") : error);
                }
            });
}

void ActionsPanel::setEngine(ActionsEngineApi* engine)
{
    if (engine_.data() == engine) {
        return;
    }
    if (engine_) {
        QObject::disconnect(engine_.data(), nullptr, this, nullptr);
    }
    engine_ = engine;
    if (engine_) {
        allowToolRecording_ = engine_->allowToolRecording();
        connectEngine();
    }
    refresh();
}

void ActionsPanel::connectEngine()
{
    connect(engine_.data(), &ActionsEngineApi::changed, this, [this] {
        QTimer::singleShot(0, this, [this] { refresh(); });
        if (!isRecording() && !playing_) {
            schedulePersist();
        }
    });
    connect(engine_.data(), &ActionsEngineApi::recordingChanged, this, [this](bool recording) {
        recordButton_->setChecked(recording);
        if (recording) {
            qApp->installEventFilter(this);
        } else {
            qApp->removeEventFilter(this);
        }
        updateToolbar();
        emit changed();
    });
    connect(engine_.data(), &ActionsEngineApi::playbackChanged, this, [this](bool playing) {
        if (!playing && playing_) {
            playing_ = false;
            playbackTimer_->stop();
            playQueue_.clear();
            playIndex_ = 0;
            playDepth_ = 0;
        }
        updateToolbar();
        emit changed();
    });
    connect(engine_.data(), &ActionsEngineApi::error, this,
            [this](const QString& message) { reportError(message); });
}

void ActionsPanel::setRegistry(CommandRegistry* registry)
{
    registry_ = registry;
    if (!registry_) {
        return;
    }
    registry_->setRecordingSink([this](const QString& id, const CommandParams& params) {
        if (engine_ && engine_->isRecording()) {
            engine_->recordCommand(id, params.values);
        }
    });
    if (!shortcutProvider_) {
        shortcutProvider_ = [this](const QKeySequence& shortcut) {
            return registry_ ? registry_->commandForShortcut(shortcut) : QString();
        };
    }
    updateActionShortcuts();
}

void ActionsPanel::refresh()
{
    if (!engine_) {
        model_->setRows({});
        updateButtonModel();
        updateActionShortcuts();
        refreshShadowedCommands();
        updateToolbar();
        return;
    }
    const QStringList selectedIds = selectedNodeIds();
    QStringList previousExpanded;
    for (const QString& id : expandedIds_) {
        previousExpanded << id;
    }
    const QStringList previousExpandedIdentities =
        model_->identitiesForIds(previousExpanded);
    const QStringList previousSelectionIdentities = model_->identitiesForIds(selectedIds);
    const QStringList pendingExpanded = pendingExpanded_;
    const QStringList pendingSelection = pendingSelection_;
    pendingExpanded_.clear();
    pendingSelection_.clear();

    restoringState_ = true;
    model_->setRows(engine_->snapshot());

    QSet<QString> expandedIds;
    for (const QString& id : model_->idsForIdentities(previousExpandedIdentities)) {
        expandedIds.insert(id);
    }
    for (const QString& id : model_->idsForIdentities(pendingExpanded)) {
        expandedIds.insert(id);
    }
    expandedIds_ = expandedIds;
    for (const QString& id : expandedIds_) {
        const QModelIndex index = model_->indexForId(id);
        if (index.isValid()) {
            tree_->expand(index);
        }
    }

    QStringList selection = model_->idsForIdentities(pendingSelection);
    for (const QString& id : model_->idsForIdentities(previousSelectionIdentities)) {
        if (!selection.contains(id)) {
            selection << id;
        }
    }
    tree_->selectionModel()->clearSelection();
    QModelIndex firstSelected;
    for (const QString& id : selection) {
        const QModelIndex index = model_->indexForId(id);
        if (index.isValid()) {
            tree_->selectionModel()->select(
                index, QItemSelectionModel::Select | QItemSelectionModel::Rows);
            if (!firstSelected.isValid()) {
                firstSelected = index;
            }
        }
    }
    if (firstSelected.isValid()) {
        tree_->selectionModel()->setCurrentIndex(firstSelected, QItemSelectionModel::NoUpdate);
    }
    restoringState_ = false;

    updateButtonModel();
    updateToolbar();
    updateActionShortcuts();
    refreshShadowedCommands();
    emit changed();
}

void ActionsPanel::updateButtonModel()
{
    if (buttonView_) {
        buttonView_->setActions(engine_ ? engine_->snapshot() : QList<ActionNodeRow>{});
    }
}

QList<QToolButton*> ActionsPanel::toolbarButtonsForTest() const
{
    return {stopButton_, recordButton_, playButton_, newSetButton_, newActionButton_,
            deleteButton_};
}

QToolButton* ActionsPanel::toolbarButtonForTest(const QString& objectName) const
{
    for (QToolButton* button : toolbarButtonsForTest()) {
        if (button && button->objectName() == objectName) {
            return button;
        }
    }
    return nullptr;
}

bool ActionsPanel::isRecording() const
{
    return engine_ && engine_->isRecording();
}

bool ActionsPanel::editingAllowed() const
{
    return engine_ && !engine_->isRecording() && !playing_;
}

QList<ActionNodeRow> ActionsPanel::selectedRows() const
{
    QList<ActionNodeRow> rows;
    QSet<QString> seen;
    if (!tree_->selectionModel()) {
        return rows;
    }
    for (const QModelIndex& index : tree_->selectionModel()->selectedIndexes()) {
        const QString id = model_->nodeId(index);
        if (id.isEmpty() || seen.contains(id)) {
            continue;
        }
        seen.insert(id);
        rows << model_->rowForId(id);
    }
    return rows;
}

QStringList ActionsPanel::selectedNodeIds() const
{
    QStringList ids;
    for (const ActionNodeRow& row : selectedRows()) {
        ids << row.id;
    }
    return ids;
}

bool ActionsPanel::selectedIsSet() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindSet()) {
            return true;
        }
    }
    return false;
}

bool ActionsPanel::selectedIsAction() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindAction()) {
            return true;
        }
    }
    return false;
}

bool ActionsPanel::selectedIsStep() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindStep()) {
            return true;
        }
    }
    return false;
}

QString ActionsPanel::selectedSetId() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindSet()) {
            return row.id;
        }
    }
    return QString();
}

QString ActionsPanel::selectedActionId() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindAction()) {
            return row.id;
        }
    }
    return QString();
}

QString ActionsPanel::selectedStepId() const
{
    for (const ActionNodeRow& row : selectedRows()) {
        if (row.kind == actionKindStep()) {
            return row.id;
        }
    }
    return QString();
}

void ActionsPanel::selectNodesForTest(const QStringList& ids)
{
    tree_->selectionModel()->clearSelection();
    for (const QString& id : ids) {
        const QModelIndex index = model_->indexForId(id);
        if (index.isValid()) {
            tree_->selectionModel()->select(
                index, QItemSelectionModel::Select | QItemSelectionModel::Rows);
        }
    }
    updateToolbar();
}

void ActionsPanel::expandAllForTest()
{
    tree_->expandAll();
    for (int i = 0; i < model_->rowCount(); ++i) {
        expandedIds_.insert(model_->nodeId(model_->index(i, 0)));
    }
}

void ActionsPanel::collapseAllForTest()
{
    tree_->collapseAll();
    expandedIds_.clear();
}

void ActionsPanel::updateToolbar()
{
    if (!stopButton_) {
        return;
    }
    const bool recording = isRecording();
    const bool editing = editingAllowed();
    bool hasSet = false;
    bool hasSelection = false;
    bool hasSetWithAction = false;
    if (engine_) {
        const QList<ActionNodeRow> rows = engine_->snapshot();
        for (const ActionNodeRow& row : rows) {
            hasSet = hasSet || row.kind == actionKindSet();
        }
        hasSelection = !selectedNodeIds().isEmpty();
        const QString setId = selectedSetId();
        hasSetWithAction = !setId.isEmpty() && !model_->actionIdsForSet(setId).isEmpty();
    }
    stopButton_->setEnabled(recording || playing_);
    tree_->setEnabled(editing);
    buttonView_->setEnabled(editing);
    recordButton_->setChecked(recording);
    recordButton_->setEnabled(recording || (editing && (selectedIsAction() || selectedIsStep()
                                                        || hasSetWithAction)));
    const bool canPlay = editing && (selectedIsSet() || selectedIsAction() || selectedIsStep());
    playButton_->setEnabled(canPlay);
    newSetButton_->setEnabled(editing);
    newActionButton_->setEnabled(editing && (selectedIsSet() || hasSet));
    deleteButton_->setEnabled(editing && hasSelection);
    stopButton_->setToolTip(recording ? tr("Stop Recording") : tr("Stop Playing"));
    recordButton_->setToolTip(recording ? tr("Stop Recording") : tr("Begin Recording"));
}

bool ActionsPanel::startRecordingSelection()
{
    if (!engine_ || playing_ || engine_->isRecording()) {
        return false;
    }
    if (const QString stepId = selectedStepId(); !stepId.isEmpty()) {
        const ActionNodeRow step = model_->rowForId(stepId);
        startRecordingOnAction(nodeActionId(model_, stepId), step.stepIndex);
        return engine_->isRecording();
    }
    if (const QString actionId = selectedActionId(); !actionId.isEmpty()) {
        startRecordingOnAction(actionId, -1);
        return engine_->isRecording();
    }
    if (const QString setId = selectedSetId(); !setId.isEmpty()) {
        const QStringList actions = model_->actionIdsForSet(setId);
        if (!actions.isEmpty()) {
            startRecordingOnAction(actions.last(), -1);
            return engine_->isRecording();
        }
    }
    return false;
}

void ActionsPanel::startRecordingOnAction(const QString& actionId, int stepIndex)
{
    if (!engine_ || actionId.isEmpty()) {
        return;
    }
    engine_->startRecording(actionId, stepIndex);
    if (!engine_->isRecording()) {
        reportError(engine_->lastError());
    } else if (qApp) {
        qApp->installEventFilter(this);
    }
    recordButton_->setChecked(engine_->isRecording());
    updateToolbar();
    schedulePersist();
}

void ActionsPanel::stopRecording()
{
    if (engine_) {
        engine_->stopRecording();
    }
    recordButton_->setChecked(false);
    updateToolbar();
    schedulePersist();
}

void ActionsPanel::recordAgain()
{
    if (!engine_ || !editingAllowed()) {
        return;
    }
    const QString actionId = selectedActionId();
    if (actionId.isEmpty() || !engine_->recordAgain(actionId)) {
        return;
    }
    QJsonObject params;
    QString stepId;
    while (!(stepId = engine_->recordAgainNext(&params)).isEmpty()) {
        const ActionNodeRow step = model_->rowForId(stepId);
        const bool modal = effectiveModal(step);
        QJsonObject edited = params;
        if (executeStep(step, &edited, modal) && modal) {
            engine_->editStep(stepId, edited);
        }
    }
    QTimer::singleShot(0, this, [this] { refresh(); });
}

QStringList ActionsPanel::includedStepsForAction(const QString& actionId) const
{
    QStringList steps;
    for (const QString& id : model_->stepIdsForAction(actionId)) {
        const ActionNodeRow row = model_->rowForId(id);
        if (row.enabled) {
            steps << id;
        }
    }
    return steps;
}

QStringList ActionsPanel::buildPlayQueue(const QStringList& ids, bool singleStep,
                                         bool* hasSteps, QString* error) const
{
    QStringList queue;
    if (hasSteps) {
        *hasSteps = false;
    }
    if (error) {
        error->clear();
    }
    QList<ActionNodeRow> rows;
    for (const QString& id : ids) {
        const ActionNodeRow row = model_->rowForId(id);
        if (row.id.isEmpty() || row.kind == actionKindParam()) {
            continue;
        }
        rows << row;
    }

    int structureCount = 0;
    int stepCount = 0;
    for (const ActionNodeRow& row : rows) {
        if (row.kind == actionKindStep()) {
            ++stepCount;
        } else if (row.kind == actionKindSet() || row.kind == actionKindAction()) {
            ++structureCount;
        }
    }

    if (structureCount > 0) {
        if (rows.size() != 1) {
            if (error) {
                *error = tr("Select one set, one action, or one step to play.");
            }
            return queue;
        }
        const ActionNodeRow row = rows.first();
        if (row.kind == actionKindAction()) {
            queue = includedStepsForAction(row.id);
        } else if (row.kind == actionKindSet()) {
            for (const QString& actionId : model_->actionIdsForSet(row.id)) {
                queue << includedStepsForAction(actionId);
            }
        }
        if (singleStep && !queue.isEmpty()) {
            queue = queue.mid(0, 1);
        }
        if (hasSteps) {
            *hasSteps = !queue.isEmpty();
        }
        return queue;
    }

    if (stepCount == 0) {
        return queue;
    }

    if (stepCount == 1) {
        const ActionNodeRow row = rows.first();
        const QString actionId = nodeActionId(model_, row.id);
        const QStringList steps = model_->stepIdsForAction(actionId);
        const int start = steps.indexOf(row.id);
        if (start < 0) {
            return queue;
        }
        const int end = singleStep ? start + 1 : steps.size();
        for (int i = start; i < end; ++i) {
            if (model_->rowForId(steps.at(i)).enabled) {
                queue << steps.at(i);
            }
        }
        if (hasSteps) {
            *hasSteps = !queue.isEmpty();
        }
        return queue;
    }

    if (error) {
        *error = tr("Select one step, one action, or one set to play.");
    }
    return queue;
}

} // namespace pictura

#include "moc_actions_panel.cpp"
