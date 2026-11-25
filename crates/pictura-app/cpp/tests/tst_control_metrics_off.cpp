// The control server's `metrics` method with PICTURA_METRICS unset: refused.
// A separate process from tst_control_metrics because the level is read once.

#include <QtTest/QtTest>

#include <QtCore/QJsonObject>
#include <QtCore/QFile>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>

#include "control_server.h"
#include "frame.h"
#include "image_view.h"
#include "qt_test_support.h"

class ControlMetricsOffTest : public QObject {
    Q_OBJECT

private slots:
    void disabledIsRefused();
    void frameCapturePreservesOutput();

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

void ControlMetricsOffTest::frameCapturePreservesOutput()
{
    const QString target = qEnvironmentVariable("PICTURA_FRAME_TEST_OUTPUT");
    if (!target.isEmpty()) {
        QVERIFY(stateHome_.isValid());
        const auto window = pictura::test::makeMainWindow();
        QVERIFY(window->newDocument(QStringLiteral("Frame output"), 64, 64,
                                    QStringLiteral("rgb"), 8, QStringLiteral("white")));
        pictura::ControlServer server(window.get(), QString());
        QVERIFY(server.dispatch(QStringLiteral("layer_op"), {{"op", "add"}, {"index", 0}})
                    .value("ok").toBool());
        window->show();
        QCoreApplication::processEvents();
        auto* canvas = window->imageView();
        canvas->setZoom(2.0, canvas->rect().center());
        canvas->panBy(QPointF(8, 5));
        canvas->setRotation(20.0);
        canvas->repaint();
        QVERIFY(canvas->grab().toImage().save(target + QStringLiteral("-canvas.png")));
        QVERIFY(server.dispatch(QStringLiteral("document"),
                                {{"op", "save_as"}, {"path", target + QStringLiteral(".psd")}})
                    .value("ok").toBool());
        return;
    }

    QTemporaryDir output;
    QVERIFY(output.isValid());
    for (const QString& mode : {QStringLiteral("0"), QStringLiteral("1")}) {
        QProcess child;
        auto env = QProcessEnvironment::systemEnvironment();
        env.insert(QStringLiteral("PICTURA_METRICS"), mode);
        env.insert(QStringLiteral("PICTURA_FRAME_TEST_OUTPUT"), output.filePath(mode));
        child.setProcessEnvironment(env);
        child.start(QCoreApplication::applicationFilePath(),
                    {QStringLiteral("--suite"), QStringLiteral("tst_control_metrics_off"),
                     QStringLiteral("frameCapturePreservesOutput")});
        QVERIFY(child.waitForFinished());
        QCOMPARE(child.exitStatus(), QProcess::NormalExit);
        QVERIFY2(child.exitCode() == 0, child.readAllStandardOutput().constData());
    }
    for (const QString& suffix : {QStringLiteral("-canvas.png"), QStringLiteral(".psd")}) {
        QFile off(output.filePath(QStringLiteral("0") + suffix));
        QFile on(output.filePath(QStringLiteral("1") + suffix));
        QVERIFY(off.open(QIODevice::ReadOnly) && on.open(QIODevice::ReadOnly));
        QCOMPARE(on.readAll(), off.readAll());
    }
}

PICTURA_QT_TEST_MAIN(ControlMetricsOffTest)
#include "tst_control_metrics_off.moc"
