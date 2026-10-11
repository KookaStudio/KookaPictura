// Crop shield scope (#282): the shield dims only the canvas area outside the
// crop box, never the workspace around the canvas.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::Fixture;
}

class CropShieldTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void shieldDoesNotDimTheWorkspace();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CropShieldTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropShieldTest::shieldDoesNotDimTheWorkspace()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(200, 200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_crop_shield"));
    QVERIFY2(f.ok(), "crop-shield fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    QCoreApplication::processEvents();

    f.canvas->fitOnScreen();
    QCoreApplication::processEvents();

    const QPointF canvasTopLeft = f.canvas->imageToWidget(QPointF(0, 0));
    const QPointF workspacePoint(canvasTopLeft.x() - 3, canvasTopLeft.y() - 3);
    QVERIFY2(workspacePoint.x() >= 0 && workspacePoint.y() >= 0,
             "workspace has a visible margin around the canvas");
    const QPoint canvasOutsideBox = f.canvas->imageToWidget(QPointF(10, 10)).toPoint();

    const QImage before = f.canvas->grab().toImage();

    f.canvas->setCropBox(QRectF(50, 50, 100, 100));
    QCoreApplication::processEvents();
    const QImage after = f.canvas->grab().toImage();

    // The workspace pixel is untouched by the shield.
    QCOMPARE(after.pixel(workspacePoint.toPoint()), before.pixel(workspacePoint.toPoint()));
    // The canvas pixel outside the box is dimmed.
    QVERIFY2(after.pixel(canvasOutsideBox) != before.pixel(canvasOutsideBox),
             "canvas outside the box must be dimmed");
}

QTEST_MAIN(CropShieldTest)
#include "tst_crop_shield.moc"
