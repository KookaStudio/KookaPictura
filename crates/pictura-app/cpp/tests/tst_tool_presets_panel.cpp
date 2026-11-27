// Tool Presets panel (#511): the dock surface, create/apply, filters, display
// modes, library commands, the shared options-bar picker, and native snapshots.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/layers_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "options_bar.h"
#include "paint_options.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "panels/tool_presets_panel.h"
#include "preset_manager.h"
#include "preset_store.h"
#include "qt_test_support.h"
#include "tools.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QListView>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include <memory>

using namespace pictura;

namespace {

QByteArray toBytes(const ::rust::Vec<std::uint8_t>& bytes)
{
    return QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
}

ToolPresetsPanel* panelOf(PicturaMainWindow& window)
{
    return window.findChild<ToolPresetsPanel*>(QStringLiteral("toolPresetsPanel"));
}

ToolController* controllerOf(PicturaMainWindow& window)
{
    auto* bar = window.findChild<OptionsBar*>();
    return bar ? bar->controllerForTest() : nullptr;
}

QStringList rowNames(ToolPresetsPanel& panel)
{
    QStringList names;
    for (int index = 0; index < panel.listForTest()->count(); ++index) {
        names.push_back(panel.listForTest()->item(index)->text());
    }
    return names;
}

bool selectRow(ToolPresetsPanel& panel, const QString& name)
{
    for (int index = 0; index < panel.listForTest()->count(); ++index) {
        QListWidgetItem* item = panel.listForTest()->item(index);
        if (item->text() == name) {
            panel.listForTest()->setCurrentItem(item);
            return true;
        }
    }
    return false;
}

QStringList storedNames()
{
    QStringList names;
    for (const auto& row : PresetStore::current().rows(PresetType::Tools)) {
        names.push_back(row.name);
    }
    return names;
}

void setMoveOptions(ToolController& tools, bool autoSelect, bool transformControls)
{
    auto* move = tools.optionsFor<MoveGroup>(ToolId::Move);
    QVERIFY(move);
    MoveOptions options = move->get();
    options.autoSelectOn = autoSelect;
    options.showTransformControls = transformControls;
    move->set(options);
}

QAction* actionWithText(QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
    }
    return nullptr;
}

} // namespace

class ToolPresetsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void panelTogglesFromWindowMenu();
    void paintingWorkspaceHostsPanel();
    void createCapturesActiveToolAndOptions();
    void applySwitchesToolAndRestoresOptions();
    void typePresetRestoresTypeOptions();
    void applyingPresetAddsNoHistory();
    void currentToolOnlyFilters();
    void sortByToolGroups();
    void displayModesChangeList();
    void panelMenuDisplayModes();
    void renameAndDeleteReachPickerAndManager();
    void reorderPersistsAcrossStoreReload();
    void loadReplaceResetSaveTools();
    void discoversToolLibraries();
    void invalidLibraryKeepsList();
    void optionsBarPickerListsEveryTool();
    void nativeToolPresetRoundTripsOptions();
    void unknownOptionFieldsSurviveResave();
    void presetManagerToolsActions();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ToolPresetsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void ToolPresetsPanelTest::init()
{
    QString error;
    QVERIFY2(PresetStore::current().reset(PresetType::Tools, false, &error), qPrintable(error));
}

void ToolPresetsPanelTest::panelTogglesFromWindowMenu()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    QVERIFY(panel);
    PanelColumn* column = window->columnForPanel(QStringLiteral("toolPresetsPanel"));
    QVERIFY(column);
    QAction* action =
        window->registry()->action(QString::fromLatin1(command_ids::WindowPanelsToolPresets));
    QVERIFY(action);
    QVERIFY(action->isCheckable());
    QVERIFY(action->isEnabled());
    QVERIFY(!column->isPanelVisible(QStringLiteral("toolPresetsPanel")));

    action->trigger();
    QVERIFY(column->isPanelVisible(QStringLiteral("toolPresetsPanel")));
    QVERIFY(action->isChecked());

    action->trigger();
    QVERIFY(!column->isPanelVisible(QStringLiteral("toolPresetsPanel")));
    QVERIFY(!action->isChecked());
}

void ToolPresetsPanelTest::paintingWorkspaceHostsPanel()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    window->applyWorkspaceLayoutForTest(workspacePresets(QStringLiteral("Painting")));
    PanelColumn* column = window->columnForPanel(QStringLiteral("toolPresetsPanel"));
    QVERIFY(column);
    QVERIFY(column->isPanelVisible(QStringLiteral("toolPresetsPanel")));
}

void ToolPresetsPanelTest::createCapturesActiveToolAndOptions()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Brush);
    auto* paint = tools->optionsFor<PaintOptions>(ToolId::Brush);
    QVERIFY(paint);
    paint->setSize(12);
    paint->setOpacity(37);
    paint->setFlow(58);

    panel->createPreset(QStringLiteral("Brush 37"));
    QVERIFY(selectRow(*panel, QStringLiteral("Brush 37")));

    const auto rows = PresetStore::current().rows(PresetType::Tools);
    QCOMPARE(rows.size(), 1);
    QCOMPARE(rows.last().name, QStringLiteral("Brush 37"));
    QCOMPARE(rows.last().values.value(QStringLiteral("tool")).toString(),
             QStringLiteral("paintbrushTool"));
    QCOMPARE(rows.last().values.value(QStringLiteral("toolId")).toString(),
             QStringLiteral("brush"));
    const QJsonObject options =
        rows.last().values.value(QStringLiteral("options")).toObject();
    QCOMPARE(options.value(QStringLiteral("brush")).toObject().value(QStringLiteral("opacity")).toInt(),
             37);
    QCOMPARE(options.value(QStringLiteral("brush")).toObject().value(QStringLiteral("flow")).toInt(),
             58);
}

void ToolPresetsPanelTest::applySwitchesToolAndRestoresOptions()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    setMoveOptions(*tools, true, false);
    panel->createPreset(QStringLiteral("Move preset"));

    tools->setActiveTool(ToolId::Brush);
    setMoveOptions(*tools, false, true);
    QVERIFY(selectRow(*panel, QStringLiteral("Move preset")));
    panel->applyCurrent();

    QCOMPARE(tools->activeTool(), ToolId::Move);
    auto* move = tools->optionsFor<MoveGroup>(ToolId::Move);
    QVERIFY(move);
    QVERIFY(move->get().autoSelectOn);
    QVERIFY(!move->get().showTransformControls);
}

void ToolPresetsPanelTest::typePresetRestoresTypeOptions()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::HorizontalType);
    auto* type = tools->optionsFor<TypeGroup>(ToolId::HorizontalType);
    QVERIFY(type);
    TypeOptions options = type->get();
    options.family = QStringLiteral("DejaVu Sans");
    options.size = 42;
    options.justification = 2;
    options.antialias = 3;
    type->set(options);
    panel->createPreset(QStringLiteral("Type preset"));

    TypeOptions changed = type->get();
    changed.family = QStringLiteral("Liberation Sans");
    changed.size = 12;
    changed.justification = 0;
    changed.antialias = 1;
    type->set(changed);

    QVERIFY(selectRow(*panel, QStringLiteral("Type preset")));
    panel->applyCurrent();

    QCOMPARE(type->get().family, QStringLiteral("DejaVu Sans"));
    QCOMPARE(type->get().size, 42.0);
    QCOMPARE(type->get().justification, 2);
    QCOMPARE(type->get().antialias, 3);
}

void ToolPresetsPanelTest::applyingPresetAddsNoHistory()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    QVERIFY(window->newDocument(QStringLiteral("preset"), 64, 64, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view);
    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("No history"));

    const int before = view->history_count();
    const bool dirty = document_api::isDirty(*view);
    QVERIFY(selectRow(*panel, QStringLiteral("No history")));
    panel->applyCurrent();
    QCOMPARE(view->history_count(), before);
    QCOMPARE(document_api::isDirty(*view), dirty);
}

void ToolPresetsPanelTest::currentToolOnlyFilters()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Brush);
    panel->createPreset(QStringLiteral("Brush preset"));
    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Move preset"));

    panel->setCurrentToolOnly(true);
    QVERIFY(panel->currentToolOnlyForTest()->isChecked());
    QCOMPARE(rowNames(*panel), QStringList{QStringLiteral("Move preset")});

    tools->setActiveTool(ToolId::Brush);
    QCOMPARE(rowNames(*panel), QStringList{QStringLiteral("Brush preset")});

    tools->setActiveTool(ToolId::Hand);
    QCOMPARE(rowNames(*panel), QStringList{});

    panel->setCurrentToolOnly(false);
    QCOMPARE(rowNames(*panel).size(), 2);
}

void ToolPresetsPanelTest::sortByToolGroups()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Brush);
    panel->createPreset(QStringLiteral("Brush one"));
    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Move one"));
    tools->setActiveTool(ToolId::Brush);
    panel->createPreset(QStringLiteral("Brush two"));

    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("Brush one"), QStringLiteral("Move one"), QStringLiteral("Brush two")}));
    panel->setSortByTool(true);
    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("Brush one"), QStringLiteral("Brush two"), QStringLiteral("Move one")}));
    panel->setSortByTool(false);
    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("Brush one"), QStringLiteral("Move one"), QStringLiteral("Brush two")}));
}

void ToolPresetsPanelTest::displayModesChangeList()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Display"));
    QListWidget* list = panel->listForTest();
    QCOMPARE(list->count(), 1);

    panel->setDisplayMode(ToolPresetsPanel::DisplayMode::TextOnly);
    QVERIFY(list->item(0)->icon().isNull());

    panel->setDisplayMode(ToolPresetsPanel::DisplayMode::SmallList);
    QCOMPARE(list->iconSize(), QSize(16, 16));
    QVERIFY(!list->item(0)->icon().isNull());

    panel->setDisplayMode(ToolPresetsPanel::DisplayMode::LargeList);
    QCOMPARE(list->iconSize(), QSize(32, 32));
}

void ToolPresetsPanelTest::panelMenuDisplayModes()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    PanelColumn* column = window->columnForPanel(QStringLiteral("toolPresetsPanel"));
    QVERIFY(panel && column);
    QVERIFY(column->showPanel(QStringLiteral("toolPresetsPanel"), true));
    PanelGroup* group = column->groupForPanel(QStringLiteral("toolPresetsPanel"));
    QVERIFY(group);
    QVERIFY(group->setCurrentPanel(QStringLiteral("toolPresetsPanel")));

    QVERIFY2(group->panelMenuEnabledForTest(QStringLiteral("Display")),
             "the Display submenu must be live");
    QVERIFY(group->triggerPanelMenuForTest(QStringLiteral("Text Only")));
    QCOMPARE(panel->displayMode(), ToolPresetsPanel::DisplayMode::TextOnly);
    QVERIFY(group->triggerPanelMenuForTest(QStringLiteral("Small List")));
    QCOMPARE(panel->displayMode(), ToolPresetsPanel::DisplayMode::SmallList);
    QVERIFY(group->triggerPanelMenuForTest(QStringLiteral("Large List")));
    QCOMPARE(panel->displayMode(), ToolPresetsPanel::DisplayMode::LargeList);
}

void ToolPresetsPanelTest::renameAndDeleteReachPickerAndManager()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Old name"));
    QVERIFY(selectRow(*panel, QStringLiteral("Old name")));
    panel->renameCurrent(QStringLiteral("New name"));
    QCOMPARE(storedNames(), QStringList{QStringLiteral("New name")});

    auto* button = window->findChild<QToolButton*>(QStringLiteral("optionsToolPreset-move"));
    QVERIFY(button && button->menu());
    QVERIFY(QMetaObject::invokeMethod(button->menu(), "aboutToShow"));
    QVERIFY(actionWithText(button->menu(), QStringLiteral("New name")));

    PresetManager manager(PresetStore::current(), PresetType::Tools);
    auto* list = manager.findChild<QListView*>(QStringLiteral("presetList"));
    QVERIFY(list);
    QCOMPARE(list->model()->rowCount(), 1);
    QCOMPARE(list->model()->index(0, 0).data(Qt::DisplayRole).toString(),
             QStringLiteral("New name"));

    QVERIFY(selectRow(*panel, QStringLiteral("New name")));
    panel->deleteCurrent();
    QVERIFY(storedNames().isEmpty());
    QVERIFY(QMetaObject::invokeMethod(button->menu(), "aboutToShow"));
    QVERIFY(!actionWithText(button->menu(), QStringLiteral("New name")));
    QCOMPARE(list->model()->rowCount(), 0);
}

void ToolPresetsPanelTest::reorderPersistsAcrossStoreReload()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("One"));
    panel->createPreset(QStringLiteral("Two"));
    panel->createPreset(QStringLiteral("Three"));
    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("One"), QStringLiteral("Two"), QStringLiteral("Three")}));

    panel->moveRowsForTest(0, 3);
    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("Two"), QStringLiteral("Three"), QStringLiteral("One")}));

    PresetStore reloaded;
    QStringList names;
    for (const auto& row : reloaded.rows(PresetType::Tools)) {
        names.push_back(row.name);
    }
    QCOMPARE(names,
             (QStringList{QStringLiteral("Two"), QStringLiteral("Three"), QStringLiteral("One")}));
}

void ToolPresetsPanelTest::loadReplaceResetSaveTools()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Saved one"));
    panel->createPreset(QStringLiteral("Saved two"));

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("saved.kpset"));
    QString error;
    QVERIFY2(savePresetFile(path, PresetType::Tools,
                            PresetStore::current().rows(PresetType::Tools), &error),
             qPrintable(error));
    QVERIFY(QFileInfo::exists(path));

    QVERIFY(PresetStore::current().reset(PresetType::Tools, false, &error));
    QVERIFY(storedNames().isEmpty());

    const auto report = readPresetFile(path, PresetType::Tools);
    QVERIFY2(report.ok, qPrintable(report.diagnostics));
    QVERIFY(PresetStore::current().commitImport(PresetType::Tools, report, false, false, &error));
    QCOMPARE(storedNames(),
             (QStringList{QStringLiteral("Saved one"), QStringLiteral("Saved two")}));
    QCOMPARE(rowNames(*panel),
             (QStringList{QStringLiteral("Saved one"), QStringLiteral("Saved two")}));
}

void ToolPresetsPanelTest::discoversToolLibraries()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    QVERIFY(panel);

    QTemporaryDir data;
    QVERIFY(data.isValid());
    const QByteArray prior = qgetenv("XDG_DATA_HOME");
    qputenv("XDG_DATA_HOME", data.path().toUtf8());
    const QString folder = data.path() + QStringLiteral("/kooka-pictura/presets/tools");
    QVERIFY(QDir().mkpath(folder));
    const QString path = folder + QStringLiteral("/Studio.kpset");
    QString error;
    QVERIFY2(savePresetFile(path, PresetType::Tools,
                            {makeBuiltinPreset(PresetType::Tools, QStringLiteral("Discovered"), 0)},
                            &error),
             qPrintable(error));
    QVERIFY(discoveredPresetFiles(PresetType::Tools).contains(path));

    QMenu menu;
    panel->updatePanelMenu(&menu);
    bool found = false;
    for (QAction* action : menu.actions()) {
        if (action->text() == QStringLiteral("Studio")) {
            found = true;
        }
    }
    qputenv("XDG_DATA_HOME", prior);
    QVERIFY(found);
}

void ToolPresetsPanelTest::invalidLibraryKeepsList()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Keep me"));
    const QStringList before = storedNames();
    const auto report = readPresetLibrary(PresetType::Tools, QStringLiteral("kpset"),
                                          QByteArray("not a native preset"));
    QVERIFY(!report.ok);
    QString error;
    QVERIFY(!PresetStore::current().commitImport(PresetType::Tools, report, true, false, &error));
    QCOMPARE(storedNames(), before);
    QCOMPARE(rowNames(*panel), before);
}

void ToolPresetsPanelTest::optionsBarPickerListsEveryTool()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    setMoveOptions(*tools, true, false);
    panel->createPreset(QStringLiteral("Picker move"));
    setMoveOptions(*tools, false, true);

    auto* button = window->findChild<QToolButton*>(QStringLiteral("optionsToolPreset-move"));
    QVERIFY(button && button->menu());
    QVERIFY(QMetaObject::invokeMethod(button->menu(), "aboutToShow"));
    QAction* action = actionWithText(button->menu(), QStringLiteral("Picker move"));
    QVERIFY(action);
    action->trigger();

    auto* move = tools->optionsFor<MoveGroup>(ToolId::Move);
    QVERIFY(move);
    QVERIFY(move->get().autoSelectOn);
    QVERIFY(!move->get().showTransformControls);
}

void ToolPresetsPanelTest::nativeToolPresetRoundTripsOptions()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::HorizontalType);
    auto* type = tools->optionsFor<TypeGroup>(ToolId::HorizontalType);
    QVERIFY(type);
    TypeOptions options = type->get();
    options.family = QStringLiteral("DejaVu Sans");
    options.size = 33;
    options.justification = 1;
    type->set(options);
    panel->createPreset(QStringLiteral("Round trip"));

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("roundtrip.kpset"));
    QString error;
    QVERIFY2(savePresetFile(path, PresetType::Tools,
                            PresetStore::current().rows(PresetType::Tools), &error),
             qPrintable(error));
    QVERIFY(PresetStore::current().reset(PresetType::Tools, false, &error));
    const auto report = readPresetFile(path, PresetType::Tools);
    QVERIFY(report.ok);
    QVERIFY(PresetStore::current().commitImport(PresetType::Tools, report, true, false, &error));

    TypeOptions changed = type->get();
    changed.family = QStringLiteral("Liberation Sans");
    changed.size = 12;
    changed.justification = 0;
    type->set(changed);
    QVERIFY(selectRow(*panel, QStringLiteral("Round trip")));
    panel->applyCurrent();
    QCOMPARE(type->get().family, QStringLiteral("DejaVu Sans"));
    QCOMPARE(type->get().size, 33.0);
    QCOMPARE(type->get().justification, 1);
}

void ToolPresetsPanelTest::unknownOptionFieldsSurviveResave()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    QVERIFY(panel);

    const QString options =
        QStringLiteral("{\"move\":{\"autoSelectOn\":true},\"future\":{\"value\":7}}");
    PresetRow row;
    row.name = QStringLiteral("Future");
    row.native = toBytes(preset_make_tool(row.name, QStringLiteral("move"), options));
    row.values = QJsonDocument::fromJson(options.toUtf8()).object();
    row.values.insert(QStringLiteral("tool"), QStringLiteral("move"));
    row.values.insert(QStringLiteral("toolId"), QStringLiteral("move"));
    QString error;
    QVERIFY2(PresetStore::current().append(PresetType::Tools, {row}, &error), qPrintable(error));

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("first.kpset"));
    const QString second = dir.filePath(QStringLiteral("second.kpset"));
    QVERIFY2(savePresetFile(first, PresetType::Tools,
                            PresetStore::current().rows(PresetType::Tools), &error),
             qPrintable(error));
    const auto report = readPresetFile(first, PresetType::Tools);
    QVERIFY(report.ok);
    QCOMPARE(report.rows.size(), 1);
    QCOMPARE(report.rows.first()
                 .values.value(QStringLiteral("options"))
                 .toObject()
                 .value(QStringLiteral("future"))
                 .toObject()
                 .value(QStringLiteral("value"))
                 .toInt(),
             7);
    QVERIFY2(savePresetFile(second, PresetType::Tools, report.rows, &error), qPrintable(error));
    const auto again = readPresetFile(second, PresetType::Tools);
    QVERIFY(again.ok);
    QCOMPARE(again.rows.first()
                 .values.value(QStringLiteral("options"))
                 .toObject()
                 .value(QStringLiteral("future"))
                 .toObject()
                 .value(QStringLiteral("value"))
                 .toInt(),
             7);
    QCOMPARE(rowNames(*panel), QStringList{QStringLiteral("Future")});
}

void ToolPresetsPanelTest::presetManagerToolsActions()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    ToolPresetsPanel* panel = panelOf(*window);
    ToolController* tools = controllerOf(*window);
    QVERIFY(panel && tools);

    tools->setActiveTool(ToolId::Move);
    panel->createPreset(QStringLiteral("Manager preset"));

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("manager.kpset"));
    PresetManager manager(PresetStore::current(), PresetType::Tools);
    manager.exportFile(path);
    QTRY_VERIFY(QFileInfo::exists(path));

    QString error;
    QVERIFY(PresetStore::current().reset(PresetType::Tools, false, &error));
    QVERIFY(storedNames().isEmpty());

    manager.importFile(path, false);
    QTRY_COMPARE(storedNames(), QStringList{QStringLiteral("Manager preset")});
    manager.importFile(path, true);
    QTRY_COMPARE(storedNames(), QStringList{QStringLiteral("Manager preset")});
}

PICTURA_QT_TEST_MAIN(ToolPresetsPanelTest)
#include "tst_tool_presets_panel.moc"
