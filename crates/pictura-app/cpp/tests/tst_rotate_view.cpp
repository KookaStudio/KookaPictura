// The Rotate View tool (#59): the canvas turns about its centre without
// touching the document.

#include <QtTest/QtTest>

#include "panels/angle_dial.h"
#include "panels/numeric_field.h"
#include "selftest_paint_fixture.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QPushButton>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

void sendKey(pictura::PicturaMainWindow& frame, QEvent::Type type, int key,
             Qt::KeyboardModifiers mods, const QString& text = QString())
{
    QKeyEvent event(type, key, mods, text);
    QApplication::sendEvent(&frame, &event);
}

// Drag on the canvas between widget points, mapping each through the view as
// the canvas widget does at that moment.
void dragWidget(const Fixture& f, const QList<QPointF>& points)
{
    f.canvas->mousePressed(f.canvas->widgetToImage(points.first()), Qt::LeftButton, 0);
    for (const QPointF& p : points.mid(1)) {
        f.canvas->mouseMoved(f.canvas->widgetToImage(p));
    }
    f.canvas->mouseReleased(f.canvas->widgetToImage(points.last()));
}

template <typename T>
T* visibleChild(pictura::PicturaMainWindow& frame, const QString& name)
{
    for (T* child : frame.findChildren<T*>(name)) {
        if (child->isVisible()) {
            return child;
        }
    }
    return nullptr;
}

bool near(const QPointF& a, const QPointF& b) { return QLineF(a, b).length() < 1e-6; }

} // namespace

class RotateViewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void rotateView();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void RotateViewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void RotateViewTest::rotateView()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(80, 60, QImage::Format_RGB32);
    seed.fill(Qt::blue);
    for (int y = 0; y < 60; ++y) {
        for (int x = 0; x < 40; ++x) {
            seed.setPixel(x, y, qRgb(255, 0, 0));
        }
    }
    Fixture f(frame, seed, QStringLiteral("pictura_rotate_view_seed"));
    QVERIFY2(f.ok(), "rotate view fixture");
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();

    // R selects the tool.
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(frame, QEvent::KeyPress, Qt::Key_R, Qt::NoModifier, QStringLiteral("r"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::RotateView);

    const int history = f.view->history_index();
    const QPointF centre(f.canvas->width() / 2.0, f.canvas->height() / 2.0);
    const QPointF anchor = f.canvas->widgetToImage(centre);

    // The mapping round-trips at any angle and turns about the centre.
    f.tools->setViewRotation(90.0);
    QCOMPARE(f.canvas->rotation(), 90.0);
    QVERIFY(near(f.canvas->widgetToImage(centre), anchor));
    const QPointF w(centre.x() + 30, centre.y() + 10);
    QVERIFY(near(f.canvas->imageToWidget(f.canvas->widgetToImage(w)), w));

    // The painted canvas puts each document pixel where the mapping says.
    f.tools->setViewRotation(180.0);
    const QImage shown = f.canvas->grab().toImage();
    const QPoint red = f.canvas->imageToWidget(QPointF(10.5, 30.5)).toPoint();
    const QPoint blue = f.canvas->imageToWidget(QPointF(70.5, 30.5)).toPoint();
    QCOMPARE(QColor(shown.pixel(red)), QColor(Qt::red));
    QCOMPARE(QColor(shown.pixel(blue)), QColor(Qt::blue));
    QVERIFY2(blue.x() < red.x(), "turned half way, the blue half is on the left");

    // A drag from right of the centre to below it turns the canvas 90°
    // clockwise; the compass shows only while dragging.
    f.tools->setViewRotation(0.0);
    f.canvas->mousePressed(f.canvas->widgetToImage(centre + QPointF(40, 0)), Qt::LeftButton, 0);
    QVERIFY(f.canvas->compassVisibleForTest());
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(28, 28)));
    f.canvas->mouseMoved(f.canvas->widgetToImage(centre + QPointF(0, 40)));
    f.canvas->mouseReleased(f.canvas->widgetToImage(centre + QPointF(0, 40)));
    QVERIFY(!f.canvas->compassVisibleForTest());
    QVERIFY(qAbs(f.canvas->rotation() - 90.0) < 1e-6);

    // Shift snaps to 15° steps: a 50° drag lands on 45°.
    f.tools->setViewRotation(0.0);
    // A spontaneous key event updates the tracked modifiers the tool reads.
    QTest::keyPress(&frame, Qt::Key_Shift, Qt::ShiftModifier);
    QCOMPARE(QGuiApplication::queryKeyboardModifiers(), Qt::KeyboardModifiers(Qt::ShiftModifier));
    const double a = qDegreesToRadians(50.0);
    dragWidget(f, {centre + QPointF(40, 0), centre + QPointF(40 * std::cos(a), 40 * std::sin(a))});
    QTest::keyRelease(&frame, Qt::Key_Shift, Qt::NoModifier);
    QVERIFY2(qAbs(f.canvas->rotation() - 45.0) < 1e-6,
             qPrintable(QString::number(f.canvas->rotation())));

    // The options bar follows the angle and sets it.
    auto* angle = visibleChild<pictura::NumericField>(frame, QStringLiteral("optionsRotateAngle"));
    auto* dial = visibleChild<pictura::AngleDial>(frame, QStringLiteral("optionsRotateDial"));
    auto* reset = visibleChild<QPushButton>(frame, QStringLiteral("optionsRotateReset"));
    QVERIFY(angle && dial && reset);
    QCOMPARE(angle->value(), 45.0);
    QCOMPARE(dial->angle(), 45.0);
    emit angle->valueChanged(-30.0);
    QCOMPARE(f.canvas->rotation(), -30.0);
    QCOMPARE(dial->angle(), 120.0);
    reset->click();
    QCOMPARE(f.canvas->rotation(), 0.0);
    f.tools->setViewRotation(270.0);
    QCOMPARE(f.canvas->rotation(), -90.0);
    sendKey(frame, QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QCOMPARE(f.canvas->rotation(), 0.0);

    // The document never changed.
    QCOMPARE(f.view->history_index(), history);
    QCOMPARE(QColor(f.view->sample_argb(10, 30)), QColor(Qt::red));
}

PICTURA_QT_TEST_MAIN(RotateViewTest)
#include "tst_rotate_view.moc"
