#include <QtTest/QtTest>
#include <QtGui/QMouseEvent>
#include <QtGui/QPointingDevice>
#include <QtGui/QTabletEvent>

#include "image_view.h"
#include "tools.h"
#include "qt_test_support.h"

class PointerPressureTest : public QObject {
    Q_OBJECT

private slots:
    void tabletAndMouse();
};

void PointerPressureTest::tabletAndMouse()
{
    pictura::ImageView canvas;
    pictura::ToolController tools;
    tools.bindCanvas(&canvas);
    QVERIFY(!tools.hasTabletPressure());
    QCOMPARE(tools.pressure(), 1.0);

    QPointingDevice stylus(QStringLiteral("test stylus"), 1, QInputDevice::DeviceType::Stylus,
                           QPointingDevice::PointerType::Pen,
                           QInputDevice::Capability::Position | QInputDevice::Capability::Pressure,
                           1, 1);
    const QPointF pos(10, 10);
    QTabletEvent tablet(QEvent::TabletMove, &stylus, pos, pos, 0.25, 0, 0, 0, 0, 0,
                        Qt::NoModifier, Qt::NoButton, Qt::NoButton);
    QApplication::sendEvent(&canvas, &tablet);
    QVERIFY(tools.hasTabletPressure());
    QCOMPARE(tools.pressure(), 0.25);

    QMouseEvent synthetic(QEvent::MouseMove, pos, pos, pos, Qt::NoButton, Qt::NoButton,
                          Qt::NoModifier, Qt::MouseEventSynthesizedByQt);
    QApplication::sendEvent(&canvas, &synthetic);
    QVERIFY(tools.hasTabletPressure());
    QCOMPARE(tools.pressure(), 0.25);

    QMouseEvent mouse(QEvent::MouseMove, pos, pos, Qt::NoButton, Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(&canvas, &mouse);
    QVERIFY(!tools.hasTabletPressure());
    QCOMPARE(tools.pressure(), 1.0);
}

PICTURA_QT_TEST_MAIN(PointerPressureTest)
#include "tst_pointer_pressure.moc"
