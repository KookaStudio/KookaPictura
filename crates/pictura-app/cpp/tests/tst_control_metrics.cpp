// The control server's `metrics` method with PICTURA_METRICS on, and the
// `layer_op add_group` op the profiler's new-group scenario drives. The level
// is read once per process, so the off case lives in tst_control_metrics_off.

#include <QtTest/QtTest>

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>

#include "control_server.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "qt_test_support.h"

class ControlMetricsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void snapshotContainsEditRecord();
    void resetClearsTotals();
    void addGroupCreatesGroup();

private:
    QJsonObject call(const QString& method, const QJsonObject& params = {});

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlMetricsTest::initTestCase()
{
    // Before anything reads the level: the Rust side reads it once.
    qputenv("PICTURA_METRICS", "1");
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Metrics"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    server_ = std::make_unique<pictura::ControlServer>(window_.get(), QString());
}

QJsonObject ControlMetricsTest::call(const QString& method, const QJsonObject& params)
{
    return server_->dispatch(method, params);
}

void ControlMetricsTest::snapshotContainsEditRecord()
{
    QVERIFY(call(QStringLiteral("metrics"), {{QStringLiteral("reset"), true}})
                .value(QStringLiteral("ok"))
                .toBool());
    QVERIFY(call(QStringLiteral("layer_op"),
                 {{QStringLiteral("op"), QStringLiteral("add")}, {QStringLiteral("index"), 0}})
                .value(QStringLiteral("ok"))
                .toBool());
    const QJsonObject response = call(QStringLiteral("metrics"));
    QVERIFY(response.value(QStringLiteral("ok")).toBool());
    const QJsonObject result = response.value(QStringLiteral("result")).toObject();
    QVERIFY(result.value(QStringLiteral("enabled")).toBool());
    QVERIFY(!result.value(QStringLiteral("phases")).toObject().isEmpty());
    QVERIFY(result.value(QStringLiteral("counters")).toObject().contains(
        QStringLiteral("declines")));
    const QJsonArray edits = result.value(QStringLiteral("edits")).toArray();
    QVERIFY(!edits.isEmpty());
    const QJsonObject last = edits.last().toObject();
    QCOMPARE(last.value(QStringLiteral("label")).toString(), QStringLiteral("New Layer"));
    QVERIFY(!last.value(QStringLiteral("composited")).toBool());
}

void ControlMetricsTest::resetClearsTotals()
{
    QVERIFY(call(QStringLiteral("layer_op"),
                 {{QStringLiteral("op"), QStringLiteral("add")}, {QStringLiteral("index"), 0}})
                .value(QStringLiteral("ok"))
                .toBool());
    QVERIFY(call(QStringLiteral("metrics"), {{QStringLiteral("reset"), true}})
                .value(QStringLiteral("ok"))
                .toBool());
    const QJsonObject result =
        call(QStringLiteral("metrics")).value(QStringLiteral("result")).toObject();
    QVERIFY(result.value(QStringLiteral("phases")).toObject().isEmpty());
    QVERIFY(result.value(QStringLiteral("edits")).toArray().isEmpty());
}

void ControlMetricsTest::addGroupCreatesGroup()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int before = view->layer_count();
    const QJsonObject response =
        call(QStringLiteral("layer_op"),
             {{QStringLiteral("op"), QStringLiteral("add_group")}, {QStringLiteral("index"), 0}});
    QVERIFY(response.value(QStringLiteral("ok")).toBool());
    QCOMPARE(view->layer_count(), before + 1);
    QCOMPARE(view->layer_kind(1), QStringLiteral("group"));

    const QJsonObject bad =
        call(QStringLiteral("layer_op"),
             {{QStringLiteral("op"), QStringLiteral("add_group")}, {QStringLiteral("index"), 99}});
    QCOMPARE(bad.value(QStringLiteral("error")).toObject().value(QStringLiteral("code")).toString(),
             QStringLiteral("invalid_param"));
}

QTEST_MAIN(ControlMetricsTest)
#include "tst_control_metrics.moc"
