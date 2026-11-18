#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QFileInfo>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QPushButton>

#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/transform_api.h"
#include "api/transform_bridge.h"
#include "api/view_bridge.h"
#include "frame.h"
#include "frame_place.h"
#include "image_view.h"
#include "import_save_coordinator.h"
#include "qt_test_support.h"
#include "tools.h"
#include "transform_commit.h"
#include "vector_pdf_fixture.h"

using namespace pictura;

namespace {

enum class PlaceResult { Accept, CancelPage, CancelTransform, DiscardOnToolSwitch, Enlarge };

struct PlaceDriver {
    QTimer timer;
    bool chosePage = false;
    bool transformed = false;
    bool reportedError = false;

    void run(PicturaMainWindow& window, const QString& path, PlaceResult result)
    {
        QObject::connect(&timer, &QTimer::timeout, &timer, [&]() {
            auto* dialog = window.findChild<QDialog*>(QStringLiteral("vectorPlaceOptions"));
            if (dialog && dialog->isVisible()) {
                auto* page = dialog->findChild<QComboBox*>(QStringLiteral("vectorPlacePage"));
                auto* crop = dialog->findChild<QComboBox*>(QStringLiteral("vectorPlaceCrop"));
                if (page && crop && page->count() > 1) {
                    chosePage = true;
                    page->setCurrentIndex(1);
                    crop->setCurrentIndex(1);
                }
                dialog->done(result == PlaceResult::CancelPage
                                 ? QDialog::Rejected : QDialog::Accepted);
                return;
            }
            if (auto* error = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                reportedError = true;
                error->accept();
                return;
            }
            PictureView* view = window.activeView();
            if (!view || !view->transform_session_active() || transformed) {
                return;
            }
            transformed = true;
            if (result == PlaceResult::DiscardOnToolSwitch) {
                window.setActiveTool(ToolId::Gradient);
                return;
            }
            if (result == PlaceResult::Enlarge) {
                const QPolygonF quad = transform_api::quad(*view).corners;
                if (quad.size() == 4) {
                    const QPointF corner = quad.at(2);
                    const QPointF size = corner - quad.at(0);
                    view->transform_press(corner.x(), corner.y(), 1, false, false);
                    view->transform_move(corner.x() + size.x(), corner.y() + size.y(),
                                         1, false, false);
                    view->transform_release();
                }
            }
            if (ImageView* canvas = window.imageView()) {
                QTest::keyClick(canvas, result == PlaceResult::CancelTransform
                                           ? Qt::Key_Escape : Qt::Key_Return);
            }
        });
        timer.start(10);
        placeDocuments(window, {path});
        timer.stop();
    }
};

std::unique_ptr<PicturaMainWindow> target(int width = 50, int height = 50)
{
    auto window = test::makeMainWindow();
    if (!window || !window->newDocument(QStringLiteral("Vector target"), width, height,
                                        QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return {};
    }
    window->show();
    QCoreApplication::processEvents();
    return window;
}

void acknowledgeWarning(QTimer& timer, bool& reported)
{
    QObject::connect(&timer, &QTimer::timeout, &timer, [&]() {
        if (auto* warning = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
            reported = true;
            warning->accept();
        }
    });
    timer.start(10);
}

struct EditorDriver {
    QString file;
    QPointer<QDialog> dialog;
    bool warning = false;

    bool launch(PicturaMainWindow& window, const QString& path, bool success = true)
    {
        auto* coordinator = window.findChild<ImportSaveCoordinator*>();
        if (!coordinator) {
            return false;
        }
        coordinator->setVectorEditorLauncherForTest([&](const QString& source) {
            file = source;
            return success;
        });
        QTimer timer;
        acknowledgeWarning(timer, warning);
        const bool opened = window.editSmartObjectContents(path);
        coordinator->setVectorEditorLauncherForTest({});
        dialog = window.findChild<QDialog*>(QStringLiteral("vectorEditContents"));
        return opened;
    }

    bool apply()
    {
        if (!dialog) {
            return false;
        }
        auto* button = dialog->findChild<QPushButton*>(QStringLiteral("vectorEditApply"));
        if (!button) {
            return false;
        }
        warning = false;
        QTimer timer;
        acknowledgeWarning(timer, warning);
        button->click();
        return !warning;
    }
};

QByteArray exported(PictureView& view, const QString& path, const QTemporaryDir& dir)
{
    const QString file = dir.filePath(QStringLiteral("exported.pdf"));
    return view.export_smart_object_contents(path, file) ? test::fixtureBytes(file) : QByteArray();
}

} // namespace

class VectorSmartObjectTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void placesSelectedPage();
    void cancelledOrInvalidPlace();
    void discardPlacementOnToolSwitch();
    void scaleRendersFromSource();
    void scaleAfterRotationRendersFromSource();
    void sourceRoundTrips();
    void sourceIdentityRoundTrips();
    void launchesExternalEditor();
    void appliesToSharedInstances();
    void refusesInvalidOrStaleEdits();
    void discardsExternalSession();
    void discardDuringApply();

private:
    test::ScopedStateHome stateHome_;
};

void VectorSmartObjectTest::initTestCase()
{
    if (!test::haveVectorRasterizer()) {
        QSKIP("Ghostscript (gs) is not installed");
    }
}

void VectorSmartObjectTest::placesSelectedPage()
{
    QTemporaryDir dir;
    const QByteArray bytes = test::vectorPdf();
    for (const auto& name : {QStringLiteral("source.pdf"), QStringLiteral("source.ai")}) {
        const QString source = dir.filePath(name);
        QVERIFY(test::writeFixture(source, bytes));
        auto window = target();
        QVERIFY(window);
        PictureView& view = *window->activeView();
        const int count = view.layer_count();
        PlaceDriver driver;
        driver.run(*window, source, PlaceResult::Accept);
        QVERIFY(driver.chosePage && driver.transformed && !driver.reportedError);
        QCOMPARE(view.layer_count(), count + 1);
        const auto row = layers_api::rowAt(view, 0);
        QVERIFY(row.smartObject);
        const auto rect = transform_api::layerRect(view, row.path);
        QCOMPARE(QSize(rect.width(), rect.height()), QSize(50, 25));
        QCOMPARE(QColor::fromRgba(view.sample_argb(rect.left + 5, rect.top + 5)), QColor(Qt::blue));
        QCOMPARE(QColor::fromRgba(view.sample_argb(rect.left + 32, rect.top + 5)), QColor(Qt::white));
    }
}

void VectorSmartObjectTest::cancelledOrInvalidPlace()
{
    QTemporaryDir dir;
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    QVERIFY(test::writeFixture(source, test::vectorPdf()));
    auto window = target();
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const int layers = view.layer_count();
    const int history = view.history_count();
    for (const auto mode : {PlaceResult::CancelPage, PlaceResult::CancelTransform}) {
        PlaceDriver driver;
        driver.run(*window, source, mode);
        QVERIFY(driver.chosePage);
        QCOMPARE(driver.transformed, mode == PlaceResult::CancelTransform);
        QCOMPARE(view.layer_count(), layers);
        QCOMPARE(view.history_count(), history);
        QVERIFY(!driver.reportedError);
    }
    QVERIFY(test::writeFixture(source, QByteArray("%PDF-1.7\n")));
    PlaceDriver invalid;
    invalid.run(*window, source, PlaceResult::Accept);
    QVERIFY(invalid.reportedError && !invalid.transformed);
    QCOMPARE(view.layer_count(), layers);
    QCOMPARE(view.history_count(), history);
}

void VectorSmartObjectTest::scaleRendersFromSource()
{
    QTemporaryDir dir;
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    QVERIFY(test::writeFixture(source, test::vectorPdf()));
    auto window = target();
    QVERIFY(window);
    PictureView& view = *window->activeView();
    PlaceDriver driver;
    driver.run(*window, source, PlaceResult::Accept);
    QVERIFY(driver.transformed && !driver.reportedError);
    const QString path = layers_api::rowAt(view, 0).path;
    QVERIFY(window->beginFreeTransform(path));
    const QPolygonF corners = transform_api::quad(view).corners;
    QCOMPARE(corners.size(), 4);
    const QPointF corner = corners.at(2);
    const QPointF size = corner - corners.at(0);
    QVERIFY(view.transform_press(corner.x(), corner.y(), 1, false, false) >= 0);
    view.transform_move(corner.x() + size.x(), corner.y() + size.y(), 1, false, false);
    view.transform_release();
    QVERIFY(runTransformCommit(window.get(), view));
    const auto scaled = transform_api::layerRect(view, path);
    QCOMPARE(QSize(scaled.width(), scaled.height()), QSize(100, 50));
    const QImage scaledImage = view.image();
    auto direct = target(100, 50);
    QVERIFY(direct);
    PlaceDriver directDriver;
    directDriver.run(*direct, source, PlaceResult::Accept);
    const QImage directImage = direct->activeView()->image();
    QCOMPARE(scaledImage.copy(0, scaled.top, 50, 50 - scaled.top),
             directImage.copy(0, 0, 50, 50 - scaled.top));
    QVERIFY(view.undo());
    const auto original = transform_api::layerRect(view, path);
    QCOMPARE(QSize(original.width(), original.height()), QSize(50, 25));
}

void VectorSmartObjectTest::discardPlacementOnToolSwitch()
{
    QTemporaryDir dir;
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    QVERIFY(test::writeFixture(source, test::vectorPdf()));
    auto window = target();
    QVERIFY(window);
    PictureView& view = *window->activeView();
    const int layers = view.layer_count();
    const int history = view.history_count();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    bool asked = false;
    bool rollbackBeforeToolSwitch = false;
    QObject latch;
    tools->setTransformDecisionPrompt([&]() {
        asked = true;
        return TransformDecision::DontApply;
    });
    QObject::connect(tools, &ToolController::activeToolChanged, &latch, [&](ToolId tool) {
        if (asked && tool == ToolId::Gradient) {
            rollbackBeforeToolSwitch = view.layer_count() == layers;
        }
    });
    PlaceDriver place;
    place.run(*window, source, PlaceResult::DiscardOnToolSwitch);
    QVERIFY(place.chosePage && place.transformed && !place.reportedError);
    QVERIFY(asked && rollbackBeforeToolSwitch);
    QCOMPARE(window->activeTool(), ToolId::Gradient);
    QCOMPARE(view.layer_count(), layers);
    QCOMPARE(view.history_count(), history);
}

void VectorSmartObjectTest::scaleAfterRotationRendersFromSource()
{
    QTemporaryDir dir;
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    QVERIFY(test::writeFixture(source, test::vectorPdf()));
    auto scaled = target(40, 140);
    auto direct = target(80, 140);
    QVERIFY(scaled && direct);
    for (auto* window : {scaled.get(), direct.get()}) {
        PictureView& view = *window->activeView();
        PlaceDriver place;
        place.run(*window, source, PlaceResult::Accept);
        QVERIFY(place.transformed && !place.reportedError);
        const QString path = layers_api::rowAt(view, 0).path;
        QVERIFY(window->beginFreeTransform(path));
        QVERIFY(transform_set_angle(view, 90.0));
        QVERIFY(runTransformCommit(window, view));
    }
    PictureView& view = *scaled->activeView();
    const QString path = layers_api::rowAt(view, 0).path;
    const QImage rotated = view.image();
    QVERIFY(scaled->beginFreeTransform(path));
    const QPolygonF corners = transform_api::quad(view).corners;
    QCOMPARE(corners.size(), 4);
    const QPointF corner = corners.at(2);
    const QPointF extent = corner - corners.at(0);
    QVERIFY(view.transform_press(corner.x(), corner.y(), 1, false, false) >= 0);
    view.transform_move(corner.x() + extent.x(), corner.y() + extent.y(), 1, false, false);
    view.transform_release();
    QVERIFY(runTransformCommit(scaled.get(), view));
    const auto actual = transform_api::layerRect(view, path);
    PictureView& reference = *direct->activeView();
    const auto expected = transform_api::layerRect(reference, layers_api::rowAt(reference, 0).path);
    QCOMPARE(QSize(actual.width(), actual.height()), QSize(expected.width(), expected.height()));
    QCOMPARE(view.image().copy(actual.left, actual.top, 28, 60),
             reference.image().copy(expected.left, expected.top, 28, 60));
    QVERIFY(view.undo());
    QCOMPARE(view.image(), rotated);
}

void VectorSmartObjectTest::sourceRoundTrips()
{
    QTemporaryDir dir;
    const QByteArray bytes = test::vectorPdf();
    for (const auto& sourceExt : {QStringLiteral("pdf"), QStringLiteral("ai")}) {
        const QString source = dir.filePath(QStringLiteral("original.") + sourceExt);
        QVERIFY(test::writeFixture(source, bytes));
        for (const auto& hostExt : {QStringLiteral("psd"), QStringLiteral("psb")}) {
            auto window = target();
            QVERIFY(window);
            PlaceDriver driver;
            driver.run(*window, source, PlaceResult::Accept);
            QVERIFY(driver.chosePage && driver.transformed);
            const QImage expected = window->activeView()->image();
            const QString host = dir.filePath(QStringLiteral("host.") + hostExt);
            QVERIFY(window->saveActiveAs(host));
            QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
            QVERIFY(window->openPath(host));
            PictureView& reopened = *window->activeView();
            QCOMPARE(reopened.image(), expected);
            const auto row = layers_api::rowAt(reopened, 0);
            QVERIFY(row.smartObject);
            const QString exported = dir.filePath(QStringLiteral("exported.") + sourceExt);
            QVERIFY(reopened.export_smart_object_contents(row.path, exported));
            QCOMPARE(test::fixtureBytes(exported), bytes);
        }
    }
}

void VectorSmartObjectTest::sourceIdentityRoundTrips()
{
    QTemporaryDir dir;
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    QVERIFY(test::writeFixture(source, test::vectorPdf()));
    for (const auto& ext : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        auto window = target();
        QVERIFY(window);
        PlaceDriver driver;
        driver.run(*window, source, PlaceResult::Accept);
        PictureView& view = *window->activeView();
        const QString original = layers_api::rowAt(view, 0).path;
        const QString separate = smart_object_new_via_copy(view, original);
        QVERIFY(!separate.isEmpty());
        QVERIFY(view.set_layer_name_path(separate, layers_api::rowForPath(view, original).name));
        const auto shared = view.duplicate_layers({original});
        QCOMPARE(shared.size(), 1);
        const QString host = dir.filePath(QStringLiteral("identity.") + ext);
        QVERIFY(window->saveActiveAs(host));
        QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
        QVERIFY(window->openPath(host));
        PictureView& reopened = *window->activeView();
        const QString identity = vector_smart_object_id(reopened, original);
        QVERIFY(!identity.isEmpty());
        const QByteArray before = test::fixtureBytes(source);
        const QByteArray edited = test::vectorPdf(Qt::green);
        EditorDriver editor;
        QVERIFY(editor.launch(*window, original));
        QVERIFY(test::writeFixture(editor.file, edited));
        QVERIFY(editor.apply());
        int sharedCount = 0;
        int independentCount = 0;
        for (const auto& row : layers_api::rows(reopened)) {
            if (!row.smartObject) {
                continue;
            }
            if (vector_smart_object_id(reopened, row.path) == identity) {
                ++sharedCount;
                QCOMPARE(exported(reopened, row.path, dir), edited);
            } else {
                ++independentCount;
                QCOMPARE(exported(reopened, row.path, dir), before);
            }
        }
        QCOMPARE(sharedCount, 2);
        QCOMPARE(independentCount, 1);
    }
}

void VectorSmartObjectTest::launchesExternalEditor()
{
    QTemporaryDir dir;
    const QByteArray original = test::vectorPdf();
    for (const auto& ext : {QStringLiteral("pdf"), QStringLiteral("ai")}) {
        auto window = target();
        QVERIFY(window);
        const QString source = dir.filePath(QStringLiteral("source.") + ext);
        QVERIFY(test::writeFixture(source, original));
        PlaceDriver place;
        place.run(*window, source, PlaceResult::Accept);
        const int tabs = window->documentCount();
        PictureView& view = *window->activeView();
        const int states = view.history_count();
        const QString path = layers_api::rowAt(view, 0).path;
        EditorDriver denied;
        QVERIFY(!denied.launch(*window, path, false));
        QVERIFY(denied.warning);
        QVERIFY(!QFile::exists(denied.file));
        EditorDriver editor;
        QVERIFY(editor.launch(*window, path));
        QVERIFY(editor.dialog);
        QCOMPARE(QFileInfo(editor.file).suffix(), ext);
        QCOMPARE(test::fixtureBytes(editor.file), original);
        QCOMPARE(window->documentCount(), tabs);
        QCOMPARE(view.history_count(), states);
    }
}

void VectorSmartObjectTest::appliesToSharedInstances()
{
    QTemporaryDir dir;
    auto window = target(160, 120);
    QVERIFY(window);
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    const QByteArray original = test::vectorPdf();
    QVERIFY(test::writeFixture(source, original));
    PlaceDriver place;
    place.run(*window, source, PlaceResult::Accept);
    PictureView& view = *window->activeView();
    const QString first = layers_api::rowAt(view, 0).path;
    const QString id = vector_smart_object_id(view, first);
    QVERIFY(!smart_object_new_via_copy(view, first).isEmpty());
    QCOMPARE(view.duplicate_layers({first}).size(), 1);
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->beginWarp(first));
    QVERIFY(tools->setTransformWarpStyle(QStringLiteral("arc")));
    QVERIFY(runTransformCommit(window.get(), view));
    const QString warpRect = transform_api::layerRect(view, first).toString();
    EditorDriver editor;
    QVERIFY(editor.launch(*window, first));
    QVERIFY(view.move_layer_path(first, 1));
    const int before = view.history_count();
    const QByteArray edited = test::vectorPdf(Qt::green);
    QVERIFY(test::writeFixture(editor.file, edited));
    QVERIFY(editor.apply());
    QCOMPARE(view.history_count(), before + 1);
    QCOMPARE(view.history_label(before), QStringLiteral("Edit Contents"));
    QVERIFY(editor.apply());
    QCOMPARE(view.history_count(), before + 1);
    bool foundWarp = false;
    for (const auto& row : layers_api::rows(view)) {
        if (!row.smartObject) {
            continue;
        }
        const bool shared = vector_smart_object_id(view, row.path) == id;
        QCOMPARE(exported(view, row.path, dir), shared ? edited : original);
        foundWarp |= shared && transform_api::layerRect(view, row.path).toString() == warpRect;
    }
    QVERIFY(foundWarp);
    const QImage changed = view.image();
    QVERIFY(view.undo());
    for (const auto& row : layers_api::rows(view)) {
        if (row.smartObject) {
            QCOMPARE(exported(view, row.path, dir), original);
        }
    }
    QVERIFY(view.redo());
    QCOMPARE(view.image(), changed);
}

void VectorSmartObjectTest::refusesInvalidOrStaleEdits()
{
    QTemporaryDir dir;
    auto window = target();
    QVERIFY(window);
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    const QByteArray original = test::vectorPdf();
    QVERIFY(test::writeFixture(source, original));
    PlaceDriver place;
    place.run(*window, source, PlaceResult::Accept);
    PictureView& view = *window->activeView();
    const QString path = layers_api::rowAt(view, 0).path;
    EditorDriver editor;
    QVERIFY(editor.launch(*window, path));
    const int before = view.history_count();
    QVERIFY(editor.apply());
    QCOMPARE(view.history_count(), before);
    for (const auto& invalid : {QByteArray("bad source"), test::vectorPdf(Qt::green, false)}) {
        QVERIFY(test::writeFixture(editor.file, invalid));
        QVERIFY(!editor.apply());
        QCOMPARE(view.history_count(), before);
        QCOMPARE(exported(view, path, dir), original);
    }
    QVERIFY(test::writeFixture(editor.file, test::vectorPdf(Qt::green)));
    QVERIFY(view.rasterize_smart_object(path));
    const int after = view.history_count();
    QVERIFY(!editor.apply());
    QCOMPARE(view.history_count(), after);
}

void VectorSmartObjectTest::discardsExternalSession()
{
    QTemporaryDir dir;
    auto window = target();
    QVERIFY(window);
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    const QByteArray original = test::vectorPdf();
    QVERIFY(test::writeFixture(source, original));
    PlaceDriver place;
    place.run(*window, source, PlaceResult::Accept);
    PictureView& view = *window->activeView();
    const QString path = layers_api::rowAt(view, 0).path;
    const int before = view.history_count();
    EditorDriver discarded;
    QVERIFY(discarded.launch(*window, path));
    QVERIFY(test::writeFixture(discarded.file, test::vectorPdf(Qt::green)));
    auto* discard = discarded.dialog->findChild<QPushButton*>(QStringLiteral("vectorEditDiscard"));
    QVERIFY(discard);
    discard->click();
    QVERIFY(!QFile::exists(discarded.file));
    QCOMPARE(view.history_count(), before);
    QCOMPARE(exported(view, path, dir), original);
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    EditorDriver closed;
    QVERIFY(closed.launch(*window, path));
    QVERIFY(window->closeDocument(window->activeDocumentIndex(), false));
    QVERIFY(!QFile::exists(closed.file));
}

void VectorSmartObjectTest::discardDuringApply()
{
    QTemporaryDir dir;
    auto window = target();
    QVERIFY(window);
    const QString source = dir.filePath(QStringLiteral("source.pdf"));
    const QByteArray original = test::vectorPdf();
    QVERIFY(test::writeFixture(source, original));
    PlaceDriver place;
    place.run(*window, source, PlaceResult::Accept);
    PictureView& view = *window->activeView();
    const QString path = layers_api::rowAt(view, 0).path;
    const int history = view.history_count();
    const QImage before = view.image();
    EditorDriver editor;
    QVERIFY(editor.launch(*window, path));
    QVERIFY(test::writeFixture(editor.file, test::vectorPdf(Qt::green)));
    document_job_set_delay(view, 800);
    auto* discard = editor.dialog->findChild<QPushButton*>(QStringLiteral("vectorEditDiscard"));
    QVERIFY(discard);
    bool wasPending = false;
    QTimer::singleShot(0, discard, [&]() {
        wasPending = document_job_pending(view);
        discard->click();
    });
    QVERIFY(editor.apply());
    QVERIFY(wasPending);
    QVERIFY(!document_job_pending(view));
    QVERIFY(!QFile::exists(editor.file));
    QCOMPARE(view.history_count(), history);
    QCOMPARE(view.image(), before);
    QCOMPARE(exported(view, path, dir), original);
}

QTEST_MAIN(VectorSmartObjectTest)
#include "tst_vector_smart_object.moc"
