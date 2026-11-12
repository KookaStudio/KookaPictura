// The Free Transform overlay is not clipped to the document edge (#308): a
// layer dragged past the canvas keeps its full bounding box and handles, while
// only the rasterised preview is clipped to the canvas.

#include <QtTest/QtTest>

#include <cmath>

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"

#include <QtWidgets/QCheckBox>

namespace {

// The drawn ring's stroke is one device pixel; the snapped centre can put the
// exact edge pixel either side of the rounded coordinate, so scan a small
// neighbourhood for the ink.
QPoint darkPixelNear(const QImage& shot, const QPointF& expected)
{
    const int cx = qRound(expected.x());
    const int cy = qRound(expected.y());
    for (int radius = 0; radius <= 2; ++radius) {
        for (int dy = -radius; dy <= radius; ++dy) {
            for (int dx = -radius; dx <= radius; ++dx) {
                if (radius > 0 && std::abs(dx) != radius && std::abs(dy) != radius) {
                    continue;
                }
                const QPoint p(cx + dx, cy + dy);
                if (QRect(QPoint(0, 0), shot.size()).contains(p) && qGray(shot.pixel(p)) < 64) {
                    return p;
                }
            }
        }
    }
    return QPoint(-1, -1);
}

} // namespace

class TransformOverlayTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void controlsBoxDrawsPastTheDocumentEdge();
    void freeTransformClearsTheBrushRing();
    void reducedPreviewDrawsAtDocumentSize();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TransformOverlayTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TransformOverlayTest::cleanup()
{
    pictura::set_transform_preview_pixels_for_test(-1);
    if (window_) {
        if (pictura::PictureView* view = window_->activeView();
            view && view->transform_session_active()) {
            view->cancel_transform();
        }
        if (auto* box = window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"))) {
            box->setChecked(false);
        }
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

void TransformOverlayTest::controlsBoxDrawsPastTheDocumentEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Overlay"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(view && canvas);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(10, 8, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    frame.setActiveTool(pictura::ToolId::Move);
    auto* box = frame.findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(box);
    box->setChecked(true);
    QVERIFY(!canvas->controlsBoxForTest().isEmpty());

    // Drag the top-left handle past the canvas edge: the box scales about the
    // fixed bottom-right corner to (-10,-2)-(30,28).
    canvas->mousePressed(QPointF(10.0, 8.0), Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseMoved(QPointF(-10.0, -2.0));
    canvas->mouseReleased(QPointF(-10.0, -2.0));
    QCOMPARE(canvas->controlsBoxForTest().boundingRect(), QRectF(-10, -2, 40, 30));

    // The off-canvas corner handle is painted (its centre pixel is the handle
    // fill), not clipped away with the document.
    const QPointF outside = canvas->imageToWidget(QPointF(-10.0, -2.0));
    const QPointF inside = canvas->imageToWidget(QPointF(30.0, 28.0));
    const QImage shot = canvas->grab().toImage();
    const QColor outsidePixel(shot.pixel(outside.toPoint()));
    const QColor insidePixel(shot.pixel(inside.toPoint()));
    QCOMPARE(outsidePixel, insidePixel);
    QVERIFY(outsidePixel.red() > 200 && outsidePixel.green() > 200 && outsidePixel.blue() > 200);
}

void TransformOverlayTest::freeTransformClearsTheBrushRing()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RingClear"), 160, 160, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();

    frame.setActiveTool(pictura::ToolId::Brush);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();
    canvas->fitOnScreen();
    canvas->setBrushOutline(80.0, QPointF(80.0, 80.0));
    QCoreApplication::processEvents();
    QVERIFY(canvas->hasBrushOutline());

    // A patch on the ring, clear of the box border and the reference point.
    const QPointF raw = QPointF(80.0, 80.0) * canvas->zoom() + canvas->offset();
    const QPointF centre(std::round(raw.x()), std::round(raw.y()));
    const double radius = canvas->brushOutlineScreenDiameterForTest() / 2.0;
    const QImage withRing = canvas->grab().toImage();
    const QPoint ring = darkPixelNear(withRing, QPointF(centre.x() + radius, centre.y()));
    QVERIFY2(ring.x() >= 0, "the brush ring is drawn");
    const QRect ringPatch(ring - QPoint(4, 4), QSize(9, 9));

    QVERIFY(frame.beginFreeTransform(layer));
    QCoreApplication::processEvents();
    QVERIFY2(!canvas->hasBrushOutline(), "Free Transform clears the brush ring");
    const QImage firstFrame = canvas->grab().toImage();
    QVERIFY2(firstFrame.copy(ringPatch) != withRing.copy(ringPatch),
             "the first transform frame repaints the ring");

    canvas->setZoom(2.0, centre);
    QCoreApplication::processEvents();
    QVERIFY2(!canvas->hasBrushOutline(), "a zoom during the session keeps the ring hidden");

    tools->cancelFreeTransform();
    QCoreApplication::processEvents();
    QVERIFY(!canvas->hasBrushOutline());
    canvas->mouseMoved(QPointF(80.0, 80.0));
    QCoreApplication::processEvents();
    QVERIFY2(canvas->hasBrushOutline(), "the ring returns on the next hover");
}

// [tpm_draw_scaled]
void TransformOverlayTest::reducedPreviewDrawsAtDocumentSize()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Reduced"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(view && canvas && tools);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    const QString layer = QString::number(added);
    frame.selectLayerPath(layer);
    QVERIFY(view->select_rect(10, 6, 20, 20, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    // At 800 % the box's handles and reference point cover about a document
    // pixel each, clear of the samples below.
    canvas->setZoom(8.0, canvas->imageToWidget(QPointF(20.0, 15.0)));

    const auto pixelAt = [canvas](const QImage& shot, double x, double y) {
        return QColor(shot.pixel(canvas->imageToWidget(QPointF(x, y)).toPoint()));
    };
    // The 400-px box is over a 300-px threshold: a half-size preview.
    for (const qint64 threshold : {qint64(300), qint64(-1)}) {
        pictura::set_transform_preview_pixels_for_test(threshold);
        QVERIFY(tools->beginFreeTransform(layer));
        const int scale = threshold > 0 ? 2 : 1;
        QCOMPARE(pictura::transform_preview_scale(*view), scale);
        QCOMPARE(canvas->transformPreviewLayerForTest().size(), QSize(20, 20) / scale);
        QCoreApplication::processEvents();
        const QImage shot = canvas->grab().toImage();
        // The mark's far corner is black only when the image is drawn at its
        // document size; around the mark the canvas stays white.
        for (const QPointF& inside : {QPointF(13.5, 9.5), QPointF(25.5, 21.5), QPointF(26.5, 11.5)}) {
            QVERIFY2(qGray(pixelAt(shot, inside.x(), inside.y()).rgb()) < 64,
                     qPrintable(QStringLiteral("scale %1 at %2,%3").arg(scale).arg(inside.x()).arg(inside.y())));
        }
        for (const QPointF& outside : {QPointF(5.5, 3.5), QPointF(35.5, 16.5), QPointF(20.5, 28.5)}) {
            QVERIFY2(qGray(pixelAt(shot, outside.x(), outside.y()).rgb()) > 200,
                     qPrintable(QStringLiteral("scale %1 at %2,%3").arg(scale).arg(outside.x()).arg(outside.y())));
        }
        tools->cancelFreeTransform();
    }
}

QTEST_MAIN(TransformOverlayTest)
#include "tst_transform_overlay.moc"
