#include <QtTest/QtTest>

#include <QtGui/QImage>
#include <QtWidgets/QApplication>

#include "notification_status_bar.h"

class NotificationStatusBarTest : public QObject {
    Q_OBJECT

private slots:
    void temporaryMessageHasYellowBackground();
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

QTEST_MAIN(NotificationStatusBarTest)
#include "tst_notification_status_bar.moc"
