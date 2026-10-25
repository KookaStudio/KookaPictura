// The control server's `metrics` method with PICTURA_METRICS unset: refused.
// A separate process from tst_control_metrics because the level is read once.

#include <QtTest/QtTest>

#include <QtCore/QJsonObject>

#include "control_server.h"
#include "frame.h"
#include "qt_test_support.h"

class ControlMetricsOffTest : public QObject {
    Q_OBJECT

private slots:
    void disabledIsRefused();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ControlMetricsOffTest::disabledIsRefused()
{
    qunsetenv("PICTURA_METRICS");
    QVERIFY(stateHome_.isValid());
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Off"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    pictura::ControlServer server(window.get(), QString());
    const QJsonObject response = server.dispatch(QStringLiteral("metrics"), {});
    QVERIFY(!response.value(QStringLiteral("ok")).toBool());
    QCOMPARE(
        response.value(QStringLiteral("error")).toObject().value(QStringLiteral("code")).toString(),
        QStringLiteral("refused"));
}

QTEST_MAIN(ControlMetricsOffTest)
#include "tst_control_metrics_off.moc"
