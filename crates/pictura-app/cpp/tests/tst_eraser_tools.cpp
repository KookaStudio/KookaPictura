// The Eraser's Block mode (#794): its side is a fixed 16 *screen* pixels, so
// the block's document size follows the canvas zoom, and the cursor shows the
// square the block erases.

#include <QtTest/QtTest>

#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <algorithm>
#include <cmath>

namespace {
using paint_fixture::Fixture;

// The Eraser's Mode menu: 0 Brush, 1 Pencil, 2 Block.
constexpr int kBlockMode = 2;

void setBlockMode(pictura::ToolController* tools)
{
    pictura::EraserOptions options =
        tools->optionsFor<pictura::EraserGroup>(pictura::ToolId::Eraser)->get();
    options.mode = kBlockMode;
    tools->optionsFor<pictura::EraserGroup>(pictura::ToolId::Eraser)->set(options);
}

// The width in device pixels of the dark run along the row through the
// outline's centre, bounded by the document so the canvas backdrop cannot be
// read as ink: the outline's on-screen size plus the antialiased edge on each
// side.
int measuredOutlineWidth(pictura::ImageView* canvas, const QPointF& imagePos,
                          const QSize& imageSize)
{
    const QImage shot = canvas->grab().toImage();
    const double z = canvas->zoom();
    const QPointF o = canvas->offset();
    const int cy = int(std::lround(o.y() + imagePos.y() * z));
    const int x0 = std::max(0, int(std::lround(o.x())) + 1);
    const int x1 = std::min(shot.width() - 1, int(std::lround(o.x() + imageSize.width() * z)) - 1);
    if (cy < 0 || cy >= shot.height() || x1 < x0) {
        return 0;
    }
    int lo = -1;
    int hi = -1;
    for (int x = x0; x <= x1; ++x) {
        if (qGray(shot.pixel(x, cy)) < 200) {
            if (lo < 0) {
                lo = x;
            }
            hi = x;
        }
    }
    return hi >= lo ? hi - lo + 1 : 0;
}

// The outline's corner in device pixels, dark for the Block's square and clear
// for a circle of the same size: a circle's ink is nowhere near its corner.
bool outlineCornerIsDark(pictura::ImageView* canvas, const QPointF& imagePos)
{
    const QImage shot = canvas->grab().toImage();
    const double r = canvas->brushOutlineDiameterForTest() / 2.0;
    const QPoint corner = canvas->imageToWidget(imagePos + QPointF(r, r)).toPoint();
    for (int y = corner.y() - 1; y <= corner.y() + 1; ++y) {
        for (int x = corner.x() - 1; x <= corner.x() + 1; ++x) {
            if (x < 0 || x >= shot.width() || y < 0 || y >= shot.height()) {
                continue;
            }
            if (qGray(shot.pixel(x, y)) < 200) {
                return true;
            }
        }
    }
    return false;
}
} // namespace

class EraserToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void blockErasesASquareThatScalesWithTheZoom();
    void theBlockCursorIsASquareOfTheBlockSize();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void EraserToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void EraserToolsTest::blockErasesASquareThatScalesWithTheZoom()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(60, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_eraser_block_seed"));
    QVERIFY2(f.ok(), "eraser fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    // The Background erases to the background colour, so the block shows up
    // as a red square.
    f.tools->setBackground(Qt::red);
    const QRgb red = QColor(Qt::red).rgba();

    // At 400 % the block is 4 document pixels a side, so it is still 16
    // screen pixels wide.
    f.canvas->setZoom(4.0, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    setBlockMode(f.tools);
    frame.setActiveTool(pictura::ToolId::Eraser);
    f.drag({QPointF(30.0, 30.0)});

    int painted = 0;
    for (int y = 0; y < 60; ++y) {
        for (int x = 0; x < 60; ++x) {
            if (f.view->sample_argb(x, y) == red) {
                ++painted;
            }
        }
    }
    QCOMPARE(painted, 4 * 4);
}

void EraserToolsTest::theBlockCursorIsASquareOfTheBlockSize()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(60, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_eraser_cursor_seed"));
    QVERIFY2(f.ok(), "eraser fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    const QPointF pointer(30.0, 30.0);
    setBlockMode(f.tools);
    frame.setActiveTool(pictura::ToolId::Eraser);
    f.canvas->setZoom(1.0, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));

    // The pointer over the canvas draws the block's square, 16 screen pixels
    // wide.
    f.canvas->mouseMoved(pointer);
    QCoreApplication::processEvents();
    QVERIFY(f.canvas->hasBrushOutline());
    QVERIFY(f.canvas->brushOutlineIsSquareForTest());
    QCOMPARE(f.canvas->brushOutlineScreenDiameterForTest(), 16.0);
    const int atOne = measuredOutlineWidth(f.canvas, pointer, seed.size());
    QVERIFY2(outlineCornerIsDark(f.canvas, pointer), "the outline is a square");

    // Zooming resizes the square to the block's new size, so it keeps its 16
    // screen pixels.
    f.canvas->setZoom(4.0, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    QCoreApplication::processEvents();
    QCOMPARE(f.canvas->brushOutlineScreenDiameterForTest(), 16.0);
    const int atFour = measuredOutlineWidth(f.canvas, pointer, seed.size());
    QVERIFY2(outlineCornerIsDark(f.canvas, pointer), "the outline is a square");

    QVERIFY2(atOne >= 16 && atOne <= 18, qPrintable(QStringLiteral("at 100 %: %1").arg(atOne)));
    QVERIFY2(atFour >= 16 && atFour <= 18, qPrintable(QStringLiteral("at 400 %: %1").arg(atFour)));
}

QTEST_MAIN(EraserToolsTest)
#include "tst_eraser_tools.moc"
