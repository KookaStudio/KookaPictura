#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QJsonDocument>
#include <QtCore/QTimer>
#include <QtCore/QtEndian>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMessageBox>

#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "control_server.h"
#include "control_test_client.h"
#include "import_save_coordinator.h"
#include "preferences_dialog.h"
#include "qt_test_support.h"
#include "session.h"

namespace {

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool hasMergedImage(const QString& path)
{
    const QByteArray bytes = fileBytes(path);
    if (bytes.size() < 34 || !bytes.startsWith("8BPS")) {
        return false;
    }
    const auto length = [&bytes](qsizetype at) { return qFromBigEndian<quint32>(bytes.constData() + at); };
    const qsizetype start = 30 + qsizetype(length(26));
    if (start + 4 > bytes.size()) {
        return false;
    }
    const qsizetype end = start + 4 + qsizetype(length(start));
    if (end > bytes.size()) {
        return false;
    }
    qsizetype at = start + 4;
    while (at + 12 <= end && bytes.mid(at, 4) == "8BIM") {
        const auto id = qFromBigEndian<quint16>(bytes.constData() + at + 4);
        const qsizetype nameBytes = (quint8(bytes[at + 6]) + 2) & ~1;
        const qsizetype sizeAt = at + 6 + nameBytes;
        if (sizeAt + 4 > end) {
            return false;
        }
        const qsizetype size = length(sizeAt);
        const qsizetype data = sizeAt + 4;
        if (size > end - data) {
            return false;
        }
        if (id == 1057 && size >= 5) {
            return bytes[data + 4] != 0;
        }
        at = data + size + (size & 1);
    }
    return true;
}

class CompatibilityPrompt {
public:
    explicit CompatibilityPrompt(QMessageBox::ButtonRole role)
    {
        QObject::connect(&timer_, &QTimer::timeout, &timer_, [this, role] {
            for (QWidget* widget : QApplication::topLevelWidgets()) {
                auto* box = qobject_cast<QMessageBox*>(widget);
                if (!box || !box->isVisible()
                    || box->windowTitle() != QStringLiteral("Maximize PSD and PSB File Compatibility")) {
                    continue;
                }
                for (auto* button : box->buttons()) {
                    if (box->buttonRole(button) == role) {
                        ++count;
                        button->click();
                        return;
                    }
                }
            }
        });
        timer_.start(1);
    }

    int count = 0;

private:
    QTimer timer_;
};

QComboBox* compatibilityCombo(pictura::PicturaMainWindow& window)
{
    window.showPreferences(pictura::PreferencesDialog::kFileHandling);
    return window.preferencesDialog()->findChild<QComboBox*>(
        QStringLiteral("preferencesNativeSaveCompatibility"));
}

bool newDocument(pictura::PicturaMainWindow& window)
{
    return window.newDocument(QStringLiteral("Compatibility"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
}

} // namespace

class PsdCompatibilityTest : public QObject {
    Q_OBJECT

private slots:
    void preferencePersists();
    void asksPerSaveAndCancels();
    void copyPreservesDocument();
    void otherSavesBypassPreference();
    void nativeModesDoNotPrompt();
};

void PsdCompatibilityTest::preferencePersists()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    for (int mode = 0; mode < 3; ++mode) {
        auto window = pictura::test::makeMainWindow();
        auto* combo = compatibilityCombo(*window);
        QVERIFY(combo);
        QCOMPARE(combo->currentIndex(), mode);
        if (mode < 2) {
            combo->setCurrentIndex(mode + 1);
        }
    }
    QFile session(pictura::sessionFilePath());
    QVERIFY(session.open(QIODevice::ReadOnly));
    QJsonObject invalid = QJsonDocument::fromJson(session.readAll()).object();
    session.close();
    invalid.insert(QStringLiteral("nativeSaveCompatibility"), 42);
    QVERIFY(session.open(QIODevice::WriteOnly | QIODevice::Truncate));
    QVERIFY(session.write(QJsonDocument(invalid).toJson()) > 0);
    session.close();
    auto window = pictura::test::makeMainWindow();
    auto* combo = compatibilityCombo(*window);
    QVERIFY(combo);
    QCOMPARE(combo->currentIndex(), 0);
}

void PsdCompatibilityTest::asksPerSaveAndCancels()
{
    pictura::test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(dir.isValid());
    QVERIFY(newDocument(*window));
    auto* view = window->activeView();
    auto* coordinator = window->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    coordinator->setNativeSaveCompatibility(1);
    const QString path = dir.filePath(QStringLiteral("working.psd"));
    window->setProfileDialogForTest([](bool, bool& embed) { embed = false; return true; });
    {
        CompatibilityPrompt prompt(QMessageBox::AcceptRole);
        QVERIFY(window->saveActiveAsWithOptions(path));
        QCOMPARE(prompt.count, 1);
    }
    QVERIFY(hasMergedImage(path));
    {
        CompatibilityPrompt prompt(QMessageBox::DestructiveRole);
        QVERIFY(window->saveActive());
        QCOMPARE(prompt.count, 1);
    }
    QVERIFY(!hasMergedImage(path));
    const QByteArray oldFile = fileBytes(path);
    QVERIFY(!oldFile.isEmpty());
    const int history = view->history_count();
    const bool dirty = view->is_dirty();
    const int embed = pictura::save_embed_profile(*view);
    window->setProfileDialogForTest([](bool, bool& choice) { choice = true; return true; });
    {
        CompatibilityPrompt prompt(QMessageBox::RejectRole);
        QVERIFY(!window->saveActiveAsWithOptions(path));
        QVERIFY(!coordinator->saveCopyAsWithOptions(path));
        QCOMPARE(prompt.count, 2);
    }
    QCOMPARE(pictura::save_embed_profile(*view), embed);
    QCOMPARE(fileBytes(path), oldFile);
    QCOMPARE(window->documentPath(0), path);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->is_dirty(), dirty);

    QVERIFY(pictura::channels_new_alpha(*view) >= 0);
    {
        CompatibilityPrompt prompt(QMessageBox::RejectRole);
        QVERIFY(window->saveActive());
        QCOMPARE(prompt.count, 0);
    }
    QVERIFY(hasMergedImage(path));
}

void PsdCompatibilityTest::copyPreservesDocument()
{
    pictura::test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(dir.isValid());
    QVERIFY(newDocument(*window));
    const QString original = dir.filePath(QStringLiteral("original.psb"));
    QVERIFY(window->saveActiveAs(original));
    auto* view = window->activeView();
    QVERIFY(view->add_layer(0) >= 0);
    QVERIFY(view->is_dirty());
    const int history = view->history_count();
    const QImage pixels = view->image();
    const QString copy = dir.filePath(QStringLiteral("copy.psb"));
    auto* coordinator = window->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    coordinator->setNativeSaveCompatibility(1);
    coordinator->setProfileDialogForTest([](bool, bool&) { return true; });
    CompatibilityPrompt prompt(QMessageBox::DestructiveRole);
    QVERIFY(coordinator->saveCopyAsWithOptions(copy));
    QCOMPARE(prompt.count, 1);
    QVERIFY(!fileBytes(copy).isEmpty());
    QVERIFY(!hasMergedImage(copy));
    QCOMPARE(view->image(), pixels);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(window->documentPath(0), original);
    QVERIFY(view->is_dirty());
}

void PsdCompatibilityTest::otherSavesBypassPreference()
{
    pictura::test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(dir.isValid());
    window->enableRecovery();
    QVERIFY(newDocument(*window));
    auto* coordinator = window->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    const QString original = dir.filePath(QStringLiteral("original.psd"));
    QVERIFY(window->saveActiveAs(original));
    coordinator->setNativeSaveCompatibility(1);
    CompatibilityPrompt prompt(QMessageBox::RejectRole);
    pictura::ControlServer server(window.get(), dir.filePath(QStringLiteral("control.sock")));
    for (const auto& args : {
             QJsonObject{{QStringLiteral("op"), QStringLiteral("save")}},
             QJsonObject{{QStringLiteral("op"), QStringLiteral("save")},
                         {QStringLiteral("path"), dir.filePath(QStringLiteral("explicit.psd"))}},
             QJsonObject{{QStringLiteral("op"), QStringLiteral("save_as")},
                         {QStringLiteral("path"), dir.filePath(QStringLiteral("save-as.psd"))}}}) {
        QVERIFY(pictura::test::succeeded(server.dispatch(QStringLiteral("document"), args)));
        QVERIFY(hasMergedImage(args.value(QStringLiteral("path")).toString(original)));
    }
    const QString replay = dir.filePath(QStringLiteral("replay.psd"));
    QVERIFY(window->registry()->executeRecorded(
        QString::fromLatin1(pictura::command_ids::FileSaveAs),
        pictura::CommandParams{{{QStringLiteral("in"), replay}}}));
    QVERIFY(hasMergedImage(replay));
    const QString png = dir.filePath(QStringLiteral("raster.png"));
    coordinator->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    QVERIFY(window->saveActiveAsWithOptions(png));
    QCOMPARE(QImage(png).pixelColor(0, 0), QColor(Qt::white));

    auto* origin = window->activeView();
    const QString object = origin->add_solid_fill(0xff2244aau);
    QVERIFY(origin->rasterize_fill_content(object));
    QVERIFY(origin->convert_to_smart_object(object));
    window->autosaveRecovery();
    const QString recovery = window->recoverySessionDirForTest();
    QVERIFY(!recovery.isEmpty());
    QTRY_VERIFY(!QDir(recovery).entryList({QStringLiteral("*.psd")}, QDir::Files).isEmpty());
    const auto snapshots = QDir(recovery).entryList({QStringLiteral("*.psd")}, QDir::Files);
    QVERIFY(hasMergedImage(QDir(recovery).filePath(snapshots.first())));

    QVERIFY(window->editSmartObjectContents(object));
    auto* editor = window->activeView();
    QVERIFY(editor && editor != origin);
    const QString fill = editor->add_solid_fill(0xff22cc44u);
    QVERIFY(editor->rasterize_fill_content(fill));
    QVERIFY(window->saveActive());
    QCOMPARE(origin->sample_argb(0, 0), 0xff22cc44u);
    QCOMPARE(prompt.count, 0);
}

void PsdCompatibilityTest::nativeModesDoNotPrompt()
{
    pictura::test::ScopedStateHome stateHome;
    QTemporaryDir dir;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(dir.isValid());
    QVERIFY(newDocument(*window));
    auto* coordinator = window->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    coordinator->setProfileDialogForTest([](bool, bool&) { return true; });
    CompatibilityPrompt prompt(QMessageBox::RejectRole);
    for (const auto& suffix : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        for (const int mode : {0, 2}) {
            coordinator->setNativeSaveCompatibility(mode);
            const QString path = dir.filePath(QStringLiteral("choice-%1.%2").arg(mode).arg(suffix));
            QVERIFY(window->saveActiveAsWithOptions(path));
            QVERIFY(!fileBytes(path).isEmpty());
            QCOMPARE(hasMergedImage(path), mode == 0);
        }
    }
    QCOMPARE(prompt.count, 0);
}

QTEST_MAIN(PsdCompatibilityTest)
#include "tst_psd_compatibility.moc"
