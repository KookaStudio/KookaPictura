#include <QtTest/QtTest>
#include <QtTest/QAbstractItemModelTester>

#include "panels/actions_button_view.h"
#include "panels/actions_dialogs.h"
#include "panels/actions_panel.h"
#include "commands.h"
#include "actions_test_engine.h"
#include "panels/actions_tree_model.h"
#include "frame.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "panels/placeholder_panel.h"
#include "session.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QMimeData>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDockWidget>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include "qt_test_support.h"

class ActionsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void cleanupTestCase();

    void ap_default_hidden();
    void ap_exists();
    void pr_panel_set();
    void pr_actions_content();
    void ap_hosts();
    void ap_flyout_width();
    void ap_menu_order();
    void ap_menu_close();
    void ap_menu_checks();
    void ap_menu_sets();
    void ap_menu_installed_lock();
    void ap_toolbar_order();
    void ap_toolbar_record();
    void ap_toolbar_playback();
    void ap_dialog_new_action();
    void ap_dialog_playback();
    void ap_dialog_action_options();
    void ap_dialog_set_options();
    void ap_tree_shape();
    void ap_tree_expansion();
    void ap_tree_alt_expand();
    void ap_check_step();
    void ap_check_parent();
    void ap_check_alt();
    void ap_check_alt_click();
    void ap_check_partial();
    void ap_modal_toggle();
    void ap_modal_required();
    void ap_modal_menu_item();
    void ap_modal_parent();
    void ap_select();
    void ap_refresh_current();
    void ap_rename();
    void ap_overwrite();
    void ap_reorder();
    void ap_drop_same_action_end();
    void ap_duplicate();
    void ap_delete_fast();
    void ap_edit_locked();
    void ap_refusal_errors();
    void ap_button_mode();
    void ap_button_play();
    void ap_button_restore();
    void ap_button_stack_restore();
    void ap_menu_enable();
    void ap_insert_stop();
    void ap_insert_anchor();
    void ap_insert_menu();
    void ap_insert_conditional();
    void ap_record_again();
    void ap_allow_tool();
    void ap_record_escape();
    void ap_play_from_step();
    void ap_play_single();
    void ap_play_key();
    void ap_play_concurrent();
    void ap_play_set();
    void ap_play_selection_split();
    void ae_play_modal();
    void ae_play_recorded();
    void ae_play_specific();
    void ae_play_stop();
    void ae_play_menu_item();
    void ae_play_path();
    void ae_play_cancel();
    void ae_play_tool();
    void ae_play_tool_missing();
    void ae_document_facts();
    void ae_history_per_command();
    void ae_history_no_group();
    void ae_key_precedence();
    void ae_key_unique();
    void ae_key_restore();
    void ae_key_shadow_rename_move();
    void ae_key_shadow_rebuild();
    void ae_tool_replay();
    void ae_tool_settings();
    void ae_tool_off();
    void ae_tool_limit();
    void ae_tool_reject_error();
    void ae_conditional_invalid();
    void ae_conditional_same_set();
    void ap_save_load();
    void ap_replace();
    void ap_text_dump();
    void ap_save_suffix();
    void ap_state_restore();
    void ap_library_restore();
    void ap_key_assign();
    void ap_key_warn();
    void ap_key_windows();
    void ap_error_parse();
    void ap_error_target();
    void ap_error_empty();
    void ap_a11y_model();
    void ap_a11y_key();

private:
    QModelIndex indexFor(const QString& id) const;
    QModelIndex modalIndexFor(const QString& id) const;
    QMenu* liveMenu();
    QAction* menuActionFor(QMenu* menu, const QString& id) const;
    void ensurePanel();
    void resetFixtureState();
    void advanceUntilDone();
    void selectAndRefresh(const QString& id);
    QVariant checkState(const QModelIndex& index) const;
    int selectedCount() const;

    pictura::test::ScopedStateHome stateHome_;
    FakeActionsEngine engine_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::ActionsPanel* panel_ = nullptr;
    QTemporaryDir presetsDir_;
    QAbstractItemModelTester* tester_ = nullptr;

    QStringList executed_;
    QList<bool> modalFlags_;
    QList<QJsonObject> paramsSeen_;
    QList<QJsonObject> toolParams_;
    bool executorSucceeds_ = true;
    bool stopContinue_ = true;
    bool stopSeen_ = false;
    QString stopMessage_;
    bool confirmResult_ = true;
    int confirmCount_ = 0;
    bool commandRan_ = false;
    int historyCount_ = 0;
    QString dialogPath_;
    bool pathSelected_ = false;
    QString lastError_;
};

void ActionsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(presetsDir_.isValid());
    qputenv("XDG_DATA_HOME", presetsDir_.path().toUtf8());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1200, 800);
    window_->show();
    panel_ = window_->findChild<pictura::ActionsPanel*>(QStringLiteral("actionsPanel"));
    QVERIFY(panel_ != nullptr);
    panel_->setPersistEnabledForTest(false);
    panel_->setPresetsRoot(presetsDir_.path());
    panel_->setEngine(&engine_);
    panel_->setStepExecutorForTest(
        [this](const ActionNodeRow& step, QJsonObject* params, bool modal) {
            executed_ << step.commandId;
            modalFlags_ << modal;
            paramsSeen_ << (params ? *params : QJsonObject());
            ++historyCount_;
            if (modal && params) {
                (*params)[QStringLiteral("edited")] = true;
            }
            return executorSucceeds_;
        });
    panel_->setStopHandlerForTest([this](const QString& message, bool allowContinue) {
        stopSeen_ = true;
        stopMessage_ = message;
        Q_UNUSED(allowContinue)
        return stopContinue_;
    });
    panel_->setConfirmHandlerForTest([this](const QString&, const QString&) {
        ++confirmCount_;
        return confirmResult_;
    });
    panel_->setFileDialogProviderForTest(
        [this](bool, const QString&, const QString&) { return dialogPath_; });
    panel_->setPathSelectionProviderForTest(
        [this]() { return pathSelected_ ? QStringLiteral("Work Path") : QString(); });
    connect(panel_, &pictura::ActionsPanel::errorOccurred, this,
            [this](const QString& message) { lastError_ = message; });
    tester_ = new QAbstractItemModelTester(
        panel_->modelForTest(), QAbstractItemModelTester::FailureReportingMode::QtTest, this);
    panel_->refresh();
}

void ActionsPanelTest::init()
{
    resetFixtureState();
}

void ActionsPanelTest::cleanup()
{
    resetFixtureState();
    if (window_) {
        window_->panelColumn()->showPanel(QStringLiteral("actionsPanel"), false);
    }
}

void ActionsPanelTest::cleanupTestCase()
{
    delete tester_;
    tester_ = nullptr;
}

void ActionsPanelTest::resetFixtureState()
{
    QDir presets(presetsDir_.path());
    for (const QFileInfo& info : presets.entryInfoList(
             {QStringLiteral("*.atn"), QStringLiteral("*.txt")}, QDir::Files)) {
        QFile::remove(info.absoluteFilePath());
    }
    engine_.resetFixture();
    executed_.clear();
    modalFlags_.clear();
    paramsSeen_.clear();
    toolParams_.clear();
    executorSucceeds_ = true;
    stopContinue_ = true;
    stopSeen_ = false;
    stopMessage_.clear();
    confirmResult_ = true;
    confirmCount_ = 0;
    commandRan_ = false;
    historyCount_ = 0;
    dialogPath_.clear();
    pathSelected_ = false;
    lastError_.clear();
    panel_->setPersistEnabledForTest(false);
    panel_->setWindowsPlatformForTest(false);
    panel_->setShortcutProviderForTest(pictura::ActionsPanel::ShortcutProvider());
    panel_->setDocumentFactsProvider({});
    panel_->setPathStepExecutor({});
    panel_->setToolStepExecutor([this](const QJsonObject& params) {
        toolParams_ << params;
        return true;
    });
    panel_->setButtonMode(false);
    panel_->setAllowToolRecording(false);
    panel_->cancelPlaybackForTest();
    panel_->refresh();
    panel_->collapseAllForTest();
    panel_->selectNodesForTest({});
    panel_->refresh();
}

void ActionsPanelTest::ensurePanel()
{
    pictura::PanelColumn* column = window_->columnForPanel(QStringLiteral("actionsPanel"));
    if (!column) {
        return;
    }
    column->showPanel(QStringLiteral("actionsPanel"), true);
    if (pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("actionsPanel"))) {
        group->setCurrentPanel(QStringLiteral("actionsPanel"));
    }
    if (column->railModeForTest()) {
        column->openIconFlyoutForTest(QStringLiteral("actionsPanel"));
    }
    window_->show();
    QCoreApplication::processEvents();
}

QMenu* ActionsPanelTest::liveMenu()
{
    ensurePanel();
    pictura::PanelGroup* group =
        window_->columnForPanel(QStringLiteral("actionsPanel"))->groupForPanel(
            QStringLiteral("actionsPanel"));
    if (!group || !group->headerMenuButtonForTest()) {
        return nullptr;
    }
    QMenu* menu = group->headerMenuButtonForTest()->menu();
    if (menu) {
        emit menu->aboutToShow();
    }
    return menu;
}

QAction* ActionsPanelTest::menuActionFor(QMenu* menu, const QString& id) const
{
    if (!menu) {
        return nullptr;
    }
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == id) {
            return action;
        }
    }
    return nullptr;
}

QModelIndex ActionsPanelTest::indexFor(const QString& id) const
{
    return panel_->modelForTest()->indexForId(id);
}

QModelIndex ActionsPanelTest::modalIndexFor(const QString& id) const
{
    return indexFor(id).siblingAtColumn(1);
}

void ActionsPanelTest::advanceUntilDone()
{
    int guard = 0;
    while (panel_->isPlaying() && guard++ < 200) {
        panel_->advancePlaybackForTest();
    }
}

void ActionsPanelTest::selectAndRefresh(const QString& id)
{
    panel_->selectNodesForTest(id.isEmpty() ? QStringList() : QStringList{id});
    panel_->refresh();
}

QVariant ActionsPanelTest::checkState(const QModelIndex& index) const
{
    return index.data(Qt::CheckStateRole);
}

int ActionsPanelTest::selectedCount() const
{
    return panel_->selectedNodeIds().size();
}

void ActionsPanelTest::ap_default_hidden()
{
    QVERIFY(!window_->panelColumn()->isPanelVisible(QStringLiteral("actionsPanel")));
    QVERIFY(window_->panelColumn()->groupForPanel(QStringLiteral("actionsPanel")) != nullptr);
    QAction* action = window_->registry()->action(QStringLiteral("window.panels.actions"));
    QVERIFY(action != nullptr);
    QVERIFY(window_->registry()->shortcuts(QStringLiteral("window.panels.actions"))
                .contains(QKeySequence(QStringLiteral("F9"))));
    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY(window_->panelColumn()->isPanelVisible(QStringLiteral("actionsPanel")));
    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY(!window_->panelColumn()->isPanelVisible(QStringLiteral("actionsPanel")));
}

void ActionsPanelTest::ap_exists()
{
    QWidget* widget = window_->findChild<QWidget*>(QStringLiteral("actionsPanel"));
    QVERIFY(widget != nullptr);
    QVERIFY(qobject_cast<pictura::ActionsPanel*>(widget) != nullptr);
    QVERIFY(window_->findChild<QDockWidget*>(QStringLiteral("actionsPanel")) == nullptr);
    QVERIFY(window_->panelColumn()->groupForPanel(QStringLiteral("actionsPanel")) != nullptr);
}

void ActionsPanelTest::pr_panel_set()
{
    const QStringList names = {
        QStringLiteral("layersPanel"),       QStringLiteral("historyPanel"),
        QStringLiteral("navigatorPanel"),    QStringLiteral("colorPanel"),
        QStringLiteral("swatchesPanel"),     QStringLiteral("stylesPanel"),
        QStringLiteral("infoPanel"),         QStringLiteral("histogramPanel"),
        QStringLiteral("gradientsPanel"),    QStringLiteral("patternsPanel"),
        QStringLiteral("notesPanel"),        QStringLiteral("propertiesPanel"),
        QStringLiteral("adjustmentsPanel"),  QStringLiteral("channelsPanel"),
        QStringLiteral("pathsPanel"),        QStringLiteral("actionsPanel"),
        QStringLiteral("brushPanel"),        QStringLiteral("cloneSourcePanel"),
        QStringLiteral("characterPanel"),    QStringLiteral("paragraphPanel"),
        QStringLiteral("glyphsPanel"),       QStringLiteral("paragraphStylesPanel"),
    };
    for (const QString& name : names) {
        QVERIFY2(window_->columnForPanel(name) != nullptr, qPrintable(name));
        QVERIFY2(qobject_cast<QWidget*>(window_->findChild<QWidget*>(name)) != nullptr,
                 qPrintable(name));
    }
}

void ActionsPanelTest::pr_actions_content()
{
    auto* actions = window_->findChild<pictura::ActionsPanel*>(QStringLiteral("actionsPanel"));
    QVERIFY(actions != nullptr);
    QVERIFY(window_->findChild<pictura::PlaceholderPanel*>(QStringLiteral("actionsPanel"))
            == nullptr);
    QVERIFY(actions->treeForTest() != nullptr);
    QVERIFY(actions->buttonViewForTest() != nullptr);
    QVERIFY(actions->toolbarButtonsForTest().size() == 6);
}

void ActionsPanelTest::ap_hosts()
{
    ensurePanel();
    pictura::PanelGroup* group =
        window_->columnForPanel(QStringLiteral("actionsPanel"))->groupForPanel(
            QStringLiteral("actionsPanel"));
    QVERIFY(group != nullptr);
    group->setFlyout(true);
    group->setFloating(true);
    QCoreApplication::processEvents();
    QVERIFY(panel_->isVisible());
    QVERIFY(panel_->treeForTest()->isVisible());
    QVERIFY(panel_->toolbarButtonsForTest().first()->isVisible());
    QMenu* menu = liveMenu();
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.buttonMode")) != nullptr);
    group->setFlyout(false);
    group->setFloating(false);
}

void ActionsPanelTest::ap_flyout_width()
{
    ensurePanel();
    panel_->resize(300, 500);
    QCoreApplication::processEvents();
    QCOMPARE(panel_->treeForTest()->horizontalScrollBar()->maximum(), 0);
    for (QToolButton* button : panel_->toolbarButtonsForTest()) {
        QVERIFY(button != nullptr);
        QVERIFY(button->isVisible());
        QVERIFY(button->geometry().right() <= panel_->width());
    }
}

void ActionsPanelTest::ap_menu_order()
{
    QMenu* menu = liveMenu();
    QVERIFY(menu != nullptr);
    QStringList texts;
    for (QAction* action : menu->actions()) {
        texts << (action->isSeparator() ? QStringLiteral("-") : action->text());
    }
    const QStringList expected = {
        QStringLiteral("Button Mode"),          QStringLiteral("-"),
        QStringLiteral("New Action…"),          QStringLiteral("New Set…"),
        QStringLiteral("Duplicate"),
        QStringLiteral("Delete"),               QStringLiteral("Play"),
        QStringLiteral("-"),                    QStringLiteral("Start Recording"),
        QStringLiteral("Record Again…"),        QStringLiteral("-"),
        QStringLiteral("Insert Menu Item…"),    QStringLiteral("Insert Stop…"),
        QStringLiteral("Insert Conditional…"),  QStringLiteral("Insert Path"),
        QStringLiteral("-"),                    QStringLiteral("Action Options…"),
        QStringLiteral("Playback Options…"),    QStringLiteral("Allow Tool Recording"),
        QStringLiteral("-"),                    QStringLiteral("Clear All Actions"),
        QStringLiteral("Reset Actions"),        QStringLiteral("Load Actions…"),
        QStringLiteral("Replace Actions…"),     QStringLiteral("Save Actions…"),
        QStringLiteral("-"),                    QStringLiteral("Close"),
        QStringLiteral("Close Tab Group"),
    };
    QCOMPARE(texts, expected);
}

void ActionsPanelTest::ap_menu_close()
{
    QMenu* menu = liveMenu();
    QVERIFY(menu != nullptr);
    int close = 0;
    int closeGroup = 0;
    for (QAction* action : menu->actions()) {
        close += action->text() == QStringLiteral("Close") ? 1 : 0;
        closeGroup += action->text() == QStringLiteral("Close Tab Group") ? 1 : 0;
    }
    QCOMPARE(close, 1);
    QCOMPARE(closeGroup, 1);
}

void ActionsPanelTest::ap_menu_checks()
{
    panel_->setButtonMode(true);
    panel_->setAllowToolRecording(true);
    QMenu* menu = liveMenu();
    QAction* buttonMode = menuActionFor(menu, QStringLiteral("actions.buttonMode"));
    QAction* allow = menuActionFor(menu, QStringLiteral("actions.allowToolRecording"));
    QVERIFY(buttonMode && allow);
    QVERIFY(buttonMode->isChecked());
    QVERIFY(allow->isChecked());
    allow->trigger();
    QVERIFY(!engine_.allowToolRecording());
    buttonMode->trigger();
    QVERIFY(!panel_->buttonMode());
}

void ActionsPanelTest::ap_menu_sets()
{
    const QString path = presetsDir_.filePath(QStringLiteral("My Set.atn"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("atn");
    file.close();
    QMenu* menu = liveMenu();
    bool found = false;
    for (QAction* action : menu->actions()) {
        if (action->text() == QStringLiteral("My Set")) {
            found = true;
            action->trigger();
            break;
        }
    }
    QVERIFY(found);
    QCoreApplication::processEvents();
    QCOMPARE(engine_.loadedPath(), path);
    QVERIFY(engine_.idForName(QStringLiteral("My Set")) != QString());
}

void ActionsPanelTest::ap_toolbar_order()
{
    const QStringList expected = {
        QStringLiteral("actionsStopButton"),      QStringLiteral("actionsRecordButton"),
        QStringLiteral("actionsPlayButton"),      QStringLiteral("actionsNewSetButton"),
        QStringLiteral("actionsNewActionButton"), QStringLiteral("actionsDeleteButton"),
    };
    const QList<QToolButton*> buttons = panel_->toolbarButtonsForTest();
    QCOMPARE(buttons.size(), expected.size());
    for (int i = 0; i < expected.size(); ++i) {
        QCOMPARE(buttons.at(i)->objectName(), expected.at(i));
        QVERIFY(!buttons.at(i)->toolTip().isEmpty());
    }
}

void ActionsPanelTest::ap_toolbar_record()
{
    const QString actionId = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionId);
    QToolButton* record = panel_->toolbarButtonForTest(QStringLiteral("actionsRecordButton"));
    QToolButton* stop = panel_->toolbarButtonForTest(QStringLiteral("actionsStopButton"));
    QVERIFY(record && stop);
    QVERIFY(record->isEnabled());
    record->click();
    QVERIFY(engine_.isRecording());
    QVERIFY(record->isChecked());
    QVERIFY(stop->isEnabled());
    stop->click();
    QVERIFY(!engine_.isRecording());
    QVERIFY(!record->isChecked());
}

void ActionsPanelTest::ap_toolbar_playback()
{
    const QString actionId = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionId);
    QVERIFY(panel_->playSelection());
    QVERIFY(panel_->isPlaying());
    QVERIFY(!panel_->toolbarButtonForTest(QStringLiteral("actionsRecordButton"))->isEnabled());
    QVERIFY(!panel_->toolbarButtonForTest(QStringLiteral("actionsNewActionButton"))->isEnabled());
    QVERIFY(!panel_->toolbarButtonForTest(QStringLiteral("actionsDeleteButton"))->isEnabled());
    QVERIFY(panel_->toolbarButtonForTest(QStringLiteral("actionsStopButton"))->isEnabled());
    panel_->cancelPlaybackForTest();
}

void ActionsPanelTest::ap_dialog_new_action()
{
    pictura::NewActionDialog dialog;
    dialog.setSets({{QStringLiteral("n1"), QStringLiteral("Set A")}});
    QString error;
    QVERIFY(!dialog.validate(&error));
    QVERIFY(!error.isEmpty());
    dialog.findChild<QLineEdit*>(QStringLiteral("actionsDialogName"))
        ->setText(QStringLiteral("Fresh Action"));
    QVERIFY(dialog.validate(&error));
    QVERIFY(panel_->createActionForTest(QStringLiteral("Fresh Action"),
                                        engine_.idForName(QStringLiteral("Set A")),
                                        Qt::Key_F5, false, false, 3, true));
    QVERIFY(engine_.idForName(QStringLiteral("Fresh Action")) != QString());
    QVERIFY(engine_.isRecording());
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(!engine_.isRecording());
}

void ActionsPanelTest::ap_dialog_playback()
{
    pictura::PlaybackOptionsDialog dialog;
    dialog.setOptions(pictura::PlaybackOptionsDialog::StepByStep, 0.0);
    QCOMPARE(dialog.mode(), static_cast<int>(pictura::PlaybackOptionsDialog::StepByStep));
    QVERIFY(panel_->setPlaybackOptionsForTest(
        pictura::PlaybackOptionsDialog::Pause, 1.5));
    QCOMPARE(engine_.playbackMode(), static_cast<int>(pictura::PlaybackOptionsDialog::Pause));
    QCOMPARE(engine_.playbackPauseSeconds(), 1.5);
    pictura::SessionState state;
    panel_->writeSessionState(&state);
    QCOMPARE(state.actionsPlaybackMode,
             static_cast<int>(pictura::PlaybackOptionsDialog::Pause));
    QCOMPARE(state.actionsPlaybackPause, 1.5);
}

void ActionsPanelTest::ap_dialog_action_options()
{
    const QString actionId = engine_.idForName(QStringLiteral("Action One"));
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    pictura::ActionOptionsDialog dialog;
    dialog.setSets({{setB, QStringLiteral("Set B")}}, setB);
    dialog.setValues(QStringLiteral("Moved"), Qt::Key_F6, true, false, 2);
    QString error;
    QVERIFY(dialog.validate(&error));
    QVERIFY(panel_->applyActionOptionsForTest(actionId, QStringLiteral("Moved"), setB,
                                              Qt::Key_F6, true, false, 2));
    QCoreApplication::processEvents();
    const ActionNodeRow row = panel_->modelForTest()->rowForId(actionId);
    QCOMPARE(row.name, QStringLiteral("Moved"));
    QCOMPARE(row.parentId, setB);
    QCOMPARE(row.functionKey, static_cast<int>(Qt::Key_F6));
    QVERIFY(row.functionShift);
}

void ActionsPanelTest::ap_dialog_set_options()
{
    const QString setId = engine_.idForName(QStringLiteral("Set B"));
    pictura::SetNameDialog dialog(QStringLiteral("Set Options"),
                                  QStringLiteral("Set B"));
    QVERIFY(dialog.findChild<QLineEdit*>(QStringLiteral("actionsDialogName")));
    QVERIFY(panel_->renameSetForTest(setId, QStringLiteral("Renamed Set")));
    QCoreApplication::processEvents();
    QCOMPARE(panel_->modelForTest()->rowForId(setId).name, QStringLiteral("Renamed Set"));
}

void ActionsPanelTest::ap_tree_shape()
{
    pictura::ActionsTreeModel* model = panel_->modelForTest();
    QCOMPARE(model->columnCount(), 3);
    QCOMPARE(model->rowCount(), 2);
    QCOMPARE(model->data(model->index(0, 2), Qt::DisplayRole).toString(),
             QStringLiteral("Set A"));
    QCOMPARE(model->data(model->index(1, 2), Qt::DisplayRole).toString(),
             QStringLiteral("Set B"));
    const QModelIndex setA = model->index(0, 0);
    QCOMPARE(model->rowCount(setA), 2);
    QCOMPARE(model->rowCount(setA), 2);
    const QModelIndex actionOne = model->index(0, 0, setA);
    QCOMPARE(model->rowCount(actionOne), 2);
    QCOMPARE(model->data(model->index(0, 2, actionOne), Qt::DisplayRole).toString(),
             QStringLiteral("Step One"));
    QCOMPARE(model->headerData(2, Qt::Horizontal, Qt::DisplayRole).toString(),
             QStringLiteral("Name"));
}

void ActionsPanelTest::ap_tree_expansion()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    const QModelIndex setA = indexFor(engine_.idForName(QStringLiteral("Set A")));
    const QModelIndex actionOne = indexFor(engine_.idForName(QStringLiteral("Action One")));
    const QModelIndex stepOne = indexFor(engine_.idForName(QStringLiteral("Step One")));
    tree->expand(setA);
    tree->expand(actionOne);
    tree->expand(stepOne);
    QVERIFY(tree->isExpanded(setA));
    QVERIFY(tree->isExpanded(actionOne));
    QVERIFY(tree->isExpanded(stepOne));
    pictura::SessionState state;
    panel_->writeSessionState(&state);
    QCOMPARE(state.actionsExpanded.size(), 3);
    panel_->collapseAllForTest();
    QVERIFY(!tree->isExpanded(setA));
    panel_->applySessionState(state);
    QCoreApplication::processEvents();
    QVERIFY(tree->isExpanded(indexFor(engine_.idForName(QStringLiteral("Set A")))));
    QVERIFY(tree->isExpanded(indexFor(engine_.idForName(QStringLiteral("Action One")))));
    QVERIFY(tree->isExpanded(indexFor(engine_.idForName(QStringLiteral("Step One")))));
}

void ActionsPanelTest::ap_tree_alt_expand()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    QCOMPARE(tree->treePosition(), 2);
    const QModelIndex setA = indexFor(engine_.idForName(QStringLiteral("Set A")));
    const QModelIndex actionOne = indexFor(engine_.idForName(QStringLiteral("Action One")));
    tree->expand(setA);
    tree->collapse(actionOne);
    QCoreApplication::processEvents();
    const QModelIndex actionName = actionOne.siblingAtColumn(2);
    tree->scrollTo(actionName);
    QCoreApplication::processEvents();
    const QRect rect = tree->visualRect(actionName);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::AltModifier,
                      QPoint(rect.left() + 5, rect.center().y()));
    QVERIFY(tree->isExpanded(actionOne));
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::AltModifier,
                      QPoint(rect.left() + 5, rect.center().y()));
    QVERIFY(!tree->isExpanded(actionOne));
}

void ActionsPanelTest::ap_check_step()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->modelForTest()->setData(indexFor(stepOne), Qt::Unchecked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    selectAndRefresh(actionOne);
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(executed_, QStringList{QStringLiteral("layer.flatten.image")});
}

void ActionsPanelTest::ap_check_parent()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->modelForTest()->setData(indexFor(actionOne), Qt::Unchecked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    for (const QString& name : {QStringLiteral("Step One"), QStringLiteral("Step Two")}) {
        QVERIFY(!panel_->modelForTest()->rowForId(engine_.idForName(name)).enabled);
    }
    QVERIFY(panel_->modelForTest()->setData(indexFor(actionOne), Qt::Checked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    for (const QString& name : {QStringLiteral("Step One"), QStringLiteral("Step Two")}) {
        QVERIFY(panel_->modelForTest()->rowForId(engine_.idForName(name)).enabled);
    }
}

void ActionsPanelTest::ap_check_alt()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    panel_->modelForTest()->toggleSiblingsIncluded(indexFor(stepOne));
    panel_->refresh();
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).enabled);
    QVERIFY(!panel_->modelForTest()->rowForId(stepTwo).enabled);
    QVERIFY(panel_->modelForTest()->setData(indexFor(stepOne), Qt::Unchecked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    panel_->modelForTest()->toggleSiblingsIncluded(indexFor(stepOne));
    panel_->refresh();
    QVERIFY(panel_->modelForTest()->rowForId(stepTwo).enabled);
}

void ActionsPanelTest::ap_check_alt_click()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    tree->expand(indexFor(engine_.idForName(QStringLiteral("Set A"))));
    tree->expand(indexFor(engine_.idForName(QStringLiteral("Action One"))));
    QCoreApplication::processEvents();
    QModelIndex one = indexFor(stepOne);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::AltModifier,
                      tree->visualRect(one).center());
    QCoreApplication::processEvents();
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).enabled);
    QVERIFY(!panel_->modelForTest()->rowForId(stepTwo).enabled);
    one = indexFor(stepOne);
    QVERIFY(panel_->modelForTest()->setData(one, Qt::Unchecked, Qt::CheckStateRole));
    panel_->refresh();
    one = indexFor(stepOne);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::AltModifier,
                      tree->visualRect(one).center());
    QCoreApplication::processEvents();
    QVERIFY(panel_->modelForTest()->rowForId(stepTwo).enabled);
}

void ActionsPanelTest::ap_check_partial()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->modelForTest()->setData(indexFor(stepOne), Qt::Unchecked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    const QModelIndex actionIndex = indexFor(actionOne);
    QCOMPARE(checkState(actionIndex).toInt(), static_cast<int>(Qt::PartiallyChecked));
    QVERIFY(actionIndex.data(Qt::DecorationRole).value<QIcon>().isNull() == false);
    QCOMPARE(actionIndex.data(Qt::ForegroundRole).value<QColor>(),
             QColor(QStringLiteral("#e74c3c")));
    QVERIFY(actionIndex.data(Qt::AccessibleTextRole).toString().contains(
        QStringLiteral("Partially")));
}

void ActionsPanelTest::ap_modal_toggle()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    QVERIFY(panel_->modelForTest()->setData(modalIndexFor(stepOne), Qt::Checked,
                                            Qt::CheckStateRole));
    panel_->refresh();
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).modal);
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QVERIFY(modalFlags_.first());
}

void ActionsPanelTest::ap_modal_required()
{
    engine_.setDialogOptions(QStringLiteral("Step One"), 1, true);
    engine_.setDialogOptions(QStringLiteral("Step Two"), 2, false);
    panel_->refresh();
    const QModelIndex one = modalIndexFor(engine_.idForName(QStringLiteral("Step One")));
    const QModelIndex two = modalIndexFor(engine_.idForName(QStringLiteral("Step Two")));
    QVERIFY(!(panel_->modelForTest()->flags(modalIndexFor(engine_.idForName(
                 QStringLiteral("Step One"))))
              & Qt::ItemIsUserCheckable));
    QVERIFY(!(panel_->modelForTest()->flags(modalIndexFor(engine_.idForName(
                 QStringLiteral("Step Two"))))
              & Qt::ItemIsUserCheckable));
    QCOMPARE(checkState(modalIndexFor(engine_.idForName(QStringLiteral("Step One")))).toInt(),
             static_cast<int>(Qt::Checked));
    QCOMPARE(checkState(modalIndexFor(engine_.idForName(QStringLiteral("Step Two")))).toInt(),
             static_cast<int>(Qt::Unchecked));
}

void ActionsPanelTest::ap_modal_menu_item()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(panel_->insertMenuItemForTest(QStringLiteral("edit.fill")));
    panel_->refresh();
    const QStringList steps = engine_.stepIdsUnder(QStringLiteral("Action One"));
    const QString menuStepId = steps.last();
    const ActionNodeRow menuStep = panel_->modelForTest()->rowForId(menuStepId);
    QCOMPARE(menuStep.dialogOptions, 3);
    QVERIFY(!menuStep.modal);
    QVERIFY(!(panel_->modelForTest()->flags(modalIndexFor(menuStepId))
              & Qt::ItemIsUserCheckable));
    selectAndRefresh(menuStepId);
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCOMPARE(executed_.last(), QStringLiteral("edit.fill"));
    QVERIFY(modalFlags_.last());
}

void ActionsPanelTest::ap_modal_parent()
{
    engine_.setDialogOptions(QStringLiteral("Step One"), 0, true);
    engine_.setDialogOptions(QStringLiteral("Step Two"), 0, false);
    panel_->refresh();
    const QModelIndex actionIndex =
        modalIndexFor(engine_.idForName(QStringLiteral("Action One")));
    QCOMPARE(actionIndex.data(Qt::CheckStateRole).toInt(),
             static_cast<int>(Qt::PartiallyChecked));
    QCOMPARE(actionIndex.data(Qt::ForegroundRole).value<QColor>(),
             QColor(QStringLiteral("#e74c3c")));
}

#include "tst_actions_panel_cases.cpp"

QTEST_MAIN(ActionsPanelTest)
#include "tst_actions_panel.moc"
