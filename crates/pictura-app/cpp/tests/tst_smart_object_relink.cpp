#include <QtTest/QtTest>

#include <QtCore/QProcess>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QMessageBox>

#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/transform_api.h"
#include "api/transform_bridge.h"
#include "frame.h"
#include "multi_file_test_support.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"
#include "transform_commit.h"
#include "vector_pdf_fixture.h"

using namespace pictura;

namespace {

QString png(const QTemporaryDir& dir, QSize size = QSize(4, 4))
{
    QImage image(size, QImage::Format_ARGB32);
    image.fill(Qt::blue);
    const QString path = dir.filePath(QStringLiteral("replacement.png"));
    return image.save(path) ? path : QString();
}

std::unique_ptr<PicturaMainWindow> linkedWindow(const QString& path, bool vector = false,
                                                bool filter = false)
{
    auto window = test::makeMainWindow();
    if (!window || !relink_fixture_for_test(path, vector, filter) || !window->openPath(path)) {
        return {};
    }
    window->show();
    QCoreApplication::processEvents();
    return window;
}

LayersPanel* panel(PicturaMainWindow& window)
{
    return window.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
}

const QString sharedPath = QStringLiteral("0");
const QString secondPath = QStringLiteral("1");
const QString independentPath = QStringLiteral("2");

struct RelinkDriver {
    bool picker = false;
    bool warning = false;
    OpenDialogProbe probe;

    bool run(PicturaMainWindow& window, const QString& path, const QString& replacement,
             const std::function<void()>& duringPicker = {})
    {
        QTimer timer;
        QObject::connect(&timer, &QTimer::timeout, &timer, [&]() {
            auto* modal = QApplication::activeModalWidget();
            if (auto* dialog = qobject_cast<QFileDialog*>(modal); dialog && !picker) {
                picker = true;
                if (duringPicker) {
                    duringPicker();
                }
                if (replacement.isEmpty()) {
                    dialog->reject();
                } else {
                    chooseFiles(dialog, {replacement}, &probe);
                }
            } else if (auto* error = qobject_cast<QMessageBox*>(modal)) {
                warning = true;
                error->accept();
            }
        });
        timer.start(10);
        return panel(window)->performRowActionForTest(QStringLiteral("relinkToFile"), path);
    }
};

QString pathFor(PictureView& view, const QString& path)
{
    return relink_source_location(view, relink_source_id(view, path));
}

void checkIndependentReader(const QString& file)
{
    if (QProcess::execute(QStringLiteral("python3"),
                          {QStringLiteral("-c"), QStringLiteral("import psd_tools")}) != 0) {
        return;
    }
    const QString script = QStringLiteral(
        "from psd_tools import PSDImage\n"
        "import sys\n"
        "image = PSDImage.open(sys.argv[1])\n"
        "links = [layer.smart_object for layer in image]\n"
        "assert all(link.kind == 'external' for link in links)\n"
        "assert sum(link.filename == 'replacement.png' for link in links) == 2\n"
        "assert len({link.unique_id for link in links}) == 2\n");
    QCOMPARE(QProcess::execute(QStringLiteral("python3"),
                               {QStringLiteral("-c"), script, file}), 0);
}

} // namespace

class SmartObjectRelinkTest : public QObject {
    Q_OBJECT

private slots:
    void opensWithStoredPreview();
    void repairsSharedSource();
    void refusesInvalidOrStaleRepair();
    void refusesIncompatibleRepair();
    void repairedLinkSurvivesReopen();
    void repairPreservesLocalTransform();
    void repairPreservesSmartFilters();
    void rowActionTargetsClickedSource();

private:
    test::ScopedStateHome stateHome_;
};

void SmartObjectRelinkTest::opensWithStoredPreview()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psd")));
    QVERIFY(window);
    PictureView& view = *window->activeView();
    QCOMPARE(QColor::fromRgba(view.sample_argb(1, 1)), QColor(Qt::red));
    QCOMPARE(pathFor(view, sharedPath), QStringLiteral("file:///missing/missing.png"));
    QVERIFY(panel(*window)->rowMenuEnabledForPathForTest(sharedPath,
                                                        QStringLiteral("Relink to File…")));
}

void SmartObjectRelinkTest::repairsSharedSource()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psd")));
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const QString replacement = png(dir);
    QVERIFY(!replacement.isEmpty());
    const QString before = pathFor(view, sharedPath);
    const auto original = view.image();
    const int history = view.history_count();
    RelinkDriver driver;
    QVERIFY(driver.run(*window, sharedPath, replacement));
    QVERIFY(driver.picker && !driver.warning);
    QCOMPARE(view.history_count(), history + 1);
    QCOMPARE(view.history_label(history), QStringLiteral("Relink to File"));
    QVERIFY(pathFor(view, sharedPath).endsWith(replacement));
    QCOMPARE(pathFor(view, secondPath), pathFor(view, sharedPath));
    QCOMPARE(pathFor(view, independentPath), before);
    QCOMPARE(QColor::fromRgba(view.sample_argb(1, 1)), QColor(Qt::blue));
    QCOMPARE(QColor::fromRgba(view.sample_argb(5, 1)), QColor(Qt::blue));
    QCOMPARE(QColor::fromRgba(view.sample_argb(9, 1)), QColor(Qt::red));
    const auto repaired = view.image();
    QVERIFY(view.undo());
    QCOMPARE(view.image(), original);
    QCOMPARE(pathFor(view, sharedPath), before);
    QVERIFY(view.redo());
    QCOMPARE(view.image(), repaired);
    QVERIFY(pathFor(view, sharedPath).endsWith(replacement));
}

void SmartObjectRelinkTest::refusesInvalidOrStaleRepair()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psd")));
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const QString before = pathFor(view, sharedPath);
    const int history = view.history_count();
    RelinkDriver cancel;
    QVERIFY(cancel.run(*window, sharedPath, {}));
    QVERIFY(cancel.picker && !cancel.warning);
    QCOMPARE(view.history_count(), history);
    const QString invalid = dir.filePath(QStringLiteral("invalid.png"));
    QVERIFY(test::writeFixture(invalid, QByteArray("unreadable image")));
    RelinkDriver bad;
    QVERIFY(bad.run(*window, sharedPath, invalid));
    QVERIFY(bad.warning);
    QCOMPARE(view.history_count(), history);
    const QString replacement = png(dir);
    QVERIFY(!replacement.isEmpty());
    RelinkDriver stale;
    QVERIFY(stale.run(*window, sharedPath, replacement, [&]() {
        view.set_layer_name_path(sharedPath, QStringLiteral("Intervening edit"));
    }));
    QVERIFY(stale.warning);
    QCOMPARE(view.history_count(), history + 1);
    QCOMPARE(pathFor(view, sharedPath), before);
    QCOMPARE(QColor::fromRgba(view.sample_argb(1, 1)), QColor(Qt::red));
}

void SmartObjectRelinkTest::refusesIncompatibleRepair()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psd")));
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const int history = view.history_count();
    const QImage image = view.image();
    const QString wrongSize = png(dir, QSize(5, 4));
    const QString wrongFormat = dir.filePath(QStringLiteral("replacement.jpg"));
    QImage jpeg(4, 4, QImage::Format_RGB32);
    jpeg.fill(Qt::blue);
    QVERIFY(jpeg.save(wrongFormat));
    for (const auto& path : {wrongSize, wrongFormat}) {
        RelinkDriver driver;
        QVERIFY(driver.run(*window, sharedPath, path));
        QVERIFY(driver.warning);
        QCOMPARE(view.history_count(), history);
        QCOMPARE(view.image(), image);
    }
    if (test::haveVectorRasterizer()) {
        auto vector = linkedWindow(dir.filePath(QStringLiteral("vector.psd")), true);
        QVERIFY(vector);
        const int vectorHistory = vector->activeView()->history_count();
        const QString pdf = dir.filePath(QStringLiteral("replacement.pdf"));
        for (const auto& bytes : {test::vectorPdf(Qt::blue, false),
                                  test::vectorPdf(Qt::blue, true, QSizeF(80, 40))}) {
            QVERIFY(test::writeFixture(pdf, bytes));
            RelinkDriver driver;
            QVERIFY(driver.run(*vector, sharedPath, pdf));
            QVERIFY(driver.warning);
            QCOMPARE(vector->activeView()->history_count(), vectorHistory);
        }
        QVERIFY(test::writeFixture(pdf, test::vectorPdf()));
        RelinkDriver compatible;
        QVERIFY(compatible.run(*vector, sharedPath, pdf));
        QVERIFY(!compatible.warning);
        QCOMPARE(vector->activeView()->history_count(), vectorHistory + 1);
        QVERIFY(pathFor(*vector->activeView(), sharedPath).endsWith(pdf));
    }
}

void SmartObjectRelinkTest::repairedLinkSurvivesReopen()
{
    QTemporaryDir dir;
    for (const auto& ext : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        auto window = linkedWindow(dir.filePath(QStringLiteral("external.") + ext));
        QVERIFY(window);
        const QString replacement = png(dir);
        QVERIFY(!replacement.isEmpty());
        RelinkDriver driver;
        QVERIFY(driver.run(*window, sharedPath, replacement));
        QVERIFY(!driver.warning);
        const QString saved = dir.filePath(QStringLiteral("repaired.") + ext);
        QVERIFY(window->saveActiveAs(saved));
        checkIndependentReader(saved);
        QVERIFY(QFile::remove(replacement));
        QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
        QVERIFY(window->openPath(saved));
        auto& view = *window->activeView();
        QCOMPARE(QColor::fromRgba(view.sample_argb(1, 1)), QColor(Qt::blue));
        QVERIFY(pathFor(view, sharedPath).endsWith(replacement));
        QCOMPARE(pathFor(view, secondPath), pathFor(view, sharedPath));
        QVERIFY(panel(*window)->rowMenuEnabledForPathForTest(sharedPath,
                                                            QStringLiteral("Relink to File…")));
    }
}

void SmartObjectRelinkTest::rowActionTargetsClickedSource()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psd")));
    QVERIFY(window);
    window->selectLayerPath(sharedPath);
    PictureView& view = *window->activeView();
    const auto original = pathFor(view, sharedPath);
    const QString replacement = png(dir);
    QVERIFY(!replacement.isEmpty());
    RelinkDriver driver;
    QVERIFY(driver.run(*window, independentPath, replacement));
    QVERIFY(driver.picker && !driver.warning);
    QCOMPARE(pathFor(view, sharedPath), original);
    QVERIFY(pathFor(view, independentPath).endsWith(replacement));
}

void SmartObjectRelinkTest::repairPreservesLocalTransform()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("external.psb")));
    QVERIFY(window);
    PictureView& view = *window->activeView();
    QVERIFY(window->beginFreeTransform(sharedPath));
    QVERIFY(transform_set_angle(view, 90.0));
    QVERIFY(runTransformCommit(window.get(), view));
    const auto placed = transform_api::layerRect(view, sharedPath);

    const QString saved = dir.filePath(QStringLiteral("transformed.psb"));
    QVERIFY(window->saveActiveAs(saved));
    QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
    QVERIFY(window->openPath(saved));
    PictureView& reopened = *window->activeView();
    QImage image(4, 4, QImage::Format_ARGB32);
    image.fill(Qt::blue);
    image.setPixelColor(0, 0, Qt::green);
    const QString replacement = dir.filePath(QStringLiteral("replacement.png"));
    QVERIFY(image.save(replacement));
    RelinkDriver driver;
    QVERIFY(driver.run(*window, sharedPath, replacement));
    QVERIFY(!driver.warning);
    QCOMPARE(transform_api::layerRect(reopened, sharedPath).toString(), placed.toString());
    QCOMPARE(QColor::fromRgba(reopened.sample_argb(placed.right - 1, placed.top)), QColor(Qt::green));
    QCOMPARE(QColor::fromRgba(reopened.sample_argb(placed.left, placed.top)), QColor(Qt::blue));
    QCOMPARE(QColor::fromRgba(reopened.sample_argb(4, 0)), QColor(Qt::green));
}

void SmartObjectRelinkTest::repairPreservesSmartFilters()
{
    QTemporaryDir dir;
    auto window = linkedWindow(dir.filePath(QStringLiteral("filtered.psd")), false, true);
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const auto replacement = png(dir);
    QVERIFY(!replacement.isEmpty());
    QCOMPARE(layer_smart_filter_count(view, sharedPath), 1);
    QVERIFY(layer_smart_filter_visible(view, sharedPath, 0));
    RelinkDriver driver;
    QVERIFY(driver.run(*window, sharedPath, replacement));
    QVERIFY(!driver.warning);
    QCOMPARE(layer_smart_filter_count(view, sharedPath), 1);
    QVERIFY(layer_smart_filter_visible(view, sharedPath, 0));
    QCOMPARE(QColor::fromRgba(view.sample_argb(0, 1)), QColor(Qt::blue));
    QCOMPARE(QColor::fromRgba(view.sample_argb(6, 1)), QColor(Qt::blue));
    const QColor filtered = QColor::fromRgba(view.sample_argb(3, 1));
    QCOMPARE(filtered.red(), filtered.blue());
    QCOMPARE(filtered.green(), filtered.blue());

    const QImage preview = view.image();
    const QString saved = dir.filePath(QStringLiteral("repaired.psd"));
    QVERIFY(window->saveActiveAs(saved));
    QVERIFY(QFile::remove(replacement));
    QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
    QVERIFY(window->openPath(saved));
    QCOMPARE(window->activeView()->image(), preview);
    QCOMPARE(layer_smart_filter_count(*window->activeView(), sharedPath), 1);
}

QTEST_MAIN(SmartObjectRelinkTest)
#include "tst_smart_object_relink.moc"
