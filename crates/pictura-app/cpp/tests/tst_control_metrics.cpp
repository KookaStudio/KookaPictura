// The control server's `metrics` method with PICTURA_METRICS on, and the
// `layer_op add_group` op the profiler's new-group scenario drives. The level
// is read once per process, so the off case lives in tst_control_metrics_off.

#include <QtTest/QtTest>

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtGui/QWheelEvent>

#include "control_server.h"
#include "frame.h"
#include "api/view_bridge.h"
#include "image_view.h"

#include "qt_test_support.h"

class ControlMetricsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void snapshotContainsEditRecord();
    void resetClearsTotals();
    void addGroupCreatesGroup();
    void typeOpAddsAndResetsAType();
    void frameCoalescesViewportInput();
    void frameCadenceIsPerCanvas();
    void frameRecordsLateTilesSeparately();
    void frameScreenshotDoesNotConsumePending();
    void frameGpuPartialDamage();
    void frameResetClearsPendingTiming();

private:
    QJsonObject call(const QString& method, const QJsonObject& params = {});
    QJsonArray frames(bool reset = false);
    pictura::ImageView* showCanvas();
    void openFixture(int width, int height);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlMetricsTest::initTestCase()
{
    // Before anything reads the level: the Rust side reads it once.
    qputenv("PICTURA_METRICS", "1");
    QVERIFY(stateHome_.isValid());
}

void ControlMetricsTest::init()
{
    openFixture(32, 32);
}

void ControlMetricsTest::openFixture(int width, int height)
{
    server_.reset();
    window_.reset();
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Metrics"), width, height, QStringLiteral("rgb"), 8,
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
    const QJsonObject counters = result.value(QStringLiteral("counters")).toObject();
    QVERIFY(counters.contains(QStringLiteral("declines")));
    QVERIFY(counters.contains(QStringLiteral("resident_entries")));
    QVERIFY(counters.contains(QStringLiteral("resident_bytes")));
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

void ControlMetricsTest::typeOpAddsAndResetsAType()
{
    const QJsonObject added = call(QStringLiteral("type_op"), {{QStringLiteral("op"), QStringLiteral("add")}});
    QVERIFY(added.value(QStringLiteral("ok")).toBool());
    const QString path =
        added.value(QStringLiteral("result")).toObject().value(QStringLiteral("path")).toString();
    QVERIFY(!path.isEmpty());

    const auto op = [&](const QString& name, const QString& field, const QJsonValue& value) {
        return call(QStringLiteral("type_op"), {{QStringLiteral("op"), name},
                                                {QStringLiteral("path"), path},
                                                {field, value}})
            .value(QStringLiteral("ok"))
            .toBool();
    };
    QVERIFY(op(QStringLiteral("set"), QStringLiteral("size"), 96.0));
    QVERIFY(op(QStringLiteral("preview"), QStringLiteral("color"), double(0xffc02030u)));
    QVERIFY(op(QStringLiteral("warp_preview"), QStringLiteral("style"), QStringLiteral("arc")));
    QVERIFY(call(QStringLiteral("type_op"), {{QStringLiteral("op"), QStringLiteral("warp_cancel")}})
                .value(QStringLiteral("ok"))
                .toBool());
    QVERIFY(!op(QStringLiteral("nonsense"), QStringLiteral("size"), 96.0));
}

QJsonArray ControlMetricsTest::frames(bool reset)
{
    return call(QStringLiteral("metrics"), {{QStringLiteral("reset"), reset}})
        .value(QStringLiteral("result")).toObject().value(QStringLiteral("frames")).toArray();
}

pictura::ImageView* ControlMetricsTest::showCanvas()
{
    window_->show();
    QCoreApplication::processEvents();
    auto* canvas = window_->imageView();
    canvas->repaint();
    frames(true);
    return canvas;
}

void ControlMetricsTest::frameCoalescesViewportInput()
{
    auto* canvas = showCanvas();
    canvas->panBy(QPointF(3, 0));
    const QPointF centre = canvas->rect().center();
    QWheelEvent wheel(centre, canvas->mapToGlobal(centre.toPoint()), {}, QPoint(0, 120),
                      Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
    QApplication::sendEvent(canvas, &wheel);
    canvas->fitOnScreen();
    canvas->actualPixels();
    window_->setActiveTool(pictura::ToolId::RotateView);
    canvas->mousePressed(canvas->widgetToImage(centre + QPointF(40, 0)), Qt::LeftButton, 0);
    canvas->mouseMoved(canvas->widgetToImage(centre + QPointF(0, 40)));
    canvas->mouseReleased(canvas->widgetToImage(centre + QPointF(0, 40)));
    QVERIFY(frames().isEmpty());
    QTRY_VERIFY(!frames().isEmpty());
    const auto capture = frames();
    QCOMPARE(capture.size(), 1);
    const auto row = capture.first().toObject();
    QCOMPARE(row.value("input_count").toInt(), 5);
    const double paint = row.value("paint_ms").toDouble();
    const double input = row.value("input_ms").toDouble();
    QVERIFY(qAbs(paint - row.value("tile_gather_ms").toDouble()
                 - row.value("qt_draw_ms").toDouble()) < 0.002);
    QVERIFY(qAbs(16.6 - input - paint - row.value("cpu_budget_slack_ms").toDouble()) < 0.002);
}

void ControlMetricsTest::frameCadenceIsPerCanvas()
{
    auto* a = showCanvas();
    pictura::ImageView b;
    QImage pixels(32, 32, QImage::Format_RGB32);
    pixels.fill(Qt::white);
    b.setImage(pixels);
    b.show();
    QCoreApplication::processEvents();
    qint64 now = 0;
    a->setFrameClockForTest([&] { return now; });
    b.setFrameClockForTest([&] { return now; });
    frames(true);
    now = 1'000'000;
    a->repaint();
    now = 3'000'000;
    b.repaint();
    now = 8'000'000;
    a->repaint();
    a->setFrameClockForTest({});
    const auto capture = frames();
    QCOMPARE(capture.size(), 3);
    const auto first = capture.at(0).toObject();
    const auto other = capture.at(1).toObject();
    const auto last = capture.at(2).toObject();
    QCOMPARE(last.value("canvas_id"), first.value("canvas_id"));
    QVERIFY(other.value("canvas_id") != first.value("canvas_id"));
    QVERIFY(!first.contains("interval_ms") && !other.contains("interval_ms"));
    QCOMPARE(last.value("interval_ms").toDouble(), 7.0);
    QCOMPARE(last.value("input_count").toInt(), 0);
    QCOMPARE(last.value("input_ms").toDouble(), 0.0);
    QVERIFY(last.value("gpu_present_ms").isNull());
}

void ControlMetricsTest::frameRecordsLateTilesSeparately()
{
    openFixture(4200, 4200);
    auto* canvas = showCanvas();
    auto* view = window_->activeView();
    QSignalSpy settled(view, &pictura::PictureView::pyramid_settled);
    canvas->setZoom(1.0, canvas->rect().center());
    canvas->panBy(QPointF(140, 180));
    canvas->repaint();
    auto capture = frames();
    QCOMPARE(capture.size(), 1);
    QCOMPARE(capture.first().toObject().value("input_count").toInt(), 2);
    QVERIFY(settled.isEmpty());
    QTRY_VERIFY(!settled.isEmpty());
    QTRY_VERIFY(frames().size() >= 2);
    capture = frames();
    const auto later = capture.last().toObject();
    QCOMPARE(later.value("canvas_id"), capture.first().toObject().value("canvas_id"));
    QCOMPARE(later.value("input_count").toInt(), 0);
    QVERIFY(later.value("tile_gather_ms").toDouble() <= later.value("paint_ms").toDouble());
}

void ControlMetricsTest::frameScreenshotDoesNotConsumePending()
{
    auto* canvas = showCanvas();
    for (const auto& scope : {QStringLiteral("window"), QStringLiteral("canvas")}) {
        frames(true);
        canvas->repaint();
        canvas->panBy(QPointF(1, 0));
        canvas->seedFrameGpuSampleForTest(QRect(0, 0, 32, 32), 1'000'000);
        QVERIFY(call(QStringLiteral("screenshot"), {{QStringLiteral("scope"), scope}})
                    .value(QStringLiteral("ok")).toBool());
        QCOMPARE(frames().size(), 1);
        canvas->repaint();
        const auto capture = frames();
        QCOMPARE(capture.size(), 2);
        const auto paint = capture.last().toObject();
        QCOMPARE(paint.value("input_count").toInt(), 1);
        QCOMPARE(paint.value("gpu_composite_ms").toDouble(), 1.0);
        QVERIFY(paint.contains("interval_ms"));
    }
}

void ControlMetricsTest::frameGpuPartialDamage()
{
    auto* canvas = showCanvas();
    canvas->setZoom(3.0, canvas->rect().center());
    canvas->setRotation(30.0);
    canvas->repaint();
    frames(true);
    canvas->seedFrameGpuSampleForTest(QRect(5, 5, 3, 3), 2'000'000);
    canvas->seedFrameGpuSampleForTest(QRect(23, 23, 3, 3), 1'000'000);
    const QPoint hit = canvas->imageToWidget(QPointF(6, 6)).toPoint();
    const QPoint miss = canvas->imageToWidget(QPointF(16, 16)).toPoint();
    canvas->repaint(QRect(miss - QPoint(1, 1), QSize(3, 3)));
    QVERIFY(!frames().last().toObject().contains("gpu_composite_ms"));
    canvas->repaint(QRect(hit - QPoint(1, 1), QSize(3, 3)));
    QCOMPARE(frames().last().toObject().value("gpu_composite_ms").toDouble(), 2.0);
    canvas->repaint(QRect(hit - QPoint(1, 1), QSize(3, 3)));
    QVERIFY(!frames().last().toObject().contains("gpu_composite_ms"));
    const QPoint other = canvas->imageToWidget(QPointF(24, 24)).toPoint();
    canvas->repaint(QRect(other - QPoint(1, 1), QSize(3, 3)));
    QCOMPARE(frames().last().toObject().value("gpu_composite_ms").toDouble(), 1.0);
    canvas->repaint(QRect(other - QPoint(1, 1), QSize(3, 3)));
    QVERIFY(!frames().last().toObject().contains("gpu_composite_ms"));
}

void ControlMetricsTest::frameResetClearsPendingTiming()
{
    auto* canvas = showCanvas();
    canvas->repaint();
    canvas->panBy(QPointF(1, 0));
    canvas->seedFrameGpuSampleForTest(QRect(0, 0, 32, 32), 1'000'000);
    QCOMPARE(frames(true).size(), 1);
    QVERIFY(frames().isEmpty());
    canvas->repaint();
    const auto row = frames().last().toObject();
    QVERIFY(!row.contains("interval_ms"));
    QVERIFY(!row.contains("gpu_composite_ms"));
    QCOMPARE(row.value("input_count").toInt(), 0);
}

QTEST_MAIN(ControlMetricsTest)
#include "tst_control_metrics.moc"
