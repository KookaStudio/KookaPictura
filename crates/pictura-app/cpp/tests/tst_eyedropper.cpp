// The Eyedropper samples continuously while the left button is held (#1040):
// the Eyedropper tool's foreground colour follows a drag, and the Color Picker
// dialog's eyedropper samples the canvas on click and drag when it is opened
// from a colour swatch (through the frame's default canvas sampler).

#include <QtTest/QtTest>

#include "color_picker_dialog.h"
#include "frame.h"
#include "image_view.h"
#include "panels/color_panel.h"
#include "tools.h"

#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/view_bridge.h"
#include <QtTest/QSignalSpy>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QPushButton>

#include "qt_test_support.h"

namespace {

// A 20x20 image split into red / green / blue vertical bands, so a sample at
// three x positions reads three distinct colours.
QImage bandedImage()
{
    QImage image(20, 20, QImage::Format_RGB32);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            image.setPixel(x, y, x < 7 ? qRgb(255, 0, 0)
                                       : (x < 14 ? qRgb(0, 255, 0) : qRgb(0, 0, 255)));
        }
    }
    return image;
}

} // namespace

class EyedropperTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void eyedropperDragSamplesContinuously();
    void pickerSwatchEyedropperSamplesTheCanvas();
    void sampleAllLayersReadsTheComposite();
    void altSamplesBackground();
    void plainSamplesForeground();
    void temporarySampling();
    void temporaryRestores();
    void temporaryCancel();
    void temporaryQuickMask();
    void reservedAltGestures();
    void altDuringStroke();
    void shiftPlacesSampler();
    void shiftSamplerLimit();
    void shiftAltPlacesSampler();
    void shiftOutsideImageDoesNotPlaceSampler();


private:
    bool openBandedImage();
    void sendMouse(QEvent::Type type, const QPointF& imagePos, Qt::MouseButtons held);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
    pictura::PictureView* view_ = nullptr;
};

void EyedropperTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void EyedropperTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool EyedropperTest::openBandedImage()
{
    const QString path = dir_.filePath(QStringLiteral("bands.png"));
    if (!bandedImage().save(path) || !window_->openDocumentAtPath(path)) {
        return false;
    }
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    return view_ != nullptr;
}

void EyedropperTest::sendMouse(QEvent::Type type, const QPointF& imagePos, Qt::MouseButtons held)
{
    auto* canvas = window_->imageView();
    const QPointF at = canvas->imageToWidget(imagePos);
    QMouseEvent event(type, at, canvas->mapToGlobal(at), Qt::LeftButton, held, Qt::NoModifier);
    QApplication::sendEvent(canvas, &event);
}

// #1040: the Eyedropper tool's foreground colour follows a held drag.
void EyedropperTest::eyedropperDragSamplesContinuously()
{
    QVERIFY(openBandedImage());
    window_->resize(900, 700);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    tools->setActiveTool(pictura::ToolId::Eyedropper);

    const QColor red = QColor::fromRgb(view_->sample_argb(3, 10));
    const QColor green = QColor::fromRgb(view_->sample_argb(10, 10));
    const QColor blue = QColor::fromRgb(view_->sample_argb(17, 10));
    QVERIFY2(red == QColor(255, 0, 0) && green == QColor(0, 255, 0) && blue == QColor(0, 0, 255),
             "banded fixture colours");

    sendMouse(QEvent::MouseButtonPress, QPointF(3, 10), Qt::LeftButton);
    QCOMPARE(tools->foreground(), red);
    sendMouse(QEvent::MouseMove, QPointF(10, 10), Qt::LeftButton);
    QCOMPARE(tools->foreground(), green);
    sendMouse(QEvent::MouseMove, QPointF(17, 10), Qt::LeftButton);
    QCOMPARE(tools->foreground(), blue);
    sendMouse(QEvent::MouseButtonRelease, QPointF(17, 10), Qt::NoButton);
    // A move with the button released is a hover, and does not sample.
    sendMouse(QEvent::MouseMove, QPointF(3, 10), Qt::NoButton);
    QCOMPARE(tools->foreground(), blue);
    window_->hide();
}

// The picker opened from the Color panel's foreground swatch arms the canvas
// eyedropper, samples on click, follows a held drag, and disarms on close.
void EyedropperTest::pickerSwatchEyedropperSamplesTheCanvas()
{
    QVERIFY(openBandedImage());
    window_->resize(900, 700);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* tools = window_->findChild<pictura::ToolController*>();
    auto* fg = window_->findChild<QPushButton*>(QStringLiteral("colorPanelForeground"));
    QVERIFY(tools && fg);

    const QColor red = QColor::fromRgb(view_->sample_argb(3, 10));
    const QColor green = QColor::fromRgb(view_->sample_argb(10, 10));
    const QColor blue = QColor::fromRgb(view_->sample_argb(17, 10));
    const int history = view_->history_count();

    bool armed = false;
    bool cursorShown = false;
    bool disarmed = false;
    QColor clicked;
    QColor dragged;
    QTimer::singleShot(0, window_.get(), [&]() {
        auto* picker = window_->findChild<pictura::ColorPickerDialog*>();
        if (!picker) {
            return;
        }
        armed = tools->canvasSamplerActive();
        cursorShown = window_->imageView()->cursor().shape() != Qt::ArrowCursor;
        sendMouse(QEvent::MouseButtonPress, QPointF(3, 10), Qt::LeftButton);
        clicked = picker->selectedColor();
        sendMouse(QEvent::MouseMove, QPointF(10, 10), Qt::LeftButton);
        const QColor mid = picker->selectedColor();
        sendMouse(QEvent::MouseMove, QPointF(17, 10), Qt::LeftButton);
        dragged = picker->selectedColor();
        if (mid != green) {
            dragged = QColor();
        }
        sendMouse(QEvent::MouseButtonRelease, QPointF(17, 10), Qt::NoButton);
        picker->accept();
        disarmed = !tools->canvasSamplerActive();
    });
    fg->click();

    QVERIFY2(armed, "the swatch picker armed the canvas sampler");
    QVERIFY2(cursorShown, "the canvas shows the eyedropper cursor");
    QCOMPARE(clicked, red);
    QCOMPARE(dragged, blue);
    QVERIFY2(disarmed, "closing the picker disarmed the sampler");
    QCOMPARE(view_->history_count(), history);
    window_->hide();
}

// [rc_sample_all] The Eyedropper reads the composite of every layer: a red
// shape layer over the green band is what it picks there, though the
// document keeps no stored composite (#1216).
void EyedropperTest::sampleAllLayersReadsTheComposite()
{
    QVERIFY(openBandedImage());
    window_->resize(900, 700);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 8;
    square.y0 = 2;
    square.x1 = 13;
    square.y1 = 18;
    QVERIFY(!pictura::shape_add_layer(*view_, square, 0xffff0000u).isEmpty());
    QCOMPARE(view_->sample_argb(10, 10), 0xffff0000u);
    QCOMPARE(view_->sample_argb(10, 19), 0xff00ff00u);

    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    tools->setActiveTool(pictura::ToolId::Eyedropper);
    sendMouse(QEvent::MouseButtonPress, QPointF(10, 10), Qt::LeftButton);
    QCOMPARE(tools->foreground(), QColor(255, 0, 0));
    sendMouse(QEvent::MouseMove, QPointF(10, 19), Qt::LeftButton);
    QCOMPARE(tools->foreground(), QColor(0, 255, 0));
    sendMouse(QEvent::MouseButtonRelease, QPointF(10, 19), Qt::NoButton);
    window_->hide();
}


void EyedropperTest::altSamplesBackground()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    t->setForeground(Qt::white);
    const int history = view_->history_count();
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
    QCOMPARE(t->background(), QColor(Qt::red));
    QSignalSpy modifiers(c, &pictura::ImageView::modifierKeyChanged);
    QTest::keyRelease(c, Qt::Key_Alt);
    QVERIFY(!modifiers.isEmpty());
    c->mouseMoved(QPointF(10, 10));
    QCOMPARE(t->background(), QColor(Qt::green));
    c->mouseReleased(QPointF(10, 10));
    QCOMPARE(t->foreground(), QColor(Qt::white));
    QCOMPARE(view_->history_count(), history);
}

void EyedropperTest::plainSamplesForeground()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    t->setBackground(Qt::white);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, 0);
    c->mouseMoved(QPointF(10, 10));
    c->mouseReleased(QPointF(10, 10));
    QCOMPARE(t->foreground(), QColor(Qt::green));
    QCOMPARE(t->background(), QColor(Qt::white));
}

void EyedropperTest::temporarySampling()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    const QString base = view_->active_layer_path();
    pictura::ShapeSpec square{};
    square.kind = 0; square.boxed = true;
    square.x0 = 4; square.y0 = 2; square.x1 = 9; square.y1 = 18;
    QVERIFY(!pictura::shape_add_layer(*view_, square, 0xff0000ffu).isEmpty());
    view_->set_active_layer(base);
    auto* opts = t->optionsFor<pictura::EyedropperGroup>(pictura::ToolId::Eyedropper);
    auto value = opts->get(); value.sampleSize = 3; value.scope = 0; opts->set(value);
    const QColor averaged = QColor::fromRgb(pictura::sample_argb_scoped(*view_, 6, 10, 3, 0));
    const QColor point = QColor::fromRgb(pictura::sample_argb_scoped(*view_, 6, 10, 1, 0));
    const QColor composite = QColor::fromRgb(pictura::sample_argb_scoped(*view_, 6, 10, 3, 2));
    QVERIFY(averaged != point);
    QVERIFY(averaged != composite);
    for (auto id : {pictura::ToolId::Brush, pictura::ToolId::Pencil,
                    pictura::ToolId::ColorReplacement, pictura::ToolId::PatternStamp,
                    pictura::ToolId::HistoryBrush, pictura::ToolId::ArtHistoryBrush}) {
        t->setActiveTool(id);
        QCOMPARE(t->activeTool(), id);
        auto* paint = t->optionsFor<pictura::PaintOptions>(id);
        const int size = paint->size();
        const int hardness = paint->hardness();
        const int opacity = paint->opacity();
        const int flow = paint->flow();
        const int history = view_->history_count();
        t->setForeground(Qt::white);
        c->mousePressed(QPointF(6, 10), Qt::LeftButton, int(Qt::AltModifier));
        QCOMPARE(t->foreground(), averaged);
        c->mouseMoved(QPointF(10, 10));
        QCOMPARE(t->foreground(), QColor::fromRgb(pictura::sample_argb_scoped(*view_, 10, 10, 3, 0)));
        c->mouseReleased(QPointF(10, 10));
        QCOMPARE(t->activeTool(), id);
        QCOMPARE(view_->history_count(), history);
        QCOMPARE(paint->size(), size);
        QCOMPARE(paint->hardness(), hardness);
        QCOMPARE(paint->opacity(), opacity);
        QCOMPARE(paint->flow(), flow);
        QCOMPARE(opts->get().sampleSize, 3);
        QCOMPARE(opts->get().scope, 0);
    }
}

void EyedropperTest::temporaryRestores()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Brush);
    const int history = view_->history_count();
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
    QSignalSpy modifiers(c, &pictura::ImageView::modifierKeyChanged);
    QTest::keyRelease(c, Qt::Key_Alt);
    QVERIFY(!modifiers.isEmpty());
    t->refreshCursor(Qt::NoModifier);
    QVERIFY(c->cursor().shape() != Qt::BlankCursor);
    c->mouseMoved(QPointF(10, 10));
    QCOMPARE(t->foreground(), QColor(Qt::green));
    c->mouseReleased(QPointF(10, 10));
    t->refreshCursor(Qt::NoModifier);
    QCOMPARE(c->cursor().shape(), Qt::BlankCursor);
    QCOMPARE(view_->history_count(), history);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, 0);
    c->mouseReleased(QPointF(3, 10));
    QVERIFY(view_->history_count() > history);
}

void EyedropperTest::temporaryCancel()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Brush);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
    t->onSessionCancelled();
    t->refreshCursor(Qt::NoModifier);
    QCOMPARE(c->cursor().shape(), Qt::BlankCursor);
    const int history = view_->history_count();
    c->mousePressed(QPointF(10, 10), Qt::LeftButton, 0);
    c->mouseReleased(QPointF(10, 10));
    QVERIFY(view_->history_count() > history);
}

void EyedropperTest::temporaryQuickMask()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    auto* opts = t->optionsFor<pictura::EyedropperGroup>(pictura::ToolId::Eyedropper);
    auto value = opts->get(); value.sampleSize = 3; value.scope = 0; opts->set(value);
    QVERIFY(pictura::quick_mask_set(*view_, true));
    QList<int> mask;
    for (int x = 3; x <= 10; ++x) {
        mask.append(pictura::quick_mask_coverage(*view_, x, 10));
    }
    for (auto id : {pictura::ToolId::Brush, pictura::ToolId::Pencil,
                    pictura::ToolId::ColorReplacement, pictura::ToolId::PatternStamp,
                    pictura::ToolId::HistoryBrush, pictura::ToolId::ArtHistoryBrush}) {
        t->setActiveTool(id);
        const int history = view_->history_count();
        t->setForeground(Qt::white);
        c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
        QCOMPARE(t->foreground(), QColor::fromRgb(pictura::sample_argb_scoped(*view_, 3, 10, 3, 0)));
        c->mouseMoved(QPointF(10, 10));
        QCOMPARE(t->foreground(), QColor::fromRgb(pictura::sample_argb_scoped(*view_, 10, 10, 3, 0)));
        c->mouseReleased(QPointF(10, 10));
        for (int x = 3; x <= 10; ++x) {
            QCOMPARE(pictura::quick_mask_coverage(*view_, x, 10), mask[x - 3]);
        }
        QCOMPARE(view_->history_count(), history);
        QCOMPARE(t->activeTool(), id);
    }
}

void EyedropperTest::reservedAltGestures()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::CloneStamp);
    t->setForeground(Qt::white);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
    c->mouseReleased(QPointF(3, 10));
    QVERIFY(t->optionsFor<pictura::CloneSourceOptions>(pictura::ToolId::CloneStamp)->active().hasSource);
    t->setActiveTool(pictura::ToolId::MixerBrush);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::AltModifier));
    c->mouseReleased(QPointF(3, 10));
    QCOMPARE(t->optionsFor<pictura::MixerGroup>(pictura::ToolId::MixerBrush)->get().reservoir, QColor(Qt::red));
    QCOMPARE(t->foreground(), QColor(Qt::white));
    window_->closeDocument(window_->activeDocumentIndex(), false);
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(17, 17, 6, 6, Qt::black);
    const QString path = dir_.filePath(QStringLiteral("healing-source.png"));
    QVERIFY(seed.save(path));
    QVERIFY(window_->openDocumentAtPath(path));
    view_ = window_->activeView();
    c = window_->imageView();
    auto* paint = t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    paint->setSize(16);
    paint->setHardness(100);
    paint->setOpacity(100);
    paint->setFlow(100);
    t->setActiveTool(pictura::ToolId::HealingBrush);
    c->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::AltModifier));
    c->mouseReleased(QPointF(30, 30));
    const quint32 blemish = view_->sample_argb(20, 20);
    c->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    c->mouseReleased(QPointF(20, 20));
    QVERIFY(view_->sample_argb(20, 20) != blemish);

    t->setActiveTool(pictura::ToolId::Eraser);
    t->setBackground(Qt::red);
    t->setForeground(Qt::white);
    c->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    c->mouseReleased(QPointF(20, 20));
    QCOMPARE(view_->sample_argb(20, 20), QColor(Qt::red).rgba());
    c->mousePressed(QPointF(20, 20), Qt::LeftButton, int(Qt::AltModifier));
    c->mouseReleased(QPointF(20, 20));
    QCOMPARE(view_->sample_argb(20, 20), blemish);
    QCOMPARE(t->foreground(), QColor(Qt::white));
}

void EyedropperTest::altDuringStroke()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Brush);
    t->setForeground(Qt::black);
    const int history = view_->history_count();
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, 0);
    QSignalSpy modifiers(c, &pictura::ImageView::modifierKeyChanged);
    QTest::keyPress(c, Qt::Key_Alt);
    QVERIFY(!modifiers.isEmpty());
    c->mouseMoved(QPointF(10, 10));
    QTest::keyRelease(c, Qt::Key_Alt);
    c->mouseReleased(QPointF(10, 10));
    QCOMPARE(t->foreground(), QColor(Qt::black));
    QCOMPARE(view_->history_count(), history + 1);
}

void EyedropperTest::shiftPlacesSampler()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    t->setForeground(Qt::white); t->setBackground(Qt::black);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::ShiftModifier));
    c->mouseMoved(QPointF(10, 10)); c->mouseReleased(QPointF(10, 10));
    QCOMPARE(pictura::marker_count(*view_, 0), 1);
    const auto pos = pictura::marker_at(*view_, 0, 0);
    QCOMPARE(pos[0], 3); QCOMPARE(pos[1], 10);
    QCOMPARE(t->foreground(), QColor(Qt::white)); QCOMPARE(t->background(), QColor(Qt::black));
}

void EyedropperTest::shiftSamplerLimit()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    for (int x = 1; x <= 4; ++x) QVERIFY(pictura::add_marker(*view_, 0, x, 10) >= 0);
    const int history = view_->history_count();
    QSignalSpy refused(t, &pictura::ToolController::pixelEditRefused);
    c->mousePressed(QPointF(10, 10), Qt::LeftButton, int(Qt::ShiftModifier));
    c->mouseReleased(QPointF(10, 10));
    QCOMPARE(pictura::marker_count(*view_, 0), 4);
    QCOMPARE(view_->history_count(), history);
    QCOMPARE(refused.size(), 1);
}

void EyedropperTest::shiftAltPlacesSampler()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    t->setForeground(Qt::white); t->setBackground(Qt::black);
    c->mousePressed(QPointF(3, 10), Qt::LeftButton, int(Qt::ShiftModifier | Qt::AltModifier));
    c->mouseReleased(QPointF(3, 10));
    QCOMPARE(pictura::marker_count(*view_, 0), 1);
    QCOMPARE(t->foreground(), QColor(Qt::white)); QCOMPARE(t->background(), QColor(Qt::black));
}

void EyedropperTest::shiftOutsideImageDoesNotPlaceSampler()
{
    QVERIFY(openBandedImage());
    auto* t = window_->findChild<pictura::ToolController*>();
    auto* c = window_->imageView();
    t->setActiveTool(pictura::ToolId::Eyedropper);
    const int history = view_->history_count();
    for (const QPointF point : {QPointF(-1, 10), QPointF(20, 10), QPointF(10, -1), QPointF(10, 20)}) {
        c->mousePressed(point, Qt::LeftButton, int(Qt::ShiftModifier));
        c->mouseReleased(point);
    }
    QCOMPARE(pictura::marker_count(*view_, 0), 0);
    QCOMPARE(view_->history_count(), history);
}

PICTURA_QT_TEST_MAIN(EyedropperTest)
#include "tst_eyedropper.moc"
