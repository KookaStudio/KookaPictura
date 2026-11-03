// The brush outline is drawn in the canvas screen frame at the scaled diameter
// and snapped to the device pixel grid, so it is crisp at every zoom (#980).

#include <QtTest/QtTest>

#include <algorithm>
#include <cmath>
#include <memory>

#include <QtCore/QEvent>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

constexpr int kDocSize = 60;

struct DarkRun {
    int lo = -1;
    int hi = -1;
    int count = 0;
    bool found() const { return lo >= 0; }
    int extent() const { return hi - lo; }
};

DarkRun scanRow(const QImage& shot, int y, int x0, int x1)
{
    DarkRun run;
    if (y < 0 || y >= shot.height()) {
        return run;
    }
    x0 = std::max(0, x0);
    x1 = std::min(shot.width() - 1, x1);
    for (int x = x0; x <= x1; ++x) {
        if (qGray(shot.pixel(x, y)) < 64) {
            if (!run.found()) {
                run.lo = x;
            }
            run.hi = x;
            ++run.count;
        }
    }
    return run;
}

DarkRun scanColumn(const QImage& shot, int x, int y0, int y1)
{
    DarkRun run;
    if (x < 0 || x >= shot.width()) {
        return run;
    }
    y0 = std::max(0, y0);
    y1 = std::min(shot.height() - 1, y1);
    for (int y = y0; y <= y1; ++y) {
        if (qGray(shot.pixel(x, y)) < 64) {
            if (!run.found()) {
                run.lo = y;
            }
            run.hi = y;
            ++run.count;
        }
    }
    return run;
}

QPointF snappedCentre(const pictura::ImageView* canvas, const QPointF& imagePos)
{
    const QPointF raw = imagePos * canvas->zoom() + canvas->offset();
    return QPointF(std::round(raw.x()), std::round(raw.y()));
}

int docLeft(const pictura::ImageView* canvas)
{
    return int(std::lround(canvas->offset().x())) + 1;
}

int docRight(const pictura::ImageView* canvas, double docWidth)
{
    return int(std::lround(canvas->offset().x() + docWidth * canvas->zoom())) - 1;
}

int docTop(const pictura::ImageView* canvas)
{
    return int(std::lround(canvas->offset().y())) + 1;
}

int docBottom(const pictura::ImageView* canvas, double docHeight)
{
    return int(std::lround(canvas->offset().y() + docHeight * canvas->zoom())) - 1;
}

QImage whiteSeed()
{
    QImage seed(kDocSize, kDocSize, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

bool showWindow(pictura::PicturaMainWindow& frame)
{
    frame.show();
    const bool exposed = QTest::qWaitForWindowExposed(&frame);
    QCoreApplication::processEvents();
    return exposed;
}

void setCentreZoom(pictura::ImageView* canvas, double zoom)
{
    canvas->setZoom(zoom, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QCoreApplication::processEvents();
}

} // namespace

class BrushOutlineTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void ringTracksBrushSize();
    void ringDiameterMatchesZoom();
    void ringEdgeSnapsToScaledPixelBoundary();
    void ringHaloIsOneDevicePixel();
    void subPixelBrushKeepsAOnePixelRing();
    void ringDrawsPastTheDocumentEdge();
    void noRingForOtherTools();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void BrushOutlineTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void BrushOutlineTest::ringTracksBrushSize()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_size"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    setCentreZoom(f.canvas, 1.0);
    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);

    f.canvas->setBrushOutline(10.0, pos);
    QCoreApplication::processEvents();
    const int cy = int(std::lround(snappedCentre(f.canvas, pos).y()));
    const DarkRun narrow =
        scanRow(f.canvas->grab().toImage(), cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
    QVERIFY2(narrow.found(), "the 10 px ring is drawn");

    f.canvas->setBrushOutline(30.0, pos);
    QCoreApplication::processEvents();
    const DarkRun wide =
        scanRow(f.canvas->grab().toImage(), cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
    QVERIFY2(wide.found(), "the 30 px ring is drawn");

    QCOMPARE(narrow.extent(), 10);
    QCOMPARE(wide.extent(), 30);
    QCOMPARE(wide.extent() - narrow.extent(), 20);
}

void BrushOutlineTest::ringDiameterMatchesZoom()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_zoom"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);
    const double zooms[] = {0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0};
    for (const double zoom : zooms) {
        setCentreZoom(f.canvas, zoom);
        f.canvas->setBrushOutline(8.0, pos);
        QCoreApplication::processEvents();
        const QImage shot = f.canvas->grab().toImage();
        const QPointF centre = snappedCentre(f.canvas, pos);
        const int cx = int(std::lround(centre.x()));
        const int cy = int(std::lround(centre.y()));
        const int expected = qRound(8.0 * zoom);
        const DarkRun row =
            scanRow(shot, cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
        const DarkRun column =
            scanColumn(shot, cx, docTop(f.canvas), docBottom(f.canvas, kDocSize));
        QVERIFY2(row.found() && column.found(),
                 qPrintable(QStringLiteral("ring at zoom %1").arg(zoom)));
        QCOMPARE(row.extent(), expected);
        QCOMPARE(column.extent(), expected);
        QCOMPARE(cx - row.lo, row.hi - cx);
        QCOMPARE(cy - column.lo, column.hi - cy);
    }

    // 12.5 rounds to 13: the extent follows round(), not the fractional product.
    setCentreZoom(f.canvas, 1.25);
    f.canvas->setBrushOutline(10.0, pos);
    QCoreApplication::processEvents();
    const QImage shot = f.canvas->grab().toImage();
    const int cy = int(std::lround(snappedCentre(f.canvas, pos).y()));
    const DarkRun row =
        scanRow(shot, cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
    QVERIFY(row.found());
    QVERIFY2(std::abs(row.extent() - 13) <= 1,
             qPrintable(QStringLiteral("D=10 at 1.25x: %1").arg(row.extent())));
}

void BrushOutlineTest::ringEdgeSnapsToScaledPixelBoundary()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_snap"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);
    const double zooms[] = {0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0};
    for (const double zoom : zooms) {
        setCentreZoom(f.canvas, zoom);
        f.canvas->setBrushOutline(8.0, pos);
        QCoreApplication::processEvents();
        const QImage shot = f.canvas->grab().toImage();
        const QPointF centre = snappedCentre(f.canvas, pos);
        const int cx = int(std::lround(centre.x()));
        const int cy = int(std::lround(centre.y()));
        const int expected = qRound(8.0 * zoom);
        const DarkRun row =
            scanRow(shot, cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
        const DarkRun column =
            scanColumn(shot, cx, docTop(f.canvas), docBottom(f.canvas, kDocSize));
        QVERIFY2(row.found() && column.found(),
                 qPrintable(QStringLiteral("ring at zoom %1").arg(zoom)));
        QCOMPARE(row.extent(), expected);
        QCOMPARE(column.extent(), expected);
        QCOMPARE(cx - row.lo, row.hi - cx);
        QCOMPARE(cy - column.lo, column.hi - cy);
        QCOMPARE(row.count, 2);
        QCOMPARE(column.count, 2);
        QVERIFY2(qGray(shot.pixel(row.lo + 1, cy)) >= 64,
                 qPrintable(QStringLiteral("left stroke is one pixel at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(row.hi - 1, cy)) >= 64,
                 qPrintable(QStringLiteral("right stroke is one pixel at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(cx, column.lo + 1)) >= 64,
                 qPrintable(QStringLiteral("top stroke is one pixel at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(cx, column.hi - 1)) >= 64,
                 qPrintable(QStringLiteral("bottom stroke is one pixel at zoom %1").arg(zoom)));
    }
}

void BrushOutlineTest::ringHaloIsOneDevicePixel()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(kDocSize, kDocSize, QImage::Format_RGB32);
    seed.fill(QColor(128, 128, 128));
    Fixture f(frame, seed, QStringLiteral("pictura_brush_outline_halo"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);
    const double zooms[] = {0.5, 1.0, 2.0, 4.0};
    for (const double zoom : zooms) {
        setCentreZoom(f.canvas, zoom);
        f.canvas->setBrushOutline(8.0, pos);
        QCoreApplication::processEvents();
        const QImage shot = f.canvas->grab().toImage();
        const int cy = int(std::lround(snappedCentre(f.canvas, pos).y()));
        const DarkRun row =
            scanRow(shot, cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
        QVERIFY2(row.found(), qPrintable(QStringLiteral("ring at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(row.lo - 1, cy)) > 192,
                 qPrintable(QStringLiteral("white halo at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(row.lo - 2, cy)) < 192,
                 qPrintable(QStringLiteral("document past the halo at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(row.hi + 1, cy)) > 192,
                 qPrintable(QStringLiteral("white halo at zoom %1").arg(zoom)));
        QVERIFY2(qGray(shot.pixel(row.hi + 2, cy)) < 192,
                 qPrintable(QStringLiteral("document past the halo at zoom %1").arg(zoom)));
    }
}

void BrushOutlineTest::subPixelBrushKeepsAOnePixelRing()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_subpixel"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);
    setCentreZoom(f.canvas, 0.25);
    f.canvas->setBrushOutline(1.0, pos);
    QCoreApplication::processEvents();

    QCOMPARE(f.canvas->brushOutlineScreenDiameterForTest(), 1.0);
    const QImage shot = f.canvas->grab().toImage();
    const int cy = int(std::lround(snappedCentre(f.canvas, pos).y()));
    const DarkRun row =
        scanRow(shot, cy, docLeft(f.canvas), docRight(f.canvas, kDocSize));
    QVERIFY2(row.found(), "the sub-pixel brush still draws a ring");
    QVERIFY2(row.extent() <= 2,
             qPrintable(QStringLiteral("sub-pixel ring extent %1").arg(row.extent())));
}

void BrushOutlineTest::ringDrawsPastTheDocumentEdge()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_edge"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    f.canvas->setCanvasColor(Qt::white);
    setCentreZoom(f.canvas, 1.0);
    const QPointF pos(55.0, kDocSize / 2.0);
    f.canvas->setBrushOutline(20.0, pos);
    QCoreApplication::processEvents();

    const QImage shot = f.canvas->grab().toImage();
    const int cy = int(std::lround(snappedCentre(f.canvas, pos).y()));
    const int edge = int(std::lround(f.canvas->offset().x() + kDocSize * f.canvas->zoom()));
    int inside = 0;
    int outside = 0;
    for (int x = std::max(0, edge - 20); x < edge; ++x) {
        if (qGray(shot.pixel(x, cy)) < 64) {
            ++inside;
        }
    }
    for (int x = edge + 1; x <= std::min(shot.width() - 1, edge + 20); ++x) {
        if (qGray(shot.pixel(x, cy)) < 64) {
            ++outside;
        }
    }
    QVERIFY2(inside > 0, "the ring is drawn inside the document");
    QVERIFY2(outside > 0, "the ring is drawn past the document edge");
}

void BrushOutlineTest::noRingForOtherTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, whiteSeed(), QStringLiteral("pictura_brush_outline_hidden"));
    QVERIFY2(f.ok(), "brush outline fixture");
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY2(showWindow(frame), "window exposed");

    const QPointF pos(kDocSize / 2.0, kDocSize / 2.0);
    f.canvas->mouseMoved(pos);
    QCoreApplication::processEvents();
    QVERIFY2(f.canvas->hasBrushOutline(), "the Brush draws a ring on hover");

    frame.setActiveTool(pictura::ToolId::Move);
    f.canvas->mouseMoved(pos);
    QCoreApplication::processEvents();
    QVERIFY2(!f.canvas->hasBrushOutline(), "the Move tool clears the ring");

    frame.setActiveTool(pictura::ToolId::Brush);
    f.canvas->mouseMoved(pos);
    QCoreApplication::processEvents();
    QVERIFY2(f.canvas->hasBrushOutline(), "the Brush draws the ring again");

    QEvent leave(QEvent::Leave);
    QApplication::sendEvent(f.canvas, &leave);
    QCoreApplication::processEvents();
    QVERIFY2(!f.canvas->hasBrushOutline(), "leaving the canvas clears the ring");
}

QTEST_MAIN(BrushOutlineTest)
#include "tst_brush_outline.moc"
