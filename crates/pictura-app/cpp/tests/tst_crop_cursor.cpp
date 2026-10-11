// Crop cursor zones and Classic/Modern move semantics (#252). The cursor is
// resolved per pointer position: resize on a handle, the workspace arrow over
// the box, rotate outside the box, and the new-crop crosshair in init mode.

#include <QtTest/QtTest>

#include "crop_grip.h"
#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>

#include <cmath>

class CropCursorTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cursorZones();
    void rotateCursorZones();
    void classicMovesBoxModernMovesContent();
    void firstBoxClampedThenGrows();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    // The CropToolHandler::CursorKind values.
    enum { None = 0, Box = 1, Rotate = 2, Resize = 3, NewCrop = 4 };

    void drag(pictura::ImageView* canvas, const QPointF& from, const QPointF& to)
    {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    }
};

void CropCursorTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropCursorTest::cursorZones()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CursorZones"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);  // Modern: full-canvas preview
    frame.setActiveTool(pictura::ToolId::Crop);
    // Preview: inside is the new-crop crosshair, outside the canvas rotates.
    QCOMPARE(tools->cropCursorKindForTest(QPointF(1, 1), 0), int(NewCrop));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(60, 60), 0), int(NewCrop));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(130, 130), 0), int(Rotate));

    // A drag inside the preview draws a new active box (60,60,60,60).
    drag(canvas, QPointF(120, 120), QPointF(60, 60));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(90, 90), 0), int(Box));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(63, 63), 0), int(Resize));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(30, 30), 0), int(Rotate));
    QCOMPARE(tools->cropCursorKindForTest(QPointF(90, 60), int(Qt::ShiftModifier)), int(Resize));

    // Cancel clears the box: init mode shows the new-crop cursor.
    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(&frame, &esc);
    QCOMPARE(tools->cropCursorKindForTest(QPointF(60, 60), 0), int(NewCrop));

    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropCursorTest::classicMovesBoxModernMovesContent()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CursorModes"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::Crop);

    // Classic: dragging inside moves the box.
    tools->setCropClassicMode(true);
    drag(canvas, QPointF(0, 0), QPointF(30, 30));      // draw (0,0,30,30)
    const QPolygonF classicBefore = canvas->cropBoxWidgetForTest();
    drag(canvas, QPointF(10, 10), QPointF(20, 20));    // move the box +10,+10
    const QPolygonF classicAfter = canvas->cropBoxWidgetForTest();
    QVERIFY2(classicBefore != classicAfter, "classic drag moves the box");
    QCOMPARE(canvas->cropContentOffset(), QPointF());

    // Modern: an inside drag pans the composite under a box fixed on screen.
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(120, 120), QPointF(60, 60));  // draw (60,60,60,60)
    const QPolygonF modernBefore = canvas->cropBoxWidgetForTest();
    drag(canvas, QPointF(90, 90), QPointF(100, 100));
    QCOMPARE(canvas->cropBoxWidgetForTest(), modernBefore);
    QCOMPARE(canvas->cropContentOffset(), QPointF(10, 10));
    // The committed region is the box mapped by minus the content offset.
    QVERIFY(tools->pendingCropRect() == QRect(50, 50, 60, 60));

    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropCursorTest::rotateCursorZones()
{
    // Four edges map to axis directions, four corners to diagonals.
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(10, 0)), 0.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(0, 10)), 90.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(-10, 0)), 180.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(0, -10)), 270.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(10, 10)), 45.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(10, -10)), 315.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(-10, 10)), 135.0);
    QCOMPARE(pictura::cropRotateZoneAngle(QPointF(-10, -10)), 225.0);

    // Corner zones are widened: 30° off an axis still lands on the diagonal,
    // while a near-edge 10° stays on the edge.
    const auto at = [](double deg) {
        const double r = deg * M_PI / 180.0;
        return QPointF(std::cos(r), std::sin(r));
    };
    QCOMPARE(pictura::cropRotateZoneAngle(at(30.0)), 45.0);
    QCOMPARE(pictura::cropRotateZoneAngle(at(10.0)), 0.0);
    QCOMPARE(pictura::cropRotateZoneAngle(at(60.0)), 45.0);
    QCOMPARE(pictura::cropRotateZoneAngle(at(80.0)), 90.0);
}

void CropCursorTest::firstBoxClampedThenGrows()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Grow"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    pictura::PictureView* view = frame.activeView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(view);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(true);  // start boxless and draw one
    frame.setActiveTool(pictura::ToolId::Crop);

    // The first box is clamped to the canvas, even if the drag goes past it.
    drag(canvas, QPointF(60, 60), QPointF(200, 200));
    QCOMPARE(tools->pendingCropRect(), QRect(60, 60, 60, 60));

    // A handle resize of the existing box may grow past the canvas.
    drag(canvas, QPointF(120, 120), QPointF(200, 200));
    QCOMPARE(tools->pendingCropRect(), QRect(60, 60, 140, 140));

    // Committing grows the document to the box (padding the added area).
    QVERIFY(tools->commitCrop());
    QCOMPARE(view->document_width(), 140);
    QCOMPARE(view->document_height(), 140);

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropCursorTest)
#include "tst_crop_cursor.moc"
