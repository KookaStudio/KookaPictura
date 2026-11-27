#include <QtTest/QtTest>
#include <QtCore/QScopeGuard>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtTest/QSignalSpy>
#include <cmath>
#include <utility>
#include <QtWidgets/QLabel>
#include <QtWidgets/QToolButton>

#include "api/filters_bridge.h"
#include "api/paint_bridge.h"
#include "image_view.h"
#include "qt_test_support.h"
#include "panels/info_panel.h"
#include "selftest_paint_fixture.h"

namespace {
QImage seed()
{
    QImage image(80, 60, QImage::Format_RGB32);
    image.fill(Qt::gray);
    return image;
}
void drag(pictura::ImageView* canvas, QPointF a, QPointF b, Qt::KeyboardModifiers mods)
{
    auto send = [canvas, mods](QEvent::Type type, QPointF image, Qt::MouseButtons held) {
        const QPointF local = canvas->imageToWidget(image);
        QMouseEvent event(type, local, canvas->mapToGlobal(local),
                          type == QEvent::MouseMove ? Qt::NoButton : Qt::LeftButton, held, mods);
        QApplication::sendEvent(canvas, &event);
    };
    send(QEvent::MouseButtonPress, a, Qt::LeftButton);
    send(QEvent::MouseMove, b, Qt::LeftButton);
    send(QEvent::MouseButtonRelease, b, Qt::NoButton);
}
}

class RulerToolTest : public QObject {
    Q_OBJECT
private slots:
    void initTestCase();
    void protractorCreate();
    void protractorDouble();
    void protractorClear();
    void primaryEditKeepsProtractor();
    void otherToolDoubleClickRoutesPress();
    void straighten();
    void straightenNoop();
private:
    pictura::test::ScopedStateHome state_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void RulerToolTest::initTestCase()
{
    QVERIFY(state_.isValid());
    window_ = pictura::test::makeMainWindow();
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void RulerToolTest::protractorCreate()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("ruler_protractor"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    const int history = f.view->history_count();
    for (const bool first : {false, true}) {
        pictura::set_ruler(*f.view, 10, 10, 30, 10, false);
        drag(f.canvas, first ? QPointF(10, 10) : QPointF(30, 10),
             first ? QPointF(10, 30) : QPointF(30, 30), Qt::AltModifier);
        const auto primary = pictura::ruler_line(*f.view);
        const auto arm = pictura::ruler_protractor(*f.view);
        const auto m = pictura::ruler_measurement(*f.view);
        QCOMPARE(primary.size(), std::size_t(4));
        QCOMPARE(primary[0], 10.0);
        QCOMPARE(primary[2], 30.0);
        QCOMPARE(arm.size(), std::size_t(4));
        QCOMPARE(arm[0], first ? 10.0 : 30.0);
        QCOMPARE(arm[1], 10.0);
        QCOMPARE(m.size(), std::size_t(7));
        QVERIFY(std::abs(m[4] - 90.0) < 0.1);
        QVERIFY(std::abs(m[5] - 20.0) < 0.1);
        QVERIFY(std::abs(m[6] - 20.0) < 0.1);
        auto* info = window_->findChild<pictura::InfoPanel*>();
        QVERIFY(info);
        QVERIFY(info->rulerTextForTest().contains(QStringLiteral("90.0")));
        QVERIFY(info->rulerTextForTest().contains(QStringLiteral("20.0")));
    }
    pictura::set_ruler(*f.view, 10, 10, 30, 10, false);
    drag(f.canvas, {30, 10}, {40, 11}, Qt::AltModifier | Qt::ShiftModifier);
    const auto constrained = pictura::ruler_protractor(*f.view);
    QVERIFY(std::abs(constrained[3] - constrained[1]) < 0.01);
    QCOMPARE(f.view->history_count(), history);
}

void RulerToolTest::protractorDouble()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("ruler_double"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    for (const auto& item : {std::pair<QPointF, double>{{12, 10}, 10},
                             {{28, 10}, 30}, {{20, 10}, 30}}) {
        pictura::set_ruler(*f.view, 10, 10, 30, 10, false);
        const QPoint local = f.canvas->imageToWidget(item.first).toPoint();
        QMouseEvent press(QEvent::MouseButtonPress, local, f.canvas->mapToGlobal(local),
                          Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &press);
        QMouseEvent firstRelease(QEvent::MouseButtonRelease, local, f.canvas->mapToGlobal(local),
                                 Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &firstRelease);
        QMouseEvent dbl(QEvent::MouseButtonDblClick, local, f.canvas->mapToGlobal(local),
                        Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &dbl);
        const QPointF end = f.canvas->imageToWidget(QPointF(item.second, 30));
        QMouseEvent move(QEvent::MouseMove, end, f.canvas->mapToGlobal(end), Qt::NoButton,
                         Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &move);
        QMouseEvent release(QEvent::MouseButtonRelease, end, f.canvas->mapToGlobal(end),
                            Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &release);
        const auto arm = pictura::ruler_protractor(*f.view);
        QCOMPARE(arm.size(), std::size_t(4));
        QCOMPARE(arm[0], item.second);
        QCOMPARE(pictura::ruler_line(*f.view)[0], 10.0);
        const auto m = pictura::ruler_measurement(*f.view);
        QCOMPARE(m.size(), std::size_t(7));
        QVERIFY(std::abs(m[4] - 90.0) < 0.1);
        QVERIFY(std::abs(m[6] - 20.0) < 0.1);
    }
}

void RulerToolTest::protractorClear()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("ruler_clear"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    pictura::set_ruler(*f.view, 10, 10, 30, 10, false);
    drag(f.canvas, {30, 10}, {30, 30}, Qt::AltModifier);
    QVERIFY(!pictura::ruler_protractor(*f.view).empty());
    window_->setActiveTool(pictura::ToolId::Move);
    QVERIFY(!f.canvas->hasRulerLineForTest());
    window_->setActiveTool(pictura::ToolId::Ruler);
    QVERIFY(f.canvas->hasRulerLineForTest());
    auto* clear = window_->findChild<QToolButton*>(QStringLiteral("optionsAnnotationClear"));
    QVERIFY(clear);
    clear->click();
    QVERIFY(pictura::ruler_line(*f.view).empty());
    QVERIFY(pictura::ruler_protractor(*f.view).empty());
}

void RulerToolTest::primaryEditKeepsProtractor()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("ruler_edit_arm"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    pictura::set_ruler(*f.view, 10, 10, 50, 10, false);
    pictura::set_ruler_protractor(*f.view, false, 50, 30, false);
    drag(f.canvas, {12, 10}, {12, 10}, Qt::NoModifier);
    auto arm = pictura::ruler_protractor(*f.view);
    QCOMPARE(arm.size(), std::size_t(4));
    QCOMPARE(arm[2], 50.0);
    drag(f.canvas, {30, 10}, {30, 20}, Qt::NoModifier);
    arm = pictura::ruler_protractor(*f.view);
    QCOMPARE(arm.size(), std::size_t(4));
    QVERIFY(std::abs(arm[1] - 20.0) < 1.0);
    QVERIFY(std::abs(arm[3] - 40.0) < 1.0);
    drag(f.canvas, {50, 20}, {55, 20}, Qt::NoModifier);
    arm = pictura::ruler_protractor(*f.view);
    QCOMPARE(arm.size(), std::size_t(4));
    QVERIFY(std::abs(arm[0] - 55.0) < 1.0);
    QVERIFY(std::abs(arm[2] - 55.0) < 1.0);
}

void RulerToolTest::otherToolDoubleClickRoutesPress()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("double_press"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Brush);
    QSignalSpy presses(f.canvas, &pictura::ImageView::mousePressed);
    const QPointF local = f.canvas->imageToWidget({20, 20});
    QMouseEvent dbl(QEvent::MouseButtonDblClick, local, f.canvas->mapToGlobal(local),
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(f.canvas, &dbl);
    QCOMPARE(presses.count(), 1);
    QMouseEvent release(QEvent::MouseButtonRelease, local, f.canvas->mapToGlobal(local),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(f.canvas, &release);
}

void RulerToolTest::straighten()
{
    QImage original(80, 60, QImage::Format_RGB32);
    original.fill(Qt::white);
    {
        QPainter painter(&original);
        painter.setPen(QPen(Qt::black, 3));
        painter.drawLine(QPoint(5, 5), QPoint(45, 15));
    }
    paint_fixture::Fixture f(*window_, original, QStringLiteral("ruler_straighten"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    pictura::set_ruler(*f.view, 5, 5, 45, 15, false);
    emit f.tools->rulerChanged();
    const int history = f.view->history_count();
    auto* button = window_->findChild<QToolButton*>(QStringLiteral("optionsRulerStraighten"));
    QVERIFY(button);
    button->click();
    QCOMPARE(f.view->history_count(), history + 1);
    QVERIFY(f.view->document_width() > 80 || f.view->document_height() > 60);
    int minX = f.view->document_width(), minY = f.view->document_height();
    int maxX = -1, maxY = -1;
    for (int y = 0; y < f.view->document_height(); ++y) {
        for (int x = 0; x < f.view->document_width(); ++x) {
            const QColor pixel = QColor::fromRgba(f.view->sample_argb(x, y));
            if (pixel.alpha() > 200 && pixel.red() < 64 && pixel.green() < 64 && pixel.blue() < 64) {
                minX = qMin(minX, x); maxX = qMax(maxX, x);
                minY = qMin(minY, y); maxY = qMax(maxY, y);
            }
        }
    }
    QVERIFY(maxX - minX > 30);
    QVERIFY(maxY - minY <= 5);
    f.view->undo();
    QCOMPARE(f.view->document_width(), 80);
    QCOMPARE(f.view->document_height(), 60);
    for (int y = 0; y < original.height(); ++y) {
        for (int x = 0; x < original.width(); ++x) {
            QCOMPARE(f.view->sample_argb(x, y), original.pixel(x, y));
        }
    }
}

void RulerToolTest::straightenNoop()
{
    paint_fixture::Fixture f(*window_, seed(), QStringLiteral("ruler_straighten_noop"));
    QVERIFY(f.ok());
    window_->setActiveTool(pictura::ToolId::Ruler);
    auto* button = window_->findChild<QToolButton*>(QStringLiteral("optionsRulerStraighten"));
    QVERIFY(button);
    const int history = f.view->history_count();
    button->click();
    pictura::set_ruler(*f.view, 5, 5, 45, 5, false);
    emit f.tools->rulerChanged();
    button->click();
    QCOMPARE(f.view->history_count(), history);
    pictura::set_ruler(*f.view, 5, 5, 45, 15, false);
    emit f.tools->rulerChanged();
    pictura::set_available_memory_for_test(260LL * 1024 * 1024);
    pictura::set_scratch_available_for_test(0);
    const auto restore = qScopeGuard([] {
        pictura::set_available_memory_for_test(-1);
        pictura::set_scratch_available_for_test(-1);
    });
    button->click();
    QCOMPARE(f.view->history_count(), history);
    QCOMPARE(f.view->document_width(), 80);
}

PICTURA_QT_TEST_MAIN(RulerToolTest)
#include "tst_ruler_tool.moc"
