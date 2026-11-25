// The Zoom tool (#120/#121): click steps, Alt-click steps out, a
// press-drag-release marquee fills the viewport, and zoom-out anchors at the
// canvas centre while zoom-in stays cursor-anchored.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

double distance(const QPointF& a, const QPointF& b) { return QLineF(a, b).length(); }

QPointF viewportCentre(const pictura::ImageView& canvas)
{
    return QPointF(canvas.width() / 2.0, canvas.height() / 2.0);
}

} // namespace

class ZoomToolTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void wheelZoomInKeepsCursorFixed();
    void wheelZoomOutAnchorsAtCentre();
    void plainClickZoomsInAtClick();
    void altClickZoomsOutAtCentre();
    void marqueeFillsViewport();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ZoomToolTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ZoomToolTest::wheelZoomInKeepsCursorFixed()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_in"));
    QVERIFY2(f.ok(), "zoom-in fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    const QPointF cursor(f.canvas->width() * 0.7, f.canvas->height() * 0.3);
    const QPointF imagePoint = f.canvas->widgetToImage(cursor);
    f.canvas->zoomAt(cursor, 240);
    QVERIFY2(distance(f.canvas->imageToWidget(imagePoint), cursor) < 1.5,
             "wheel zoom-in must keep the point under the cursor fixed");
}

void ZoomToolTest::wheelZoomOutAnchorsAtCentre()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_out"));
    QVERIFY2(f.ok(), "zoom-out fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    const QPointF docCentre(f.canvas->image().width() / 2.0,
                            f.canvas->image().height() / 2.0);
    const QPointF cursor(f.canvas->width() * 0.85, f.canvas->height() * 0.15);
    const double before = f.canvas->zoom();
    f.canvas->zoomAt(cursor, -240);
    QVERIFY2(f.canvas->zoom() < before, "wheel zoom-out must reduce the zoom");
    QVERIFY2(distance(f.canvas->imageToWidget(docCentre), viewportCentre(*f.canvas)) < 2.0,
             "wheel zoom-out must anchor at the canvas centre");
}

void ZoomToolTest::plainClickZoomsInAtClick()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_click"));
    QVERIFY2(f.ok(), "zoom-click fixture");
    frame.setActiveTool(pictura::ToolId::Zoom);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    const QPointF click(f.canvas->width() * 0.75, f.canvas->height() * 0.35);
    const QPointF imagePoint = f.canvas->widgetToImage(click);
    const double before = f.canvas->zoom();
    f.canvas->mousePressed(imagePoint, Qt::LeftButton, 0);
    f.canvas->mouseReleased(imagePoint);
    QVERIFY2(f.canvas->zoom() > before, "a click must step the zoom in once");
    QVERIFY2(distance(f.canvas->imageToWidget(imagePoint), click) < 1.5,
             "a plain click must stay anchored on the clicked image point");
}

void ZoomToolTest::altClickZoomsOutAtCentre()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_alt"));
    QVERIFY2(f.ok(), "zoom-alt fixture");
    frame.setActiveTool(pictura::ToolId::Zoom);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    const QPointF docCentre(f.canvas->image().width() / 2.0,
                            f.canvas->image().height() / 2.0);
    const QPointF click(f.canvas->width() * 0.8, f.canvas->height() * 0.2);
    const QPointF imagePoint = f.canvas->widgetToImage(click);
    const double before = f.canvas->zoom();
    f.canvas->mousePressed(imagePoint, Qt::LeftButton, int(Qt::AltModifier));
    f.canvas->mouseReleased(imagePoint);
    QVERIFY2(f.canvas->zoom() < before, "Alt+click must step the zoom out once");
    QVERIFY2(distance(f.canvas->imageToWidget(docCentre), viewportCentre(*f.canvas)) < 2.0,
             "Alt+click zoom-out must anchor at the canvas centre");
}

void ZoomToolTest::marqueeFillsViewport()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_marquee"));
    QVERIFY2(f.ok(), "zoom-marquee fixture");
    frame.setActiveTool(pictura::ToolId::Zoom);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->setZoom(1.0, viewportCentre(*f.canvas));
    const QRectF region(120.0, 90.0, 80.0, 60.0);
    f.canvas->mousePressed(region.topLeft(), Qt::LeftButton, 0);
    f.canvas->mouseMoved(region.bottomRight());
    QVERIFY2(f.canvas->hasSelectionPreviewForTest(), "the marquee must be drawn while dragging");
    f.canvas->mouseReleased(region.bottomRight());
    QVERIFY2(!f.canvas->hasSelectionPreviewForTest(), "the marquee must clear on release");

    const double fit = std::min(f.canvas->width() / region.width(),
                                f.canvas->height() / region.height());
    const double expected = std::min(fit, 32.0);
    QVERIFY2(qAbs(f.canvas->zoom() - expected) < 0.05,
             "the marquee must zoom to the highest magnification that fits it");
    QVERIFY2(distance(f.canvas->imageToWidget(region.center()), viewportCentre(*f.canvas)) < 2.0,
             "the marquee area must be centred in the viewport");
}

PICTURA_QT_TEST_MAIN(ZoomToolTest)
#include "tst_zoom_tool.moc"
