// Crop rotate band (#282, #320): a press outside the box within the band
// (beyond the resize margin, within the 100 px far margin) enters the straighten
// rotation, regardless of the canvas edge; beyond the far margin it is the
// apply zone; within the margin it is a resize (crop_grip.h cropRotateZone).

#include <QtTest/QtTest>

#include "crop_grip.h"

#include <cmath>

class CropGripTest : public QObject {
    Q_OBJECT

private slots:
    void zoneClassifiesOutsideAndHandles();
    void zoneScalesWithZoom();
    void farZoneBeyondTheBand();
    void handleGlyphGeometry();
    void edgeGripDrivesTheRatio();
    void altMirrorScalesAboutTheCentre();
    void straightenAngleFromLine();

private:
    static QRectF box() { return QRectF(10, 20, 100, 60); }
};

void CropGripTest::zoneClassifiesOutsideAndHandles()
{
    const QRectF b = box();
    // Inside the body is never a rotate.
    QVERIFY(!pictura::cropRotateZone(b, b.center(), 1.0));
    // Outside the box, within the band, rotates.
    QVERIFY(pictura::cropRotateZone(b, QPointF(60, 5), 1.0));
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

void CropGripTest::farZoneBeyondTheBand()
{
    const QRectF b = box();
    // (200,200) is ~150 px from the nearest corner: beyond the band, in the
    // apply zone, and no longer a rotate.
    QVERIFY(pictura::cropFarZone(b, QPointF(200, 200), 1.0));
    QVERIFY(!pictura::cropRotateZone(b, QPointF(200, 200), 1.0));
    // Exactly 100 px right of the box edge is still the band; one px more is
    // the apply zone.
    QVERIFY(pictura::cropRotateZone(b, QPointF(210, 50), 1.0));
    QVERIFY(!pictura::cropFarZone(b, QPointF(210, 50), 1.0));
    QVERIFY(!pictura::cropRotateZone(b, QPointF(211, 50), 1.0));
    QVERIFY(pictura::cropFarZone(b, QPointF(211, 50), 1.0));
    // The far margin scales with zoom: 150 image px at zoom 0.5 is 75 screen px.
    QVERIFY(!pictura::cropFarZone(b, QPointF(200, 200), 0.5));
    QVERIFY(pictura::cropFarZone(b, QPointF(200, 200), 2.0));
    // A null box is never the apply zone.
    QVERIFY(!pictura::cropFarZone(QRectF(), QPointF(5, 5), 1.0));
}

void CropGripTest::handleGlyphGeometry()
{
    const pictura::CropHandleGlyphs g = pictura::cropHandleGlyphs(QRectF(40, 30, 200, 120));
    // Four corners with two strokes each, four edge bars.
    QCOMPARE(int(g.cornerStrokes.size()), 8);
    QCOMPARE(int(g.edgeBars.size()), 4);
    // The top-left bracket sits inside the corner and runs into the box.
    QCOMPARE(g.cornerStrokes[0], QLineF(42.0, 32.0, 42.0, 50.0));
    QCOMPARE(g.cornerStrokes[1], QLineF(42.0, 32.0, 60.0, 32.0));
    // The top bar hugs the inside of the top edge, centred on the box.
    QVERIFY(g.edgeBars[0].top() > 30);
    QVERIFY(g.edgeBars[0].bottom() <= 36);
    QCOMPARE(g.edgeBars[0].center().x(), 140.0);

    // Glyphs keep a fixed screen size regardless of box size.
    const pictura::CropHandleGlyphs big = pictura::cropHandleGlyphs(QRectF(0, 0, 800, 600));
    QCOMPARE(big.cornerStrokes[0].length(), g.cornerStrokes[0].length());
    QCOMPARE(big.edgeBars[0].height(), g.edgeBars[0].height());

    // A degenerate box yields no glyph geometry.
    const pictura::CropHandleGlyphs empty = pictura::cropHandleGlyphs(QRectF());
    QCOMPARE(empty.cornerStrokes[0], QLineF());
}

void CropGripTest::edgeGripDrivesTheRatio()
{
    // An edge drag drives its own dimension and derives the other from the
    // ratio, rather than shrinking the dragged edge back to nothing.
    const QRectF r = pictura::fitRatio(QRectF(0, 0, 200, 50), 2.0, pictura::BoxGrip::Right);
    QCOMPARE(r, QRectF(0, 0, 200, 100));
    const QRectF b = pictura::fitRatio(QRectF(0, 0, 100, 90), 2.0, pictura::BoxGrip::Bottom);
    QCOMPARE(b, QRectF(0, 0, 180, 90));
    // A corner still fits inside the dragged box.
    const QRectF c = pictura::fitRatio(QRectF(0, 0, 200, 50), 1.0, pictura::BoxGrip::BottomRight);
    QCOMPARE(c, QRectF(0, 0, 50, 50));
}

void CropGripTest::altMirrorScalesAboutTheCentre()
{
    const QRectF start(0, 0, 10, 10);
    // Dragging the right edge out 2 widens both ways: the left edge mirrors.
    const QRectF right = pictura::dragBox(start, pictura::BoxGrip::Right, QPointF(2, 0), true);
    QCOMPARE(right.width(), 14.0);
    QCOMPARE(right.center(), QPointF(5, 5));
    // A corner mirrors both axes.
    const QRectF corner =
        pictura::dragBox(start, pictura::BoxGrip::TopLeft, QPointF(2, 3), true);
    QCOMPARE(corner, QRectF(2, 3, 6, 4));
    QCOMPARE(corner.center(), QPointF(5, 5));
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
