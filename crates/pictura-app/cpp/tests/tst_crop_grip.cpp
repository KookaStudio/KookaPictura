// Crop rotate zone (#282): a press outside the box beyond the resize margin
// enters the straighten rotation (regardless of the canvas edge); within the
// margin it is a resize (crop_grip.h cropRotateZone).

#include <QtTest/QtTest>

#include "crop_grip.h"

#include <cmath>

class CropGripTest : public QObject {
    Q_OBJECT

private slots:
    void zoneClassifiesOutsideAndHandles();
    void zoneScalesWithZoom();
    void straightenAngleFromLine();

private:
    static QRectF box() { return QRectF(10, 20, 100, 60); }
};

void CropGripTest::zoneClassifiesOutsideAndHandles()
{
    const QRectF b = box();
    // Inside the body is never a rotate.
    QVERIFY(!pictura::cropRotateZone(b, b.center(), 1.0));
    // Outside the box (past the margin) rotates, even past the canvas edge.
    QVERIFY(pictura::cropRotateZone(b, QPointF(60, 5), 1.0));
    QVERIFY(pictura::cropRotateZone(b, QPointF(200, 200), 1.0));
    // On the edge (within the resize margin) stays a resize, not a rotate.
    QVERIFY(!pictura::cropRotateZone(b, QPointF(60, 17), 1.0));
    // A null box never rotates.
    QVERIFY(!pictura::cropRotateZone(QRectF(), QPointF(5, 5), 1.0));
}

void CropGripTest::zoneScalesWithZoom()
{
    const QRectF b = box();
    // At zoom 2, the 8 screen px margin is 4 image px.
    QVERIFY(pictura::cropRotateZone(b, QPointF(60, 14), 2.0));
    QVERIFY(!pictura::cropRotateZone(b, QPointF(60, 17), 2.0));
}

void CropGripTest::straightenAngleFromLine()
{
    // A level line needs no rotation; a down-right line rotates the content the
    // other way; a vertical line is -90°; a degenerate line is a no-op.
    const auto near = [](double a, double b) { return std::abs(a - b) < 1e-6; };
    QVERIFY(near(pictura::straightenAngle(QPointF(0, 0), QPointF(10, 0)), 0.0));
    QVERIFY(near(pictura::straightenAngle(QPointF(0, 0), QPointF(10, 10)), -45.0));
    QVERIFY(near(pictura::straightenAngle(QPointF(0, 0), QPointF(0, 10)), -90.0));
    QVERIFY(near(pictura::straightenAngle(QPointF(5, 5), QPointF(5, 5)), 0.0));
}

QTEST_GUILESS_MAIN(CropGripTest)
#include "tst_crop_grip.moc"
