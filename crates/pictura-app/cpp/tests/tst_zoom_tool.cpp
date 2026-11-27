// The Zoom tool (#120/#121): click steps, Alt-click steps out, a
// press-drag-release marquee fills the viewport, and zoom-out anchors at the
// canvas centre while zoom-in stays cursor-anchored.

#include <QtTest/QtTest>
#include <QtCore/QTimer>
#include <QtGui/QWheelEvent>
#include <cmath>
#include "preferences.h"
#include "preferences_dialog.h"
#include <QtWidgets/QCheckBox>

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
    void scrubDirection();
    void retainedWheel();
    void retainedClamp();
    void scrubLimitsAndCancel();
    void scrubMultiwindow();
    void scrubLeavesHistory();
    void animatedHold();
    void animatedFallback();
    void animatedInterrupt();
    void animatedDrag();
    void rotatedAnchor();

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
    auto* zoom = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = zoom->get();
    options.scrubby = false;
    zoom->set(options);
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

void ZoomToolTest::scrubDirection()
{
    auto& frame = *window_;
    QImage seed(400, 300, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    Fixture f(frame, seed, QStringLiteral("pictura_zoom_scrub"));
    QVERIFY(f.ok());
    frame.setActiveTool(pictura::ToolId::Zoom);
    auto* zoom = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = zoom->get();
    options.scrubby = true;
    zoom->set(options);
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    f.canvas->setZoom(1.0, viewportCentre(*f.canvas));
    const QPointF widget = viewportCentre(*f.canvas);
    const QPointF image = f.canvas->widgetToImage(widget);
    f.canvas->mousePressed(image, Qt::LeftButton, 0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(widget + QPointF(40, 0)));
    const double increased = f.canvas->zoom();
    QVERIFY(increased > 1.0);
    QVERIFY(distance(f.canvas->imageToWidget(image), widget) < 1.5);
    f.canvas->mouseMoved(f.canvas->widgetToImage(widget + QPointF(10, 0)));
    QVERIFY(f.canvas->zoom() < increased);
    f.canvas->mouseReleased(f.canvas->widgetToImage(widget + QPointF(10, 0)));
}

namespace {
void wheel(pictura::ImageView* canvas, QPoint angle, Qt::KeyboardModifiers modifiers)
{
    QPointF pos(canvas->width() / 2.0, canvas->height() / 2.0);
    QWheelEvent event(pos, canvas->mapToGlobal(pos.toPoint()), QPoint(), angle, Qt::NoButton,
                      modifiers, Qt::NoScrollPhase, false);
    QApplication::sendEvent(canvas, &event);
}
void setAnimated(bool enabled)
{
    const auto before = pictura::loadPreferences();
    auto after = before;
    after.animatedZoom = enabled;
    pictura::savePreferences(before, after);
}
}

void ZoomToolTest::retainedWheel()
{
    QImage seed(2000, 2000, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_wheel")); QVERIFY(f.ok());
    window_->show(); QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    const QPointF centre = viewportCentre(*f.canvas);
    f.canvas->setZoom(1.0, centre); wheel(f.canvas, QPoint(0, 120), Qt::NoModifier);
    const double plain = f.canvas->zoom();
    f.canvas->setZoom(1.0, centre); wheel(f.canvas, QPoint(0, 120), Qt::ShiftModifier);
    QVERIFY(qAbs(std::log(f.canvas->zoom()) - 2.0 * std::log(plain)) < 1e-6);
    const auto checkPan = [&](QPoint angle, Qt::KeyboardModifiers mods, bool horizontal) {
        f.canvas->setZoom(1.0, centre); f.canvas->setOffset(QPointF(-500, -500));
        const QPointF before = f.canvas->offset(); const double zoom = f.canvas->zoom();
        wheel(f.canvas, angle, mods);
        QCOMPARE(f.canvas->zoom(), zoom);
        if (horizontal) { QVERIFY(f.canvas->offset().x() != before.x()); QCOMPARE(f.canvas->offset().y(), before.y()); }
        else { QVERIFY(f.canvas->offset().y() != before.y()); QCOMPARE(f.canvas->offset().x(), before.x()); }
    };
    checkPan(QPoint(0, 120), Qt::AltModifier, true);
    checkPan(QPoint(0, 120), Qt::AltModifier | Qt::ControlModifier, false);
    checkPan(QPoint(120, 0), Qt::NoModifier, true);
}

void ZoomToolTest::retainedClamp()
{
    QImage seed(2000, 2000, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_clamp")); QVERIFY(f.ok());
    window_->show(); QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    f.canvas->setZoom(1.0, viewportCentre(*f.canvas));
    for (const QPointF delta : {QPointF(1e6, 0), QPointF(-1e6, 0), QPointF(0, 1e6), QPointF(0, -1e6)}) {
        f.canvas->panBy(delta);
        const QRectF image(f.canvas->offset(), QSizeF(seed.size()) * f.canvas->zoom());
        const QRectF visible = image.intersected(QRectF(QPointF(), QSizeF(f.canvas->size())));
        QVERIFY(visible.width() >= 95.0); QVERIFY(visible.height() >= 95.0);
    }
    f.canvas->setZoom(0.5, QPointF(0, 0));
    QVERIFY(QRectF(f.canvas->offset(), QSizeF(seed.size()) * f.canvas->zoom()).intersects(QRectF(QPointF(), QSizeF(f.canvas->size()))));
    f.canvas->fitOnScreen();
    QVERIFY(QRectF(f.canvas->offset(), QSizeF(seed.size()) * f.canvas->zoom()).intersects(QRectF(QPointF(), QSizeF(f.canvas->size()))));
}

void ZoomToolTest::scrubLimitsAndCancel()
{
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_limits")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    auto* group = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = group->get(); options.scrubby = true; group->set(options);
    const QPointF centre = viewportCentre(*f.canvas);
    f.canvas->setZoom(32.0, centre);
    QPointF start = f.canvas->widgetToImage(centre);
    f.canvas->mousePressed(start, Qt::LeftButton, 0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(50, 0)));
    QCOMPARE(f.canvas->zoom(), 32.0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(40, 0)));
    QVERIFY(f.canvas->zoom() < 32.0);
    f.tools->onSessionCancelled(); const double stopped = f.canvas->zoom();
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre)); QCOMPARE(f.canvas->zoom(), stopped);
    f.canvas->setZoom(0.000001, centre); const double minimum = f.canvas->zoom();
    start = f.canvas->widgetToImage(centre);
    f.canvas->mousePressed(start, Qt::LeftButton, 0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre - QPointF(50, 0)));
    QCOMPARE(f.canvas->zoom(), minimum);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre - QPointF(40, 0)));
    QVERIFY(f.canvas->zoom() > minimum);
    window_->setActiveTool(pictura::ToolId::Hand);
    const double deactivated = f.canvas->zoom();
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre));
    QCOMPARE(f.canvas->zoom(), deactivated);
}

void ZoomToolTest::scrubMultiwindow()
{
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_fanout")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    auto* group = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = group->get(); options.scrubby = true; options.zoomAll = true; options.resizeToFit = true; group->set(options);
    QSignalSpy spy(f.tools, &pictura::ToolController::toolZoomed);
    const QPointF centre = viewportCentre(*f.canvas); f.canvas->setZoom(1.0, centre);
    auto drag = [&](Qt::KeyboardModifiers mods) {
        f.canvas->mousePressed(f.canvas->widgetToImage(centre), Qt::LeftButton, int(mods));
        f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(30, 0)));
        f.canvas->mouseReleased(f.canvas->widgetToImage(centre + QPointF(30, 0)));
    };
    drag(Qt::NoModifier); QVERIFY(!spy.isEmpty()); QVERIFY(spy.last().at(1).toBool()); QVERIFY(spy.last().at(2).toBool());
    options.zoomAll = false; group->set(options); spy.clear(); drag(Qt::ShiftModifier);
    QVERIFY(!spy.isEmpty()); QVERIFY(spy.last().at(1).toBool());
    spy.clear(); drag(Qt::NoModifier); QVERIFY(!spy.isEmpty()); QVERIFY(!spy.last().at(1).toBool());
}

void ZoomToolTest::scrubLeavesHistory()
{
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_history")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    auto* group = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = group->get(); options.scrubby = true; group->set(options);
    const auto image = f.canvas->image(); const int history = f.view->history_index();
    const QPointF centre = viewportCentre(*f.canvas);
    f.canvas->mousePressed(f.canvas->widgetToImage(centre), Qt::LeftButton, 0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(40, 0)));
    f.canvas->mouseReleased(f.canvas->widgetToImage(centre + QPointF(40, 0)));
    QCOMPARE(f.view->history_index(), history); QCOMPARE(f.canvas->image(), image);
}

void ZoomToolTest::animatedHold()
{
    setAnimated(true);
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_hold")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    const QPointF centre = viewportCentre(*f.canvas); f.canvas->setZoom(1.0, centre);
    const QPointF widget = centre + QPointF(60, 30);
    const QPointF point = f.canvas->widgetToImage(widget);
    const auto image = f.canvas->image(); const int history = f.view->history_index();
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    auto* timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    const double first = f.canvas->zoom(); QVERIFY(first > 1.0);
    QVERIFY(distance(f.canvas->imageToWidget(point), widget) < 1.5);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QVERIFY(f.canvas->zoom() > first);
    const double last = f.canvas->zoom(); f.canvas->mouseReleased(point); QCOMPARE(f.canvas->zoom(), last);
    QCOMPARE(f.canvas->image(), image); QCOMPARE(f.view->history_index(), history);
    const QPointF centrePoint = f.canvas->widgetToImage(centre);
    f.canvas->mousePressed(point, Qt::LeftButton, int(Qt::AltModifier));
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    QVERIFY(f.canvas->zoom() < last);
    QVERIFY(distance(f.canvas->imageToWidget(centrePoint), centre) < 1.5);
    const double out = f.canvas->zoom(); f.canvas->mouseReleased(point); QCOMPARE(f.canvas->zoom(), out);
    f.canvas->mousePressed(point, Qt::LeftButton, 0); f.canvas->mouseReleased(point);
    QVERIFY(qAbs(f.canvas->zoom() - out * 1.2) < 1e-6);
    f.canvas->setZoom(32.0, centre);
    f.canvas->mousePressed(f.canvas->widgetToImage(centre), Qt::LeftButton, 0);
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    QCOMPARE(f.canvas->zoom(), 32.0); QVERIFY(!f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")));
    f.canvas->mouseReleased(f.canvas->widgetToImage(centre));
    setAnimated(false);
}

void ZoomToolTest::animatedFallback()
{
    setAnimated(true); QCoreApplication::instance()->setProperty("picturaTimedZoomUnavailable", true);
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    pictura::PreferencesDialog prefs;
    auto* checkbox = prefs.findChild<QCheckBox*>(QStringLiteral("preferences_animatedZoom"));
    QVERIFY(checkbox); QVERIFY(!checkbox->isEnabled()); QVERIFY(!checkbox->toolTip().isEmpty());
    Fixture f(*window_, seed, QStringLiteral("zoom_fallback")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    const QPointF centre = viewportCentre(*f.canvas); f.canvas->setZoom(1.0, centre);
    const QPointF point = f.canvas->widgetToImage(centre);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    QVERIFY(!f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")));
    f.canvas->mouseReleased(point); QVERIFY(qAbs(f.canvas->zoom() - 1.2) < 1e-6);
    QCoreApplication::instance()->setProperty("picturaTimedZoomUnavailable", false);
    setAnimated(false);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    QVERIFY(!f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")));
    f.canvas->mouseReleased(point); QVERIFY(qAbs(f.canvas->zoom() - 1.44) < 1e-6);
    pictura::PreferencesDialog enabledPrefs;
    auto* enabled = enabledPrefs.findChild<QCheckBox*>(QStringLiteral("preferences_animatedZoom"));
    QVERIFY(enabled); enabled->setChecked(true);
    pictura::PreferencesDialog reopened;
    QVERIFY(reopened.findChild<QCheckBox*>(QStringLiteral("preferences_animatedZoom"))->isChecked());
    setAnimated(false);
}

void ZoomToolTest::animatedInterrupt()
{
    setAnimated(true);
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_interrupt")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    const QPointF point = f.canvas->widgetToImage(viewportCentre(*f.canvas));
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    auto* timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    f.tools->onSessionCancelled(); const double stopped = f.canvas->zoom();
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QCOMPARE(f.canvas->zoom(), stopped);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    window_->setActiveTool(pictura::ToolId::Hand);
    const double switched = f.canvas->zoom();
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QCOMPARE(f.canvas->zoom(), switched);
    window_->setActiveTool(pictura::ToolId::Zoom);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    f.canvas->panBy(QPointF(10, 0)); const double panned = f.canvas->zoom();
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QCOMPARE(f.canvas->zoom(), panned);
    f.canvas->mouseReleased(point);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    f.canvas->zoomIn(); const double externallyZoomed = f.canvas->zoom();
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    QCOMPARE(f.canvas->zoom(), externallyZoomed); f.canvas->mouseReleased(point);
    setAnimated(false);
}

void ZoomToolTest::animatedDrag()
{
    setAnimated(true);
    QImage seed(400, 300, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_drag_hold")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    auto* group = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = group->get(); options.scrubby = true; group->set(options);
    const QPointF centre = viewportCentre(*f.canvas); f.canvas->setZoom(1.0, centre);
    f.canvas->mousePressed(f.canvas->widgetToImage(centre), Qt::LeftButton, 0);
    auto* timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(40, 0)));
    const double scrubbed = f.canvas->zoom(); QVERIFY(scrubbed > 1.0);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QCOMPARE(f.canvas->zoom(), scrubbed);
    f.canvas->mouseReleased(f.canvas->widgetToImage(centre + QPointF(40, 0)));
    QCOMPARE(f.canvas->zoom(), scrubbed);
    options.scrubby = false; group->set(options);
    f.canvas->setZoom(1.0, centre);
    f.canvas->mousePressed(f.canvas->widgetToImage(centre), Qt::LeftButton, 0);
    timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(40, 40)));
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection); QCOMPARE(f.canvas->zoom(), 1.0);
    f.canvas->mouseReleased(f.canvas->widgetToImage(centre + QPointF(40, 40)));
    QVERIFY(f.canvas->zoom() > 1.0); setAnimated(false);
}

void ZoomToolTest::rotatedAnchor()
{
    setAnimated(true);
    QImage seed(2000, 2000, QImage::Format_RGB32); seed.fill(Qt::blue);
    Fixture f(*window_, seed, QStringLiteral("zoom_rotated")); QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Zoom);
    auto* group = f.tools->optionsFor<pictura::ZoomGroup>(pictura::ToolId::Zoom);
    auto options = group->get(); options.scrubby = true; group->set(options);
    const QPointF centre = viewportCentre(*f.canvas);
    f.canvas->setZoom(1.0, centre); f.canvas->setRotation(45.0);
    const QPointF widget = centre + QPointF(70, 30);
    QPointF point = f.canvas->widgetToImage(widget);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    f.canvas->mouseMoved(f.canvas->widgetToImage(widget + QPointF(30, 0)));
    QVERIFY(distance(f.canvas->imageToWidget(point), widget) < 1.5);
    f.canvas->mouseReleased(f.canvas->widgetToImage(widget + QPointF(30, 0)));
    point = f.canvas->widgetToImage(widget);
    f.canvas->mousePressed(point, Qt::LeftButton, 0);
    auto* timer = f.canvas->findChild<QTimer*>(QStringLiteral("zoomHoldTimer")); QVERIFY(timer);
    QMetaObject::invokeMethod(timer, "timeout", Qt::DirectConnection);
    QVERIFY(distance(f.canvas->imageToWidget(point), widget) < 1.5);
    f.canvas->mouseReleased(point); setAnimated(false);
}

PICTURA_QT_TEST_MAIN(ZoomToolTest)
#include "tst_zoom_tool.moc"
