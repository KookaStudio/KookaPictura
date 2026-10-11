// Canvas pan/zoom fixes (#252): the brush-size ring is hidden while Space (or
// middle-button) panning, and switching tools preserves the zoom and offset.

#include <QtTest/QtTest>

#include <QtGui/QImage>
#include <QtGui/QPainter>

#include "frame.h"
#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;
}

class CanvasPanTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void panHidesTheBrushRing();
    void toolSwitchKeepsZoomAndPosition();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CanvasPanTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CanvasPanTest::panHidesTheBrushRing()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(160, 160, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_pan_ring"));
    QVERIFY2(f.ok(), "canvas-pan fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    f.canvas->setBrushOutline(80.0, QPointF(80, 80));
    QCoreApplication::processEvents();
    QVERIFY(f.canvas->hasBrushOutlineForTest());

    const QPointF center = f.canvas->imageToWidget(QPointF(80, 80));
    const double radius = f.canvas->brushOutlineScreenDiameterForTest() / 2.0;
    const QPoint ring = QPointF(center.x() + radius, center.y()).toPoint();
    const QImage withRing = f.canvas->grab().toImage();

    f.canvas->setSpacePan(true);
    const QImage panned = f.canvas->grab().toImage();
    QVERIFY2(panned.pixel(ring) != withRing.pixel(ring),
             "the brush ring is hidden while panning");
    // The document pixel under the ring is the red fill, not the ring.
    const QColor shown = panned.pixelColor(ring);
    QVERIFY2(shown.red() > 200 && shown.green() < 80 && shown.blue() < 80,
             "the canvas shows the document, not the ring");

    f.canvas->setSpacePan(false);
}

void CanvasPanTest::toolSwitchKeepsZoomAndPosition()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(120, 120, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_pan_zoom"));
    QVERIFY2(f.ok(), "canvas-zoom fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->setZoom(2.0, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    QCoreApplication::processEvents();
    const double zoom = f.canvas->zoom();
    const QPointF offset = f.canvas->offset();

    frame.setActiveTool(pictura::ToolId::Brush);
    QCoreApplication::processEvents();
    QCOMPARE(f.canvas->zoom(), zoom);
    QVERIFY2(std::abs(f.canvas->offset().x() - offset.x()) < 0.5
                 && std::abs(f.canvas->offset().y() - offset.y()) < 0.5,
             "switching tools keeps the canvas position");

    frame.setActiveTool(pictura::ToolId::Move);
}

QTEST_MAIN(CanvasPanTest)
#include "tst_canvas_pan.moc"
