#include <QtTest/QtTest>

#include "api/view_bridge.h"
#include "canvas_range.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "preferences.h"
#include "preferences_dialog.h"
#include "qt_test_support.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include <QtGui/QContextMenuEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QProxyStyle>
#include <QtWidgets/QPushButton>

using namespace pictura;

namespace {

QPointF centre(const ImageView& canvas)
{
    return QPointF(canvas.width() / 2.0, canvas.height() / 2.0);
}

void pointer(ImageView& canvas, QEvent::Type type, const QPointF& pos, ulong timestamp,
             Qt::MouseButton button = Qt::LeftButton,
             Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    const auto change = type == QEvent::MouseMove ? Qt::NoButton : button;
    const Qt::MouseButtons held = type == QEvent::MouseButtonRelease ? Qt::NoButton : button;
    QMouseEvent event(type, pos, QPointF(canvas.mapToGlobal(pos.toPoint())), change, held, mods);
    event.setTimestamp(timestamp);
    QCoreApplication::sendEvent(&canvas, &event);
}

void drag(ImageView& canvas, const QPointF& delta, ulong releaseTime = 140,
          Qt::MouseButton button = Qt::LeftButton, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    const QPointF from = centre(canvas);
    pointer(canvas, QEvent::MouseButtonPress, from, 100, button, mods);
    pointer(canvas, QEvent::MouseMove, from + delta / 2.0, 118, button, mods);
    pointer(canvas, QEvent::MouseMove, from + delta, 136, button, mods);
    pointer(canvas, QEvent::MouseButtonRelease, from + delta, releaseTime, button, mods);
}

void zoomed(ImageView& canvas)
{
    canvas.setZoom(2.0, centre(canvas));
    canvas.setOffset(centre(canvas) - QPointF(1280, 800));
}

std::unique_ptr<PicturaMainWindow> visibleFrame()
{
    auto frame = pictura::test::makeMainWindow();
    frame->resize(1200, 800);
    frame->show();
    frame->activateWindow();
    return frame;
}

void focusCanvas(PicturaMainWindow& frame)
{
    frame.imageView()->setFocus();
    QCoreApplication::processEvents();
}

class ReducedMotionStyle final : public QProxyStyle {
    int styleHint(StyleHint hint, const QStyleOption* option, const QWidget* widget,
                  QStyleHintReturn* result) const override
    {
        return hint == SH_Widget_Animation_Duration ? 0
            : QProxyStyle::styleHint(hint, option, widget, result);
    }
};

ImageView* prepareCanvas(ImageView& view)
{
    view.resize(480, 360);
    QImage image(1280, 800, QImage::Format_RGB32);
    image.fill(Qt::darkGray);
    view.setImage(image);
    view.show();
    zoomed(view);
    return &view;
}

QRect cyanOutline(const QImage& shot)
{
    QRect bounds;
    for (int y = 0; y < shot.height(); ++y) {
        for (int x = 0; x < shot.width(); ++x) {
            if (shot.pixelColor(x, y) == QColor(110, 220, 245)) {
                bounds |= QRect(x, y, 1, 1);
            }
        }
    }
    return bounds;
}

bool newDoc(PicturaMainWindow& frame)
{
    return frame.newDocument(QStringLiteral("Hand navigation"), 1280, 800,
                             QStringLiteral("rgb"), 8, QStringLiteral("white"));
}

} // namespace

class HandNavigationTest : public QObject {
    Q_OBJECT

private slots:
    void birdRelocates();
    void birdCancels();
    void ordinaryPan();
    void birdShortcuts();
    void birdFallbackAndCapture();
    void birdPreviewOverlays();
    void flickContinues();
    void flickGates();
    void flickOverride();
    void preferencePersists();
    void flickPeers();
};

void HandNavigationTest::birdRelocates()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto frame = visibleFrame();
    QImage image(1280, 800, QImage::Format_RGB32);
    image.fill(Qt::red);
    {
        QPainter painter(&image);
        painter.fillRect(QRect(640, 0, 640, 800), Qt::blue);
    }
    paint_fixture::Fixture fixture(*frame, image, QStringLiteral("pictura_birds_eye"));
    QVERIFY(fixture.ok());
    ImageView& canvas = *fixture.canvas;
    focusCanvas(*frame);
    QTRY_VERIFY(canvas.hasFocus());
    zoomed(canvas);
    canvas.setRotation(90);
    const QPointF middle = centre(canvas);
    const QPointF offset = canvas.offset();
    const int history = fixture.view->history_index();
    QSignalSpy presses(&canvas, &ImageView::mousePressed);
    QSignalSpy releases(&canvas, &ImageView::mouseReleased);
    QSignalSpy changes(&canvas, &ImageView::viewChanged);
    const ToolId tool = frame->activeTool();
    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 100);

    const double fit = 0.95 * std::min(canvas.width() / 800.0, canvas.height() / 1280.0);
    const auto overviewIsComplete = [&] {
        const QImage shot = canvas.grab().toImage();
        const auto sample = [&](double dy) {
            const QPointF pt = middle + QPointF(0, dy);
            return shot.pixelColor((pt * shot.devicePixelRatio()).toPoint());
        };
        return sample(-500 * fit) == QColor(Qt::red) && sample(500 * fit) == QColor(Qt::blue);
    };
    QTRY_VERIFY(overviewIsComplete());
    const QRect initial = cyanOutline(canvas.grab().toImage());
    QVERIFY(!initial.isEmpty());
    QVERIFY(qAbs(initial.width() - canvas.width() * fit / 2.0
                 * canvas.devicePixelRatioF()) < 5.0);
    const QPointF target = middle + QPointF(0, 120.0 * fit);
    pointer(canvas, QEvent::MouseMove, target, 120);
    const QRect moved = cyanOutline(canvas.grab().toImage());
    QVERIFY(moved.center().y() > initial.center().y());
    QCOMPARE(canvas.zoom(), 2.0);
    QCOMPARE(canvas.offset(), offset);
    QCOMPARE(changes.count(), 0);
    pointer(canvas, QEvent::MouseButtonRelease, target, 135);
    QTest::keyRelease(&canvas, Qt::Key_H);
    QCOMPARE(canvas.zoom(), 2.0);
    QCOMPARE(canvas.rotation(), 90.0);
    QVERIFY(QLineF(canvas.widgetToImage(middle), QPointF(760, 400)).length() < 0.1);
    QCOMPARE(frame->activeTool(), tool);
    QCOMPARE(fixture.view->history_index(), history);
    QCOMPARE(presses.count(), 0);
    QCOMPARE(releases.count(), 0);
}

void HandNavigationTest::birdCancels()
{
    ImageView view;
    prepareCanvas(view);
    QSignalSpy presses(&view, &ImageView::mousePressed);
    QSignalSpy releases(&view, &ImageView::mouseReleased);
    const QPointF start = centre(view);
    const QPointF before = view.offset();
    for (int reason = 0; reason != 4; ++reason) {
        QVERIFY(view.armBirdsEye());
        pointer(view, QEvent::MouseButtonPress, start, 100);
        pointer(view, QEvent::MouseMove, start + QPointF(45, 0), 120);
        QVERIFY(!cyanOutline(view.grab().toImage()).isEmpty());
        switch (reason) {
        case 0:
            QTest::keyClick(&view, Qt::Key_Escape);
            break;
        case 1:
            QVERIFY(!view.releaseBirdsEyeKey());
            break;
        case 2: {
            QFocusEvent focusOut(QEvent::FocusOut);
            QCoreApplication::sendEvent(&view, &focusOut);
            break;
        }
        default: {
            QEvent interrupted(QEvent::UngrabMouse);
            QCoreApplication::sendEvent(&view, &interrupted);
            break;
        }
        }
        QVERIFY(cyanOutline(view.grab().toImage()).isEmpty());
        pointer(view, QEvent::MouseButtonRelease, start + QPointF(45, 0), 138);
        QCOMPARE(view.offset(), before);
        QCOMPARE(view.zoom(), 2.0);
    }
    QCOMPARE(presses.count(), 0);
    QCOMPARE(releases.count(), 0);
}

void HandNavigationTest::ordinaryPan()
{
    ImageView view;
    prepareCanvas(view);
    QVERIFY(view.armBirdsEye());
    QVERIFY(view.releaseBirdsEyeKey());
    for (const auto button : {Qt::LeftButton, Qt::MiddleButton, Qt::LeftButton}) {
        const QPointF before = view.offset();
        drag(view, QPointF(20, 0), 150, button);
        QCOMPARE(view.offset(), before + QPointF(20, 0));
        QCOMPARE(view.zoom(), 2.0);
        QVERIFY(cyanOutline(view.grab().toImage()).isEmpty());
        if (button == Qt::MiddleButton) {
            view.setPanEnabled(false);
            view.setSpacePan(true);
        }
    }
}

void HandNavigationTest::birdShortcuts()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto frame = visibleFrame();
        QVERIFY(newDoc(*frame));
        focusCanvas(*frame);
        auto& canvas = *frame->imageView();
        QTRY_VERIFY(canvas.hasFocus());
        zoomed(canvas);
        auto* tools = frame->findChild<ToolController*>();
        QVERIFY(tools);
        frame->setActiveTool(ToolId::Crop);
        auto* crop = tools->optionsFor<CropOptions>(ToolId::Crop);
        QVERIFY(crop);
        crop->setClassicMode(false);
        const bool previous = crop->settings().showCroppedArea;
        QTest::keyClick(&canvas, Qt::Key_H);
        QCOMPARE(frame->activeTool(), ToolId::Crop);
        QCOMPARE(crop->settings().showCroppedArea, !previous);
        frame->setActiveTool(ToolId::Move);

        QLineEdit input(frame.get());
        input.show();
        input.setFocus();
        QTest::keyClick(&input, Qt::Key_H);
        QCOMPARE(input.text(), QStringLiteral("h"));
        QCOMPARE(frame->activeTool(), ToolId::Move);
        input.hide();

        QPushButton panelButton(QStringLiteral("Panel"), frame.get());
        panelButton.show();
        panelButton.setFocus();
        QTRY_VERIFY(panelButton.hasFocus());
        QTest::keyPress(&panelButton, Qt::Key_H);
        focusCanvas(*frame);
        const QPointF middle = centre(canvas);
        pointer(canvas, QEvent::MouseButtonPress, middle, 100);
        QKeyEvent repeat(QEvent::KeyPress, Qt::Key_H, Qt::NoModifier, QStringLiteral("h"), true);
        QCoreApplication::sendEvent(&canvas, &repeat);
        QVERIFY(!cyanOutline(canvas.grab().toImage()).isEmpty());
        pointer(canvas, QEvent::MouseButtonRelease, middle, 120);
        QTest::keyRelease(&canvas, Qt::Key_H);
        QCOMPARE(frame->activeTool(), ToolId::Move);
        QTest::keyClick(&canvas, Qt::Key_H);
        QCOMPARE(frame->activeTool(), ToolId::Hand);
        QVERIFY(frame->closeDocument(0, false));
    }
    auto before = loadPreferences();
    auto after = before;
    after.shortcuts.insert(QLatin1String(command_ids::ViewFitOnScreen), {QStringLiteral("H")});
    for (const ToolId id : allToolIds()) {
        if (toolShortcut(id) == QLatin1Char('K')) {
            after.toolShortcuts.insert(toolIdName(id), QString());
        }
    }
    after.toolShortcuts.insert(QStringLiteral("hand"), QStringLiteral("K"));
    QVERIFY(savePreferences(before, after));
    auto frame = visibleFrame();
    QVERIFY(newDoc(*frame));
    focusCanvas(*frame);
    auto& canvas = *frame->imageView();
    QTRY_VERIFY(canvas.hasFocus());
    zoomed(canvas);
    QTest::keyPress(&canvas, Qt::Key_H);
    QVERIFY(canvas.zoom() < 2.0);
    QCOMPARE(frame->activeTool(), ToolId::Move);
    QTest::keyRelease(&canvas, Qt::Key_H);
    zoomed(canvas);
    QTest::keyPress(&canvas, Qt::Key_K);
    QCOMPARE(frame->activeTool(), ToolId::Hand);
    const QPointF middle = centre(canvas);
    pointer(canvas, QEvent::MouseButtonPress, middle, 100);
    QVERIFY(cyanOutline(canvas.grab().toImage()).isEmpty());
    pointer(canvas, QEvent::MouseButtonRelease, middle, 120);
    QTest::keyRelease(&canvas, Qt::Key_K);
    QVERIFY(frame->closeDocument(0, false));
}

void HandNavigationTest::birdFallbackAndCapture()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto frame = visibleFrame();
    QVERIFY(newDoc(*frame));
    focusCanvas(*frame);
    auto& canvas = *frame->imageView();
    QTRY_VERIFY(canvas.hasFocus());
    zoomed(canvas);
    frame->setActiveTool(ToolId::Move);
    canvas.setFlickPanning(FlickPanningMode::Off, false);
    QSignalSpy presses(&canvas, &ImageView::mousePressed);
    QSignalSpy releases(&canvas, &ImageView::mouseReleased);
    const QPointF middle = centre(canvas);
    const QPointF before = canvas.offset();
    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 100, Qt::LeftButton, Qt::ShiftModifier);
    pointer(canvas, QEvent::MouseMove, middle + QPointF(30, 0), 120, Qt::LeftButton,
            Qt::ShiftModifier);
    pointer(canvas, QEvent::MouseButtonRelease, middle + QPointF(30, 0), 130, Qt::LeftButton,
            Qt::ShiftModifier);
    QTest::keyRelease(&canvas, Qt::Key_H);
    QCOMPARE(canvas.offset(), before + QPointF(30, 0));
    QCOMPARE(frame->activeTool(), ToolId::Move);
    QCOMPARE(presses.count(), 0);
    QCOMPARE(releases.count(), 0);

    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 200);
    QVERIFY(!cyanOutline(canvas.grab().toImage()).isEmpty());
    for (const auto button : {Qt::RightButton, Qt::MiddleButton}) {
        QMouseEvent release(QEvent::MouseButtonRelease, middle,
                            QPointF(canvas.mapToGlobal(middle.toPoint())), button,
                            Qt::LeftButton, Qt::NoModifier);
        QCoreApplication::sendEvent(&canvas, &release);
    }
    QCOMPARE(releases.count(), 0);
    QVERIFY(!cyanOutline(canvas.grab().toImage()).isEmpty());
    QSignalSpy menus(&canvas, &ImageView::contextMenuRequested);
    QContextMenuEvent menu(QContextMenuEvent::Mouse, middle.toPoint());
    QCoreApplication::sendEvent(&canvas, &menu);
    QCOMPARE(menus.count(), 0);
    QFocusEvent focusOut(QEvent::FocusOut);
    QCoreApplication::sendEvent(&canvas, &focusOut);
    QSignalSpy moves(&canvas, &ImageView::mouseMoved);
    pointer(canvas, QEvent::MouseMove, middle + QPointF(5, 0), 220, Qt::NoButton);
    QCOMPARE(moves.count(), 1);
    QTest::keyRelease(&canvas, Qt::Key_H);

    // No hover or release is delivered after capture is lost.
    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 300);
    QCoreApplication::sendEvent(&canvas, &focusOut);
    QTest::keyRelease(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 320);
    pointer(canvas, QEvent::MouseButtonRelease, middle, 330);
    QCOMPARE(presses.count(), 1);
    QCOMPARE(releases.count(), 1);

    // A canceled fallback pan consumes its release and does not leak state.
    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 400, Qt::LeftButton, Qt::ShiftModifier);
    QVERIFY(canvas.isPanning());
    QCoreApplication::sendEvent(&canvas, &focusOut);
    QVERIFY(!canvas.isPanning());
    pointer(canvas, QEvent::MouseButtonRelease, middle, 410);
    QTest::keyRelease(&canvas, Qt::Key_H);
    QCOMPARE(releases.count(), 1);
    pointer(canvas, QEvent::MouseButtonPress, middle, 420);
    pointer(canvas, QEvent::MouseButtonRelease, middle, 430);
    QCOMPARE(presses.count(), 2);
    QCOMPARE(releases.count(), 2);

    QTest::keyPress(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 500, Qt::LeftButton, Qt::ShiftModifier);
    QCoreApplication::sendEvent(&canvas, &focusOut);
    QTest::keyRelease(&canvas, Qt::Key_H);
    pointer(canvas, QEvent::MouseButtonPress, middle, 520);
    pointer(canvas, QEvent::MouseButtonRelease, middle, 530);
    QCOMPARE(presses.count(), 3);
    QCOMPARE(releases.count(), 3);
    QVERIFY(frame->closeDocument(0, false));
}

void HandNavigationTest::birdPreviewOverlays()
{
    ImageView view;
    prepareCanvas(view);
    view.setCropBox(QRectF(500, 300, 100, 100));
    view.setCropContentOffset(QPointF(200, 0));
    view.setGuides({{true, 640.0}});
    view.setGuideAppearance(Qt::magenta, false);
    view.setGuidesVisible(true);
    QVERIFY(view.armBirdsEye());
    const QPointF middle = centre(view);
    pointer(view, QEvent::MouseButtonPress, middle, 100);
    const QImage shot = view.grab().toImage();
    const qreal dpr = shot.devicePixelRatio();
    // Both image edges and the centred guide must survive the crop preview.
    QCOMPARE(shot.pixelColor((middle + QPointF(-100, 20)).toPoint() * dpr), QColor(Qt::darkGray));
    QCOMPARE(shot.pixelColor((middle + QPointF(100, 20)).toPoint() * dpr), QColor(Qt::darkGray));
    bool guideFound = false;
    for (int dx = -2; dx <= 2; ++dx) {
        guideFound |= shot.pixelColor((middle + QPointF(dx, 40)).toPoint() * dpr) == QColor(Qt::magenta);
    }
    QVERIFY(guideFound);
    pointer(view, QEvent::MouseButtonRelease, middle, 120);
    view.releaseBirdsEyeKey();
}

void HandNavigationTest::flickContinues()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto frame = visibleFrame();
    QVERIFY(newDoc(*frame));
    focusCanvas(*frame);
    frame->setActiveTool(ToolId::Hand);
    auto& canvas = *frame->imageView();
    zoomed(canvas);
    canvas.setFlickPanning(FlickPanningMode::On, true);
    const int history = frame->activeView()->history_index();
    const QPointF starting = canvas.offset();
    drag(canvas, QPointF(60, 0));
    QCOMPARE(canvas.offset(), starting + QPointF(60, 0));
    QTRY_VERIFY(canvas.offset().x() > starting.x() + 60.0);
    canvas.setOffset(starting);
    QTest::qWait(70); // A canceled timer must remain quiet across several frame intervals.
    QCOMPARE(canvas.offset(), starting);

    for (int setting = 0; setting != 2; ++setting) {
        canvas.setOffset(starting);
        drag(canvas, QPointF(60, 0));
        QTRY_VERIFY(canvas.offset().x() > starting.x() + 60.0);
        if (setting == 0) canvas.setFreePan(!canvas.freePan());
        else canvas.setSnapToWorkspace(!canvas.snapToWorkspace());
        const QPointF stopped = canvas.offset();
        QTest::qWait(70); // An interrupted timer emits no signal to confirm it stays stopped.
        QCOMPARE(canvas.offset(), stopped);
    }
    canvas.setFreePan(false);
    canvas.setSnapToWorkspace(false);
    const auto range = offsetRangeFor(QSizeF(1280, 800), 2.0, QSizeF(canvas.size()));
    canvas.setOffset(QPointF(range.maxX - 64.0, starting.y()));
    drag(canvas, QPointF(60, 0));
    QTRY_COMPARE(canvas.offset().x(), range.maxX);
    QCOMPARE(frame->activeView()->history_index(), history);
    QVERIFY(frame->closeDocument(0, false));
}

void HandNavigationTest::flickGates()
{
    ReducedMotionStyle style;
    ImageView view;
    view.setStyle(&style);
    prepareCanvas(view);
    for (int gate = 0; gate != 4; ++gate) {
        zoomed(view);
        view.setFlickPanning(gate == 0 ? FlickPanningMode::Off
            : gate == 3 ? FlickPanningMode::Automatic : FlickPanningMode::On, gate != 1);
        const QPointF before = view.offset();
        drag(view, QPointF(60, 0), gate == 2 ? 300 : 140);
        QTest::qWait(70); // Ineligible release has no signal that marks a stopped timer.
        QCOMPARE(view.offset(), before + QPointF(60, 0));
    }
}

void HandNavigationTest::flickOverride()
{
    ReducedMotionStyle style;
    ImageView view;
    view.setStyle(&style);
    prepareCanvas(view);
    view.setPanEnabled(false);
    view.setSpacePan(true);
    view.setFlickPanning(FlickPanningMode::On, true);
    const QPointF before = view.offset();
    drag(view, QPointF(60, 0));
    view.setSpacePan(false);
    QTRY_VERIFY(view.offset().x() > before.x() + 60.0);
}

void HandNavigationTest::preferencePersists()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QCOMPARE(defaultPreferences().flickPanning, int(FlickPanningMode::Automatic));
    auto before = loadPreferences();
    auto after = before;
    after.gpuCompute = false;
    QVERIFY(savePreferences(before, after));
    int expected = int(FlickPanningMode::Automatic);
    for (const int choice : {1, 2, 0, -1}) {
        auto frame = visibleFrame();
        QVERIFY(frame->registry()->dispatch(QLatin1String(command_ids::EditPreferencesGeneral)));
        auto* dialog = frame->preferencesDialog();
        QVERIFY(dialog);
        auto* mode = dialog->findChild<QComboBox*>(QStringLiteral("preferencesFlickPanning"));
        QVERIFY(mode);
        QCOMPARE(mode->count(), 3);
        QCOMPARE(mode->currentIndex(), expected);
        QVERIFY(!mode->isEnabled());
        if (choice >= 0) {
            dialog->setFlickPanning(expected, true);
            mode->setCurrentIndex(choice);
            expected = choice;
        }
        QCOMPARE(loadPreferences().flickPanning, expected);
        dialog->close();
    }
}

void HandNavigationTest::flickPeers()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto frame = visibleFrame();
    QVERIFY(newDoc(*frame));
    QVERIFY(newDoc(*frame));
    QVERIFY(frame->registry()->dispatch(QStringLiteral("window.arrange.tile")));
    frame->setActiveDocumentIndex(0);
    frame->setActiveTool(ToolId::Hand);
    focusCanvas(*frame);
    auto& source = *frame->canvasAt(0);
    auto& peer = *frame->canvasAt(1);
    QTRY_VERIFY(source.isVisible() && peer.isVisible());
    auto* scrollAll = frame->findChild<QCheckBox*>(QStringLiteral("optionsHandScrollAll"));
    QVERIFY(scrollAll);
    for (int mode = 0; mode != 3; ++mode) {
        zoomed(source);
        zoomed(peer);
        source.setFlickPanning(FlickPanningMode::On, true);
        scrollAll->setChecked(mode == 0);
        const auto range = offsetRangeFor(QSizeF(1280, 800), 2.0, QSizeF(source.size()));
        source.setOffset(QPointF(range.maxX - 65.0, source.offset().y()));
        const auto peerRange = offsetRangeFor(QSizeF(1280, 800), 2.0, QSizeF(peer.size()));
        if (mode == 1) {
            peer.setOffset(QPointF(peerRange.maxX - 62.0, peer.offset().y()));
        }
        const QPointF before = peer.offset();
        drag(source, QPointF(60, 0), 140, Qt::LeftButton,
             mode == 1 ? Qt::ShiftModifier : Qt::NoModifier);
        QTRY_COMPARE(source.offset().x(), range.maxX);
        if (mode == 0) {
            QCOMPARE(peer.offset(), before + QPointF(65, 0));
        } else if (mode == 1) {
            QCOMPARE(peer.offset().x(), peerRange.maxX);
        } else {
            QCOMPARE(peer.offset(), before);
        }
    }
    QVERIFY(frame->closeDocument(1, false));
    QVERIFY(frame->closeDocument(0, false));
}

PICTURA_QT_TEST_MAIN(HandNavigationTest)
#include "tst_hand_navigation.moc"
