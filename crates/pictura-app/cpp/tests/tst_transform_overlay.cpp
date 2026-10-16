// The Free Transform overlay is not clipped to the document edge (#308): a
// layer dragged past the canvas keeps its full bounding box and handles, while
// only the rasterised preview is clipped to the canvas.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QCheckBox>

class TransformOverlayTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void controlsBoxDrawsPastTheDocumentEdge();

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
    if (window_) {
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

QTEST_MAIN(TransformOverlayTest)
#include "tst_transform_overlay.moc"
