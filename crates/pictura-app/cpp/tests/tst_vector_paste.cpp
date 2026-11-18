#include <QtTest/QtTest>

#include <QtCore/QMimeData>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QClipboard>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QRadioButton>

#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "qt_test_support.h"
#include "vector_pdf_fixture.h"

using namespace pictura;

namespace {

void setPdfClipboard(const QByteArray& bytes)
{
    auto* data = new QMimeData;
    data->setData(QStringLiteral("application/pdf"), bytes);
    QGuiApplication::clipboard()->setMimeData(data);
}

QAction* action(PicturaMainWindow& window, const char* id)
{
    window.registry()->refresh();
    return window.registry()->action(QString::fromLatin1(id));
}

struct PasteDriver {
    bool shown = false;
    bool choicesCorrect = false;
    bool reportedError = false;

    void trigger(PicturaMainWindow& window, QAction& paste, bool pixels = false, bool cancel = false)
    {
        QTimer timer;
        QObject::connect(&timer, &QTimer::timeout, &timer, [&]() {
            if (auto* error = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) {
                reportedError = true;
                error->accept();
                return;
            }
            auto* dialog = window.findChild<QDialog*>(QStringLiteral("vectorPasteAs"));
            if (!dialog || !dialog->isVisible()) {
                return;
            }
            shown = true;
            auto* smart = dialog->findChild<QRadioButton*>(QStringLiteral("vectorPasteSmart"));
            auto* pixel = dialog->findChild<QRadioButton*>(QStringLiteral("vectorPastePixels"));
            auto* paths = dialog->findChild<QRadioButton*>(QStringLiteral("vectorPastePaths"));
            auto* shape = dialog->findChild<QRadioButton*>(QStringLiteral("vectorPasteShape"));
            choicesCorrect = smart && smart->isEnabled() && pixel && pixel->isEnabled()
                && paths && !paths->isEnabled() && shape && !shape->isEnabled();
            if (pixels && pixel) {
                pixel->setChecked(true);
            }
            if (cancel) {
                dialog->reject();
            } else {
                dialog->accept();
            }
        });
        timer.start(10);
        paste.trigger();
    }
};

} // namespace

class VectorPasteTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void offersSupportedDestinations();
    void regularPasteIsUnchanged();
    void createsLayerDestinations();
    void cancelledOrInvalidPaste();

private:
    test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void VectorPasteTest::initTestCase()
{
    if (!test::haveVectorRasterizer()) {
        QSKIP("Ghostscript (gs) is not installed");
    }
}

void VectorPasteTest::init()
{
    window_ = test::makeMainWindow();
    QVERIFY(window_);
    QVERIFY(window_->newDocument(QStringLiteral("Paste target"), 100, 50,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    window_->show();
    QGuiApplication::clipboard()->clear();
    clipboard_purge();
}

void VectorPasteTest::cleanup()
{
    QGuiApplication::clipboard()->clear();
    clipboard_purge();
    window_.reset();
}

void VectorPasteTest::offersSupportedDestinations()
{
    setPdfClipboard(test::vectorPdf());
    QVERIFY(QGuiApplication::clipboard()->image().isNull());
    QVERIFY(!clipboard_has_contents());
    QAction* paste = action(*window_, command_ids::EditPaste);
    QVERIFY(paste && paste->isEnabled());
    QVERIFY(!action(*window_, command_ids::EditPasteInPlace)->isEnabled());
    QVERIFY(!action(*window_, command_ids::EditPasteInto)->isEnabled());
    QVERIFY(!action(*window_, command_ids::EditPasteOutside)->isEnabled());
    PasteDriver driver;
    driver.trigger(*window_, *paste, false, true);
    QVERIFY(driver.shown && driver.choicesCorrect && !driver.reportedError);
}

void VectorPasteTest::regularPasteIsUnchanged()
{
    auto& view = *window_->activeView();
    QImage source(3, 2, QImage::Format_RGBA8888);
    source.fill(Qt::green);
    QGuiApplication::clipboard()->setImage(source);
    const int start = view.history_count();
    PasteDriver raster;
    raster.trigger(*window_, *action(*window_, command_ids::EditPaste));
    QVERIFY(!raster.shown && !raster.reportedError);
    QCOMPARE(view.history_count(), start + 1);
    QCOMPARE(QColor::fromRgba(view.sample_argb(0, 0)), QColor(Qt::green));

    view.select_rect(1, 0, 1, 1, QStringLiteral("new"), 0.0);
    const int beforeNativePaste = view.history_count();
    action(*window_, command_ids::EditCopy)->trigger();
    QVERIFY(clipboard_has_contents());
    PasteDriver native;
    native.trigger(*window_, *action(*window_, command_ids::EditPaste));
    QCOMPARE(view.history_count(), beforeNativePaste + 1);
    QVERIFY(!native.shown && !native.reportedError);
}

void VectorPasteTest::createsLayerDestinations()
{
    auto& view = *window_->activeView();
    const int start = view.history_count();
    const QByteArray bytes = test::vectorPdf();
    QTemporaryDir dir;
    for (const bool pixels : {false, true}) {
        setPdfClipboard(bytes);
        PasteDriver driver;
        driver.trigger(*window_, *action(*window_, command_ids::EditPaste), pixels);
        QVERIFY(driver.shown && driver.choicesCorrect && !driver.reportedError);
        QCOMPARE(view.history_count(), start + (pixels ? 2 : 1));
        const auto row = layers_api::rowAt(view, 0);
        QCOMPARE(row.smartObject, !pixels);
        QCOMPARE(QColor::fromRgba(view.sample_argb(10, 10)), QColor(Qt::red));
        QCOMPARE(QColor::fromRgba(view.sample_argb(60, 10)), QColor(Qt::white));
        if (!pixels) {
            const QString exported = dir.filePath(QStringLiteral("exported.pdf"));
            QVERIFY(view.export_smart_object_contents(row.path, exported));
            QCOMPARE(test::fixtureBytes(exported), bytes);
        }
    }
}

void VectorPasteTest::cancelledOrInvalidPaste()
{
    auto& view = *window_->activeView();
    action(*window_, command_ids::EditCopy)->trigger();
    QVERIFY(clipboard_has_contents());
    const int start = view.history_count();
    const int layers = view.layer_count();
    setPdfClipboard(test::vectorPdf());
    PasteDriver cancelled;
    cancelled.trigger(*window_, *action(*window_, command_ids::EditPaste), false, true);
    QVERIFY(cancelled.shown && !cancelled.reportedError);
    setPdfClipboard(QByteArray("%PDF-1.7\n"));
    PasteDriver invalid;
    invalid.trigger(*window_, *action(*window_, command_ids::EditPaste));
    QVERIFY(invalid.reportedError);
    QCOMPARE(view.history_count(), start);
    QCOMPARE(view.layer_count(), layers);
}

QTEST_MAIN(VectorPasteTest)
#include "tst_vector_paste.moc"
