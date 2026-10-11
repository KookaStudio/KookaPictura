// Crop preview/active states and the straighten preview (#282, #252): Modern
// starts with a centered, ratio-fitted preview box; a drag inside draws a new
// active box; a click adopts the preview; Escape clears to no box. The content
// rotation preview and the canvas frame still work as before.

#include <QtTest/QtTest>

#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <cmath>

#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "theme.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;

void sendEsc(pictura::PicturaMainWindow& frame)
{
    QKeyEvent event(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(&frame, &event);
}
} // namespace

class CropPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void straightenRotatesTheContent();
    void canvasGrowsToTheRotatedBounds();
    void movingTheBoxKeepsThePivot();
    void modernStartsInPreview();
    void previewDragDrawsAndClickAdopts();
    void escapeClearsToNoBoxInBothModes();
    void paddedMoveShowsTheBackdrop();
    void ratioDragPinsTheStartCorner();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    void drag(pictura::ImageView* canvas, const QPointF& from, const QPointF& to)
    {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    }
};

void CropPreviewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropPreviewTest::straightenRotatesTheContent()
{
    pictura::PicturaMainWindow& frame = *window_;
    // A left/right split so any rotation visibly changes the canvas.
    QImage seed(120, 120, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(0, 0, 60, 120, Qt::red);
    Fixture f(frame, seed, QStringLiteral("pictura_crop_preview"));
    QVERIFY2(f.ok(), "crop-preview fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    f.canvas->setCropBox(QRectF(40, 40, 40, 40));
    QCoreApplication::processEvents();
    const QImage straight = f.canvas->grab().toImage();

    f.canvas->setCropStraighten(30.0, QPointF(60, 60));
    QCoreApplication::processEvents();
    const QImage rotated = f.canvas->grab().toImage();
    QVERIFY2(rotated != straight, "a straighten angle rotates the preview");

    // The crop overlay stays axis-aligned even while the content rotates.
    const QPolygonF box = f.canvas->cropBoxWidgetForTest();
    QCOMPARE(box.size(), 4);
    QVERIFY2(std::abs(box[0].y() - box[1].y()) < 1e-6
                 && std::abs(box[0].x() - box[3].x()) < 1e-6,
             "the crop box stays axis-aligned under straighten");

    f.canvas->setCropStraighten(0.0, QPointF(60, 60));
    QCoreApplication::processEvents();
    QCOMPARE(f.canvas->grab().toImage(), straight);
}

void CropPreviewTest::canvasGrowsToTheRotatedBounds()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(120, 120, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_crop_preview_grow"));
    QVERIFY2(f.ok(), "crop-preview grow fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->setCropBox(QRectF(40, 40, 40, 40));
    QCOMPARE(f.canvas->cropCanvasImageRect(), QRectF(0, 0, 120, 120));

    f.canvas->setCropStraighten(30.0, QPointF(60, 60));
    const QRectF grown = f.canvas->cropCanvasImageRect();
    QVERIFY2(grown.width() > 120.0 && grown.height() > 120.0,
             "the canvas grows to the rotated content's bounding box");
}

void CropPreviewTest::movingTheBoxKeepsThePivot()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Pivot"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(true);  // boxless: draw a box, then move it
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(40, 40), QPointF(80, 80));  // box (40,40,40,40)
    // Rotate about the box centre (press outside the box, inside the canvas).
    drag(canvas, QPointF(10, 60), QPointF(10, 40));
    QVERIFY2(tools->cropAngle() != 0.0, "the outside drag rotates");
    const QPointF pivot = tools->cropPivot();
    QVERIFY2(std::abs(pivot.x() - 60.0) < 1.0 && std::abs(pivot.y() - 60.0) < 1.0,
             "the pivot is the box centre");

    // Moving the box leaves the composite's pivot in place.
    drag(canvas, QPointF(60, 60), QPointF(70, 70));
    QVERIFY2(std::abs(tools->cropPivot().x() - pivot.x()) < 1e-6
                 && std::abs(tools->cropPivot().y() - pivot.y()) < 1e-6,
             "moving the box leaves the pivot fixed");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::modernStartsInPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Preview"), 120, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY2(canvas->hasCropBoxForTest() && canvas->cropPreviewForTest(),
             "Modern starts with a preview box");
    QVERIFY2(!tools->cropActive(), "the preview is not an active crop");

    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY2(!canvas->hasCropBoxForTest(), "Classic starts with no box");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::previewDragDrawsAndClickAdopts()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PreviewDraw"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->cropPreviewForTest());

    // A drag inside the preview draws a new active box.
    drag(canvas, QPointF(40, 40), QPointF(20, 20));
    QVERIFY2(!canvas->cropPreviewForTest() && tools->cropActive(),
             "a preview drag draws an active box");
    QVERIFY(frame.hasPendingCrop());

    // Re-select to get a fresh preview, then a click adopts it.
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->cropPreviewForTest());
    drag(canvas, QPointF(40, 40), QPointF(40, 40));
    QVERIFY2(!canvas->cropPreviewForTest() && tools->cropActive(),
             "a click adopts the preview as the active box");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::escapeClearsToNoBoxInBothModes()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("EscapeBoth"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->hasCropBoxForTest());
    sendEsc(frame);
    QVERIFY2(!canvas->hasCropBoxForTest() && !tools->cropActive(),
             "Escape clears the Modern preview to no box");

    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(10, 10), QPointF(50, 50));
    QVERIFY(canvas->hasCropBoxForTest());
    sendEsc(frame);
    QVERIFY2(!canvas->hasCropBoxForTest(), "Escape clears a Classic box to no box");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::paddedMoveShowsTheBackdrop()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Backdrop"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();

    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    canvas->actualPixels();
    QCoreApplication::processEvents();

    // A box that extends 10px past every canvas edge; the frame follows it.
    canvas->setCropBox(QRectF(-10, -10, 60, 60));
    QCoreApplication::processEvents();
    QCOMPARE(canvas->cropCanvasImageRect(), QRectF(-10, -10, 60, 60));

    // A point just outside the image, inside the box: the padded area shows the
    // backdrop (the Background layer's white), not the workspace colour.
    const QPoint sample = canvas->imageToWidget(QPointF(-5, 20)).toPoint();
    QVERIFY(sample.x() >= 0 && sample.y() >= 0 && sample.x() < canvas->width()
            && sample.y() < canvas->height());
    const QImage shot = canvas->grab().toImage();
    const QColor pixel = shot.pixelColor(sample);
    QVERIFY2(pixel != pictura::Theme::workspaceColor(),
             "the padded area is not the workspace colour");
    QVERIFY2(pixel.red() > 200 && pixel.green() > 200 && pixel.blue() > 200,
             "the padded area shows the light backdrop");

    frame.closeDocument(doc, false);
}

void CropPreviewTest::ratioDragPinsTheStartCorner()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PinCorner"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    tools->setCropRatio(2.0);
    // Drag up-left from the press point (40,40); the press corner must stay put
    // (the box's bottom-right), not become the fitted top-left.
    drag(canvas, QPointF(40, 40), QPointF(0, 0));
    QCOMPARE(tools->pendingCropRect(), QRect(0, 20, 40, 20));

    tools->setCropRatio(0.0);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropPreviewTest)
#include "tst_crop_preview.moc"
