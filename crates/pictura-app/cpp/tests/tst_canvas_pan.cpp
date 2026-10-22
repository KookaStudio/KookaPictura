// Canvas pan/zoom fixes (#252): the brush-size ring is hidden while Space (or
// middle-button) panning, and switching tools preserves the zoom and offset.
// Resizing an adjusted view keeps its centre (#125).

#include <QtTest/QtTest>

#include <cmath>

#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QScrollBar>

#include "frame.h"
#include "canvas_range.h"
#include "canvas_scrollbars.h"
#include "commands.h"
#include "image_view.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;

void sendMouse(pictura::ImageView* canvas, QEvent::Type type, const QPointF& local,
               Qt::MouseButton button, Qt::MouseButtons buttons)
{
    QMouseEvent event(type, local, QPointF(canvas->mapToGlobal(local.toPoint())), button, buttons,
                      Qt::NoModifier);
    QApplication::sendEvent(canvas, &event);
}
}

class CanvasPanTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void panHidesTheBrushRing();
    void toolSwitchKeepsZoomAndPosition();
    void resizeKeepsAdjustedViewCentred();
    void offsetRangeConstrainsFittingAxes();
    void freePanConstrainsFittingImage();
    void freePanReclampsResizedDocument();
    void freePanCollapsesScrollbar();
    void scrollbarRangeFollowsThePanRange();
    void scrollbarHandleMovesAgainstTheContent();
    void freePanCommandAppliesToCanvas();
    void offsetRangeSnapsToWorkspace();
    void workspaceSnapPinsFittingImage();
    void workspaceSnapHidesScrollbars();
    void workspaceSnapCommandAppliesToCanvas();
    void spacePanOverrideHolds();
    void spacePanHoverAndMiddleButton();

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
    // The ring is a brush's; on the Move tool the controls box (on by
    // default) would draw its edge where the ring is sampled.
    frame.setActiveTool(pictura::ToolId::Brush);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    f.canvas->setBrushOutline(80.0, QPointF(80, 80));
    QCoreApplication::processEvents();
    QVERIFY(f.canvas->hasBrushOutline());

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

void CanvasPanTest::resizeKeepsAdjustedViewCentred()
{
    pictura::ImageView view;
    view.resize(400, 300);
    QImage seed(1000, 800, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view.setImage(seed);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));
    QCoreApplication::processEvents();

    view.setZoom(1.0, QPointF(view.width() / 2.0, view.height() / 2.0));
    view.panBy(QPointF(-60, 40));
    const auto centreDocPoint = [&view] {
        return view.widgetToImage(QPointF(view.width() / 2.0, view.height() / 2.0));
    };
    const QPointF before = centreDocPoint();

    for (const QSize size : {QSize(900, 700), QSize(400, 300)}) {
        view.resize(size);
        QCoreApplication::processEvents();
        QCOMPARE(view.zoom(), 1.0);
        const QPointF after = centreDocPoint();
        QVERIFY2(std::abs(after.x() - before.x()) < 0.5 && std::abs(after.y() - before.y()) < 0.5,
                 "resizing an adjusted view keeps the document point at the centre");
    }
}

void CanvasPanTest::offsetRangeConstrainsFittingAxes()
{
    const QSizeF viewport(400.0, 300.0);
    const auto close = [](double a, double b) { return std::abs(a - b) < 1e-9; };

    // A document that fits both axes collapses to the centre.
    const pictura::OffsetRange fit =
        pictura::offsetRangeFor(QSizeF(100.0, 80.0), 1.0, viewport, false);
    QVERIFY(close(fit.minX, fit.maxX));
    QVERIFY(close(fit.minX, (400.0 - 100.0) / 2.0));
    QVERIFY(close(fit.minY, fit.maxY));
    QVERIFY(close(fit.minY, (300.0 - 80.0) / 2.0));

    // Mixed: a fitting axis is pinned, an overflowing axis keeps the margin.
    const pictura::OffsetRange mixed =
        pictura::offsetRangeFor(QSizeF(100.0, 900.0), 1.0, viewport, false);
    QVERIFY(close(mixed.minX, mixed.maxX));
    QVERIFY(mixed.minY < mixed.maxY);

    // An overflowing document on both axes is byte-for-byte unchanged.
    const pictura::OffsetRange overflowFree =
        pictura::offsetRangeFor(QSizeF(1000.0, 900.0), 1.0, viewport, true);
    const pictura::OffsetRange overflowConstrained =
        pictura::offsetRangeFor(QSizeF(1000.0, 900.0), 1.0, viewport, false);
    QVERIFY(close(overflowConstrained.minX, overflowFree.minX));
    QVERIFY(close(overflowConstrained.maxX, overflowFree.maxX));
    QVERIFY(close(overflowConstrained.minY, overflowFree.minY));
    QVERIFY(close(overflowConstrained.maxY, overflowFree.maxY));

    // An empty document is unaffected: no centring offset is produced.
    const pictura::OffsetRange empty =
        pictura::offsetRangeFor(QSizeF(0.0, 0.0), 1.0, viewport, false);
    QVERIFY(close(empty.minX, 0.0));
    QVERIFY(close(empty.maxX, 400.0));
}

void CanvasPanTest::freePanConstrainsFittingImage()
{
    pictura::ImageView view;
    view.resize(400, 300);
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view.setImage(seed);
    QCoreApplication::processEvents();

    const double centreX = (400.0 - 100.0) / 2.0;
    const double centreY = (300.0 - 80.0) / 2.0;

    view.setFreePan(true);
    view.panBy(QPointF(120, 90));
    QVERIFY2(std::abs(view.offset().x() - centreX) > 1.0,
             "free pan moves a fitting image off centre");

    view.setFreePan(false);
    QVERIFY2(std::abs(view.offset().x() - centreX) < 1e-9
                 && std::abs(view.offset().y() - centreY) < 1e-9,
             "disabling free pan re-centres the fitting image");

    view.panBy(QPointF(500, 500));
    QVERIFY2(std::abs(view.offset().x() - centreX) < 1e-9
                 && std::abs(view.offset().y() - centreY) < 1e-9,
             "a constrained pan cannot move a fitting image");

    view.setFreePan(true);
    view.panBy(QPointF(500, 500));
    QVERIFY2(std::abs(view.offset().x() - centreX) > 1.0,
             "re-enabling free pan allows the pan again");
}

void CanvasPanTest::freePanReclampsResizedDocument()
{
    pictura::ImageView view;
    view.resize(400, 300);
    QImage big(1000, 800, QImage::Format_RGB32);
    big.fill(Qt::white);
    view.setImage(big);
    view.setFreePan(false);
    view.setZoom(1.0, QPointF(view.width() / 2.0, view.height() / 2.0));
    view.panBy(QPointF(50, 40));
    QVERIFY2(std::abs(view.offset().x() - (400.0 - 1000.0) / 2.0) > 1.0,
             "an overflowing document can pan while constrained");

    view.replaceDocument(QSize(100, 80));
    QVERIFY2(std::abs(view.offset().x() - (400.0 - 100.0) / 2.0) < 1e-9
                 && std::abs(view.offset().y() - (300.0 - 80.0) / 2.0) < 1e-9,
             "a document resized to fit re-centres under the constraint");
}

void CanvasPanTest::freePanCollapsesScrollbar()
{
    pictura::CanvasScrollBars bars;
    auto* view = new pictura::ImageView(&bars);
    bars.setView(view);
    bars.resize(600, 500);
    bars.show();
    QVERIFY(QTest::qWaitForWindowExposed(&bars));
    view->resize(400, 300);
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view->setImage(seed);
    QCoreApplication::processEvents();

    view->setFreePan(true);
    const int freeMaxX = bars.horizontalBarForTest()->maximum();
    const int freeMaxY = bars.verticalBarForTest()->maximum();
    QVERIFY2(freeMaxX > 0 && freeMaxY > 0, "free pan offers a pan range");

    view->setFreePan(false);
    QCOMPARE(bars.horizontalBarForTest()->maximum(), 0);
    QCOMPARE(bars.verticalBarForTest()->maximum(), 0);
    QCOMPARE(bars.horizontalBarForTest()->value(), 0);
    QCOMPARE(bars.verticalBarForTest()->value(), 0);

    view->setFreePan(true);
    QCOMPARE(bars.horizontalBarForTest()->maximum(), freeMaxX);
    QCOMPARE(bars.verticalBarForTest()->maximum(), freeMaxY);
}

void CanvasPanTest::scrollbarRangeFollowsThePanRange()
{
    pictura::CanvasScrollBars bars;
    auto* view = new pictura::ImageView(&bars);
    bars.setView(view);
    bars.resize(600, 500);
    bars.show();
    QVERIFY(QTest::qWaitForWindowExposed(&bars));
    view->resize(400, 300);
    QImage seed(1000, 800, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view->setImage(seed);
    // The initial view fits the document, so the zoom is set explicitly to
    // make it overflow the canvas.
    view->setZoom(1.0, QPointF(view->width() / 2.0, view->height() / 2.0));
    QCoreApplication::processEvents();

    QScrollBar* hbar = bars.horizontalBarForTest();
    QScrollBar* vbar = bars.verticalBarForTest();
    const double zoom = view->zoom();
    const pictura::OffsetRange range = pictura::offsetRangeFor(
        QSizeF(view->documentSize()), zoom, QSizeF(view->size()), view->freePan(),
        pictura::kCanvasRevealMarginPx, view->snapToWorkspace());

    // The bar's range is the pan range, so every position the canvas can be
    // panned to has a handle position.
    QCOMPARE(hbar->maximum(), int(std::lround(range.maxX - range.minX)));
    QCOMPARE(vbar->maximum(), int(std::lround(range.maxY - range.minY)));
    QVERIFY2(hbar->isVisible() && vbar->isVisible(),
             "the bars show a document that overflows the viewport");

    // The bar's ends are the pan range's ends: the start of the document at the
    // bar's minimum, its end at the bar's maximum.
    hbar->setValue(hbar->minimum());
    QVERIFY2(std::abs(view->offset().x() - range.maxX) < 1.0,
             "the bar's minimum shows the start of the document");
    vbar->setValue(vbar->minimum());
    QVERIFY2(std::abs(view->offset().y() - range.maxY) < 1.0,
             "the vertical bar's minimum shows the top of the document");
    hbar->setValue(hbar->maximum());
    QVERIFY2(std::abs(view->offset().x() - range.minX) < 1.0,
             "the bar's maximum shows the end of the document");
    vbar->setValue(vbar->maximum());
    QVERIFY2(std::abs(view->offset().y() - range.minY) < 1.0,
             "the vertical bar's maximum shows the bottom of the document");

    // A hand pan moves the handle one pixel per pixel.
    view->panBy(QPointF(30.0, 20.0));
    QCOMPARE(hbar->value(), int(std::lround(range.maxX - view->offset().x())));
    QCOMPARE(vbar->value(), int(std::lround(range.maxY - view->offset().y())));

    // The canvas can be panned into the reveal margin and the handle follows
    // there: it reaches the end exactly when the canvas reaches the pannable
    // limit, and it never rests at the end while the canvas can still move.
    view->panBy(QPointF(-5000.0, -5000.0));
    QVERIFY2(std::abs(view->offset().x() - range.minX) < 1.0,
             "the canvas is panned to the pan range's end");
    QCOMPARE(hbar->value(), hbar->maximum());
    QCOMPARE(vbar->value(), vbar->maximum());
    hbar->setValue(hbar->maximum() - 30);
    QVERIFY2(std::abs(view->offset().x() - (range.minX + 30.0)) < 1.0,
             "the bar leaves the margin with the canvas");

    // A document that fits keeps the bars visible and their pan range.
    view->replaceDocument(QSize(100, 80));
    QCoreApplication::processEvents();
    const pictura::OffsetRange fit = pictura::offsetRangeFor(
        QSizeF(view->documentSize()), zoom, QSizeF(view->size()), view->freePan(),
        pictura::kCanvasRevealMarginPx, view->snapToWorkspace());
    QCOMPARE(hbar->maximum(), int(std::lround(fit.maxX - fit.minX)));
    QVERIFY2(hbar->isVisible() && vbar->isVisible(),
             "the bars stay visible when the document fits");
    QVERIFY2(hbar->maximum() > 0, "free pan offers a pan range");
    const QPointF before = view->offset();
    view->panBy(QPointF(20.0, 10.0));
    QVERIFY2(view->offset() != before, "the canvas is still pannable when the document fits");

    // Free Pan off collapses the fitting axis to a point, and back on re-derives
    // the range.
    view->setFreePan(false);
    QCoreApplication::processEvents();
    QCOMPARE(hbar->maximum(), 0);
    QCOMPARE(hbar->value(), 0);
    QCOMPARE(vbar->maximum(), 0);
    QCOMPARE(vbar->value(), 0);
    view->setFreePan(true);
    QCoreApplication::processEvents();
    QCOMPARE(hbar->maximum(), int(std::lround(fit.maxX - fit.minX)));
}

void CanvasPanTest::scrollbarHandleMovesAgainstTheContent()
{
    pictura::CanvasScrollBars bars;
    auto* view = new pictura::ImageView(&bars);
    bars.setView(view);
    bars.resize(600, 500);
    bars.show();
    QVERIFY(QTest::qWaitForWindowExposed(&bars));
    view->resize(400, 300);
    QImage seed(1000, 800, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view->setImage(seed);
    view->setZoom(1.0, QPointF(view->width() / 2.0, view->height() / 2.0));
    QCoreApplication::processEvents();

    QScrollBar* hbar = bars.horizontalBarForTest();
    QScrollBar* vbar = bars.verticalBarForTest();
    QVERIFY2(hbar->maximum() > 0 && vbar->maximum() > 0,
             "the bars offer a travel range");

    // Raising the handle lowers the offset: the content travels against it.
    hbar->setValue(hbar->minimum());
    const double x0 = view->offset().x();
    hbar->setValue(hbar->minimum() + 20);
    QVERIFY2(view->offset().x() < x0, "the horizontal handle moves against the content");

    vbar->setValue(vbar->minimum());
    const double y0 = view->offset().y();
    vbar->setValue(vbar->minimum() + 20);
    QVERIFY2(view->offset().y() < y0, "the vertical handle moves against the content");
}

void CanvasPanTest::freePanCommandAppliesToCanvas()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_free_pan_command"));
    QVERIFY2(f.ok(), "free-pan command fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    QAction* action =
        frame.registry()->action(QString::fromLatin1(pictura::command_ids::ViewFreePan));
    QVERIFY2(action != nullptr, "Free Pan command exists");
    QVERIFY2(action->isCheckable(), "Free Pan is checkable");
    QVERIFY2(action->isChecked(), "Free Pan defaults on");
    QVERIFY(f.canvas->freePan());

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY2(!action->isChecked(), "toggled off");
    QVERIFY(!f.canvas->freePan());

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY(action->isChecked());
    QVERIFY(f.canvas->freePan());
}

void CanvasPanTest::offsetRangeSnapsToWorkspace()
{
    const QSizeF viewport(400.0, 300.0);
    const auto close = [](double a, double b) { return std::abs(a - b) < 1e-9; };
    const auto range = [&viewport](const QSizeF& image, bool snap) {
        return pictura::offsetRangeFor(image, 1.0, viewport, true,
                                       pictura::kCanvasRevealMarginPx, snap);
    };

    // A document that fits both axes collapses to the centre even with free pan.
    const pictura::OffsetRange fit = range(QSizeF(100.0, 80.0), true);
    QVERIFY(close(fit.minX, fit.maxX));
    QVERIFY(close(fit.minX, (400.0 - 100.0) / 2.0));
    QVERIFY(close(fit.minY, fit.maxY));
    QVERIFY(close(fit.minY, (300.0 - 80.0) / 2.0));

    // A document that overflows one axis is the free-pan result, byte-for-byte.
    const pictura::OffsetRange overflowSnap = range(QSizeF(100.0, 900.0), true);
    const pictura::OffsetRange overflowFree = range(QSizeF(100.0, 900.0), false);
    QVERIFY(close(overflowSnap.minX, overflowFree.minX));
    QVERIFY(close(overflowSnap.maxX, overflowFree.maxX));
    QVERIFY(close(overflowSnap.minY, overflowFree.minY));
    QVERIFY(close(overflowSnap.maxY, overflowFree.maxY));

    // An empty document is unaffected: no centring offset is produced.
    const pictura::OffsetRange empty = range(QSizeF(0.0, 0.0), true);
    QVERIFY(close(empty.minX, 0.0));
    QVERIFY(close(empty.maxX, 400.0));
}

void CanvasPanTest::workspaceSnapPinsFittingImage()
{
    pictura::ImageView view;
    view.resize(400, 300);
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view.setImage(seed);
    QCoreApplication::processEvents();

    const double centreX = (400.0 - 100.0) / 2.0;
    const double centreY = (300.0 - 80.0) / 2.0;

    view.setFreePan(true);
    view.panBy(QPointF(120, 90));
    QVERIFY2(std::abs(view.offset().x() - centreX) > 1.0,
             "free pan moves a fitting image off centre");

    view.setSnapToWorkspace(true);
    QVERIFY(view.workspaceSnapLocked());
    QVERIFY2(std::abs(view.offset().x() - centreX) < 1e-9
                 && std::abs(view.offset().y() - centreY) < 1e-9,
             "the lock re-centres the fitting image");

    // A middle-button drag cannot move the pinned canvas.
    const QPointF centre(view.width() / 2.0, view.height() / 2.0);
    sendMouse(&view, QEvent::MouseButtonPress, centre, Qt::MiddleButton, Qt::MiddleButton);
    sendMouse(&view, QEvent::MouseMove, centre + QPointF(60.0, 40.0), Qt::NoButton,
              Qt::MiddleButton);
    sendMouse(&view, QEvent::MouseButtonRelease, centre + QPointF(60.0, 40.0), Qt::MiddleButton,
              Qt::NoButton);
    QVERIFY2(std::abs(view.offset().x() - centreX) < 1e-9
                 && std::abs(view.offset().y() - centreY) < 1e-9,
             "a locked canvas cannot be panned");

    view.setSnapToWorkspace(false);
    QVERIFY(!view.workspaceSnapLocked());
    view.panBy(QPointF(120, 90));
    QVERIFY2(std::abs(view.offset().x() - centreX) > 1.0,
             "clearing the lock allows the pan again");
}

void CanvasPanTest::workspaceSnapHidesScrollbars()
{
    pictura::CanvasScrollBars bars;
    auto* view = new pictura::ImageView(&bars);
    bars.setView(view);
    bars.resize(600, 500);
    bars.show();
    QVERIFY(QTest::qWaitForWindowExposed(&bars));
    view->resize(400, 300);
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    view->setImage(seed);
    QCoreApplication::processEvents();

    view->setSnapToWorkspace(true);
    QVERIFY(view->workspaceSnapLocked());
    QVERIFY2(!bars.horizontalBarForTest()->isVisible(), "horizontal bar hides under the lock");
    QVERIFY2(!bars.verticalBarForTest()->isVisible(), "vertical bar hides under the lock");

    // Zoom past the workspace: the lock releases and the bars return.
    view->setZoom(12.0, QPointF(view->width() / 2.0, view->height() / 2.0));
    QVERIFY(!view->workspaceSnapLocked());
    QVERIFY2(bars.horizontalBarForTest()->isVisible(),
             "horizontal bar returns past the workspace");
    QVERIFY2(bars.verticalBarForTest()->isVisible(), "vertical bar returns past the workspace");

    view->setSnapToWorkspace(false);
}

void CanvasPanTest::workspaceSnapCommandAppliesToCanvas()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 80, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_ws_snap_command"));
    QVERIFY2(f.ok(), "workspace-snap command fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    QAction* action = frame.registry()->action(
        QString::fromLatin1(pictura::command_ids::ViewSnapCanvasToWorkspace));
    QVERIFY2(action != nullptr, "Snap Canvas To Workspace command exists");
    QVERIFY2(action->isCheckable(), "Snap Canvas To Workspace is checkable");
    QVERIFY2(!action->isChecked(), "Snap Canvas To Workspace defaults off");
    QVERIFY(!f.canvas->snapToWorkspace());

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY2(action->isChecked(), "toggled on");
    QVERIFY(f.canvas->snapToWorkspace());

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY(!action->isChecked());
    QVERIFY(!f.canvas->snapToWorkspace());
}

void CanvasPanTest::spacePanOverrideHolds()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(160, 160, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_space_override"));
    QVERIFY2(f.ok(), "space-override fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    frame.setActiveTool(pictura::ToolId::Brush);
    QCoreApplication::processEvents();

    const auto sendSpace = [&frame](QEvent::Type type, bool autorep) {
        QKeyEvent event(type, Qt::Key_Space, Qt::NoModifier, QString(), autorep);
        QApplication::sendEvent(&frame, &event);
    };

    // Space held: the hand cursor replaces the Brush's blank cursor.
    sendSpace(QEvent::KeyPress, false);
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);

    // A tool switch runs applyToolPolicy (setPanEnabled + refreshCursor) and
    // must not clobber the override.
    frame.setActiveTool(pictura::ToolId::Move);
    QCoreApplication::processEvents();
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);
    QVERIFY(f.canvas->panEnabled());

    // An auto-repeat release is not a physical release.
    sendSpace(QEvent::KeyRelease, true);
    QVERIFY(f.canvas->spacePanForTest());
    QVERIFY(f.canvas->panEnabled());

    // Press/drag pans the canvas with no tool edit.
    const QPointF centre(f.canvas->width() / 2.0, f.canvas->height() / 2.0);
    const QPointF base = f.canvas->offset();
    const int history = f.view->history_count();
    sendMouse(f.canvas, QEvent::MouseButtonPress, centre, Qt::LeftButton, Qt::LeftButton);
    sendMouse(f.canvas, QEvent::MouseMove, centre + QPointF(40.0, 30.0), Qt::NoButton,
              Qt::LeftButton);
    sendMouse(f.canvas, QEvent::MouseButtonRelease, centre + QPointF(40.0, 30.0), Qt::LeftButton,
              Qt::NoButton);
    QVERIFY2(f.canvas->offset() != base, "the Space drag pans the canvas");
    QCOMPARE(f.view->history_count(), history);

    // Release Space: the override ends and the tool's pan state is restored.
    sendSpace(QEvent::KeyRelease, false);
    QVERIFY(!f.canvas->spacePanForTest());
    QVERIFY(!f.canvas->panEnabled());
}

void CanvasPanTest::spacePanHoverAndMiddleButton()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(160, 160, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(frame, seed, QStringLiteral("pictura_canvas_space_middle"));
    QVERIFY2(f.ok(), "space middle-button fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    frame.setActiveTool(pictura::ToolId::Brush);
    QCoreApplication::processEvents();

    const auto sendSpace = [&frame](QEvent::Type type, bool autorep) {
        QKeyEvent event(type, Qt::Key_Space, Qt::NoModifier, QString(), autorep);
        QApplication::sendEvent(&frame, &event);
    };
    const QPointF centre(f.canvas->width() / 2.0, f.canvas->height() / 2.0);

    // Space held: hand cursor. A hover with no button must not clobber it.
    sendSpace(QEvent::KeyPress, false);
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);
    sendMouse(f.canvas, QEvent::MouseMove, centre + QPointF(20.0, 10.0), Qt::NoButton,
              Qt::NoButton);
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);

    // Space + middle-button pan: closed hand while dragging, hand on release
    // (not the default arrow).
    sendMouse(f.canvas, QEvent::MouseButtonPress, centre, Qt::MiddleButton, Qt::MiddleButton);
    QCOMPARE(f.canvas->cursor().shape(), Qt::ClosedHandCursor);
    sendMouse(f.canvas, QEvent::MouseMove, centre + QPointF(30.0, 20.0), Qt::NoButton,
              Qt::MiddleButton);
    sendMouse(f.canvas, QEvent::MouseButtonRelease, centre + QPointF(30.0, 20.0),
              Qt::MiddleButton, Qt::NoButton);
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);

    // Releasing Space restores the Brush's blank cursor.
    sendSpace(QEvent::KeyRelease, false);
    QVERIFY(!f.canvas->spacePanForTest());
    QCOMPARE(f.canvas->cursor().shape(), Qt::BlankCursor);

    // Space pressed mid middle-drag keeps the closed hand.
    sendMouse(f.canvas, QEvent::MouseButtonPress, centre, Qt::MiddleButton, Qt::MiddleButton);
    QCOMPARE(f.canvas->cursor().shape(), Qt::ClosedHandCursor);
    sendSpace(QEvent::KeyPress, false);
    QCOMPARE(f.canvas->cursor().shape(), Qt::ClosedHandCursor);
    sendMouse(f.canvas, QEvent::MouseButtonRelease, centre, Qt::MiddleButton, Qt::NoButton);
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);
    sendSpace(QEvent::KeyRelease, false);
    QVERIFY(!f.canvas->spacePanForTest());
}

QTEST_MAIN(CanvasPanTest)
#include "tst_canvas_pan.moc"
