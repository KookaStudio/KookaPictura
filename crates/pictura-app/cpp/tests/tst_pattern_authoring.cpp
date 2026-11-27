#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "fill_dialog.h"
#include "preset_store.h"
#include "session.h"
#include "qt_test_support.h"
#include "selftest_paint_fixture.h"

#include <QtCore/QTimer>
#include <QtCore/QFileInfo>
#include <QtCore/QLockFile>
#include <QtGui/QStandardItemModel>
#include <QtTest/QtTest>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>

class PatternAuthoringTest : public QObject {
    Q_OBJECT
private slots:
    void defineAndReuse();
    void invalidAndCancel();
    void consumerReuse();
    void defineFailure();
    void fillPreset();
    void fillReplay();
    void emptyList();
    void scriptBounds();
    void scriptEdit();
    void scriptCancel();
    void shapePatternAtomicity();
    void captureMemoryAndStaleEdit();
    void builtinSnapshot();
};


void PatternAuthoringTest::defineAndReuse()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(8, 8, QImage::Format_ARGB32);
    seed.fill(Qt::black);
    seed.setPixelColor(2, 2, Qt::red);
    seed.setPixelColor(3, 2, Qt::green);
    seed.setPixelColor(2, 3, Qt::blue);
    seed.setPixelColor(3, 3, Qt::yellow);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("define_pattern"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(2, 2, 2, 2, QStringLiteral("new"), 0.0));
    auto& store = pictura::PresetStore::current();
    const auto count = store.rows(pictura::PresetType::Patterns).size();
    const int before = f.view->history_index();
    bool named = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
        if (!dialog || dialog->objectName() != QStringLiteral("definePatternDialog")) return;
        named = true;
        dialog->setTextValue(QStringLiteral("Asymmetric tile"));
        dialog->accept();
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    QVERIFY(named);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(2, 2), QColor(Qt::red).rgba());
    const auto rows = store.rows(pictura::PresetType::Patterns);
    QCOMPARE(rows.size(), count + 1);
    const auto row = rows.last();
    QCOMPARE(row.name, QStringLiteral("Asymmetric tile"));
    QVERIFY(row.native.contains(QByteArray::fromHex("ff0000ff00ff00ff0000ffffffff00ff")));
    pictura::PresetStore reloaded;
    const auto persisted = reloaded.rows(pictura::PresetType::Patterns);
    QCOMPARE(persisted.size(), count + 1);
    QCOMPARE(persisted.last().name, row.name);
    QCOMPARE(persisted.last().native, row.native);
}


void PatternAuthoringTest::consumerReuse()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(32, 32, QImage::Format_RGB32);
    seed.fill(Qt::white);
    for (int y = 0; y < 2; ++y) {
        for (int x = 0; x < 2; ++x) seed.setPixelColor(x, y, QColor(0x22, 0x55, 0xcc));
    }
    paint_fixture::Fixture f(*window, seed, QStringLiteral("pattern_consumers"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(0, 0, 2, 2, QStringLiteral("new"), 0.0));
    QTimer::singleShot(0, [] {
        if (auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget())) {
            input->setTextValue(QStringLiteral("Consumer tile"));
            input->accept();
        }
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    const auto row = pictura::PresetStore::current().rows(pictura::PresetType::Patterns).last();
    QCOMPARE(row.name, QStringLiteral("Consumer tile"));
    const QString id = row.values.value(QStringLiteral("patternId")).toString();
    QVERIFY(!id.isEmpty());
    QVERIFY(!row.native.isEmpty());
    f.view->deselect();
    const QRgb blue = QColor(0x22, 0x55, 0xcc).rgba();

    window->setActiveTool(pictura::ToolId::PaintBucket);
    auto* fill = window->findChild<QComboBox*>(QStringLiteral("optionsBucketFill"));
    auto* picker = window->findChild<QComboBox*>(QStringLiteral("optionsBucketPattern"));
    QVERIFY(fill && picker);
    fill->setCurrentIndex(1);
    const int pattern = picker->findText(QStringLiteral("Consumer tile"));
    QVERIFY(pattern >= 0);
    picker->setCurrentIndex(pattern);
    f.drag({QPointF(10, 10)});
    QCOMPARE(f.view->sample_argb(10, 10), blue);
    QVERIFY(f.view->undo());

    const pictura::PaintTip tip{10, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    const ::rust::Slice<const std::uint8_t> bytes(
        reinterpret_cast<const std::uint8_t*>(row.native.constData()), size_t(row.native.size()));
    QVERIFY(pictura::begin_pattern_preset(*f.view, tip, 100, 100, QStringLiteral("normal"), bytes, 0, 0));
    QVERIFY(f.view->paint_dab(10, 10, 1.0));
    QVERIFY(f.view->end_paint());
    QCOMPARE(f.view->sample_argb(10, 10), blue);
    QVERIFY(f.view->undo());

    const QString payload = id + QStringLiteral("|100|1|0|0|") + QString::fromLatin1(row.native.toHex());
    const QString layer = pictura::add_fill_layer(*f.view, QStringLiteral("pattern"), payload,
        pictura::layer_selection_anchor(*f.view), QString(), 0, QStringLiteral("norm"), 255, false);
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::document_pattern_ids(*f.view).contains(id));
    QVERIFY(pictura::layer_fill_pattern(*f.view, layer).startsWith(id + QLatin1Char('|')));
    QCOMPARE(f.view->sample_argb(10, 10), blue);
    QVERIFY(f.view->undo());

    window->setActiveTool(pictura::ToolId::Rectangle);
    auto group = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle);
    auto options = group->get();
    options.mode = 0;
    options.fillEnabled = true;
    options.fillKind = 3;
    options.fillPattern = payload;
    options.strokeEnabled = false;
    group->set(options);
    f.drag({QPointF(5, 5), QPointF(20, 20)});
    QVERIFY(pictura::document_pattern_ids(*f.view).contains(id));
    QCOMPARE(f.view->sample_argb(10, 10), blue);
}

void PatternAuthoringTest::invalidAndCancel()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::green);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("invalid_pattern"));
    QVERIFY(f.ok());
    auto& store = pictura::PresetStore::current();
    const auto count = store.rows(pictura::PresetType::Patterns).size();
    auto attempt = [&] {
        const int before = f.view->history_index();
        bool nameOffered = false;
        QTimer timer;
        timer.setSingleShot(true);
        QObject::connect(&timer, &QTimer::timeout, [&] {
            auto* modal = QApplication::activeModalWidget();
            if (auto* input = qobject_cast<QInputDialog*>(modal)) {
                nameOffered = true;
                input->reject();
            } else if (auto* warning = qobject_cast<QMessageBox*>(modal)) {
                warning->accept();
            }
        });
        timer.start(0);
        window->registry()->dispatch(QStringLiteral("edit.define.pattern"));
        timer.stop();
        QCOMPARE(f.view->history_index(), before);
        QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::green).rgba());
        QCOMPARE(store.rows(pictura::PresetType::Patterns).size(), count);
        QVERIFY(!nameOffered);
    };
    attempt();
    QVERIFY(f.view->select_rect(2, 2, 10, 10, QStringLiteral("new"), 2.0));
    attempt();
    QVERIFY(f.view->select_ellipse(2, 2, 10, 10, QStringLiteral("new"), 0.0));
    attempt();
    QVERIFY(f.view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0));
    const int before = f.view->history_index();
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
        if (!input) return;
        opened = true;
        input->reject();
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    QVERIFY(opened);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::green).rgba());
    QCOMPARE(store.rows(pictura::PresetType::Patterns).size(), count);
}

void PatternAuthoringTest::defineFailure()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::green);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("define_pattern_failure"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0));
    auto& store = pictura::PresetStore::current();
    const auto count = store.rows(pictura::PresetType::Patterns).size();
    const int before = f.view->history_index();
    QLockFile lock(QFileInfo(pictura::sessionFilePath()).absolutePath()
                   + QStringLiteral("/preset-current/active.lock"));
    QVERIFY(lock.tryLock());
    bool opened = false;
    bool warned = false;
    QTimer::singleShot(0, [&] {
        auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
        if (!input) return;
        opened = true;
        input->setTextValue(QStringLiteral("Cannot save"));
        QTimer::singleShot(0, [&] {
            if (auto* warning = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                warned = true;
                warning->accept();
            }
        });
        input->accept();
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    QVERIFY(opened);
    QVERIFY(warned);
    QCOMPARE(store.rows(pictura::PresetType::Patterns).size(), count);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::green).rgba());
    lock.unlock();

    for (bool changeDocument : {false, true}) {
        QVERIFY(f.view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0));
        int afterExternalChange = -1;
        bool changed = false;
        bool staleWarning = false;
        QTimer::singleShot(0, [&] {
            auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
            if (!input) return;
            if (changeDocument) {
                changed = window->newDocument(QStringLiteral("Other"), 8, 8,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
            } else {
                changed = f.view->select_rect(6, 6, 3, 3, QStringLiteral("new"), 0.0);
            }
            afterExternalChange = f.view->history_index();
            input->setTextValue(QStringLiteral("Stale"));
            QTimer::singleShot(0, [&] {
                if (auto* warning = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                    staleWarning = true;
                    warning->accept();
                }
            });
            input->accept();
        });
        QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
        QVERIFY(changed);
        QVERIFY(staleWarning);
        QCOMPARE(store.rows(pictura::PresetType::Patterns).size(), count);
        QCOMPARE(f.view->history_index(), afterExternalChange);
        QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::green).rgba());
    }
}


namespace {
QByteArray patternWord(quint16 v) { return QByteArray(1, char(v >> 8)) + char(v); }
QByteArray patternNumber(quint32 v) { return patternWord(quint16(v >> 16)) + patternWord(quint16(v)); }
QByteArray patternBlob(const QByteArray& v) { return patternNumber(v.size()) + v; }

pictura::PresetImport bluePattern(bool asymmetric = false)
{
    const auto pixels = asymmetric ? QByteArray::fromHex("ff0000ff00ff00ff0000ffffffff00ff")
                                   : QByteArray::fromHex("2255ccff2255ccff2255ccff2255ccff");
    const auto payload = patternNumber(2) + patternNumber(2) + patternBlob("blue-source")
        + patternBlob(pixels);
    const auto entry = patternBlob("Imported blue") + patternNumber(0) + char(4) + payload;
    return pictura::readPresetLibrary(pictura::PresetType::Patterns, QStringLiteral("kpset"),
        QByteArray("KPPRESET\0\1", 10) + char(pictura::PresetType::Patterns)
            + patternNumber(1) + patternBlob(entry));
}

bool chooseFill(pictura::PicturaMainWindow& window, const QString& name, int scriptId)
{
    bool configured = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::FillDialog*>(QApplication::activeModalWidget());
        if (!dialog) return;
        auto* contents = dialog->findChild<QComboBox*>(QStringLiteral("fillContents"));
        auto* scripted = dialog->findChild<QCheckBox*>(QStringLiteral("fillScriptedPatterns"));
        auto* script = dialog->findChild<QComboBox*>(QStringLiteral("fillScript"));
        if (contents && scripted && script) {
            const int pattern = contents->findText(QStringLiteral("Pattern"));
            if (pattern >= 0) {
                contents->setCurrentIndex(pattern);
                for (auto* button : dialog->findChildren<QToolButton*>()) {
                    if (button->toolTip() != name) continue;
                    button->click();
                    scripted->setChecked(scriptId != 0);
                    if (scriptId == 0 || script->findData(scriptId) >= 0) {
                        if (scriptId != 0) script->setCurrentIndex(script->findData(scriptId));
                        configured = true;
                    }
                    break;
                }
            }
        }
        if (configured) dialog->accept(); else dialog->reject();
    });
    const bool dispatched = window.registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFill));
    return dispatched && configured;
}
} // namespace

void PatternAuthoringTest::fillPreset()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::white);
    seed.setPixelColor(0, 0, Qt::red);
    seed.setPixelColor(1, 0, Qt::green);
    seed.setPixelColor(0, 1, Qt::blue);
    seed.setPixelColor(1, 1, Qt::yellow);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("fill_current_patterns"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(0, 0, 2, 2, QStringLiteral("new"), 0.0));
    QTimer::singleShot(0, [] {
        if (auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget())) {
            input->setTextValue(QStringLiteral("Four colors"));
            input->accept();
        }
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    auto& store = pictura::PresetStore::current();
    const auto imported = bluePattern();
    QVERIFY(imported.ok);
    QVERIFY(store.commitImport(pictura::PresetType::Patterns, imported, false, true));
    QVERIFY(f.view->select_rect(4, 4, 6, 6, QStringLiteral("new"), 0.0));
    int before = f.view->history_index();
    QVERIFY(chooseFill(*window, QStringLiteral("Four colors"), 0));
    QVERIFY(f.committedOnce(before, "Fill"));
    QCOMPARE(f.view->sample_argb(4, 4), QColor(Qt::red).rgba());
    QCOMPARE(f.view->sample_argb(5, 4), QColor(Qt::green).rgba());
    QCOMPARE(f.view->sample_argb(6, 4), QColor(Qt::red).rgba());
    QCOMPARE(f.view->sample_argb(10, 4), QColor(Qt::white).rgba());
    QVERIFY(f.view->undo());
    QCOMPARE(f.view->sample_argb(4, 4), QColor(Qt::white).rgba());
    before = f.view->history_index();
    QVERIFY(chooseFill(*window, QStringLiteral("Imported blue"), 0));
    QVERIFY(f.committedOnce(before, "Fill"));
    QCOMPARE(f.view->sample_argb(4, 4), QColor(0x22, 0x55, 0xcc).rgba());
    QVERIFY(store.replace(pictura::PresetType::Patterns, {}));
    QCOMPARE(f.view->sample_argb(4, 4), QColor(0x22, 0x55, 0xcc).rgba());
    QVERIFY(f.view->undo());
    QCOMPARE(f.view->sample_argb(4, 4), QColor(Qt::white).rgba());
}

void PatternAuthoringTest::fillReplay()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("replay_pattern_fill"));
    QVERIFY(f.ok());
    auto& store = pictura::PresetStore::current();
    const auto imported = bluePattern(true);
    QVERIFY(imported.ok);
    QVERIFY(store.commitImport(pictura::PresetType::Patterns, imported, true, true));
    QList<pictura::CommandParams> recordings;
    QList<QList<QRgb>> images;
    const auto snapshot = [&] {
        QList<QRgb> pixels;
        for (int y = 0; y < 16; ++y) {
            for (int x = 0; x < 16; ++x) pixels.push_back(f.view->sample_argb(x, y));
        }
        return pixels;
    };
    window->registry()->setRecordingSink([&](const QString& id, const pictura::CommandParams& params) {
        if (id == QString::fromLatin1(pictura::command_ids::EditFill)) recordings.push_back(params);
    });
    for (int script : {0, 1}) {
        QVERIFY(chooseFill(*window, QStringLiteral("Imported blue"), script));
        QCOMPARE(recordings.size(), script == 0 ? 1 : 2);
        QVERIFY(!recordings.last().values.value(QStringLiteral("patternSnapshot")).toString().isEmpty());
        QCOMPARE(recordings.last().values.value(QStringLiteral("patternScript")).toInt(), script);
        images.push_back(snapshot());
        QVERIFY(f.view->undo());
    }
    QVERIFY(images.at(0) != images.at(1));
    window->registry()->setRecordingSink({});
    QVERIFY(store.replace(pictura::PresetType::Patterns, {}));
    for (qsizetype i = 0; i < recordings.size(); ++i) {
        QVERIFY(window->registry()->executeRecorded(QString::fromLatin1(pictura::command_ids::EditFill), recordings.at(i)));
        QCOMPARE(snapshot(), images.at(i));
        QVERIFY(f.view->undo());
    }
    pictura::CommandParams legacy;
    legacy.values.insert(QStringLiteral("contents"), QStringLiteral("pattern"));
    legacy.values.insert(QStringLiteral("pattern"), 0);
    legacy.values.insert(QStringLiteral("mode"), QStringLiteral("normal"));
    legacy.values.insert(QStringLiteral("opacity"), 100);
    legacy.values.insert(QStringLiteral("preserveTransparency"), false);
    QVERIFY(window->registry()->executeRecorded(QString::fromLatin1(pictura::command_ids::EditFill), legacy));
    QCOMPARE(f.view->sample_argb(0, 0), QColor(215, 215, 215).rgba());
}

void PatternAuthoringTest::emptyList()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto& store = pictura::PresetStore::current();
    QVERIFY(store.replace(pictura::PresetType::Patterns, {}));
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::green);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("empty_pattern_fill"));
    QVERIFY(f.ok());
    const int before = f.view->history_index();
    bool inspected = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::FillDialog*>(QApplication::activeModalWidget());
        if (!dialog) return;
        auto* contents = dialog->findChild<QComboBox*>(QStringLiteral("fillContents"));
        if (contents) {
            const int pattern = contents->findText(QStringLiteral("Pattern"));
            auto* model = qobject_cast<QStandardItemModel*>(contents->model());
            inspected = pattern >= 0 && model && model->item(pattern)
                && !model->item(pattern)->isEnabled();
        }
        dialog->reject();
    });
    QVERIFY(window->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFill)));
    QVERIFY(inspected);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(0, 0), QColor(Qt::green).rgba());
}



void PatternAuthoringTest::scriptBounds()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("scripted_pattern_budget"));
    QVERIFY(f.ok());
    auto& store = pictura::PresetStore::current();
    const auto imported = bluePattern(true);
    QVERIFY(imported.ok);
    QVERIFY(store.commitImport(pictura::PresetType::Patterns, imported, true, true));
    const int before = f.view->history_index();
    QVERIFY(chooseFill(*window, QStringLiteral("Imported blue"), 1));
    QVERIFY(f.committedOnce(before, "Fill"));
    const QRgb first = f.view->sample_argb(4, 4);
    QVERIFY(f.view->undo());
    QVERIFY(chooseFill(*window, QStringLiteral("Imported blue"), 1));
    QCOMPARE(f.view->sample_argb(4, 4), first);
    QVERIFY(f.view->undo());
    struct Restore {
        ~Restore() { pictura::set_available_memory_for_test(-1); }
    } restore;
    pictura::set_available_memory_for_test(64);
    const int refused = f.view->history_index();
    QVERIFY(chooseFill(*window, QStringLiteral("Imported blue"), 1));
    QVERIFY(pictura::edit_refused_for_memory());
    QCOMPARE(f.view->history_index(), refused);
    QCOMPARE(f.view->sample_argb(4, 4), QColor(Qt::white).rgba());
}

void PatternAuthoringTest::scriptEdit()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_ARGB32);
    seed.fill(Qt::transparent);
    for (int y = 0; y < 16; ++y) {
        for (int x = 0; x < 8; ++x) seed.setPixelColor(x, y, Qt::black);
    }
    paint_fixture::Fixture f(*window, seed, QStringLiteral("scripted_pattern_edit"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(2, 2, 10, 10, QStringLiteral("new"), 0.0));
    const int before = f.view->history_index();
    bool configured = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::FillDialog*>(QApplication::activeModalWidget());
        if (!dialog) return;
        auto* contents = dialog->findChild<QComboBox*>(QStringLiteral("fillContents"));
        auto* scripted = dialog->findChild<QCheckBox*>(QStringLiteral("fillScriptedPatterns"));
        auto* script = dialog->findChild<QComboBox*>(QStringLiteral("fillScript"));
        auto* opacity = dialog->findChild<QSpinBox*>(QStringLiteral("fillOpacity"));
        auto* preserve = dialog->findChild<QCheckBox*>(QStringLiteral("fillPreserveTransparency"));
        if (contents && scripted && script && opacity && preserve) {
            const int pattern = contents->findText(QStringLiteral("Pattern"));
            const int brick = script->findData(1);
            if (pattern >= 0 && brick >= 0) {
                contents->setCurrentIndex(pattern);
                scripted->setChecked(true);
                script->setCurrentIndex(brick);
                opacity->setValue(50);
                preserve->setChecked(true);
                configured = true;
            }
        }
        if (configured) dialog->accept(); else dialog->reject();
    });
    QVERIFY(window->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFill)));
    QVERIFY(configured);
    QVERIFY(f.committedOnce(before, "Fill"));
    const QRgb inside = f.view->sample_argb(3, 3);
    QVERIFY(qRed(inside) > 0 && qRed(inside) < 255);
    QCOMPARE(f.view->sample_argb(0, 0), QColor(Qt::black).rgba());
    QCOMPARE(qAlpha(f.view->sample_argb(9, 3)), 0);
    QVERIFY(f.view->undo());
    QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::black).rgba());
    QCOMPARE(qAlpha(f.view->sample_argb(9, 3)), 0);
}

void PatternAuthoringTest::scriptCancel()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::green);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("scripted_pattern_cancel"));
    QVERIFY(f.ok());
    const int before = f.view->history_index();
    bool configured = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::FillDialog*>(QApplication::activeModalWidget());
        if (!dialog) return;
        auto* contents = dialog->findChild<QComboBox*>(QStringLiteral("fillContents"));
        auto* scripted = dialog->findChild<QCheckBox*>(QStringLiteral("fillScriptedPatterns"));
        auto* script = dialog->findChild<QComboBox*>(QStringLiteral("fillScript"));
        if (contents && scripted && script) {
            const int pattern = contents->findText(QStringLiteral("Pattern"));
            const int spiral = script->findData(4);
            if (pattern >= 0 && spiral >= 0) {
                contents->setCurrentIndex(pattern);
                scripted->setChecked(true);
                script->setCurrentIndex(spiral);
                configured = true;
            }
        }
        dialog->reject();
    });
    QVERIFY(window->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFill)));
    QVERIFY(configured);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::green).rgba());
}

void PatternAuthoringTest::shapePatternAtomicity()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(32, 32, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("shape_pattern_atomicity"));
    QVERIFY(f.ok());
    const auto imported = bluePattern(true);
    QVERIFY(imported.ok);
    const auto row = imported.rows.first();
    const QString id = row.values.value(QStringLiteral("patternId")).toString();
    const QByteArray payload = (id + QStringLiteral("|100|1|0|0|") + QString::fromLatin1(row.native.toHex())).toUtf8();
    const QByteArray invalid = (id + QStringLiteral("|100|1|0|0|ffff")).toUtf8();
    const ::rust::Str empty("", 0);
    const auto asStr = [](const QByteArray& b) { return ::rust::Str(b.constData(), size_t(b.size())); };
    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 4; spec.y0 = 4; spec.x1 = 24; spec.y1 = 24;
    const auto shape = pictura::shape_add_layer(*f.view, spec, 0xff000000u);
    QVERIFY(!shape.isEmpty());
    f.view->set_active_layer(shape);
    const auto initialIds = pictura::document_pattern_ids(*f.view);
    const auto initialStyle = pictura::shape_active_style(*f.view);
    const int before = f.view->history_index();
    QVERIFY(!pictura::shape_set_active_fill_style(*f.view, 3, 0, empty, asStr(invalid)));
    QVERIFY(!pictura::shape_set_active_stroke_style(*f.view, 3, 0, empty, asStr(invalid), 3, 0, 0));
    QCOMPARE(pictura::document_pattern_ids(*f.view), initialIds);
    QCOMPARE(pictura::shape_active_style(*f.view), initialStyle);
    QCOMPARE(f.view->history_index(), before);
    QVERIFY(pictura::shape_set_active_fill_style(*f.view, 3, 0, empty, asStr(payload)));
    QVERIFY(pictura::document_pattern_ids(*f.view).contains(id));
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::document_pattern_ids(*f.view), initialIds);
    QVERIFY(pictura::shape_set_active_stroke_style(*f.view, 3, 0, empty, asStr(payload), 3, 0, 0));
    QVERIFY(pictura::document_pattern_ids(*f.view).contains(id));
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::document_pattern_ids(*f.view), initialIds);
    spec.fill_kind = 3;
    spec.stroke = true;
    spec.stroke_kind = 3;
    for (const char c : payload) spec.fill_pattern.push_back(std::uint8_t(c));
    for (const char c : invalid) spec.stroke_pattern.push_back(std::uint8_t(c));
    const int rollback = f.view->history_index();
    QVERIFY(pictura::shape_add_layer(*f.view, spec, 0xff000000u).isEmpty());
    QCOMPARE(pictura::document_pattern_ids(*f.view), initialIds);
    QCOMPARE(pictura::shape_active_style(*f.view), initialStyle);
    QCOMPARE(f.view->history_index(), rollback);
}

void PatternAuthoringTest::captureMemoryAndStaleEdit()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("pattern_capture_budget"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0));
    const auto count = pictura::PresetStore::current().rows(pictura::PresetType::Patterns).size();
    struct RestoreMemory { ~RestoreMemory() { pictura::set_available_memory_for_test(-1); } } restoreMemory;
    pictura::set_available_memory_for_test(0);
    QVERIFY(pictura::selection_pattern_valid(*f.view));
    QVERIFY(pictura::selection_pattern_preset(*f.view, QStringLiteral("No memory")).empty());
    QVERIFY(pictura::edit_refused_for_memory());
    pictura::set_available_memory_for_test(-1);
    bool edited = false;
    bool warned = false;
    QTimer::singleShot(0, [&] {
        auto* input = qobject_cast<QInputDialog*>(QApplication::activeModalWidget());
        if (!input) return;
        edited = pictura::edit_fill(*f.view, 0xffff0000u, -1, QStringLiteral("normal"), 100, false);
        QTimer::singleShot(0, [&] {
            if (auto* warning = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                warned = true;
                warning->accept();
            }
        });
        input->accept();
    });
    QVERIFY(window->registry()->dispatch(QStringLiteral("edit.define.pattern")));
    QVERIFY(edited);
    QVERIFY(warned);
    QCOMPARE(pictura::PresetStore::current().rows(pictura::PresetType::Patterns).size(), count);
    QCOMPARE(f.view->sample_argb(3, 3), QColor(Qt::red).rgba());
}

void PatternAuthoringTest::builtinSnapshot()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window, seed, QStringLiteral("builtin_snapshot"));
    QVERIFY(f.ok());
    const auto& rows = pictura::PresetStore::current().rows(pictura::PresetType::Patterns);
    QVERIFY(!rows.isEmpty());
    pictura::CommandParams recorded;
    window->registry()->setRecordingSink([&](const QString& id, const pictura::CommandParams& params) {
        if (id == QString::fromLatin1(pictura::command_ids::EditFill)) recorded = params;
    });
    QVERIFY(chooseFill(*window, rows.first().name, 0));
    window->registry()->setRecordingSink({});
    const auto bytes = QByteArray::fromBase64(recorded.values.value(QStringLiteral("patternSnapshot")).toString().toLatin1());
    const auto decoded = pictura::readPresetLibrary(pictura::PresetType::Patterns, QStringLiteral("kpset"), bytes);
    QVERIFY(decoded.ok);
    QCOMPARE(decoded.rows.size(), 1);
    QVERIFY(!decoded.rows.first().values.value(QStringLiteral("patternId")).toString().isEmpty());
    QVERIFY(!decoded.rows.first().values.contains(QStringLiteral("index")));
    const QRgb sample = f.view->sample_argb(3, 3);
    QVERIFY(f.view->undo());
    QVERIFY(pictura::PresetStore::current().replace(pictura::PresetType::Patterns, {}));
    QVERIFY(window->registry()->executeRecorded(QString::fromLatin1(pictura::command_ids::EditFill), recorded));
    QCOMPARE(f.view->sample_argb(3, 3), sample);
}

PICTURA_QT_TEST_MAIN(PatternAuthoringTest)
#include "tst_pattern_authoring.moc"
