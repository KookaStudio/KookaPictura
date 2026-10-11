// Perspective Crop corner placement: four clicks stage the quad, corners snap
// to the canvas, the pointer rotates outside a complete quad, and the options
// bar shows Apply/Cancel only while the quad is complete.

#include <QtTest/QtTest>

#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "panels/numeric_field.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QToolButton>

class PerspectiveCropTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void fourClicksStageAndSnap();
    void clickOrderIsOrientedUpright();
    void rotateCursorOutsideCompleteQuad();
    void optionsBarApplyCancelFollowQuad();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    static void click(pictura::ImageView* canvas, const QPointF& p)
    {
        canvas->mousePressed(p, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseReleased(p);
    }
};

void PerspectiveCropTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void PerspectiveCropTest::fourClicksStageAndSnap()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PerspClicks"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);

    // The first click paints its corner immediately, not only once the quad
    // closes on the fourth.
    const QImage staged = canvas->grab().toImage();
    click(canvas, QPointF(20, 20));
    QCoreApplication::processEvents();
    QVERIFY2(canvas->grab().toImage() != staged,
             "a staged corner paints before the quad is complete");

    // Three clicks leave the quad staged but incomplete; the fourth closes it.
    click(canvas, QPointF(100, 20));
    click(canvas, QPointF(100, 100));
    QVERIFY(!canvas->hasPerspectiveCropQuadForTest());
    QCOMPARE(canvas->perspectiveCropQuadForTest().size(), 3);
    click(canvas, QPointF(100, 100));
    click(canvas, QPointF(20, 100));
    QVERIFY(canvas->hasPerspectiveCropQuadForTest());

    // A corner clicked near the canvas corner snaps exactly onto it.
    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(&frame, &esc);
    QVERIFY(!canvas->hasPerspectiveCropQuadForTest());
    click(canvas, QPointF(3, 3));
    QCOMPARE(canvas->perspectiveCropQuadForTest().at(0), QPointF(0, 0));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void PerspectiveCropTest::clickOrderIsOrientedUpright()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PerspOrder"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);

    const auto expectUpright = [canvas]() {
        const QPolygonF q = canvas->perspectiveCropQuadForTest();
        QCOMPARE(q.size(), 4);
        QCOMPARE(q.at(0), QPointF(20, 20));
        QCOMPARE(q.at(1), QPointF(100, 20));
        QCOMPARE(q.at(2), QPointF(100, 100));
        QCOMPARE(q.at(3), QPointF(20, 100));
    };

    // Starting top-right and going clockwise must still read TL, TR, BR, BL.
    click(canvas, QPointF(100, 20));
    click(canvas, QPointF(100, 100));
    click(canvas, QPointF(20, 100));
    click(canvas, QPointF(20, 20));
    expectUpright();

    // A counter-clockwise order must not mirror.
    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(&frame, &esc);
    click(canvas, QPointF(20, 20));
    click(canvas, QPointF(20, 100));
    click(canvas, QPointF(100, 100));
    click(canvas, QPointF(100, 20));
    expectUpright();

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void PerspectiveCropTest::rotateCursorOutsideCompleteQuad()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PerspRotate"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);
    click(canvas, QPointF(30, 30));
    click(canvas, QPointF(90, 30));
    click(canvas, QPointF(90, 90));
    click(canvas, QPointF(30, 90));

    // Just outside the quad (within the rotate band) the cursor is a bitmap
    // rotate cursor, not the crosshair.
    canvas->mouseMoved(QPointF(20, 60));
    QCOMPARE(canvas->cursor().shape(), Qt::BitmapCursor);
    QVERIFY(canvas->cursor().pixmap().toImage()
            != pictura::cursor(QStringLiteral("tool.marquee"), 12, 12).pixmap().toImage());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void PerspectiveCropTest::optionsBarApplyCancelFollowQuad()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("PerspBar"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);
    auto* width = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsPerspectiveWidth"));
    auto* height = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsPerspectiveHeight"));
    auto* clear = frame.findChild<QToolButton*>(QStringLiteral("optionsPerspectiveClear"));
    auto* apply = frame.findChild<QToolButton*>(QStringLiteral("optionsPerspectiveApply"));
    auto* cancel = frame.findChild<QToolButton*>(QStringLiteral("optionsPerspectiveCancel"));
    QVERIFY(width && height && clear && apply && cancel);

    // Apply/Cancel are hidden until the quad is complete.
    QVERIFY(apply->isHidden());
    QVERIFY(cancel->isHidden());
    click(canvas, QPointF(30, 30));
    click(canvas, QPointF(90, 30));
    click(canvas, QPointF(90, 90));
    click(canvas, QPointF(30, 90));
    QVERIFY(!apply->isHidden());
    QVERIFY(!cancel->isHidden());

    // Clear empties the fields.
    width->setValue(200.0);
    height->setValue(100.0);
    clear->click();
    QVERIFY(width->isBlank());
    QVERIFY(height->isBlank());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(PerspectiveCropTest)
#include "tst_perspective_crop.moc"
