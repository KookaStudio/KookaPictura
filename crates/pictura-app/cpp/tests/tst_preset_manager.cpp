#include "qt_test_support.h"

#include "command_ids.h"
#include "commands.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "gradient_editor_dialog.h"
#include "layer_style_contour.h"
#include "layer_style_dialog.h"
#include "options_bar.h"
#include "panels/brush_preset_picker.h"
#include "panels/swatches_panel.h"
#include "pattern_fill_dialog.h"
#include "preferences_dialog.h"
#include "preset_manager.h"
#include "preset_store.h"
#include "session.h"
#include "tools.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QLockFile>
#include <QtCore/QScopeGuard>
#include <QtCore/QSemaphore>
#include <QtCore/QThreadPool>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QListView>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>

#include <algorithm>
#include <cstdint>

using namespace pictura;

class PresetManagerTest : public QObject {
    Q_OBJECT

private slots:
    void opensWithEightTypesAndViewModes();
    void editsDuplicateNamesByIdentity();
    void loadsReplacesAndResets();
    void commitsOnlyAcceptedImports();
    void ignoresCancelledOrSupersededImport();
    void savesSelectedSetsAtomically();
    void migratesAndPersistsCurrentSets();
    void discoversLibrariesAcrossPreferenceReset();
    void emptyNativeSetLoadAndReplace();
    void migrationResetSurvivesRestart();
    void swatchPersistenceRegressions();
    void appliesImportedSwatchBrushAndToolOptions();
    void appliesImportedGradientStyleAndContour();
    void appliesImportedPatternsAndCustomShape();
    void libraryEditsPreserveStrokeAndDocumentContent();
};

namespace {

PresetRows swatches(std::initializer_list<std::pair<QString, QColor>> colors)
{
    PresetRows rows;
    for (const auto& [name, color] : colors) {
        rows.push_back(makeSwatchPreset(name, color));
    }
    return rows;
}

QStringList names(const PresetRows& rows)
{
    QStringList result;
    for (const auto& row : rows) {
        result.push_back(row.name);
    }
    return result;
}

QByteArray word(quint16 value)
{
    return QByteArray(1, char(value >> 8)) + char(value & 255);
}

QByteArray number(quint32 value)
{
    return word(quint16(value >> 16)) + word(quint16(value));
}

QByteArray blob(const QByteArray& value) { return number(value.size()) + value; }

QByteArray code(const QByteArray& value)
{
    return value.size() == 4 ? number(0) + value : blob(value);
}

QByteArray descriptor(const QByteArray& cls, std::initializer_list<QByteArray> items)
{
    auto bytes = number(0) + code(cls) + number(items.size());
    for (const auto& item : items) bytes += item;
    return bytes;
}

QByteArray integer(const QByteArray& key, int value) { return code(key) + "long" + number(value); }

PresetImport importPreset(PresetType type, const QByteArray& name, int tag, const QByteArray& payload)
{
    const auto entry = blob(name) + number(0) + char(tag) + payload;
    const auto bytes = QByteArray("KPPRESET\0\1", 10) + char(type) + number(1) + blob(entry);
    return readPresetLibrary(type, QStringLiteral("kpset"), bytes);
}

PresetImport brushPreset()
{
    const auto tip = descriptor("samp", {integer("Dmtr", 16), integer("Spcn", 25)});
    return importPreset(PresetType::Brushes, "Sample tip", 8, number(2) + number(2)
        + blob(QByteArray(4, char(255))) + blob(descriptor("Brsh", {code("Brsh") + "Objc" + tip})));
}

PresetImport patternPreset()
{
    return importPreset(PresetType::Patterns, "Blue tile", 4, number(2) + number(2)
        + blob("source-tile") + blob(QByteArray::fromHex("2255ccff2255ccff2255ccff2255ccff")));
}

bool createDocument(PicturaMainWindow& window)
{
    return window.newDocument(QStringLiteral("Preset"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"));
}

::rust::Slice<const std::uint8_t> presetBytes(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), size_t(bytes.size())};
}

} // namespace

void PresetManagerTest::opensWithEightTypesAndViewModes()
{
    test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = test::makeMainWindow();
    bool checked = false;
    QTimer::singleShot(0, window.get(), [&] {
        auto* dialog = window->findChild<PresetManager*>();
        if (!dialog) {
            return;
        }
        dialog->reject();
        auto* type = dialog->findChild<QComboBox*>(QStringLiteral("presetType"));
        auto* view = dialog->findChild<QComboBox*>(QStringLiteral("presetDisplay"));
        QVERIFY(type && view);
        QCOMPARE(type->count(), 8);
        type->setCurrentIndex(0);
        QCOMPARE(view->count(), 6);
        type->setCurrentIndex(1);
        QCOMPARE(view->count(), 5);
        checked = true;
    });
    QVERIFY(window->registry()->dispatch(QString::fromLatin1(command_ids::EditPresetManager)));
    QVERIFY(checked);
}

void PresetManagerTest::editsDuplicateNamesByIdentity()
{
    test::ScopedStateHome home;
    auto& store = PresetStore::current();
    const auto first = test::makeMainWindow();
    const auto second = test::makeMainWindow();
    QVERIFY(store.replace(PresetType::Swatches, swatches({{QStringLiteral("A"), Qt::red}, {QStringLiteral("A"), Qt::blue}})));
    PresetManager dialog(store, PresetType::Swatches);
    auto* model = dialog.findChild<QAbstractItemModel*>(QStringLiteral("presetListModel"));
    QVERIFY(model);
    const auto id = model->index(1, 0).data(Qt::UserRole).toULongLong();
    QVERIFY(model->moveRow(QModelIndex(), 1, QModelIndex(), 0));
    QCOMPARE(model->index(0, 0).data(Qt::UserRole).toULongLong(), id);
    QVERIFY(model->setData(model->index(0, 0), QStringLiteral("藍")));
    QVERIFY(model->removeRow(1));
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("藍")});
    for (const auto* window : {first.get(), second.get()}) {
        const auto* grid = window->findChild<SwatchGrid*>();
        QVERIFY(grid);
        QCOMPARE(grid->swatches().size(), 1);
        QCOMPARE(grid->swatches().front().color, QColor(Qt::blue));
    }
    const auto spot = importPreset(PresetType::Swatches, "same", 1,
        QByteArray::fromHex("013f800000003f8000000000000000000000"));
    QVERIFY(spot.ok);
    QCOMPARE(spot.rows.size(), 1);
    const auto duplicate = spot.rows.front();
    auto original = makeSwatchPreset(QStringLiteral("same"), Qt::red);
    QCOMPARE(duplicate.color, original.color);
    QVERIFY(duplicate.native != original.native);
    QVERIFY(store.replace(PresetType::Swatches, {original, duplicate}));
    const auto secondId = store.rows(PresetType::Swatches).at(1).id;
    auto* grid = first->findChild<SwatchGrid*>();
    QVERIFY(grid->removeSwatch(0));
    QCOMPARE(store.rows(PresetType::Swatches).front().id, secondId);
    QCOMPARE(store.rows(PresetType::Swatches).front().native, duplicate.native);

    auto* list = dialog.findChild<QListView*>(QStringLiteral("presetList"));
    list->setCurrentIndex(model->index(0, 0));
    QTimer::singleShot(0, &dialog, [&] {
        auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
        QVERIFY(input);
        QVERIFY(store.replace(PresetType::Swatches, swatches({{QStringLiteral("new"), Qt::blue}})));
        input->setTextValue(QStringLiteral("stale"));
        input->accept();
    });
    QPushButton* rename = nullptr;
    for (auto* button : dialog.findChildren<QPushButton*>()) {
        if (button->text().startsWith(QStringLiteral("Rename"))) { rename = button; break; }
    }
    QVERIFY(rename);
    rename->click();
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("new")});
}

void PresetManagerTest::loadsReplacesAndResets()
{
    test::ScopedStateHome home;
    auto& store = PresetStore::current();
    const auto rows = swatches({{QStringLiteral("Extra"), Qt::blue}});
    const auto defaultCount = store.rows(PresetType::Swatches).size();
    QVERIFY(store.append(PresetType::Swatches, rows));
    QCOMPARE(store.rows(PresetType::Swatches).size(), defaultCount + 1);
    QVERIFY(store.replace(PresetType::Swatches, rows));
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("Extra")});
    QVERIFY(store.reset(PresetType::Swatches, true));
    QCOMPARE(store.rows(PresetType::Swatches).size(), defaultCount + 1);
    QVERIFY(store.reset(PresetType::Swatches, false));
    QCOMPARE(store.rows(PresetType::Swatches).size(), defaultCount);
}

void PresetManagerTest::commitsOnlyAcceptedImports()
{
    test::ScopedStateHome home;
    auto& store = PresetStore::current();
    const auto before = names(store.rows(PresetType::Swatches));
    const QByteArray partial = QByteArray::fromHex("00010002 00630000000000000000 0000ffff000000000000");
    const auto report = readPresetLibrary(PresetType::Swatches, QStringLiteral("aco"), partial);
    QVERIFY(report.ok);
    QCOMPARE(report.rows.size(), 1);
    QVERIFY(!report.diagnostics.isEmpty());
    QVERIFY(!store.commitImport(PresetType::Swatches, report, false, false));
    QCOMPARE(names(store.rows(PresetType::Swatches)), before);
    QVERIFY(store.commitImport(PresetType::Swatches, report, false, true));
    QCOMPARE(store.rows(PresetType::Swatches).size(), before.size() + 1);
    const auto bad = readPresetLibrary(PresetType::Swatches, QStringLiteral("aco"), QByteArrayLiteral("bad"));
    QVERIFY(!store.commitImport(PresetType::Swatches, bad, true, true));
    QCOMPARE(store.rows(PresetType::Swatches).size(), before.size() + 1);
}

void PresetManagerTest::ignoresCancelledOrSupersededImport()
{
    test::ScopedStateHome home;
    QTemporaryDir files;
    auto& store = PresetStore::current();
    const auto originalRows = store.rows(PresetType::Swatches);
    const auto before = names(originalRows);
    const auto stagedRows = swatches({{QStringLiteral("Staged"), Qt::blue}});
    QString stageError;
    auto staged = stagePresetImport(PresetType::Swatches, stagedRows, &stageError);
    QVERIFY2(staged, qPrintable(stageError));
    QVERIFY(store.replace(PresetType::Gradients, {}));
    QVERIFY(store.commitStagedImport(PresetType::Swatches, stagedRows, staged, &stageError));
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("Staged")});
    QVERIFY(store.replace(PresetType::Swatches, originalRows));
    const auto path = files.filePath(QStringLiteral("delayed.kpset"));
    QVERIFY(savePresetFile(path, PresetType::Swatches, swatches({{QStringLiteral("Stale"), Qt::red}})));
    PresetManager dialog(store, PresetType::Swatches);
    auto* type = dialog.findChild<QComboBox*>(QStringLiteral("presetType"));
    QVERIFY(type);
    const auto stale = store.beginImport(PresetType::Swatches);
    const auto latest = store.beginImport(PresetType::Swatches);
    QVERIFY(!store.isCurrentImport(PresetType::Swatches, stale));
    QVERIFY(store.isCurrentImport(PresetType::Swatches, latest));
    store.cancelImport(PresetType::Swatches, latest);
    QVERIFY(!store.isCurrentImport(PresetType::Swatches, latest));
    auto* pool = QThreadPool::globalInstance();
    const int oldThreads = pool->maxThreadCount();
    pool->setMaxThreadCount(1);
    const auto restore = qScopeGuard([pool, oldThreads] { pool->setMaxThreadCount(oldThreads); });
    QSemaphore worker;
    pool->start([&worker] { worker.acquire(); });
    dialog.importFile(path, true);
    type->setCurrentIndex(int(PresetType::Gradients));
    type->setCurrentIndex(int(PresetType::Swatches));
    worker.release();
    QVERIFY(pool->waitForDone(5000));
    QCoreApplication::sendPostedEvents();
    QCOMPARE(names(store.rows(PresetType::Swatches)), before);

    dialog.importFile(path, true);
    QVERIFY(pool->waitForDone(5000));
    QSemaphore stagingBlock;
    pool->start([&stagingBlock] { stagingBlock.acquire(); });
    QCoreApplication::sendPostedEvents();
    type->setCurrentIndex(int(PresetType::Gradients));
    type->setCurrentIndex(int(PresetType::Swatches));
    stagingBlock.release();
    QVERIFY(pool->waitForDone(5000));
    QCoreApplication::sendPostedEvents();
    QCOMPARE(names(store.rows(PresetType::Swatches)), before);
}

void PresetManagerTest::savesSelectedSetsAtomically()
{
    test::ScopedStateHome home;
    QTemporaryDir files;
    auto& store = PresetStore::current();
    const auto entries = swatches({{QStringLiteral("1"), Qt::red}, {QStringLiteral("2"), Qt::green}, {QStringLiteral("3"), Qt::blue}});
    QVERIFY(store.replace(PresetType::Swatches, entries));
    PresetManager dialog(store, PresetType::Swatches);
    auto* list = dialog.findChild<QListView*>(QStringLiteral("presetList"));
    QVERIFY(list);
    QItemSelectionModel* selected = list->selectionModel();
    selected->select(list->model()->index(2, 0), QItemSelectionModel::Select);
    selected->select(list->model()->index(0, 0), QItemSelectionModel::Select);
    const QString path = files.filePath(QStringLiteral("selected.kpset"));
    dialog.exportFile(path);
    QVERIFY(QThreadPool::globalInstance()->waitForDone(5000));
    const auto saved = readPresetFile(path, PresetType::Swatches);
    QCOMPARE(names(saved.rows), (QStringList{QStringLiteral("1"), QStringLiteral("3")}));
    selected->clearSelection();
    const QString all = files.filePath(QStringLiteral("all.kpset"));
    dialog.exportFile(all);
    QVERIFY(QThreadPool::globalInstance()->waitForDone(5000));
    QCOMPARE(names(readPresetFile(all, PresetType::Swatches).rows), names(entries));
    QVERIFY(!savePresetFile(path, PresetType::Gradients, entries));
    QCOMPARE(names(readPresetFile(path, PresetType::Swatches).rows), names(saved.rows));
}

void PresetManagerTest::migratesAndPersistsCurrentSets()
{
    test::ScopedStateHome home;
    SessionState old;
    old.swatchesLoaded = true;
    old.swatches.push_back(QJsonObject{{QStringLiteral("name"), QStringLiteral("Custom")},
                                      {QStringLiteral("color"), QStringLiteral("#123456")}});
    QVERIFY(saveSession(old));
    PresetStore store;
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("Custom")});
    auto rows = store.rows(PresetType::Swatches);
    rows[0].name = QStringLiteral("Renamed");
    QVERIFY(store.replace(PresetType::Swatches, rows));
    QVERIFY(store.replace(PresetType::Gradients, {}));
    PresetStore next;
    QCOMPARE(names(next.rows(PresetType::Swatches)), QStringList{QStringLiteral("Renamed")});
    QVERIFY(next.rows(PresetType::Gradients).isEmpty());
    {
        test::ScopedStateHome emptyHome;
        SessionState empty;
        empty.swatchesLoaded = true;
        QVERIFY(saveSession(empty));
        PresetStore migrated;
        QVERIFY(migrated.rows(PresetType::Swatches).isEmpty());
        PresetStore reopened;
        QVERIFY(reopened.rows(PresetType::Swatches).isEmpty());
    }
}

void PresetManagerTest::discoversLibrariesAcrossPreferenceReset()
{
    test::ScopedStateHome home;
    QTemporaryDir data;
    const auto prior = qgetenv("XDG_DATA_HOME");
    const auto restore = qScopeGuard([prior] {
        if (prior.isNull()) qunsetenv("XDG_DATA_HOME");
        else qputenv("XDG_DATA_HOME", prior);
    });
    qputenv("XDG_DATA_HOME", data.path().toUtf8());
    auto& store = PresetStore::current();
    const auto folder = presetLibraryFolder(PresetType::Swatches);
    QVERIFY(QDir().mkpath(folder));
    const QString file = QDir(folder).filePath(QStringLiteral("External.kpset"));
    QVERIFY(savePresetFile(file, PresetType::Swatches, swatches({{QStringLiteral("A"), Qt::red}})));
    QVERIFY(store.replace(PresetType::Swatches, {}));
    PresetManager manager(store, PresetType::Swatches);
    auto* menu = manager.findChild<QMenu*>(QStringLiteral("presetLibraries"));
    QVERIFY(menu);
    QVERIFY(QMetaObject::invokeMethod(menu, "aboutToShow"));
    const auto actions = menu->actions();
    QVERIFY(std::any_of(actions.cbegin(), actions.cend(),
        [](QAction* action) { return action->text() == QStringLiteral("External"); }));
    PreferencesDialog preferences;
    auto* reset = preferences.findChild<QPushButton*>(QStringLiteral("preferencesResetPresetLists"));
    QVERIFY(reset);
    bool warned = false;
    QTimer::singleShot(0, &preferences, [&] {
        auto* prompt = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
        if (!prompt) return;
        warned = prompt->text().contains(QStringLiteral("Saved libraries are kept"));
        prompt->button(QMessageBox::Yes)->click();
    });
    reset->click();
    QVERIFY(warned);
    QVERIFY(!store.rows(PresetType::Swatches).isEmpty());
    QVERIFY(discoveredPresetFiles(PresetType::Swatches).contains(file));
    QCOMPARE(names(readPresetFile(file, PresetType::Swatches).rows), QStringList{QStringLiteral("A")});
}

void PresetManagerTest::emptyNativeSetLoadAndReplace()
{
    test::ScopedStateHome home;
    QTemporaryDir files;
    auto& store = PresetStore::current();
    const QString path = files.filePath(QStringLiteral("empty.kpset"));
    QVERIFY(savePresetFile(path, PresetType::Swatches, {}));
    const auto report = readPresetFile(path, PresetType::Swatches);
    QVERIFY(report.ok);
    QVERIFY(report.rows.isEmpty());
    const auto before = names(store.rows(PresetType::Swatches));
    QVERIFY(store.commitImport(PresetType::Swatches, report, false, true));
    QCOMPARE(names(store.rows(PresetType::Swatches)), before);
    QVERIFY(store.commitImport(PresetType::Swatches, report, true, true));
    QVERIFY(store.rows(PresetType::Swatches).isEmpty());
}

void PresetManagerTest::migrationResetSurvivesRestart()
{
    test::ScopedStateHome home;
    SessionState old;
    old.swatchesLoaded = true;
    old.swatches.push_back(QJsonObject{{QStringLiteral("name"), QStringLiteral("Legacy")},
                                      {QStringLiteral("color"), QStringLiteral("#654321")}});
    QVERIFY(saveSession(old));
    PresetStore store;
    QCOMPARE(names(store.rows(PresetType::Swatches)), QStringList{QStringLiteral("Legacy")});
    QVERIFY(store.replace(PresetType::Gradients, {}));
    QLockFile lock(QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/preset-current/active.lock"));
    QVERIFY(lock.tryLock());
    QString error;
    QVERIFY(!store.resetAll(&error));
    QVERIFY(!error.isEmpty());
    PresetStore prior;
    QCOMPARE(names(prior.rows(PresetType::Swatches)), QStringList{QStringLiteral("Legacy")});
    QVERIFY(prior.rows(PresetType::Gradients).isEmpty());
    QVERIFY(loadSession().swatchesLoaded);
    lock.unlock();
    QVERIFY(store.resetAll());
    const QDir current(QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/preset-current"));
    QCOMPARE(current.entryList({QStringLiteral("*.kpset")}, QDir::Files).size(), presetTypeCount);
    QVERIFY(store.replace(PresetType::Swatches, swatches({{QStringLiteral("new"), Qt::red}})));
    QCOMPARE(current.entryList({QStringLiteral("*.kpset")}, QDir::Files).size(), presetTypeCount);
    PresetStore next;
    QVERIFY(!names(next.rows(PresetType::Swatches)).contains(QStringLiteral("Legacy")));
    QVERIFY(loadSession().swatchesLoaded);
}

void PresetManagerTest::swatchPersistenceRegressions()
{
    test::ScopedStateHome home;
    PresetStore store;
    QVERIFY(store.replace(PresetType::Swatches, swatches({{QStringLiteral("Saved"), Qt::green}})));
    QVERIFY(saveSession(loadSession()));
    PresetStore next;
    QCOMPARE(names(next.rows(PresetType::Swatches)), QStringList{QStringLiteral("Saved")});
}

void PresetManagerTest::appliesImportedSwatchBrushAndToolOptions()
{
    test::ScopedStateHome home;
    auto window = test::makeMainWindow();
    auto& store = PresetStore::current();
    const auto color = readPresetLibrary(PresetType::Swatches, QStringLiteral("aco"),
        QByteArray::fromHex("00010001 0000ffff000000000000"));
    QVERIFY(store.commitImport(PresetType::Swatches, color, true, false));
    auto* grid = window->findChild<SwatchGrid*>();
    QVERIFY(grid);
    QPoint swatch(5, 5);
    QCOMPARE(grid->indexAt(swatch), 0);
    QTest::mouseClick(grid, Qt::LeftButton, Qt::NoModifier, swatch);
    QCOMPARE(window->foregroundColor(), QColor(Qt::red));

    const auto brush = brushPreset();
    QVERIFY(store.commitImport(PresetType::Brushes, brush, true, false));
    auto* options = window->findChild<OptionsBar*>();
    QVERIFY(options);
    auto* controller = options->controllerForTest();
    BrushPresetPicker picker(controller, window.get());
    picker.choosePresetForTest(0);
    auto* paint = controller->optionsFor<PaintOptions>(ToolId::Brush);
    QVERIFY(paint);
    QCOMPARE(paint->size(), 16);
    QVERIFY(!paint->sampledPreset().isEmpty());
    QVERIFY(createDocument(*window));
    auto* view = window->activeView();
    view->set_gpu_compute(false);
    const PaintTip tip{16, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    QVERIFY(begin_sampled_brush(*view, 0xffff0000u, 0xffffffffu, tip, 100, 100,
        QStringLiteral("normal"), false, false, false, false, presetBytes(paint->sampledPreset())));
    QVERIFY(view->paint_dab(20, 18, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->sample_argb(20, 18), 0xffff0000u);

    const auto tool = importPreset(PresetType::Tools, "Test Brush Settings", 7,
        blob(descriptor("paintbrushTool", {integer("Opct", 37), integer("Flw ", 58)})));
    QVERIFY(store.commitImport(PresetType::Tools, tool, true, false));
    auto* button = window->findChild<QToolButton*>(QStringLiteral("optionsToolPreset-brush"));
    QVERIFY(button && button->menu());
    QVERIFY(QMetaObject::invokeMethod(button->menu(), "aboutToShow"));
    QVERIFY(QMetaObject::invokeMethod(button->menu(), "aboutToShow"));
    const auto actions = button->menu()->actions();
    QCOMPARE(actions.size(), BrushPresetPicker::additionalPresetCount() + 1);
    const auto choice = std::find_if(actions.cbegin(), actions.cend(), [](QAction* action) {
        return action->text() == QStringLiteral("Test Brush Settings");
    });
    QVERIFY(choice != actions.cend());
    (*choice)->trigger();
    QCOMPARE(paint->opacity(), 37);
    QCOMPARE(paint->flow(), 58);
}

void PresetManagerTest::appliesImportedGradientStyleAndContour()
{
    test::ScopedStateHome home;
    auto& store = PresetStore::current();
    const auto stop = descriptor("Clrt", {integer("Lctn", 0), integer("Mdpn", 40),
        code("Type") + "enum" + code("Clry") + code("FrgC")});
    const auto last = descriptor("Clrt", {integer("Lctn", 4096), integer("Mdpn", 50),
        code("Type") + "enum" + code("Clry") + code("BckC")});
    const auto alpha = descriptor("TrnS", {integer("Lctn", 0), integer("Opct", 70)});
    const auto ramp = descriptor("Grdn", {code("Clrs") + "VlLs" + number(2) + "Objc" + stop + "Objc" + last,
        code("Trns") + "VlLs" + number(1) + "Objc" + alpha});
    QVERIFY(store.commitImport(PresetType::Gradients, importPreset(PresetType::Gradients,
        "Test ramp", 5, blob(ramp)), true, false));
    GradientEditorDialog gradients(GradientSpec{}, Qt::red, Qt::blue);
    gradients.choosePreset(0);
    const auto stops = gradients.spec().stops;
    QVERIFY(stops.size() >= 3);
    QCOMPARE(stops.first().color.red(), 255);
    QCOMPARE(stops.last().color.blue(), 255);
    QCOMPARE(stops.first().color.alpha(), 179);

    const auto contour = number(0) + number(255) + number(3) + word(0) + word(0) + char(0)
        + word(128) + word(200) + char(1) + word(255) + word(255) + char(0);
    QVERIFY(store.commitImport(PresetType::Contours, importPreset(PresetType::Contours,
        "Test contour", 2, contour), true, false));
    ContourEditorDialog contours{QString()};
    auto* contourPicker = contours.findChild<QComboBox*>(QStringLiteral("contourPreset"));
    QVERIFY(contourPicker);
    contourPicker->setCurrentIndex(1);
    QVERIFY(contours.pointsText().contains(QStringLiteral("128,200")));

    const auto effect = descriptor("DrSh", {code("enab") + "bool" + char(1),
        code("present") + "bool" + char(1), integer("Opct", 42)});
    const auto style = descriptor("Lefx", {code("DrSh") + "Objc" + effect});
    QVERIFY(store.commitImport(PresetType::Styles, importPreset(PresetType::Styles,
        "Test shadow", 6, blob(style)), true, false));
    auto window = test::makeMainWindow();
    QVERIFY(createDocument(*window));
    auto* view = window->activeView();
    QVERIFY(view->select_rect(10, 10, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(!view->layer_via_copy(QStringLiteral("0")).isEmpty());
    view->deselect();
    LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
    auto* stylePicker = dialog.findChild<QComboBox*>(QStringLiteral("layerStylePreset"));
    QVERIFY(stylePicker && stylePicker->count() == 1);
    QVERIFY(QMetaObject::invokeMethod(stylePicker, "activated", Q_ARG(int, 0)));
    QCOMPARE(layer_style_value(*view, QStringLiteral("1"), QStringLiteral("dropShadow.opacity")), 42.0);
    dialog.accept();
}

void PresetManagerTest::appliesImportedPatternsAndCustomShape()
{
    test::ScopedStateHome home;
    auto window = test::makeMainWindow();
    QVERIFY(createDocument(*window));
    auto* view = window->activeView();
    view->set_gpu_compute(false);
    auto& store = PresetStore::current();
    const auto pattern = patternPreset();
    QVERIFY(store.commitImport(PresetType::Patterns, pattern, true, false));
    auto* patternPicker = window->findChild<QComboBox*>(QStringLiteral("optionsPatternStampPattern"));
    QVERIFY(patternPicker);
    QCOMPARE(patternPicker->itemText(0), QStringLiteral("Blue tile"));
    const PaintTip tip{10, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    QVERIFY(begin_pattern_preset(*view, tip, 100, 100, QStringLiteral("normal"),
        presetBytes(pattern.rows.first().native), 0, 0));
    QVERIFY(view->paint_dab(15, 15, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->sample_argb(15, 15), 0xff2255ccu);

    auto records = word(0) + word(2) + QByteArray(22, '\0');
    for (quint32 x : {0u, 0x1000000u}) {
        records += word(1) + number(0x200000) + number(x) + number(0x800000)
            + number(x) + number(0xe00000) + number(x);
    }
    const auto shape = importPreset(PresetType::CustomShapes, "Test shape", 3,
        number(0) + number(0) + number(20) + number(40) + blob(records));
    QVERIFY(store.commitImport(PresetType::CustomShapes, shape, true, false));
    auto* list = window->findChild<QListWidget*>(QStringLiteral("optionsShapeCustomGrid"));
    QVERIFY(list);
    QCOMPARE(list->item(0)->toolTip(), QStringLiteral("Test shape"));
    list->setCurrentRow(0);
    ShapeSpec spec{};
    spec.kind = 5;
    spec.boxed = true;
    spec.x0 = 5;
    spec.y0 = 5;
    spec.x1 = 25;
    spec.y1 = 15;
    for (const unsigned char byte : list->item(0)->data(Qt::UserRole + 1).toByteArray()) {
        spec.custom_preset.push_back(byte);
    }
    const auto outline = shape_outline(spec);
    QVERIFY(outline.size() > 4);
    QVERIFY(shape_add_path(*view, spec));
}

void PresetManagerTest::libraryEditsPreserveStrokeAndDocumentContent()
{
    test::ScopedStateHome home;
    QTemporaryDir files;
    auto window = test::makeMainWindow();
    QVERIFY(createDocument(*window));
    auto* view = window->activeView();
    view->set_gpu_compute(false);
    auto& store = PresetStore::current();
    const auto brush = brushPreset();
    QVERIFY(store.commitImport(PresetType::Brushes, brush, true, false));
    const PaintTip tip{16, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    QVERIFY(begin_sampled_brush(*view, 0xffff0000u, 0xffffffffu, tip, 100, 100,
        QStringLiteral("normal"), false, false, false, false, presetBytes(brush.rows.first().native)));
    const int history = view->history_count();
    QVERIFY(store.reset(PresetType::Brushes, false));
    QCOMPARE(view->history_count(), history);
    QVERIFY(view->paint_dab(20, 18, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->sample_argb(20, 18), 0xffff0000u);

    const auto pattern = patternPreset();
    QVERIFY(store.commitImport(PresetType::Patterns, pattern, true, false));
    PatternFillDialog fill(*view, QString());
    const auto items = fill.listForTest()->findItems(QStringLiteral("Blue tile"), Qt::MatchExactly);
    QCOMPARE(items.size(), 1);
    fill.listForTest()->setCurrentItem(items.first());
    const auto layer = add_fill_layer(*view, QStringLiteral("pattern"), fill.payload(),
        layer_selection_anchor(*view), QString(), 0, QStringLiteral("norm"), 255, false);
    QVERIFY(!layer.isEmpty());
    QCOMPARE(view->composite_argb(4, 4), 0xff2255ccu);
    const int afterFill = view->history_count();
    QVERIFY(store.replace(PresetType::Patterns, {}));
    QCOMPARE(view->history_count(), afterFill);
    const auto path = files.filePath(QStringLiteral("pattern.psd"));
    QVERIFY(view->save(path));
    auto reopened = test::makeMainWindow();
    QVERIFY(reopened->openDocumentAtPath(path, false));
    QCOMPARE(reopened->activeView()->composite_argb(4, 4), 0xff2255ccu);
}

QTEST_MAIN(PresetManagerTest)

#include "tst_preset_manager.moc"
