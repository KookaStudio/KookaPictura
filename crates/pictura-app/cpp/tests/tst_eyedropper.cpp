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

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QMouseEvent>
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

QTEST_MAIN(EyedropperTest)
#include "tst_eyedropper.moc"
