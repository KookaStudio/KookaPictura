#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QPointF>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>

#include "panels/angle_dial.h"
#include "panels/curve_widget.h"
#include "panels/jump_slider.h"
#include "panels/ramp_slider.h"
#include "panels/spectrum_bar.h"

#include <cmath>
#include <cstdint>

class SharedWidgetsTest : public QObject {
    Q_OBJECT

private slots:
    void angleDialWrapsAndEmits();
    void rampSliderRampAndTracking();
    void spectrumBarHueShiftRepaints();
    void curveWidgetDefaultsAndLut();
    void jumpSliderJumpsToClickAndTracks();
};

void SharedWidgetsTest::angleDialWrapsAndEmits()
{
    pictura::AngleDial dial;
    dial.setAngle(370.0);
    QVERIFY2(std::abs(dial.angle() - 10.0) < 1e-9, "positive wrap into [0, 360)");
    dial.setAngle(-90.0);
    QVERIFY2(std::abs(dial.angle() - 270.0) < 1e-9, "negative wrap into [0, 360)");

    dial.setMouseTracking(true);
    QSignalSpy spy(&dial, &pictura::AngleDial::angleChanged);
    QVERIFY(spy.isValid());

    QTest::mousePress(&dial, Qt::LeftButton, Qt::NoModifier, QPoint(31, 4));
    QVERIFY2(spy.count() >= 1, "press emits angleChanged");
    const double pressed = spy.takeFirst().at(0).toDouble();
    QVERIFY(std::isfinite(pressed));

    QTest::mouseMove(&dial, QPoint(4, 31));
    QVERIFY2(spy.count() >= 1, "move emits angleChanged");
    QVERIFY(std::isfinite(spy.last().at(0).toDouble()));
}

void SharedWidgetsTest::rampSliderRampAndTracking()
{
    pictura::RampSlider slider;
    slider.setRange(0, 100);
    slider.resize(200, 24);

    slider.setValue(40);
    QCOMPARE(slider.value(), 40);

    const QImage plain = slider.grab().toImage();
    slider.setRamp({QColor(Qt::red), QColor(Qt::blue)});
    const QImage ramp = slider.grab().toImage();
    QVERIFY2(ramp != plain, "a two-stop ramp paints a gradient groove");
    // The left end of the band is red, the right end blue.
    const int y = ramp.height() / 2;
    QVERIFY(qRed(ramp.pixel(12, y)) > qBlue(ramp.pixel(12, y)));
    QVERIFY(qBlue(ramp.pixel(ramp.width() - 12, y)) > qRed(ramp.pixel(ramp.width() - 12, y)));
    QVERIFY2(slider.styleSheet().isEmpty(), "the handle stays the style's own");

    slider.setRamp({QColor(Qt::red)});
    QVERIFY2(slider.grab().toImage() == plain, "fewer than two stops restores the groove");

    slider.setValue(70);
    QCOMPARE(slider.value(), 70);

    QTest::mousePress(&slider, Qt::LeftButton, Qt::NoModifier, QPoint(180, 12));
    QVERIFY2(slider.value() > 50, "press-drag tracks to the clicked position");
}

void SharedWidgetsTest::spectrumBarHueShiftRepaints()
{
    pictura::SpectrumBar bar;
    bar.resize(200, 14);

    const QImage before = bar.grab().toImage();
    QVERIFY2(!before.isNull(), "spectrum bar renders");

    bar.setHueShift(120);
    const QImage after = bar.grab().toImage();
    QVERIFY2(after != before, "hue shift changes the rendered pixmap");
}

void SharedWidgetsTest::curveWidgetDefaultsAndLut()
{
    pictura::CurveWidget curve;
    QCOMPARE(curve.points().size(), 2);
    QCOMPARE(curve.points().at(0), QPointF(0.0, 0.0));
    QCOMPARE(curve.points().at(1), QPointF(1.0, 1.0));

    uint8_t lut[256];
    curve.buildLut(lut);
    QCOMPARE(int(lut[0]), 0);
    QCOMPARE(int(lut[255]), 255);
    for (int i = 0; i < 256; ++i) {
        QVERIFY2(std::abs(int(lut[i]) - i) <= 1, "default curve is the identity");
        if (i > 0) {
            QVERIFY2(lut[i] >= lut[i - 1], "buildLut is monotonic");
        }
    }

    QVector<QPointF> pts;
    pts << QPointF(0.0, 0.0) << QPointF(0.5, 0.8) << QPointF(1.0, 1.0);
    curve.setPoints(pts);
    uint8_t bent[256];
    curve.buildLut(bent);
    QVERIFY2(bent[128] > lut[128], "a control point changes the LUT");

    curve.resetCurve();
    QCOMPARE(curve.points().size(), 2);
    QCOMPARE(curve.points().at(0), QPointF(0.0, 0.0));
    QCOMPARE(curve.points().at(1), QPointF(1.0, 1.0));
    uint8_t resetLut[256];
    curve.buildLut(resetLut);
    QCOMPARE(int(resetLut[128]), 128);
}

void SharedWidgetsTest::jumpSliderJumpsToClickAndTracks()
{
    pictura::JumpSlider slider(Qt::Horizontal);
    slider.setRange(0, 100);
    slider.setPageStep(10);
    slider.setValue(0);
    slider.resize(200, 24);

    QTest::mousePress(&slider, Qt::LeftButton, Qt::NoModifier, QPoint(190, 12));
    QVERIFY2(slider.value() > 50, "a groove press jumps to the click, not a page step");
    const int pressed = slider.value();

    QTest::mouseMove(&slider, QPoint(15, 12));
    QVERIFY2(slider.value() < pressed, "a held drag tracks the cursor");
    QTest::mouseRelease(&slider, Qt::LeftButton, Qt::NoModifier, QPoint(15, 12));
}

#include "qt_test_support.h"

PICTURA_QT_TEST_MAIN(SharedWidgetsTest)
#include "tst_shared_widgets.moc"
