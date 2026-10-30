// Crop preview/active states and the straighten preview (#282, #252): both
// modes start with a centered, ratio-fitted preview box showing its handles; a
// handle drag resizes it, a drag inside draws a new active box, a click adopts
// the preview; Escape clears to no box. The content
// rotation preview and the canvas frame still work as before.

#include <QtTest/QtTest>

#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <cmath>

#include "image_view.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "theme.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"

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
    void bothModesStartInPreview();
    void previewShowsItsHandles();
    void previewHandleDragResizes();
    void rotateShowsGridAndAngle();
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);  // draw a box, then move it
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(40, 40), QPointF(80, 80));  // box (40,40,40,40)
    // Rotate about the box centre (press outside the box, inside the canvas).
    drag(canvas, QPointF(10, 60), QPointF(10, 40));
    QVERIFY2(tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->angle() != 0.0, "the outside drag rotates");
    const QPointF pivot = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->pivot();
    QVERIFY2(std::abs(pivot.x() - 60.0) < 1.0 && std::abs(pivot.y() - 60.0) < 1.0,
             "the pivot is the box centre");

    // Moving the box leaves the composite's pivot in place.
    drag(canvas, QPointF(60, 60), QPointF(70, 70));
    QVERIFY2(std::abs(tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->pivot().x() - pivot.x()) < 1e-6
                 && std::abs(tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->pivot().y() - pivot.y()) < 1e-6,
             "moving the box leaves the pivot fixed");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::bothModesStartInPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Preview"), 120, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY2(canvas->hasCropBoxForTest() && canvas->cropPreviewForTest(),
             "Modern starts with a preview box");
    QVERIFY2(!tools->cropBoxSession()->cropActive(), "the preview is not an active crop");

    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY2(canvas->hasCropBoxForTest() && canvas->cropPreviewForTest(),
             "Classic also starts with a preview box");
    QVERIFY(!tools->cropBoxSession()->cropActive());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// CS6 frames the image with its corner and edge handles as soon as the tool is
// chosen: the dark handle shadow shows inside the corner of a white canvas.
void CropPreviewTest::previewShowsItsHandles()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PreviewHandles"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->cropPreviewForTest());
    const QPolygonF box = canvas->cropBoxWidgetForTest();
    QVERIFY(!box.isEmpty());
    const QImage shot = canvas->grab().toImage();
    const QPointF corner = box.boundingRect().topLeft() * shot.devicePixelRatio();
    int dark = 0;
    for (int y = 1; y < 12; ++y) {
        for (int x = 1; x < 12; ++x) {
            const QPoint p = (corner + QPointF(x, y)).toPoint();
            if (shot.rect().contains(p) && shot.pixelColor(p).lightness() < 128) {
                ++dark;
            }
        }
    }
    QVERIFY2(dark > 0, "the preview's corner bracket is drawn");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropPreviewTest::previewHandleDragResizes()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PreviewResize"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    for (const bool classic : {true, false}) {
        tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(classic);
        frame.setActiveTool(pictura::ToolId::Move);
        frame.setActiveTool(pictura::ToolId::Crop);
        QVERIFY(canvas->cropPreviewForTest());
        // The bottom-right corner handle resizes the start box.
        drag(canvas, QPointF(80, 80), QPointF(50, 60));
        QCOMPARE(tools->pendingCropRect(), QRect(0, 0, 50, 60));
        QVERIFY2(!canvas->cropPreviewForTest() && tools->cropBoxSession()->cropActive(),
                 "a resized preview is the active crop");
        // So does an edge handle.
        frame.setActiveTool(pictura::ToolId::Move);
        frame.setActiveTool(pictura::ToolId::Crop);
        drag(canvas, QPointF(0, 40), QPointF(20, 40));
        QCOMPARE(tools->pendingCropRect(), QRect(20, 0, 60, 80));
    }

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Rotating the box (any overlay) shows a dense grid in the box and the angle
// beside the pointer, both gone on release.
void CropPreviewTest::rotateShowsGridAndAngle()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotateGrid"), 80, 80, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int overlay = crop->gridOverlay();

    canvas->actualPixels();
    crop->setGridOverlay(2);  // Diagonal: the rotate grid replaces any overlay
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(!canvas->cropRotatingForTest());

    // Press in the rotate band just right of the box, then swing downwards.
    canvas->mousePressed(QPointF(95, 40), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->cropRotatingForTest());
    canvas->mouseMoved(QPointF(95, 60));
    QVERIFY(crop->angle() != 0.0);
    QVERIFY(canvas->cropRotatingForTest());
    QVERIFY(canvas->hasDragSizeHintForTest());
    QCOMPARE(canvas->dragSizeHintForTest(),
             QStringLiteral("\u2220: %1\u00B0").arg(crop->angle(), 0, 'f', 1));
    canvas->mouseReleased(QPointF(95, 60));
    QVERIFY2(!canvas->cropRotatingForTest(), "the grid goes on release");
    QVERIFY2(!canvas->hasDragSizeHintForTest(), "the angle readout goes on release");
    QVERIFY(crop->angle() != 0.0);

    crop->setGridOverlay(overlay);
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->cropPreviewForTest());

    // A drag inside the preview draws a new active box.
    drag(canvas, QPointF(40, 40), QPointF(20, 20));
    QVERIFY2(!canvas->cropPreviewForTest() && tools->cropBoxSession()->cropActive(),
             "a preview drag draws an active box");
    QVERIFY(frame.hasPendingCrop());

    // Re-select to get a fresh preview, then a click adopts it.
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->cropPreviewForTest());
    drag(canvas, QPointF(40, 40), QPointF(40, 40));
    QVERIFY2(!canvas->cropPreviewForTest() && tools->cropBoxSession()->cropActive(),
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(canvas->hasCropBoxForTest());
    sendEsc(frame);
    QVERIFY2(!canvas->hasCropBoxForTest() && !tools->cropBoxSession()->cropActive(),
             "Escape clears the Modern preview to no box");

    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);
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

    // A mask target swaps the paint pair to white/black; the canvas backdrop
    // still follows the Background layer, not the mask pair.
    auto* view = frame.activeView();
    QVERIFY(view);
    const int layerIndex = view->add_layer(-1);
    QVERIFY(layerIndex >= 0);
    const QString layer = QString::number(layerIndex);
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_edit_target_set(*view, layer));
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->refresh();
    QTest::qWait(0);

    // A box that extends 10px past every canvas edge; the frame follows it.
    canvas->setCropBox(QRectF(-10, -10, 60, 60));
    QCoreApplication::processEvents();
    QCOMPARE(canvas->cropCanvasImageRect(), QRectF(-10, -10, 60, 60));

    // A point just outside the image, inside the box: the padded area shows the
    // backdrop (the Background layer's white), not the workspace colour. Kept
    // clear of the edge bars and corner brackets that sit on the box rim.
    const QPoint sample = canvas->imageToWidget(QPointF(-5, 2)).toPoint();
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setRatio(2.0);
    // Drag up-left from the press point (40,40); the press corner must stay put
    // (the box's bottom-right), not become the fitted top-left.
    drag(canvas, QPointF(40, 40), QPointF(0, 0));
    QCOMPARE(tools->pendingCropRect(), QRect(0, 20, 40, 20));

    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setRatio(0.0);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropPreviewTest)
#include "tst_crop_preview.moc"
