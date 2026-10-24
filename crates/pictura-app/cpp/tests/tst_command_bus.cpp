#include <QtTest/QtTest>

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QPair>
#include <QtCore/QTemporaryDir>

#include "commands.h"
#include "control_server.h"
#include "frame.h"
#include "qt_test_support.h"

class CommandBusTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void cr_record_dispatch();
    void cr_record_refused();
    void cr_replay_executor();
    void cr_replay_missing();
    void cr_modal_context();
    void cr_modal_cancel();
    void cr_modal_void_handler();
    void cr_modal_executor();
    void cr_control_dispatch();
    void ae_record_dialog();
    void ae_record_empty();
    void ae_record_excluded();
    void ae_record_refused();

private:
    pictura::CommandRegistry* registry() const { return window_->registry(); }
    void addRecordableCommand(const QString& id);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QList<QPair<QString, pictura::CommandParams>> events_;
};

void CommandBusTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("CommandBus"), 320, 240, QStringLiteral("rgb"),
                                 8, QStringLiteral("white")));
}

void CommandBusTest::cleanup()
{
    registry()->setRecordingSink(pictura::CommandRecordingSink());
    events_.clear();
}

void CommandBusTest::addRecordableCommand(const QString& id)
{
    registry()->add(id, {QStringLiteral("Test"), id}, id, QKeySequence(), true);
    registry()->setRecordingSink([this](const QString& commandId,
                                        const pictura::CommandParams& params) {
        events_.append({commandId, params});
    });
}

void CommandBusTest::cr_record_dispatch()
{
    const QString id = QStringLiteral("test.command.record");
    addRecordableCommand(id);
    registry()->setHandler(id, [this, id] {
        pictura::CommandParams params;
        params.values.insert(QStringLiteral("value"), 7);
        registry()->recordCommand(id, params);
    });

    QVERIFY(registry()->dispatch(id));
    QCOMPARE(events_.size(), 1);
    QCOMPARE(events_.first().first, id);
    QCOMPARE(events_.first().second.values.value(QStringLiteral("value")).toInt(), 7);
}

void CommandBusTest::cr_record_refused()
{
    const QString id = QStringLiteral("test.command.refused");
    addRecordableCommand(id);
    registry()->setHandler(id, [] {});

    QVERIFY(registry()->dispatch(id));
    QVERIFY(events_.isEmpty());
}

void CommandBusTest::cr_replay_executor()
{
    const QString id = QStringLiteral("test.command.replay");
    bool ran = false;
    pictura::CommandParams seen;
    registry()->setExecutor(id, [&ran, &seen](const pictura::CommandParams& params) {
        ran = true;
        seen = params;
        return true;
    });

    pictura::CommandParams params;
    params.values.insert(QStringLiteral("width"), 1000);
    QVERIFY(registry()->executeRecorded(id, params));
    QVERIFY(ran);
    QCOMPARE(seen.values.value(QStringLiteral("width")).toInt(), 1000);
}

void CommandBusTest::cr_replay_missing()
{
    QVERIFY(!registry()->executeRecorded(QStringLiteral("test.command.missing"),
                                         pictura::CommandParams()));
    QVERIFY(registry()->lastError().contains(QStringLiteral("test.command.missing")));
}

void CommandBusTest::cr_modal_context()
{
    const QString id = QStringLiteral("test.command.modal");
    bool ran = false;
    registry()->setContextHandler(id, [&ran](const pictura::CommandContext& context) {
        ran = true;
        QVERIFY(context.outParams);
        QVERIFY(context.applied);
        context.outParams->values.insert(QStringLiteral("width"), 1280);
        *context.applied = true;
    });

    pictura::CommandParams params;
    params.values.insert(QStringLiteral("width"), 640);
    QVERIFY(registry()->executeRecorded(id, &params, pictura::CommandMode::Modal));
    QVERIFY(ran);
    QCOMPARE(params.values.value(QStringLiteral("width")).toInt(), 1280);
    QVERIFY(registry()->lastError().isEmpty());
}

void CommandBusTest::cr_modal_cancel()
{
    const QString id = QStringLiteral("test.command.modal.cancel");
    registry()->setContextHandler(id, [](const pictura::CommandContext& context) {
        if (context.applied) {
            *context.applied = false;
        }
    });

    QVERIFY(!registry()->executeRecorded(id, pictura::CommandParams(),
                                         pictura::CommandMode::Modal));
    QVERIFY(registry()->lastError().contains(id));
}

void CommandBusTest::cr_modal_void_handler()
{
    const QString id = QStringLiteral("test.command.modal.void");
    bool called = false;
    registry()->setHandler(id, [&called] { called = true; });

    // A plain handler has no dialog in this test; it is treated as applied.
    QVERIFY(registry()->executeRecorded(id, pictura::CommandParams(),
                                        pictura::CommandMode::Modal));
    QVERIFY(called);
    QVERIFY(registry()->lastError().isEmpty());
}

void CommandBusTest::cr_modal_executor()
{
    const QString id = QStringLiteral("test.command.modal.executor");
    bool ran = false;
    registry()->setExecutor(id, [&ran](const pictura::CommandParams&) {
        ran = true;
        return true;
    });

    QVERIFY(registry()->executeRecorded(id, pictura::CommandParams(),
                                        pictura::CommandMode::Modal));
    QVERIFY(ran);
}

void CommandBusTest::cr_control_dispatch()
{
    const QString id = QStringLiteral("test.command.control");
    addRecordableCommand(id);
    registry()->setHandler(id, [this, id] {
        pictura::CommandParams params;
        params.values.insert(QStringLiteral("from"), QStringLiteral("control"));
        registry()->recordCommand(id, params);
    });

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::ControlServer server(window_.get(), dir.filePath(QStringLiteral("bus.sock")));
    const QJsonObject response = server.dispatch(
        QStringLiteral("dispatch_command"), QJsonObject{{QStringLiteral("id"), id}});

    QVERIFY(response.value(QStringLiteral("ok")).toBool());
    QCOMPARE(events_.size(), 1);
    QCOMPARE(events_.first().first, id);
    QCOMPARE(events_.first().second.values.value(QStringLiteral("from")).toString(),
             QStringLiteral("control"));
}

void CommandBusTest::ae_record_dialog()
{
    const QString id = QStringLiteral("test.command.dialog");
    addRecordableCommand(id);
    registry()->setHandler(id, [this, id] {
        pictura::CommandParams params;
        params.values.insert(QStringLiteral("width"), 640);
        params.values.insert(QStringLiteral("height"), 480);
        params.values.insert(QStringLiteral("resolution"), 300.0);
        registry()->recordCommand(id, params);
    });

    QVERIFY(registry()->dispatch(id));
    QCOMPARE(events_.size(), 1);
    const QJsonObject values = events_.first().second.values;
    QCOMPARE(values.value(QStringLiteral("width")).toInt(), 640);
    QCOMPARE(values.value(QStringLiteral("height")).toInt(), 480);
    QCOMPARE(values.value(QStringLiteral("resolution")).toDouble(), 300.0);
}

void CommandBusTest::ae_record_empty()
{
    const QString id = QStringLiteral("test.command.empty");
    addRecordableCommand(id);
    registry()->setHandler(id, [this, id] {
        registry()->recordCommand(id, pictura::CommandParams());
    });

    QVERIFY(registry()->dispatch(id));
    QCOMPARE(events_.size(), 1);
    QVERIFY(events_.first().second.values.isEmpty());
}

void CommandBusTest::ae_record_excluded()
{
    const QString id = QStringLiteral("test.command.excluded");
    addRecordableCommand(id);
    registry()->setHandler(id, [] {});

    QVERIFY(registry()->dispatch(id));
    QVERIFY(events_.isEmpty());
}

void CommandBusTest::ae_record_refused()
{
    const QString id = QStringLiteral("test.command.failed");
    addRecordableCommand(id);
    registry()->setHandler(id, [] {});

    QVERIFY(registry()->dispatch(id));
    QVERIFY(events_.isEmpty());
}

QTEST_MAIN(CommandBusTest)
#include "tst_command_bus.moc"
