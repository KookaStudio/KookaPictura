// Image > Image Rotation > Arbitrary (#282): rotates the document about its
// centre by an arbitrary angle through pictura_render::rotate_document_in.
// The rotations and flips are gated as spilling commits (#1294).

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QStatusBar>

#include <limits>

#include "commands.h"
#include "frame.h"
#include "qt_test_support.h"

#include "api/filters_bridge.h"
#include "api/paint_bridge.h"

class ImageRotateTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void arbitraryRotatesAndRecords();
    void invalidAngleIsRefused();
    void rotationRefusalsNameMemory();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ImageRotateTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ImageRotateTest::arbitraryRotatesAndRecords()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Rot"), 20, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int before = view->history_count();

    QVERIFY((pictura::rotate_document_arbitrary(*view, 30.0)));
    // The 20x16 canvas grows to the rotated bounding box.
    QVERIFY(view->document_width() != 20 || view->document_height() != 16);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Rotate"));

    // Undo restores the original dimensions.
    QVERIFY(view->undo());
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 16);

    frame.closeDocument(doc, false);
}

void ImageRotateTest::invalidAngleIsRefused()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotBad"), 12, 12, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    const int before = view->history_count();
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, 0.0)));
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, std::numeric_limits<double>::quiet_NaN())));
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, 360.0)));
    QCOMPARE(view->history_count(), before);
    QCOMPARE(view->document_width(), 12);
    QCOMPARE(view->document_height(), 12);

    frame.closeDocument(doc, false);
}

// [ocg_rotate_refused] 4 MiB of room, under one worker's window, and no
// scratch space: every rotation and flip is a memory refusal.
void ImageRotateTest::rotationRefusalsNameMemory()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotRefused"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();
    pictura::set_available_memory_for_test(260LL * 1024 * 1024);
    pictura::set_scratch_available_for_test(0);

    // The Arbitrary prompt is answered with 15°.
    QTimer answer;
    answer.setInterval(20);
    connect(&answer, &QTimer::timeout, this, [&] {
        if (auto* prompt = frame.findChild<QInputDialog*>(); prompt && prompt->isVisible()) {
            prompt->setDoubleValue(15.0);
            prompt->accept();
        }
    });
    answer.start();
    const struct {
        const char* id;
        const char* action;
    } commands[] = {
        {pictura::command_ids::ImageRotate90Cw, "rotate the canvas"},
        {pictura::command_ids::ImageFlipHorizontal, "flip the canvas"},
        {pictura::command_ids::ImageRotateArbitrary, "rotate the canvas"},
    };
    for (const auto& command : commands) {
        frame.statusBar()->clearMessage();
        QVERIFY(frame.registry()->dispatch(QString::fromLatin1(command.id)));
        const QString message = frame.statusBar()->currentMessage();
        QVERIFY2(message.contains(QLatin1String(command.action)), qPrintable(message));
        QVERIFY2(message.contains(QStringLiteral("memory"), Qt::CaseInsensitive),
                 qPrintable(message));
        QCOMPARE(view->document_width(), 40);
        QCOMPARE(view->document_height(), 20);
        QCOMPARE(view->history_count(), history);
    }
    answer.stop();

    pictura::set_available_memory_for_test(-1);
    pictura::set_scratch_available_for_test(-1);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(ImageRotateTest)
#include "tst_image_rotate.moc"
