// Crop snapping (#282): a handle dragged near a canvas (or layer) edge lands
// exactly on it; a drag beyond the threshold does not snap.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

class CropSnapTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void handleSnapsToTheCanvasEdge();
    void handleBeyondTheThresholdDoesNotSnap();

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

void CropSnapTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropSnapTest::handleSnapsToTheCanvasEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Snap"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);  // Modern: full-canvas preview
    frame.setActiveTool(pictura::ToolId::Crop);
    // Adopt the full-canvas preview as the active box.
    drag(canvas, QPointF(40, 40), QPointF(40, 40));
    QVERIFY2(tools->cropActive(), "clicking the preview activates it");
    // Drag the bottom-right handle to 37,37 — within the 6px threshold of the
    // canvas edge — so it snaps back to 40 and the box stays canvas-sized.
    drag(canvas, QPointF(40, 40), QPointF(37, 37));
    QVERIFY2(!frame.hasPendingCrop(), "a near-edge drag snaps back to the canvas edge");

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropSnapTest::handleBeyondTheThresholdDoesNotSnap()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("NoSnap"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);  // Modern: full-canvas preview
    frame.setActiveTool(pictura::ToolId::Crop);
    // Adopt the preview, then drag its bottom-right handle well past the
    // threshold from both 40 and 0, so it does not snap.
    drag(canvas, QPointF(40, 40), QPointF(40, 40));
    drag(canvas, QPointF(40, 40), QPointF(20, 20));
    QVERIFY(tools->pendingCropRect() == QRect(0, 0, 20, 20));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropSnapTest)
#include "tst_crop_snap.moc"
