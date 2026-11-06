#include <QtTest/QtTest>

#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QToolButton>

#include "frame.h"
#include "image_view.h"
#include "notification_status_bar.h"
#include "panels/numeric_field.h"
#include "tool_hint_bar.h"

#include "qt_test_support.h"

namespace {

// The readouts the footer hides while a temporary message owns its row.
QList<QWidget*> footerReadouts(pictura::PicturaMainWindow& window)
{
    return {
        window.findChild<pictura::NumericField*>(QStringLiteral("statusZoomField")),
        window.findChild<QLabel*>(QStringLiteral("statusSizeLabel")),
        window.findChild<QLabel*>(QStringLiteral("statusBackendLabel")),
        window.findChild<QToolButton*>(QStringLiteral("statusOptionsButton")),
        window.findChild<pictura::ToolHintBar*>(QStringLiteral("toolHintBar")),
    };
}

// The banner may not sit on anything the bar still draws. Hidden readouts keep
// their last geometry in Qt, so the check is against the visible children; the
// readouts themselves are asserted hidden separately.
bool bannerClearsVisibleChildren(const pictura::NotificationStatusBar& bar)
{
    const QRect banner = bar.messageRect();
    for (QWidget* child : bar.findChildren<QWidget*>(Qt::FindDirectChildrenOnly)) {
        if (child->isVisible() && banner.intersects(child->geometry())) {
            return false;
        }
    }
    return true;
}

} // namespace

class NotificationStatusBarTest : public QObject {
    Q_OBJECT

private slots:
    void temporaryMessageHasYellowBackground();
    void messageReplacesReadouts();
    void messageSurvivesAStatusRefresh();
};

void NotificationStatusBarTest::temporaryMessageHasYellowBackground()
{
    pictura::NotificationStatusBar bar;
    bar.resize(500, 30);
    bar.show();
    QCoreApplication::processEvents();
    bar.showMessage(QStringLiteral("Kooka Pictura is up to date."), 5000);
    QCoreApplication::processEvents();

    const QImage image = bar.grab().toImage();
    QVERIFY(!image.isNull());
    const QColor pixel = image.pixelColor(40, 5);
    QVERIFY2(pixel.red() > 200 && pixel.green() > 180 && pixel.blue() < 220,
             qPrintable(pixel.name()));
    QCOMPARE(bar.currentMessage(), QStringLiteral("Kooka Pictura is up to date."));
}

void NotificationStatusBarTest::messageReplacesReadouts()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QVERIFY(window->newDocument(QStringLiteral("Footer"), 640, 480, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    QCoreApplication::processEvents();

    auto* bar = qobject_cast<pictura::NotificationStatusBar*>(window->statusBar());
    QVERIFY(bar != nullptr);
    const QList<QWidget*> readouts = footerReadouts(*window);
    for (QWidget* readout : readouts) {
        QVERIFY(readout != nullptr);
        QVERIFY2(readout->isVisible(), "readout is visible before the message");
    }

    bar->showMessage(QStringLiteral("Kooka Pictura is up to date."), 5000);
    QCoreApplication::processEvents();

    QVERIFY2(!bar->messageRect().isEmpty(), "the banner owns the footer");
    for (QWidget* readout : readouts) {
        QVERIFY2(!readout->isVisible(), "a temporary message replaces the readouts");
    }
    QVERIFY2(bannerClearsVisibleChildren(*bar),
             "the banner intersects no visible footer child");

    bar->clearMessage();
    QCoreApplication::processEvents();
    QVERIFY2(bar->currentMessage().isEmpty(), "the message cleared");
    for (QWidget* readout : readouts) {
        QVERIFY2(readout->isVisible(), "readouts return when the message clears");
    }
}

void NotificationStatusBarTest::messageSurvivesAStatusRefresh()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    QVERIFY(window->newDocument(QStringLiteral("Footer"), 640, 480, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    QCoreApplication::processEvents();

    auto* bar = qobject_cast<pictura::NotificationStatusBar*>(window->statusBar());
    QVERIFY(bar != nullptr);
    const QList<QWidget*> readouts = footerReadouts(*window);
    for (QWidget* readout : readouts) {
        QVERIFY(readout != nullptr);
    }
    bar->showMessage(QStringLiteral("Could not open image.png"), 5000);
    QCoreApplication::processEvents();
    const QRect banner = bar->messageRect();

    pictura::ImageView* canvas = window->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setZoom(2.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QCoreApplication::processEvents();

    QCOMPARE(bar->currentMessage(), QStringLiteral("Could not open image.png"));
    for (QWidget* readout : readouts) {
        QVERIFY2(!readout->isVisible(), "a refresh keeps the readouts out of the banner");
    }
    QCOMPARE(bar->messageRect(), banner);
    QVERIFY2(bannerClearsVisibleChildren(*bar),
             "the banner still intersects no visible footer child");
}

QTEST_MAIN(NotificationStatusBarTest)
#include "tst_notification_status_bar.moc"
