#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QPoint>
#include <QtGui/QAction>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "commands.h"
#include "frame.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "session.h"
#include "workspace_store.h"

#include "qt_test_support.h"

namespace {

QString storeFilePath(const QString& fileName)
{
    return pictura::workspaceStoreDir() + QLatin1Char('/') + fileName;
}

QJsonObject readJsonObject(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    return QJsonDocument::fromJson(file.readAll()).object();
}

bool writeBytes(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        return false;
    }
    return file.write(bytes) >= 0;
}

QJsonObject groupSpec(const QStringList& order, const QStringList& visible,
                      bool minimized = false, bool collapsed = false)
{
    QJsonObject group;
    group.insert(QStringLiteral("order"), QJsonArray::fromStringList(order));
    group.insert(QStringLiteral("visible"), QJsonArray::fromStringList(visible));
    group.insert(QStringLiteral("minimized"), minimized);
    group.insert(QStringLiteral("collapsed"), collapsed);
    return group;
}

QJsonObject columnSpec(const QString& side, int order, int width, const QString& railMode,
                       const QJsonArray& groups)
{
    QJsonObject column;
    column.insert(QStringLiteral("side"), side);
    column.insert(QStringLiteral("order"), order);
    column.insert(QStringLiteral("width"), width);
    column.insert(QStringLiteral("railMode"), railMode);
    column.insert(QStringLiteral("groups"), groups);
    return column;
}

} // namespace

class WorkspacesTest : public QObject {
    Q_OBJECT

private slots:
    void applyReGroupsPanel();
    void applyRedocksFloatingPanel();
    void applyLeavesToolsColumnUntouched();
    void applyKeepsOmittedPanelReachable();
    void freshSessionUsesEssentialsDefault();
    void presetSwitchRegroups();
    void presetCommandsReflectActive();
    void switchingPresetRearrangesAndChecks();
    void createUserWorkspacePersists();
    void createRejectsInvalidNames();
    void deleteNonActiveWorkspaceRemoves();
    void activeCannotBeDeletedAndPresetsCan();
    void resetRestoresFactory();
    void autoRememberSurvivesSwitch();
    void workspaceMenuDoesNotDuplicate();
    void optionsBarSwitcherShowsActive();
    void workspaceMenuSections();
    void storeRoundTrip();
    void storeUnknownKeysSurvive();
    void storeCorruptIndexReturnsFalse();
    void storeCorruptWorkspaceIsSkipped();
    void lostSessionKeepsActiveWorkspace();
    void wholeColumnFloatDoesNotPersistPhantom();
    void rejectsAndReservesNames();
    void deleteChooserExcludesActive();
    void deletedPresetHidesMenuAction();
    void menuSectionsAreExact();
    void windowPanelsReflectAppliedWorkspace();
    void activeWorkspaceSurvivesRestart();
    void toolsColumnLeftBranch();
    void emptySpecApplyIsSafe();
    void switcherVisibleForEveryTool();
};

void WorkspacesTest::applyReGroupsPanel()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    const QString before = window->columnForPanel(QStringLiteral("colorPanel"))
                               ->groupOfForTest(QStringLiteral("colorPanel"));
    QVERIFY(before != window->columnForPanel(QStringLiteral("layersPanel"))
                          ->groupOfForTest(QStringLiteral("layersPanel")));

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("layersPanel"), QStringLiteral("colorPanel")},
                            {QStringLiteral("layersPanel"), QStringLiteral("colorPanel")}));
    QJsonArray columns;
    columns.append(columnSpec(QStringLiteral("right"), 0, 320, QStringLiteral("normal"),
                              groups));
    window->applyWorkspaceLayoutForTest(columns);

    const QString colorGroup = window->columnForPanel(QStringLiteral("colorPanel"))
                                   ->groupOfForTest(QStringLiteral("colorPanel"));
    QCOMPARE(colorGroup, window->columnForPanel(QStringLiteral("layersPanel"))
                             ->groupOfForTest(QStringLiteral("layersPanel")));
    QVERIFY(colorGroup != before);
    QCOMPARE(colorGroup, QStringLiteral("panelGroup_layersPanel"));
}

void WorkspacesTest::applyRedocksFloatingPanel()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    pictura::PanelColumn* source = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(source != nullptr);
    QVERIFY(source->tearOffPanelForTest(QStringLiteral("colorPanel")));
    QCOMPARE(source->floatCountForTest(), 1);

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("colorPanel")}, {QStringLiteral("colorPanel")}));
    QJsonArray columns;
    columns.append(columnSpec(QStringLiteral("right"), 0, 300, QStringLiteral("normal"),
                              groups));
    window->applyWorkspaceLayoutForTest(columns);

    pictura::PanelColumn* restored = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(restored != nullptr);
    QCOMPARE(restored->floatCountForTest(), 0);
    QVERIFY(restored->isPanelVisible(QStringLiteral("colorPanel")));
}

void WorkspacesTest::applyLeavesToolsColumnUntouched()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    pictura::PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    QWidget* toolsContent = tools->toolsContentForTest();
    QVERIFY(toolsContent != nullptr);
    const int toolsIndex = window->panelColumns().indexOf(tools);
    QVERIFY(toolsIndex >= 0);

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")},
                            {QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")}));
    QJsonArray columns;
    columns.append(columnSpec(QStringLiteral("right"), 0, 320, QStringLiteral("normal"),
                              groups));
    window->applyWorkspaceLayoutForTest(columns);

    QCOMPARE(window->panelColumns().indexOf(tools), toolsIndex);
    QCOMPARE(tools->toolsContentForTest(), toolsContent);
}

void WorkspacesTest::applyKeepsOmittedPanelReachable()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(!window->columnForPanel(QStringLiteral("navigatorPanel"))
                 ->isPanelVisible(QStringLiteral("navigatorPanel")));

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")}));
    QJsonArray columns;
    columns.append(columnSpec(QStringLiteral("right"), 0, 300, QStringLiteral("normal"),
                              groups));
    window->applyWorkspaceLayoutForTest(columns);

    pictura::PanelColumn* navigatorOwner =
        window->columnForPanel(QStringLiteral("navigatorPanel"));
    QVERIFY(navigatorOwner != nullptr);
    QCOMPARE(navigatorOwner, window->panelColumn());
    QVERIFY(!navigatorOwner->isPanelVisible(QStringLiteral("navigatorPanel")));

    QVERIFY(window->panelColumn()->showPanel(QStringLiteral("navigatorPanel"), true));
    QVERIFY(window->panelColumn()->isPanelVisible(QStringLiteral("navigatorPanel")));
}

void WorkspacesTest::freshSessionUsesEssentialsDefault()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    const auto groupOf = [&window](const QString& panel) {
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };

    pictura::PanelColumn* main = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(main != nullptr);
    QVERIFY(!main->railModeForTest());
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("swatchesPanel")));
    QCOMPARE(groupOf(QStringLiteral("adjustmentsPanel")), groupOf(QStringLiteral("stylesPanel")));
    QCOMPARE(groupOf(QStringLiteral("layersPanel")), groupOf(QStringLiteral("channelsPanel")));
    QCOMPARE(groupOf(QStringLiteral("layersPanel")), groupOf(QStringLiteral("pathsPanel")));
    QVERIFY(groupOf(QStringLiteral("colorPanel")) != groupOf(QStringLiteral("layersPanel")));

    pictura::PanelColumn* secondary = window->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY(secondary != nullptr);
    QVERIFY(secondary != main);
    QVERIFY(secondary->railModeForTest());
    QCOMPARE(window->columnForPanel(QStringLiteral("propertiesPanel")), secondary);
    QVERIFY(secondary->groupForPanel(QStringLiteral("historyPanel"))
            != secondary->groupForPanel(QStringLiteral("propertiesPanel")));

    const QList<pictura::PanelColumn*> columns = window->panelColumns();
    QVERIFY(columns.indexOf(secondary) < columns.indexOf(main));
}

void WorkspacesTest::presetSwitchRegroups()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    window->applyWorkspaceLayoutForTest(pictura::workspacePresets(QStringLiteral("Painting")));
    QCoreApplication::processEvents();

    const auto groupOf = [&window](const QString& panel) {
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };
    QCOMPARE(groupOf(QStringLiteral("navigatorPanel")), groupOf(QStringLiteral("swatchesPanel")));
    QCOMPARE(groupOf(QStringLiteral("brushPanel")), groupOf(QStringLiteral("cloneSourcePanel")));
    QCOMPARE(groupOf(QStringLiteral("layersPanel")), groupOf(QStringLiteral("pathsPanel")));
    QVERIFY(groupOf(QStringLiteral("navigatorPanel")) != groupOf(QStringLiteral("layersPanel")));

    pictura::PanelColumn* secondary = window->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY(secondary != nullptr);
    QVERIFY(secondary->railModeForTest());
}

void WorkspacesTest::presetCommandsReflectActive()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("Essentials"));
    pictura::CommandRegistry* registry = window->registry();
    const QStringList presetIds{
        QString::fromLatin1(pictura::command_ids::WindowWorkspaceEssentials),
        QString::fromLatin1(pictura::command_ids::WindowWorkspacePainting),
        QString::fromLatin1(pictura::command_ids::WindowWorkspacePhotography),
        QString::fromLatin1(pictura::command_ids::WindowWorkspaceTypography),
    };
    for (const QString& id : presetIds) {
        QAction* action = registry->action(id);
        QVERIFY2(action != nullptr, qPrintable(id));
        QVERIFY(action->isCheckable());
        QVERIFY(action->isEnabled());
    }
    QVERIFY(registry->action(pictura::command_ids::WindowWorkspaceEssentials)->isChecked());
    QVERIFY(!registry->action(pictura::command_ids::WindowWorkspacePainting)->isChecked());

    for (const QString& gone : {QStringLiteral("3D"), QStringLiteral("Advanced 3D"),
                                QStringLiteral("Motion"), QStringLiteral("New Features")}) {
        QVERIFY2(!registry->action(pictura::commandIdForPath(
                     {QStringLiteral("Window"), QStringLiteral("Workspace"), gone})),
                 qPrintable(gone));
    }

    QVERIFY(registry->action(pictura::command_ids::WindowWorkspaceNew)->isEnabled());
    QVERIFY(registry->action(pictura::command_ids::WindowWorkspaceReset)->isEnabled());
    // Delete is always available; the chooser filters the active workspace out.
    QVERIFY(registry->action(pictura::command_ids::WindowWorkspaceDelete)->isEnabled());
    QCOMPARE(registry->action(pictura::command_ids::WindowWorkspaceReset)->text(),
             QStringLiteral("Reset Essentials"));
}

void WorkspacesTest::switchingPresetRearrangesAndChecks()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("Painting"));
    pictura::CommandRegistry* registry = window->registry();
    QVERIFY(registry->action(pictura::command_ids::WindowWorkspacePainting)->isChecked());
    QVERIFY(!registry->action(pictura::command_ids::WindowWorkspaceEssentials)->isChecked());
    QCOMPARE(registry->action(pictura::command_ids::WindowWorkspaceReset)->text(),
             QStringLiteral("Reset Painting"));

    const auto groupOf = [&window](const QString& panel) {
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };
    QVERIFY(!groupOf(QStringLiteral("navigatorPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("swatchesPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("brushPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("cloneSourcePanel")).isEmpty());
    QCOMPARE(groupOf(QStringLiteral("navigatorPanel")), groupOf(QStringLiteral("swatchesPanel")));
    QCOMPARE(groupOf(QStringLiteral("brushPanel")), groupOf(QStringLiteral("cloneSourcePanel")));
    pictura::PanelColumn* secondary = window->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY(secondary != nullptr);
    QVERIFY(secondary->railModeForTest());
}

void WorkspacesTest::createUserWorkspacePersists()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
    QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("My Edit"));
    QVERIFY(window->userWorkspacesForTest().contains(QStringLiteral("My Edit")));
    QVERIFY(window->registry()->action(pictura::command_ids::WindowWorkspaceDelete)->isEnabled());

    pictura::WorkspaceStore loaded;
    QVERIFY(pictura::loadWorkspaceStore(&loaded));
    QCOMPARE(loaded.activeWorkspace, QStringLiteral("My Edit"));
    bool found = false;
    for (const pictura::WorkspaceRecord& record : loaded.workspaces) {
        if (record.name == QStringLiteral("My Edit") && record.kind == QStringLiteral("user")) {
            found = true;
        }
    }
    QVERIFY(found);
}

void WorkspacesTest::createRejectsInvalidNames()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(!window->createWorkspaceForTest(QString()));
    QVERIFY(!window->createWorkspaceForTest(QStringLiteral("   ")));
    QVERIFY(!window->createWorkspaceForTest(QStringLiteral("Essentials")));
    QVERIFY(!window->createWorkspaceForTest(QString(65, QLatin1Char('a'))));
    QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
    QVERIFY(!window->createWorkspaceForTest(QStringLiteral("My Edit")));
    QCOMPARE(window->userWorkspacesForTest().size(), 1);
}

void WorkspacesTest::deleteNonActiveWorkspaceRemoves()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Temp")));
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    QVERIFY(window->deleteWorkspaceForTest(QStringLiteral("Temp")));
    QVERIFY(!window->userWorkspacesForTest().contains(QStringLiteral("Temp")));

    pictura::WorkspaceStore loaded;
    QVERIFY(pictura::loadWorkspaceStore(&loaded));
    for (const pictura::WorkspaceRecord& record : loaded.workspaces) {
        QVERIFY(record.name != QStringLiteral("Temp"));
    }
}

void WorkspacesTest::activeCannotBeDeletedAndPresetsCan()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto window = pictura::test::makeMainWindow();
        QVERIFY(window != nullptr);
        window->resize(1100, 700);
        window->show();
        QCoreApplication::processEvents();

        // The active workspace (Essentials by default) cannot be deleted.
        QVERIFY(!window->deleteWorkspaceForTest(QStringLiteral("Essentials")));
        // A preset that is not active is deletable like any other.
        QVERIFY(window->deleteWorkspaceForTest(QStringLiteral("Painting")));
        QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("Essentials"));
    }
    // A fresh window over the same state home must not resurrect the preset.
    {
        auto window2 = pictura::test::makeMainWindow();
        QVERIFY(window2 != nullptr);
        window2->resize(1100, 700);
        window2->show();
        QCoreApplication::processEvents();
        bool painting = false;
        pictura::WorkspaceStore loaded;
        QVERIFY(pictura::loadWorkspaceStore(&loaded));
        for (const pictura::WorkspaceRecord& record : loaded.workspaces) {
            if (record.name == QStringLiteral("Painting")) {
                painting = true;
            }
        }
        QVERIFY(!painting);
    }
}

void WorkspacesTest::resetRestoresFactory()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    const auto groupOf = [&window](const QString& panel) {
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("layersPanel"), QStringLiteral("colorPanel")},
                            {QStringLiteral("layersPanel"), QStringLiteral("colorPanel")}));
    window->applyWorkspaceLayoutForTest(
        QJsonArray{columnSpec(QStringLiteral("right"), 0, 320, QStringLiteral("normal"), groups)});
    QVERIFY(!groupOf(QStringLiteral("colorPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("layersPanel")).isEmpty());
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("layersPanel")));

    QVERIFY(window->registry()->dispatch(pictura::command_ids::WindowWorkspaceReset));
    QCoreApplication::processEvents();

    QVERIFY(!groupOf(QStringLiteral("colorPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("swatchesPanel")).isEmpty());
    QVERIFY(groupOf(QStringLiteral("colorPanel")) != groupOf(QStringLiteral("layersPanel")));
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("swatchesPanel")));
    pictura::PanelColumn* secondary = window->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY(secondary != nullptr);
    QVERIFY(secondary->railModeForTest());
}

void WorkspacesTest::autoRememberSurvivesSwitch()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    const auto groupOf = [&window](const QString& panel) {
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };

    QJsonArray groups;
    groups.append(groupSpec({QStringLiteral("layersPanel"), QStringLiteral("colorPanel")},
                            {QStringLiteral("layersPanel"), QStringLiteral("colorPanel")}));
    window->applyWorkspaceLayoutForTest(
        QJsonArray{columnSpec(QStringLiteral("right"), 0, 320, QStringLiteral("normal"), groups)});

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    QVERIFY(!groupOf(QStringLiteral("colorPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("layersPanel")).isEmpty());
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("layersPanel")));

    QVERIFY(window->registry()->dispatch(pictura::command_ids::WindowWorkspaceReset));
    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    QCoreApplication::processEvents();
    QVERIFY(!groupOf(QStringLiteral("colorPanel")).isEmpty());
    QVERIFY(!groupOf(QStringLiteral("swatchesPanel")).isEmpty());
    QVERIFY(groupOf(QStringLiteral("colorPanel")) != groupOf(QStringLiteral("layersPanel")));
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("swatchesPanel")));
}

void WorkspacesTest::workspaceMenuDoesNotDuplicate()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Alpha")));
    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Beta")));

    window->refreshWorkspaceMenuForTest();
    window->refreshWorkspaceMenuForTest();
    const QStringList labels = window->workspaceMenuLabelsForTest();
    QCOMPARE(labels.count(QStringLiteral("Alpha")), 1);
    QCOMPARE(labels.count(QStringLiteral("Beta")), 1);
    QCOMPARE(labels.count(QStringLiteral("Essentials")), 1);
}

void WorkspacesTest::optionsBarSwitcherShowsActive()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    auto* switcher = window->findChild<QToolButton*>(QStringLiteral("workspaceSwitcher"));
    QVERIFY(switcher != nullptr);
    QVERIFY(switcher->menu() != nullptr);
    QCOMPARE(switcher->text(), QStringLiteral("Essentials"));

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    QCOMPARE(switcher->text(), QStringLiteral("Painting"));
}

void WorkspacesTest::workspaceMenuSections()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Alpha")));
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    window->refreshWorkspaceMenuForTest();

    // presets | actions | additional: two separators.
    QVERIFY2(window->workspaceMenuSeparatorCountForTest() >= 2, "menu has separators");
    const QStringList labels = window->workspaceMenuLabelsForTest();
    // The user workspace lands in the presets section, not the actions section.
    const int user = labels.indexOf(QStringLiteral("Alpha"));
    const int lastPreset = labels.indexOf(QStringLiteral("Typography"));
    const int newAction = labels.indexOf(QStringLiteral("New Workspace…"));
    QVERIFY(user >= 0 && lastPreset >= 0 && newAction >= 0);
    QVERIFY2(user > lastPreset, "user workspace follows the presets");
    QVERIFY2(user < newAction, "user workspace precedes the actions");
}

void WorkspacesTest::storeRoundTrip()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    const QJsonArray factory{columnSpec(
        QStringLiteral("right"), 0, 320, QStringLiteral("normal"),
        QJsonArray{groupSpec({QStringLiteral("colorPanel")}, {QStringLiteral("colorPanel")})})};
    const QJsonArray layout{columnSpec(
        QStringLiteral("right"), 0, 400, QStringLiteral("normal"),
        QJsonArray{groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")})})};

    pictura::WorkspaceStore store;
    store.activeWorkspace = QStringLiteral("user-1");
    pictura::WorkspaceRecord essentials;
    essentials.id = QStringLiteral("essentials");
    essentials.name = QStringLiteral("Essentials");
    essentials.kind = QStringLiteral("builtin");
    essentials.factory = factory;
    pictura::WorkspaceRecord user;
    user.id = QStringLiteral("user-1");
    user.name = QStringLiteral("My Edit");
    user.kind = QStringLiteral("user");
    user.layout = layout;
    user.factory = layout;
    user.hasLayout = true;
    store.workspaces = {essentials, user};

    QVERIFY(pictura::saveWorkspaceStore(store));

    pictura::WorkspaceStore loaded;
    QVERIFY(pictura::loadWorkspaceStore(&loaded));
    QCOMPARE(loaded.schemaVersion, store.schemaVersion);
    QCOMPARE(loaded.activeWorkspace, store.activeWorkspace);
    QCOMPARE(loaded.workspaces.size(), 2);

    const pictura::WorkspaceRecord& loadedEssentials = loaded.workspaces.at(0);
    QCOMPARE(loadedEssentials.id, QStringLiteral("essentials"));
    QCOMPARE(loadedEssentials.name, QStringLiteral("Essentials"));
    QCOMPARE(loadedEssentials.kind, QStringLiteral("builtin"));
    QVERIFY(!loadedEssentials.hasLayout);
    QVERIFY(loadedEssentials.layout.isEmpty());
    QVERIFY(loadedEssentials.factory == factory);

    const pictura::WorkspaceRecord& loadedUser = loaded.workspaces.at(1);
    QCOMPARE(loadedUser.id, QStringLiteral("user-1"));
    QCOMPARE(loadedUser.name, QStringLiteral("My Edit"));
    QCOMPARE(loadedUser.kind, QStringLiteral("user"));
    QVERIFY(loadedUser.hasLayout);
    QVERIFY(loadedUser.layout == layout);
    QVERIFY(loadedUser.factory == layout);
}

void WorkspacesTest::storeUnknownKeysSurvive()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    pictura::WorkspaceStore store;
    store.activeWorkspace = QStringLiteral("user-1");
    pictura::WorkspaceRecord user;
    user.id = QStringLiteral("user-1");
    user.name = QStringLiteral("My Edit");
    user.kind = QStringLiteral("user");
    user.hasLayout = true;
    user.layout = QJsonArray{columnSpec(
        QStringLiteral("right"), 0, 300, QStringLiteral("normal"),
        QJsonArray{groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")})})};
    store.workspaces = {user};
    QVERIFY(pictura::saveWorkspaceStore(store));

    const QString indexPath = storeFilePath(QStringLiteral("index.json"));
    QJsonObject index = readJsonObject(indexPath);
    index.insert(QStringLiteral("futureIndexKey"), 42);
    QVERIFY(writeBytes(indexPath, QJsonDocument(index).toJson(QJsonDocument::Compact)));

    const QString workspacePath = storeFilePath(QStringLiteral("user-1.json"));
    QJsonObject workspace = readJsonObject(workspacePath);
    workspace.insert(QStringLiteral("futureWorkspaceKey"), QStringLiteral("keep"));
    QVERIFY(writeBytes(workspacePath,
                       QJsonDocument(workspace).toJson(QJsonDocument::Compact)));

    QVERIFY(pictura::saveWorkspaceStore(store));

    QVERIFY(readJsonObject(indexPath).contains(QStringLiteral("futureIndexKey")));
    QVERIFY(readJsonObject(workspacePath).contains(QStringLiteral("futureWorkspaceKey")));
}

void WorkspacesTest::storeCorruptIndexReturnsFalse()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    pictura::WorkspaceStore store;
    store.activeWorkspace = QStringLiteral("essentials");
    pictura::WorkspaceRecord essentials;
    essentials.id = QStringLiteral("essentials");
    essentials.name = QStringLiteral("Essentials");
    essentials.kind = QStringLiteral("builtin");
    store.workspaces = {essentials};
    QVERIFY(pictura::saveWorkspaceStore(store));

    QVERIFY(writeBytes(storeFilePath(QStringLiteral("index.json")),
                       QByteArrayLiteral("not json {")));

    pictura::WorkspaceStore loaded;
    QVERIFY(!pictura::loadWorkspaceStore(&loaded));
    QVERIFY(loaded.workspaces.isEmpty());
    QVERIFY(loaded.activeWorkspace.isEmpty());
}

void WorkspacesTest::storeCorruptWorkspaceIsSkipped()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    pictura::WorkspaceStore store;
    store.activeWorkspace = QStringLiteral("user-1");
    pictura::WorkspaceRecord essentials;
    essentials.id = QStringLiteral("essentials");
    essentials.name = QStringLiteral("Essentials");
    essentials.kind = QStringLiteral("builtin");
    pictura::WorkspaceRecord user;
    user.id = QStringLiteral("user-1");
    user.name = QStringLiteral("My Edit");
    user.kind = QStringLiteral("user");
    store.workspaces = {essentials, user};
    QVERIFY(pictura::saveWorkspaceStore(store));

    QVERIFY(writeBytes(storeFilePath(QStringLiteral("user-1.json")),
                       QByteArrayLiteral("{{{broken")));

    pictura::WorkspaceStore loaded;
    QVERIFY(pictura::loadWorkspaceStore(&loaded));
    QCOMPARE(loaded.workspaces.size(), 1);
    QCOMPARE(loaded.workspaces.at(0).id, QStringLiteral("essentials"));
    QCOMPARE(loaded.activeWorkspace, QStringLiteral("user-1"));
}

void WorkspacesTest::lostSessionKeepsActiveWorkspace()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto window = pictura::test::makeMainWindow();
        QVERIFY(window != nullptr);
        window->resize(1100, 700);
        window->show();
        QCoreApplication::processEvents();

        // A visibly modified arrangement: Color and Layers share one group.
        QJsonArray groups;
        groups.append(groupSpec({QStringLiteral("layersPanel"), QStringLiteral("colorPanel")},
                                {QStringLiteral("layersPanel"), QStringLiteral("colorPanel")}));
        window->applyWorkspaceLayoutForTest(
            QJsonArray{columnSpec(QStringLiteral("right"), 0, 320, QStringLiteral("normal"),
                                  groups)});
        QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
        QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("My Edit"));
    }
    // Lose only the session; the workspace store (and My Edit's layout) survives.
    QVERIFY(QFile::remove(pictura::sessionFilePath()));

    auto window2 = pictura::test::makeMainWindow();
    QVERIFY(window2 != nullptr);
    window2->resize(1100, 700);
    window2->show();
    QCoreApplication::processEvents();

    QCOMPARE(window2->activeWorkspaceForTest(), QStringLiteral("My Edit"));
    const auto groupOf = [&window2](const QString& panel) {
        pictura::PanelColumn* owner = window2->columnForPanel(panel);
        return owner ? owner->groupOfForTest(panel) : QString();
    };
    QVERIFY(!groupOf(QStringLiteral("colorPanel")).isEmpty());
    QCOMPARE(groupOf(QStringLiteral("colorPanel")), groupOf(QStringLiteral("layersPanel")));
    // The saved arrangement is restored, not overwritten by Essentials (where
    // Color groups with Swatches).
    QVERIFY(groupOf(QStringLiteral("colorPanel")) != groupOf(QStringLiteral("swatchesPanel")));
}

void WorkspacesTest::wholeColumnFloatDoesNotPersistPhantom()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    pictura::PanelColumn* main = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(main != nullptr);
    QVERIFY(!main->isColumnFloating());

    QWidget* tabs = window->findChild<QWidget*>(QStringLiteral("documentTabs"));
    QVERIFY(tabs != nullptr);
    const QPoint parked = tabs->mapToGlobal(tabs->rect().center());
    main->beginColumnHeaderDragForTest(
        main->mapToGlobal(QPoint(qMax(1, main->width() / 2), 8)));
    main->dragColumnHeaderToForTest(parked);
    QVERIFY(main->dropColumnHeaderForTest(parked));
    QCoreApplication::processEvents();
    QVERIFY(main->columnFloatForTest() != nullptr);

    // The floating column must have its own entry, not be folded into the
    // primary column. Read the live layout before switching away.
    const QJsonArray layout = window->workspaceLayoutForTest();
    bool sawFloating = false;
    for (const QJsonValue& value : layout) {
        const QJsonObject entry = value.toObject();
        if (entry.value(QStringLiteral("tools")).toBool()) {
            continue;
        }
        QVERIFY2(!entry.value(QStringLiteral("groups")).toArray().isEmpty(),
                 "a serialized widget column must carry groups");
        if (entry.value(QStringLiteral("floating")).toBool()) {
            sawFloating = true;
            QCOMPARE(entry.value(QStringLiteral("floatGeometry")).toArray().size(), 4);
        }
    }
    QVERIFY2(sawFloating, "the torn-off column keeps its own floating entry");

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    QCoreApplication::processEvents();
    for (pictura::PanelColumn* column : window->allPanelColumns()) {
        QVERIFY(column);
        QVERIFY(!column->isColumnFloating());
        QCOMPARE(column->floatCountForTest(), 0);
    }
}

void WorkspacesTest::rejectsAndReservesNames()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(!window->workspaceNameErrorForTest(QString()).isEmpty());
    QVERIFY(!window->workspaceNameErrorForTest(QStringLiteral("   ")).isEmpty());
    QVERIFY(!window->workspaceNameErrorForTest(QStringLiteral("Essentials")).isEmpty());
    QVERIFY(!window->workspaceNameErrorForTest(QStringLiteral("Painting")).isEmpty());
    QVERIFY(!window->workspaceNameErrorForTest(QString(65, QLatin1Char('a'))).isEmpty());
    QVERIFY(window->workspaceNameErrorForTest(QStringLiteral("My Edit")).isEmpty());
    QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
    QVERIFY(!window->workspaceNameErrorForTest(QStringLiteral("My Edit")).isEmpty());

    // A deleted preset's name stays reserved.
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    QVERIFY(window->deleteWorkspaceForTest(QStringLiteral("Painting")));
    QVERIFY(!window->workspaceNameErrorForTest(QStringLiteral("Painting")).isEmpty());
    QVERIFY(!window->createWorkspaceForTest(QStringLiteral("Painting")));
    QVERIFY(!window->userWorkspacesForTest().contains(QStringLiteral("Painting")));
}

void WorkspacesTest::deleteChooserExcludesActive()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
    QStringList choices = window->deleteWorkspaceChoicesForTest();
    QVERIFY(choices.contains(QStringLiteral("Essentials")));
    QVERIFY(choices.contains(QStringLiteral("Painting")));
    QVERIFY(choices.contains(QStringLiteral("Photography")));
    QVERIFY(choices.contains(QStringLiteral("Typography")));
    QVERIFY(!choices.contains(QStringLiteral("My Edit")));

    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    choices = window->deleteWorkspaceChoicesForTest();
    QVERIFY(choices.contains(QStringLiteral("My Edit")));
    QVERIFY(!choices.contains(QStringLiteral("Essentials")));
}

void WorkspacesTest::deletedPresetHidesMenuAction()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->deleteWorkspaceForTest(QStringLiteral("Painting")));
    window->refreshWorkspaceMenuForTest();
    QAction* painting = window->registry()->action(pictura::command_ids::WindowWorkspacePainting);
    QVERIFY(painting != nullptr);
    QVERIFY(!painting->isVisible());
    QAction* essentials = window->registry()->action(pictura::command_ids::WindowWorkspaceEssentials);
    QVERIFY(essentials != nullptr);
    QVERIFY(essentials->isVisible());
}

void WorkspacesTest::menuSectionsAreExact()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Alpha")));
    window->switchWorkspaceForTest(QStringLiteral("Essentials"));
    window->refreshWorkspaceMenuForTest();

    QCOMPARE(window->workspaceMenuSeparatorCountForTest(), 2);
    const QStringList labels = window->workspaceMenuLabelsForTest();
    const int user = labels.indexOf(QStringLiteral("Alpha"));
    const int newAction = labels.indexOf(QStringLiteral("New Workspace…"));
    const int delAction = labels.indexOf(QStringLiteral("Delete Workspace…"));
    const int resetAction = labels.indexOf(QStringLiteral("Reset Essentials"));
    const int shortcuts = labels.indexOf(QStringLiteral("Keyboard Shortcuts & Menus…"));
    QVERIFY(user >= 0 && newAction >= 0 && delAction >= 0 && resetAction >= 0 && shortcuts >= 0);
    QVERIFY2(user < newAction, "user workspace precedes the actions");
    QVERIFY2(newAction < shortcuts, "the actions precede the additional items");
    QVERIFY2(delAction > user && delAction < shortcuts, "delete sits in the actions section");
    QVERIFY2(resetAction > user && resetAction < shortcuts, "reset sits in the actions section");
}

void WorkspacesTest::windowPanelsReflectAppliedWorkspace()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    QCoreApplication::processEvents();

    struct Case {
        const char* command;
        const char* panel;
    };
    const Case cases[] = {
        {pictura::command_ids::WindowPanelsNavigator, "navigatorPanel"},
        {pictura::command_ids::WindowPanelsColor, "colorPanel"},
    };
    for (const Case& entry : cases) {
        const QString panel = QString::fromLatin1(entry.panel);
        pictura::PanelColumn* owner = window->columnForPanel(panel);
        QVERIFY2(owner != nullptr, entry.panel);
        QAction* action = window->registry()->action(QString::fromLatin1(entry.command));
        QVERIFY2(action != nullptr, entry.command);
        QCOMPARE(action->isChecked(), owner->isPanelVisible(panel));
    }
    QVERIFY(window->columnForPanel(QStringLiteral("navigatorPanel"))
                ->isPanelVisible(QStringLiteral("navigatorPanel")));
    QVERIFY(!window->columnForPanel(QStringLiteral("colorPanel"))
                 ->isPanelVisible(QStringLiteral("colorPanel")));
}

void WorkspacesTest::activeWorkspaceSurvivesRestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto window = pictura::test::makeMainWindow();
        QVERIFY(window != nullptr);
        window->resize(1100, 700);
        window->show();
        QCoreApplication::processEvents();
        QVERIFY(window->createWorkspaceForTest(QStringLiteral("My Edit")));
        QCOMPARE(window->activeWorkspaceForTest(), QStringLiteral("My Edit"));
    }

    auto window2 = pictura::test::makeMainWindow();
    QVERIFY(window2 != nullptr);
    window2->resize(1100, 700);
    window2->show();
    QCoreApplication::processEvents();
    QCOMPARE(window2->activeWorkspaceForTest(), QStringLiteral("My Edit"));
    auto* switcher = window2->findChild<QToolButton*>(QStringLiteral("workspaceSwitcher"));
    QVERIFY(switcher != nullptr);
    QCOMPARE(switcher->text(), QStringLiteral("My Edit"));
}

void WorkspacesTest::toolsColumnLeftBranch()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    pictura::PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    const int toolsIndex = window->panelColumns().indexOf(tools);
    QVERIFY(toolsIndex >= 0);
    QWidget* toolsContent = tools->toolsContentForTest();
    QVERIFY(toolsContent != nullptr);

    QJsonArray leftGroups;
    leftGroups.append(groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")}));
    QJsonArray rightGroups;
    rightGroups.append(groupSpec({QStringLiteral("colorPanel")}, {QStringLiteral("colorPanel")}));
    window->applyWorkspaceLayoutForTest(QJsonArray{
        columnSpec(QStringLiteral("left"), 0, 320, QStringLiteral("normal"), leftGroups),
        columnSpec(QStringLiteral("right"), 1, 320, QStringLiteral("normal"), rightGroups)});
    QCoreApplication::processEvents();

    QCOMPARE(window->panelColumns().indexOf(tools), toolsIndex);
    QCOMPARE(tools->toolsContentForTest(), toolsContent);

    pictura::PanelColumn* left = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(left != nullptr);
    QVERIFY(left != tools);
    // The left entry lands left of the document tabs.
    QVERIFY(window->sideOf(left) == pictura::PanelSide::Left);
    pictura::PanelColumn* right = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(right != nullptr);
    QVERIFY(window->sideOf(right) == pictura::PanelSide::Right);
}

void WorkspacesTest::emptySpecApplyIsSafe()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    QVERIFY(window->columnForPanel(QStringLiteral("colorPanel")) != nullptr);
    window->applyWorkspaceLayoutForTest(QJsonArray{});
    QCoreApplication::processEvents();

    // No crash, and a previously visible panel stays reachable (hidden).
    pictura::PanelColumn* color = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(color != nullptr);
    QVERIFY(!color->isPanelVisible(QStringLiteral("colorPanel")));
    QVERIFY(window->columnForPanel(QStringLiteral("layersPanel")) != nullptr);
}

void WorkspacesTest::switcherVisibleForEveryTool()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QCoreApplication::processEvents();

    auto* switcher = window->findChild<QToolButton*>(QStringLiteral("workspaceSwitcher"));
    QVERIFY(switcher != nullptr);
    const pictura::ToolId tools[] = {
        pictura::ToolId::Move,   pictura::ToolId::Marquee, pictura::ToolId::Brush,
        pictura::ToolId::Crop,   pictura::ToolId::Gradient, pictura::ToolId::HorizontalType,
    };
    for (pictura::ToolId id : tools) {
        window->setActiveTool(id);
        QCoreApplication::processEvents();
        QVERIFY2(switcher->isVisible(),
                 qPrintable(QStringLiteral("tool %1").arg(static_cast<int>(id))));
    }
}

QTEST_MAIN(WorkspacesTest)

#include "tst_workspaces.moc"
