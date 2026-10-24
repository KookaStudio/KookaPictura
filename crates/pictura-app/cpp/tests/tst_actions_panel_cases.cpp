// Included by tst_actions_panel.cpp; kept separate to hold the test-file size cap.
void ActionsPanelTest::ap_select()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    tree->expand(indexFor(engine_.idForName(QStringLiteral("Set A"))));
    tree->expand(indexFor(engine_.idForName(QStringLiteral("Action One"))));
    const QModelIndex one =
        indexFor(engine_.idForName(QStringLiteral("Step One"))).siblingAtColumn(2);
    const QModelIndex two =
        indexFor(engine_.idForName(QStringLiteral("Step Two"))).siblingAtColumn(2);
    tree->scrollTo(one);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier,
                      tree->visualRect(one).center());
    QCOMPARE(selectedCount(), 1);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::ShiftModifier,
                      tree->visualRect(two).center());
    QCOMPARE(selectedCount(), 2);
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::ControlModifier,
                      tree->visualRect(one).center());
    QCOMPARE(selectedCount(), 1);
    QCOMPARE(panel_->selectedNodeIds().first(), engine_.idForName(QStringLiteral("Step Two")));
}

void ActionsPanelTest::ap_rename()
{
    const QString actionId = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->modelForTest()->setData(indexFor(actionId).siblingAtColumn(2),
                                            QStringLiteral("Renamed Action"), Qt::EditRole));
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(engine_.idForName(QStringLiteral("Renamed Action")), actionId);
    QVERIFY(panel_->modelForTest()->data(indexFor(actionId).siblingAtColumn(2))
                .toString()
                .contains(QStringLiteral("Renamed Action")));
    panel_->saveLibraryNowForTest();
    engine_.clearForLoadTest();
    panel_->loadLibraryNowForTest();
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Renamed Action")) != QString());
}

void ActionsPanelTest::ap_overwrite()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    QVERIFY(panel_->overwriteStepForTest(stepOne));
    QCoreApplication::processEvents();
    QVERIFY(engine_.idForName(QStringLiteral("Step One")) != QString());
    bool edited = false;
    for (const ActionNodeRow& row : engine_.snapshot()) {
        if (row.id == stepOne) {
            edited = row.params.value(QStringLiteral("edited")).toBool();
        }
    }
    QVERIFY(edited);
}

void ActionsPanelTest::ap_reorder()
{
    pictura::ActionsTreeModel* model = panel_->modelForTest();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionTwo = engine_.idForName(QStringLiteral("Action Two"));
    QScopedPointer<QMimeData> mime(model->mimeData({indexFor(stepOne)}));
    QVERIFY(mime);
    QVERIFY(model->dropMimeData(mime.data(), Qt::MoveAction, 0, 0, indexFor(actionTwo)));
    panel_->refresh();
    QCOMPARE(model->rowForId(stepOne).parentId, actionTwo);

    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    mime.reset(model->mimeData({indexFor(actionOne)}));
    QVERIFY(model->dropMimeData(mime.data(), Qt::MoveAction, 0, 0, indexFor(setB)));
    panel_->refresh();
    QCOMPARE(model->rowForId(actionOne).parentId, setB);

    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    mime.reset(model->mimeData({indexFor(setA)}));
    QVERIFY(model->dropMimeData(mime.data(), Qt::MoveAction, 0, 0, QModelIndex()));
    panel_->refresh();
    QCOMPARE(model->rowForId(setA).parentId, QString());
    QCOMPARE(model->data(model->index(1, 2)).toString(), QStringLiteral("Set A"));
}

void ActionsPanelTest::ap_duplicate()
{
    pictura::ActionsTreeModel* model = panel_->modelForTest();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionTwo = engine_.idForName(QStringLiteral("Action Two"));
    const int before = engine_.stepCount();
    QScopedPointer<QMimeData> mime(model->mimeData({indexFor(stepOne)}));
    QVERIFY(model->dropMimeData(mime.data(), Qt::CopyAction, 0, 0, indexFor(actionTwo)));
    panel_->refresh();
    QCOMPARE(engine_.stepCount(), before + 1);
    QCOMPARE(model->rowForId(stepOne).parentId,
             engine_.idForName(QStringLiteral("Action One")));
    int copies = 0;
    for (const ActionNodeRow& row : engine_.snapshot()) {
        if (row.parentId == actionTwo && row.kind == pictura::actionKindStep()) {
            ++copies;
        }
    }
    QCOMPARE(copies, 2);
}

void ActionsPanelTest::ap_delete_fast()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(stepOne);
    QVERIFY(panel_->deleteSelectionForTest(true));
    QCOMPARE(confirmCount_, 0);
    QVERIFY(engine_.idForName(QStringLiteral("Step One")).isEmpty());

    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    selectAndRefresh(stepTwo);
    confirmResult_ = true;
    QVERIFY(panel_->deleteSelectionForTest(false));
    QCOMPARE(confirmCount_, 1);
    QVERIFY(engine_.idForName(QStringLiteral("Step Two")).isEmpty());
}

void ActionsPanelTest::ap_button_mode()
{
    ensurePanel();
    panel_->setButtonMode(true);
    QVERIFY(panel_->buttonMode());
    const QStringList ids = panel_->buttonViewForTest()->actionIdsForTest();
    QCOMPARE(ids.size(), 3);
    QCOMPARE(ids.at(0), engine_.idForName(QStringLiteral("Action One")));
    QCOMPARE(ids.at(1), engine_.idForName(QStringLiteral("Action Two")));
    QCOMPARE(ids.at(2), engine_.idForName(QStringLiteral("Action Three")));
    QCOMPARE(panel_->buttonViewForTest()->actionColorsForTest().at(0),
             pictura::actionColor(4));
    QVERIFY(panel_->buttonViewForTest()->isVisible());
}

void ActionsPanelTest::ap_button_play()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    panel_->modelForTest()->setData(indexFor(stepOne), Qt::Unchecked, Qt::CheckStateRole);
    panel_->refresh();
    panel_->setButtonMode(true);
    panel_->buttonViewForTest()->activateActionForTest(0);
    advanceUntilDone();
    QCOMPARE(executed_, QStringList{QStringLiteral("layer.flatten.image")});
}

void ActionsPanelTest::ap_button_restore()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    const QModelIndex actionOne = indexFor(engine_.idForName(QStringLiteral("Action One")));
    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    tree->expand(actionOne);
    selectAndRefresh(stepTwo);
    panel_->setButtonMode(true);
    panel_->setButtonMode(false);
    QVERIFY(tree->isExpanded(indexFor(engine_.idForName(QStringLiteral("Action One")))));
    QCOMPARE(panel_->selectedNodeIds(), QStringList{stepTwo});
}

void ActionsPanelTest::ap_menu_enable()
{
    QMenu* menu = liveMenu();
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.newAction"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.duplicate"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.save"))->isEnabled());
    QAction* clear = menuActionFor(menu, QStringLiteral("actions.clear"));
    QVERIFY(clear && clear->isEnabled());

    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    menu = liveMenu();
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.play"))->isEnabled());
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.recordAgain"))->isEnabled());
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.insertMenuItem"))->isEnabled());
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.options"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.save"))->isEnabled());

    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    selectAndRefresh(setA);
    menu = liveMenu();
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.save"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.recordAgain"))->isEnabled());
    QCOMPARE(menuActionFor(menu, QStringLiteral("actions.options"))->text(),
             QStringLiteral("Set Options…"));

    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(stepOne);
    menu = liveMenu();
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.options"))->isEnabled());
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.insertStop"))->isEnabled());

    selectAndRefresh(actionOne);
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(engine_.isRecording());
    menu = liveMenu();
    QCOMPARE(menuActionFor(menu, QStringLiteral("actions.toggleRecording"))->text(),
             QStringLiteral("Stop Recording"));
    QVERIFY(menuActionFor(menu, QStringLiteral("actions.toggleRecording"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.play"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.newAction"))->isEnabled());
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));

    QVERIFY(panel_->playSelection());
    menu = liveMenu();
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.play"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.load"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.save"))->isEnabled());
    QVERIFY(!menuActionFor(menu, QStringLiteral("actions.duplicate"))->isEnabled());
    panel_->cancelPlaybackForTest();
}

void ActionsPanelTest::ap_insert_stop()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(panel_->insertStopForTest(QStringLiteral("Pause here"), true));
    panel_->refresh();
    stopContinue_ = true;
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QVERIFY(stopSeen_);
    QCOMPARE(stopMessage_, QStringLiteral("Pause here"));
}

void ActionsPanelTest::ap_insert_menu()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(panel_->insertMenuItemForTest(QStringLiteral("edit.fill")));
    panel_->refresh();
    const QString menuStep = engine_.stepIdsUnder(QStringLiteral("Action One")).last();
    selectAndRefresh(menuStep);
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCOMPARE(executed_.last(), QStringLiteral("edit.fill"));
    QVERIFY(modalFlags_.last());
}

void ActionsPanelTest::ap_insert_conditional()
{
    pictura::InsertConditionalDialog dialog;
    dialog.setActions({{QStringLiteral("foreign"), QStringLiteral("Foreign")}});
    dialog.setAllowedActionIds({engine_.idForName(QStringLiteral("Action Two"))});
    QString error;
    QVERIFY(!dialog.validate(&error));
    dialog.findChild<QComboBox*>(QStringLiteral("actionsConditionalThen"))
        ->setCurrentIndex(1);
    QVERIFY(!dialog.validate(&error));
    QVERIFY(error.contains(QStringLiteral("set")));

    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(!panel_->insertConditionalForTest(QStringLiteral("square"), QString(), QString(),
                                              &error));
    QVERIFY(!panel_->insertConditionalForTest(QStringLiteral("square"),
                                              QStringLiteral("foreign"), QString(), &error));
    QVERIFY(panel_->insertConditionalForTest(
        QStringLiteral("square"), engine_.idForName(QStringLiteral("Action Two")), QString(),
        &error));
}

void ActionsPanelTest::ap_record_again()
{
    engine_.setDialogOptions(QStringLiteral("Step One"), 0, true);
    engine_.setDialogOptions(QStringLiteral("Step Two"), 2, true);
    panel_->refresh();
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    selectAndRefresh(actionOne);
    panel_->performPanelMenuAction(QStringLiteral("actions.recordAgain"));
    QCoreApplication::processEvents();
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).params
                .value(QStringLiteral("edited"))
                .toBool());
    QVERIFY(!panel_->modelForTest()->rowForId(stepTwo).params
                 .value(QStringLiteral("edited"))
                 .toBool());
}

void ActionsPanelTest::ap_allow_tool()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    const int before = engine_.stepCount();
    panel_->setAllowToolRecording(true);
    QVERIFY(panel_->recordToolStrokeForTest(QStringLiteral("brush"), {}, 10));
    QCOMPARE(engine_.stepCount(), before + 1);
    panel_->setAllowToolRecording(false);
    QVERIFY(!panel_->recordToolStrokeForTest(QStringLiteral("brush"), {}, 10));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
}

void ActionsPanelTest::ap_record_escape()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(engine_.isRecording());
    ensurePanel();
    QTest::keyClick(panel_->treeForTest(), Qt::Key_Escape);
    QVERIFY(!engine_.isRecording());
}

void ActionsPanelTest::ap_play_from_step()
{
    const QString stepTwo = engine_.idForName(QStringLiteral("Step Two"));
    selectAndRefresh(stepTwo);
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(executed_, QStringList{QStringLiteral("layer.flatten.image")});
}

void ActionsPanelTest::ap_play_single()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(stepOne);
    QVERIFY(panel_->playSelection(true));
    advanceUntilDone();
    QCOMPARE(executed_, QStringList{QStringLiteral("edit.fill")});
}

void ActionsPanelTest::ap_play_key()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, Qt::Key_F5, true, false, nullptr));
    panel_->refresh();
    QVERIFY(panel_->handleFunctionKeyForTest(Qt::Key_F5, Qt::ShiftModifier));
    advanceUntilDone();
    QCOMPARE(executed_.size(), 2);
}

void ActionsPanelTest::ap_play_concurrent()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(panel_->playSelection());
    QVERIFY(!panel_->playSelection());
    QCOMPARE(executed_.size(), 0);
    panel_->cancelPlaybackForTest();
}

void ActionsPanelTest::ap_play_set()
{
    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    selectAndRefresh(setA);
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(executed_, (QStringList{QStringLiteral("edit.fill"),
                                     QStringLiteral("layer.flatten.image"),
                                     QStringLiteral("image.rotate90cw")}));
}

void ActionsPanelTest::ae_play_modal()
{
    engine_.setDialogOptions(QStringLiteral("Step One"), 0, true);
    panel_->refresh();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY(modalFlags_.first());
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).params
                .value(QStringLiteral("edited"))
                .toBool());
    panel_->cancelPlaybackForTest();
}

void ActionsPanelTest::ae_play_recorded()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QVERIFY(!modalFlags_.first());
    QCOMPARE(paramsSeen_.first().value(QStringLiteral("value")).toInt(), 1);
    panel_->cancelPlaybackForTest();
}

void ActionsPanelTest::ae_play_specific()
{
    engine_.setDialogOptions(QStringLiteral("Step One"), 3, false);
    panel_->refresh();
    selectAndRefresh(engine_.idForName(QStringLiteral("Step One")));
    QVERIFY(panel_->playSelection(true));
    panel_->advancePlaybackForTest();
    QVERIFY(modalFlags_.first());
}

void ActionsPanelTest::ae_play_stop()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->insertStopForTest(QStringLiteral("Continue?"), true));
    panel_->refresh();
    stopContinue_ = true;
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QVERIFY(stopSeen_);
    QCOMPARE(stopMessage_, QStringLiteral("Continue?"));
}

void ActionsPanelTest::ae_play_menu_item()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->insertMenuItemForTest(QStringLiteral("layer.flatten.image")));
    panel_->refresh();
    const QString step = engine_.stepIdsUnder(QStringLiteral("Action One")).last();
    selectAndRefresh(step);
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCOMPARE(executed_.last(), QStringLiteral("layer.flatten.image"));
    QVERIFY(modalFlags_.last());
}

void ActionsPanelTest::ae_play_path()
{
    pathSelected_ = true;
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->insertPathForTest(QStringLiteral("Work Path")));
    panel_->refresh();
    const QString step = engine_.stepIdsUnder(QStringLiteral("Action One")).last();
    selectAndRefresh(step);
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCOMPARE(engine_.appliedPath(), QStringLiteral("Work Path"));
}

void ActionsPanelTest::ae_play_cancel()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QCOMPARE(executed_.size(), 1);
    panel_->cancelPlaybackForTest();
    QCOMPARE(executed_.size(), 1);
    QVERIFY(!panel_->isPlaying());
}

void ActionsPanelTest::ae_history_per_command()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(historyCount_, 2);
}

void ActionsPanelTest::ae_history_no_group()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(historyCount_, executed_.size());
}

void ActionsPanelTest::ae_key_precedence()
{
    auto* registry = window_->registry();
    QVERIFY(!registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());
    registry->setHandler(QStringLiteral("edit.fill"), [this] { commandRan_ = true; });
    panel_->setShortcutProviderForTest([registry](const QKeySequence& shortcut) {
        return registry->commandForShortcut(shortcut);
    });
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, Qt::Key_F5, true, false, nullptr));
    panel_->refresh();
    QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());
    ensurePanel();
    window_->activateWindow();
    QTest::keyClick(window_.get(), Qt::Key_F5, Qt::ShiftModifier);
    advanceUntilDone();
    QVERIFY(!commandRan_);
    QCOMPARE(executed_.size(), 2);
}

void ActionsPanelTest::ae_key_unique()
{
    QString error;
    QVERIFY(panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                             Qt::Key_F7, false, false, &error));
    QVERIFY(!panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action Two")),
                                              Qt::Key_F7, false, false, &error));
    QVERIFY(!error.isEmpty());
}

void ActionsPanelTest::ae_key_restore()
{
    auto* registry = window_->registry();
    const QList<QKeySequence> before = registry->shortcuts(QStringLiteral("edit.fill"));
    QVERIFY(!before.isEmpty());
    panel_->setShortcutProviderForTest([registry](const QKeySequence& shortcut) {
        return registry->commandForShortcut(shortcut);
    });
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, Qt::Key_F5, true, false, nullptr));
    QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());
    selectAndRefresh(actionOne);
    QVERIFY(panel_->deleteSelectionForTest(true));
    panel_->refresh();
    QVERIFY(!panel_->handleFunctionKeyForTest(Qt::Key_F5, Qt::ShiftModifier));
    QCOMPARE(registry->shortcuts(QStringLiteral("edit.fill")), before);
}

void ActionsPanelTest::ae_tool_replay()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->setAllowToolRecording(true);
    QVERIFY(panel_->recordToolStrokeForTest(
        QStringLiteral("brush"), QJsonObject{{QStringLiteral("geometry"), QStringLiteral("p")}},
        20));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->refresh();
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QVERIFY(executed_.contains(QStringLiteral("tool.brush")));
}

void ActionsPanelTest::ae_tool_settings()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->setAllowToolRecording(true);
    QVERIFY(panel_->recordToolStrokeForTest(
        QStringLiteral("brush"), QJsonObject{{QStringLiteral("size"), 7}}, 20));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->refresh();
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    bool seen = false;
    for (const QJsonObject& params : paramsSeen_) {
        seen = seen || params.value(QStringLiteral("size")).toInt() == 7;
    }
    QVERIFY(seen);
}

void ActionsPanelTest::ae_tool_off()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->setAllowToolRecording(false);
    const int before = engine_.stepCount();
    QVERIFY(!panel_->recordToolStrokeForTest(QStringLiteral("brush"), {}, 10));
    QCOMPARE(engine_.stepCount(), before);
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
}

void ActionsPanelTest::ae_tool_limit()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->setAllowToolRecording(true);
    const int before = engine_.stepCount();
    lastError_.clear();
    QVERIFY(!panel_->recordToolStrokeForTest(QStringLiteral("brush"), {}, 1000001));
    QCOMPARE(engine_.stepCount(), before);
    QVERIFY(lastError_.contains(QStringLiteral("payload")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
}

void ActionsPanelTest::ae_conditional_invalid()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QString error;
    QVERIFY(!panel_->insertConditionalForTest(QStringLiteral("square"), QString(), QString(),
                                              &error));
    QVERIFY(error.contains(QStringLiteral("Then")));
}

void ActionsPanelTest::ae_conditional_same_set()
{
    pictura::InsertConditionalDialog dialog;
    dialog.setActions({{QStringLiteral("foreign"), QStringLiteral("Foreign")}});
    dialog.setAllowedActionIds({engine_.idForName(QStringLiteral("Action Two"))});
    dialog.findChild<QComboBox*>(QStringLiteral("actionsConditionalThen"))->setCurrentIndex(1);
    QString error;
    QVERIFY(!dialog.validate(&error));
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    QVERIFY(!panel_->insertConditionalForTest(QStringLiteral("square"),
                                              QStringLiteral("foreign"), QString(), &error));
}

void ActionsPanelTest::ap_save_load()
{
    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    const QString path = presetsDir_.filePath(QStringLiteral("saved.atn"));
    QVERIFY(panel_->saveSetForTest(setA, path, false));
    engine_.clearForLoadTest();
    panel_->refresh();
    QVERIFY(panel_->loadForTest(path, false));
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Set A")) != QString());
    QVERIFY(engine_.idForName(QStringLiteral("Action One")) != QString());
    QCOMPARE(engine_.stepIdsUnder(QStringLiteral("Action One")).size(), 2);
}

void ActionsPanelTest::ap_replace()
{
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    const QString path = presetsDir_.filePath(QStringLiteral("replace.atn"));
    QVERIFY(panel_->saveSetForTest(setB, path, false));
    QVERIFY(panel_->createSetForTest(QStringLiteral("Extra Set")));
    QVERIFY(panel_->loadForTest(path, true));
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Set B")) != QString());
    QVERIFY(engine_.idForName(QStringLiteral("Set A")).isEmpty());
    QVERIFY(engine_.idForName(QStringLiteral("Extra Set")).isEmpty());
}

void ActionsPanelTest::ap_text_dump()
{
    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    const QString path = presetsDir_.filePath(QStringLiteral("dump.txt"));
    QVERIFY(panel_->saveSetForTest(setA, path, true));
    QVERIFY(!panel_->loadForTest(path, false));
    QVERIFY(lastError_.contains(QStringLiteral("not an action set")));
}

void ActionsPanelTest::ap_state_restore()
{
    panel_->setButtonMode(true);
    panel_->setAllowToolRecording(true);
    QVERIFY(panel_->setPlaybackOptionsForTest(1, 2.0));
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    ensurePanel();
    panel_->treeForTest()->expand(indexFor(engine_.idForName(QStringLiteral("Set A"))));
    panel_->treeForTest()->expand(indexFor(actionOne));
    pictura::SessionState state;
    panel_->writeSessionState(&state);
    state.actionsExpanded << QStringLiteral("Missing Identity");
    QVERIFY(pictura::saveSession(state));
    panel_->setButtonMode(false);
    panel_->setAllowToolRecording(false);
    panel_->collapseAllForTest();
    panel_->selectNodesForTest({});
    const pictura::SessionState loaded = pictura::loadSession();
    panel_->applySessionState(loaded);
    QCoreApplication::processEvents();
    QVERIFY(panel_->buttonMode());
    QVERIFY(panel_->allowToolRecording());
    QCOMPARE(engine_.playbackMode(), 1);
    QCOMPARE(engine_.playbackPauseSeconds(), 2.0);
    QVERIFY(panel_->treeForTest()->isExpanded(indexFor(engine_.idForName(
        QStringLiteral("Set A")))));
    QCOMPARE(panel_->selectedNodeIds(), QStringList{actionOne});
    QCOMPARE(panel_->treeForTest()->currentIndex(), indexFor(actionOne));
}

void ActionsPanelTest::ap_library_restore()
{
    QVERIFY(panel_->createSetForTest(QStringLiteral("Persisted Set")));
    panel_->refresh();
    panel_->saveLibraryNowForTest();
    engine_.clearForLoadTest();
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Persisted Set")).isEmpty());
    panel_->loadLibraryNowForTest();
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Persisted Set")) != QString());
}

void ActionsPanelTest::ap_key_assign()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, Qt::Key_F5, true, false, nullptr));
    panel_->refresh();
    QVERIFY(panel_->modelForTest()->data(indexFor(actionOne).siblingAtColumn(2))
                .toString()
                .contains(QStringLiteral("Shift+F5")));
    QVERIFY(panel_->handleFunctionKeyForTest(Qt::Key_F5, Qt::ShiftModifier));
    advanceUntilDone();
    QCOMPARE(executed_.size(), 2);
}

void ActionsPanelTest::ap_key_warn()
{
    panel_->setShortcutProviderForTest(
        [](const QKeySequence&) { return QStringLiteral("layer.new"); });
    QVERIFY(panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                             Qt::Key_F8, false, false, nullptr));
    QVERIFY(panel_->lastWarningForTest().contains(QStringLiteral("layer.new")));
}

void ActionsPanelTest::ap_key_windows()
{
    panel_->setWindowsPlatformForTest(true);
    QString error;
    QVERIFY(!panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                              Qt::Key_F1, false, false, &error));
    QVERIFY(!panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                              Qt::Key_F4, false, true, &error));
    QVERIFY(!panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                              Qt::Key_F6, false, true, &error));
    QVERIFY(panel_->assignFunctionKeyForTest(engine_.idForName(QStringLiteral("Action One")),
                                             Qt::Key_F5, true, false, &error));
}

void ActionsPanelTest::ap_error_parse()
{
    const QString path = presetsDir_.filePath(QStringLiteral("partial.atn"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("atn");
    file.close();
    QVERIFY(panel_->loadForTest(path, false));
    QVERIFY(lastError_.contains(QStringLiteral("parse error")));
    QVERIFY(engine_.idForName(QStringLiteral("partial")) != QString()
            || engine_.idForName(QStringLiteral("Set A")) != QString());
}

void ActionsPanelTest::ap_error_target()
{
    executorSucceeds_ = false;
    selectAndRefresh(engine_.idForName(QStringLiteral("Step One")));
    QVERIFY(panel_->playSelection(true));
    panel_->advancePlaybackForTest();
    QVERIFY(!panel_->isPlaying());
    QVERIFY(lastError_.contains(QStringLiteral("Step One")));
    QVERIFY(lastError_.contains(QStringLiteral("edit.fill")));
}

void ActionsPanelTest::ap_error_empty()
{
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    const QString empty = engine_.createAction(setB, QStringLiteral("Empty"), 0, false, false,
                                               0);
    panel_->refresh();
    selectAndRefresh(empty);
    lastError_.clear();
    QVERIFY(panel_->playSelection());
    QVERIFY(executed_.isEmpty());
    QVERIFY(lastError_.isEmpty());
    QVERIFY(!panel_->isPlaying());
}

void ActionsPanelTest::ap_a11y_model()
{
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QModelIndex check = indexFor(stepOne);
    QCOMPARE(check.data(Qt::CheckStateRole).toInt(), static_cast<int>(Qt::Checked));
    QCOMPARE(check.data(Qt::AccessibleTextRole).toString(), QStringLiteral("Included"));
    const QModelIndex modal = modalIndexFor(stepOne);
    QCOMPARE(modal.data(Qt::CheckStateRole).toInt(), static_cast<int>(Qt::Unchecked));
    QVERIFY(modal.data(Qt::AccessibleTextRole).toString().contains(QStringLiteral("No dialog")));
    panel_->modelForTest()->setData(check, Qt::Unchecked, Qt::CheckStateRole);
    panel_->refresh();
    const QModelIndex action = indexFor(engine_.idForName(QStringLiteral("Action One")));
    QVERIFY(action.data(Qt::AccessibleTextRole).toString().contains(
        QStringLiteral("Partially")));
}

void ActionsPanelTest::ap_a11y_key()
{
    ensurePanel();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QModelIndex check = indexFor(stepOne);
    QTreeView* tree = panel_->treeForTest();
    tree->setCurrentIndex(check);
    tree->setFocus();
    QTest::keyClick(tree, Qt::Key_Space);
    QVERIFY(!engine_.idForName(QStringLiteral("Step One")).isEmpty());
    QVERIFY(!panel_->modelForTest()->rowForId(stepOne).enabled);
    QTest::keyClick(tree, Qt::Key_Space, Qt::ControlModifier);
    QVERIFY(panel_->modelForTest()->rowForId(stepOne).modal);
}


void ActionsPanelTest::ap_refresh_current()
{
    ensurePanel();
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(stepOne);
    QTreeView* tree = panel_->treeForTest();
    QCOMPARE(tree->currentIndex(), indexFor(stepOne));
    tree->setFocus();
    QTest::keyClick(tree, Qt::Key_Space);
    QCoreApplication::processEvents();
    QVERIFY(!engine_.rowForId(stepOne).enabled);
}

void ActionsPanelTest::ap_edit_locked()
{
    ensurePanel();
    QTreeView* tree = panel_->treeForTest();
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionTwo = engine_.idForName(QStringLiteral("Action Two"));
    selectAndRefresh(actionOne);
    QVERIFY(tree->isEnabled());
    QVERIFY(panel_->buttonViewForTest()->isEnabled());
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(engine_.isRecording());
    QVERIFY(!tree->isEnabled());
    QVERIFY(!panel_->buttonViewForTest()->isEnabled());
    QVERIFY(!panel_->overwriteStepForTest(stepOne));

    QVERIFY(panel_->modelForTest()->setData(indexFor(actionOne).siblingAtColumn(2),
                                            QStringLiteral("Blocked"), Qt::EditRole));
    panel_->refresh();
    QVERIFY(engine_.idForName(QStringLiteral("Action One")) != QString());
    QVERIFY(engine_.idForName(QStringLiteral("Blocked")).isEmpty());

    QScopedPointer<QMimeData> mime(panel_->modelForTest()->mimeData({indexFor(stepOne)}));
    QVERIFY(mime);
    QVERIFY(panel_->modelForTest()->dropMimeData(mime.data(), Qt::MoveAction, 0, 0,
                                                 indexFor(actionTwo)));
    panel_->refresh();
    QCOMPARE(engine_.rowForId(stepOne).parentId, actionOne);

    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(tree->isEnabled());
    QVERIFY(panel_->buttonViewForTest()->isEnabled());

    selectAndRefresh(actionOne);
    panel_->setPlaybackOptionsForTest(2, 10.0);
    QVERIFY(panel_->playSelection());
    QVERIFY(!tree->isEnabled());
    QVERIFY(!panel_->buttonViewForTest()->isEnabled());
    panel_->cancelPlaybackForTest();
    QVERIFY(tree->isEnabled());
}

void ActionsPanelTest::ap_refusal_errors()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString actionTwo = engine_.idForName(QStringLiteral("Action Two"));
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    selectAndRefresh(stepOne);
    engine_.setFailDelete(true);
    QVERIFY(!panel_->deleteSelectionForTest(true));
    QVERIFY(lastError_.contains(QStringLiteral("delete refused")));
    engine_.setFailDelete(false);
    engine_.setFailMove(true);
    QVERIFY(!panel_->moveNodeForTest(stepOne, actionTwo, 0, false));
    QVERIFY(lastError_.contains(QStringLiteral("move refused")));
    engine_.setFailMove(false);
    engine_.setFailOptions(true);
    QVERIFY(!panel_->applyActionOptionsForTest(actionOne, QStringLiteral("Renamed"), setB, 0,
                                               false, false, 0));
    QVERIFY(lastError_.contains(QStringLiteral("options refused")));
}

void ActionsPanelTest::ap_button_stack_restore()
{
    ensurePanel();
    panel_->setButtonMode(true);
    pictura::SessionState state;
    state.actionsButtonMode = true;
    panel_->setButtonMode(false);
    panel_->applySessionState(state);
    QCoreApplication::processEvents();
    QVERIFY(panel_->buttonMode());
    QVERIFY(panel_->buttonViewForTest()->isVisible());
    QVERIFY(!panel_->treeForTest()->isVisible());
}

void ActionsPanelTest::ap_menu_installed_lock()
{
    const QString path = presetsDir_.filePath(QStringLiteral("Locked.atn"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("atn");
    file.close();
    QMenu* menu = liveMenu();
    QAction* installed = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->data().toString().startsWith(QStringLiteral("actions.loadInstalled:"))) {
            installed = action;
        }
    }
    QVERIFY(installed);
    QVERIFY(installed->isEnabled());
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    selectAndRefresh(actionOne);
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    QVERIFY(engine_.isRecording());
    menu = liveMenu();
    installed = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->data().toString().startsWith(QStringLiteral("actions.loadInstalled:"))) {
            installed = action;
        }
    }
    QVERIFY(installed);
    QVERIFY(!installed->isEnabled());
    QVERIFY(!panel_->performPanelMenuAction(QStringLiteral("actions.loadInstalled:") + path));
    QVERIFY(engine_.loadedPath().isEmpty());
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
}

void ActionsPanelTest::ap_save_suffix()
{
    const QString setA = engine_.idForName(QStringLiteral("Set A"));
    selectAndRefresh(setA);
    dialogPath_ = presetsDir_.filePath(QStringLiteral("myset"));
    QVERIFY(panel_->performPanelMenuAction(QStringLiteral("actions.save")));
    QVERIFY(QFileInfo::exists(dialogPath_ + QStringLiteral(".atn")));
    QVERIFY(!QFileInfo::exists(dialogPath_));
}

void ActionsPanelTest::ap_play_selection_split()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QString stepThree = engine_.appendStepForTest(
        actionOne, QStringLiteral("Step Extra"), QStringLiteral("image.rotate90cw"),
        QJsonObject());
    const QString stepFour = engine_.idForName(QStringLiteral("Step Four"));
    panel_->refresh();
    panel_->selectNodesForTest({stepOne, stepThree});
    lastError_.clear();
    QVERIFY(!panel_->playSelection());
    QVERIFY(!panel_->isPlaying());
    QVERIFY(lastError_.contains(QStringLiteral("one step")));
    QVERIFY(executed_.isEmpty());

    panel_->selectNodesForTest({stepOne, stepFour});
    lastError_.clear();
    QVERIFY(!panel_->playSelection());
    QVERIFY(lastError_.contains(QStringLiteral("one step")));
}

void ActionsPanelTest::ae_play_tool()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    engine_.appendToolStepForTest(
        actionOne, QJsonObject{{QStringLiteral("geometry"), QStringLiteral("p")}});
    panel_->refresh();
    selectAndRefresh(actionOne);
    toolParams_.clear();
    lastError_.clear();
    QVERIFY(panel_->playSelection());
    advanceUntilDone();
    QCOMPARE(toolParams_.size(), 1);
    QCOMPARE(toolParams_.first().value(QStringLiteral("geometry")).toString(),
             QStringLiteral("p"));
    QCOMPARE(executed_.size(), 2);
    QVERIFY(lastError_.isEmpty());
}

void ActionsPanelTest::ae_play_tool_missing()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString toolId = engine_.appendToolStepForTest(actionOne, QJsonObject());
    panel_->refresh();
    selectAndRefresh(toolId);
    panel_->setToolStepExecutor({});
    lastError_.clear();
    QVERIFY(panel_->playSelection());
    panel_->advancePlaybackForTest();
    QVERIFY(!panel_->isPlaying());
    QVERIFY(lastError_.contains(QStringLiteral("executor")));
}

void ActionsPanelTest::ae_document_facts()
{
    panel_->setDocumentFactsProvider([] {
        pictura::ActionsPanel::DocumentFacts facts;
        facts.present = true;
        facts.square = true;
        facts.mode = QStringLiteral("CMYK");
        facts.hasSelection = true;
        return facts;
    });
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    engine_.clearDocumentFacts();
    QVERIFY(panel_->playSelection());
    QCOMPARE(engine_.factsCalls(), 1);
    QVERIFY(engine_.factsPresent());
    QVERIFY(engine_.factsSquare());
    QCOMPARE(engine_.factsMode(), QStringLiteral("CMYK"));
    QVERIFY(engine_.factsHasSelection());
    advanceUntilDone();

    panel_->setDocumentFactsProvider({});
    engine_.clearDocumentFacts();
    QVERIFY(panel_->playSelection());
    QCOMPARE(engine_.factsCalls(), 1);
    QVERIFY(!engine_.factsPresent());
    QVERIFY(!engine_.factsSquare());
    QVERIFY(engine_.factsMode().isEmpty());
    QVERIFY(!engine_.factsHasSelection());
    advanceUntilDone();
}

void ActionsPanelTest::ae_key_shadow_rename_move()
{
    auto* registry = window_->registry();
    const QList<QKeySequence> before = registry->shortcuts(QStringLiteral("edit.fill"));
    QVERIFY(!before.isEmpty());
    panel_->setShortcutProviderForTest([registry](const QKeySequence& shortcut) {
        return registry->commandForShortcut(shortcut);
    });
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString setB = engine_.idForName(QStringLiteral("Set B"));
    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, Qt::Key_F5, true, false, nullptr));
    QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());

    QVERIFY(engine_.renameNode(actionOne, QStringLiteral("Renamed One")));
    panel_->refresh();
    QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());

    QVERIFY(engine_.moveNode(actionOne, setB, 0, false));
    panel_->refresh();
    QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());
    QCOMPARE(engine_.rowForId(actionOne).parentId, setB);

    QVERIFY(panel_->assignFunctionKeyForTest(actionOne, 0, false, false, nullptr));
    panel_->refresh();
    QCOMPARE(registry->shortcuts(QStringLiteral("edit.fill")), before);
}

void ActionsPanelTest::ae_tool_reject_error()
{
    selectAndRefresh(engine_.idForName(QStringLiteral("Action One")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
    panel_->setAllowToolRecording(true);
    engine_.setRejectToolStroke(true);
    lastError_.clear();
    QVERIFY(!panel_->recordToolStrokeForTest(QStringLiteral("brush"), {}, 10));
    QVERIFY(lastError_.contains(QStringLiteral("tool stroke refused")));
    panel_->performPanelMenuAction(QStringLiteral("actions.toggleRecording"));
}

void ActionsPanelTest::ap_drop_same_action_end()
{
    pictura::ActionsTreeModel* model = panel_->modelForTest();
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    const QModelIndex parent = indexFor(actionOne);
    const int end = model->rowCount(parent);
    QScopedPointer<QMimeData> mime(model->mimeData({indexFor(stepOne)}));
    QVERIFY(mime);
    QVERIFY(model->dropMimeData(mime.data(), Qt::MoveAction, end, 0, parent));
    QCOMPARE(engine_.lastMoveParentId(), actionOne);
    QCOMPARE(engine_.lastMoveRow(), end);
    panel_->refresh();
    const QStringList steps = engine_.stepIdsUnder(QStringLiteral("Action One"));
    QCOMPARE(steps.size(), 2);
    QCOMPARE(steps.last(), stepOne);
}

void ActionsPanelTest::ap_insert_anchor()
{
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    const QString stepOne = engine_.idForName(QStringLiteral("Step One"));
    selectAndRefresh(stepOne);
    QVERIFY(panel_->insertStopForTest(QStringLiteral("Pause"), true));
    QCOMPARE(engine_.lastInsertAnchor(), stepOne);
    selectAndRefresh(actionOne);
    QVERIFY(panel_->insertMenuItemForTest(QStringLiteral("edit.fill")));
    QCOMPARE(engine_.lastInsertAnchor(), actionOne);
}

void ActionsPanelTest::ae_key_shadow_rebuild()
{
    auto* registry = window_->registry();
    const QList<QKeySequence> defaults = registry->defaultShortcuts(QStringLiteral("edit.fill"));
    registry->setShortcuts(QStringLiteral("edit.fill"), defaults);
    const QString actionOne = engine_.idForName(QStringLiteral("Action One"));
    QVERIFY(engine_.assignFunctionKey(actionOne, Qt::Key_F5, true, false, nullptr));
    QCOMPARE(registry->commandForShortcut(QKeySequence(Qt::Key_F5 | Qt::ShiftModifier)),
             QStringLiteral("edit.fill"));
    {
        pictura::ActionsPanel fresh;
        fresh.setPersistEnabledForTest(false);
        fresh.setEngine(&engine_);
        fresh.setRegistry(registry);
        QVERIFY(registry->shortcuts(QStringLiteral("edit.fill")).isEmpty());
    }
    panel_->setRegistry(registry);
    engine_.assignFunctionKey(actionOne, 0, false, false, nullptr);
    registry->setShortcuts(QStringLiteral("edit.fill"), defaults);
    panel_->refresh();
}
