// The control server's request queue: order, limits, and a client that leaves
// mid-request. These go through a real socket, because the queue sits in front
// of the in-process dispatch().

#include <QtTest/QtTest>

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtNetwork/QLocalSocket>

#include "control_server.h"
#include "frame.h"
#include "qt_test_support.h"

namespace {

constexpr int kTimeoutMs = 10000;

} // namespace

class ControlSerialTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void repliesKeepRequestOrder();
    void queueOverItsLimitIsRefused();
    void fifthConnectionIsClosed();
    void disconnectDuringARequestIsSafe();

private:
    QByteArray line(int id, const QString& method, const QJsonObject& params = {}) const;
    bool connectClient(QLocalSocket& client) const;
    QList<QJsonObject> readReplies(QLocalSocket& client, int count) const;

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
    QString documentPath_;
};

void ControlSerialTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Serial"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    documentPath_ = dir_.filePath(QStringLiteral("serial.psd"));
    QVERIFY(window_->saveActiveAs(documentPath_));
}

void ControlSerialTest::init()
{
    server_ = std::make_unique<pictura::ControlServer>(
        window_.get(), dir_.filePath(QStringLiteral("control.sock")));
    QVERIFY(server_->listen());
}

void ControlSerialTest::cleanup()
{
    server_.reset();
}

QByteArray ControlSerialTest::line(int id, const QString& method, const QJsonObject& params) const
{
    const QJsonObject request{{QStringLiteral("id"), id},
                              {QStringLiteral("method"), method},
                              {QStringLiteral("params"), params},
                              {QStringLiteral("token"), QString::fromLatin1(server_->token())}};
    return QJsonDocument(request).toJson(QJsonDocument::Compact) + '\n';
}

bool ControlSerialTest::connectClient(QLocalSocket& client) const
{
    client.connectToServer(server_->socketPath());
    return client.waitForConnected(kTimeoutMs);
}

QList<QJsonObject> ControlSerialTest::readReplies(QLocalSocket& client, int count) const
{
    QList<QJsonObject> replies;
    QElapsedTimer timer;
    timer.start();
    while (replies.size() < count && timer.elapsed() < kTimeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
        while (client.canReadLine()) {
            replies.append(QJsonDocument::fromJson(client.readLine()).object());
        }
    }
    return replies;
}

void ControlSerialTest::repliesKeepRequestOrder()
{
    const int before = window_->documentCount();
    QLocalSocket client;
    QVERIFY(connectClient(client));
    client.write(line(1, QStringLiteral("document"),
                      {{QStringLiteral("op"), QStringLiteral("open")},
                       {QStringLiteral("path"), documentPath_}})
                 + line(2, QStringLiteral("status")) + line(3, QStringLiteral("list_layers")));
    client.flush();

    const QList<QJsonObject> replies = readReplies(client, 3);
    QCOMPARE(replies.size(), 3);
    for (int i = 0; i < 3; ++i) {
        QCOMPARE(replies.at(i).value(QStringLiteral("id")).toInt(), i + 1);
        QVERIFY(replies.at(i).value(QStringLiteral("ok")).toBool());
    }
    const int seen = replies.at(1)
                         .value(QStringLiteral("result"))
                         .toObject()
                         .value(QStringLiteral("documents"))
                         .toInt();
    QVERIFY(seen >= before);
    QCOMPARE(seen, window_->documentCount());
}

void ControlSerialTest::queueOverItsLimitIsRefused()
{
    const int total = pictura::ControlServer::kMaxQueuedPerConnection + 6;
    QLocalSocket client;
    QVERIFY(connectClient(client));
    QByteArray burst;
    for (int i = 0; i < total; ++i) {
        burst += line(i, QStringLiteral("status"));
    }
    client.write(burst);
    client.flush();

    const QList<QJsonObject> replies = readReplies(client, total);
    QCOMPARE(replies.size(), total);
    int served = 0;
    int refused = 0;
    for (const QJsonObject& reply : replies) {
        if (reply.value(QStringLiteral("ok")).toBool()) {
            ++served;
        } else {
            QCOMPARE(reply.value(QStringLiteral("error")).toObject().value(QStringLiteral("code")).toString(),
                     QStringLiteral("refused"));
            ++refused;
        }
    }
    QVERIFY(served >= pictura::ControlServer::kMaxQueuedPerConnection);
    QCOMPARE(served + refused, total);
}

void ControlSerialTest::fifthConnectionIsClosed()
{
    std::vector<std::unique_ptr<QLocalSocket>> clients;
    for (int i = 0; i < pictura::ControlServer::kMaxConnections; ++i) {
        clients.push_back(std::make_unique<QLocalSocket>());
        QVERIFY(connectClient(*clients.back()));
        // A reply proves the server has taken this client, so the next counts.
        clients.back()->write(line(0, QStringLiteral("status")));
        clients.back()->flush();
        QCOMPARE(readReplies(*clients.back(), 1).size(), 1);
    }
    QLocalSocket extra;
    extra.connectToServer(server_->socketPath());
    QTRY_COMPARE_WITH_TIMEOUT(extra.state(), QLocalSocket::UnconnectedState, kTimeoutMs);

    for (const auto& client : clients) {
        client->write(line(1, QStringLiteral("status")));
        client->flush();
        const QList<QJsonObject> replies = readReplies(*client, 1);
        QCOMPARE(replies.size(), 1);
        QVERIFY(replies.first().value(QStringLiteral("ok")).toBool());
    }
}

void ControlSerialTest::disconnectDuringARequestIsSafe()
{
    {
        QLocalSocket leaving;
        QVERIFY(connectClient(leaving));
        leaving.write(line(1, QStringLiteral("document"),
                           {{QStringLiteral("op"), QStringLiteral("open")},
                            {QStringLiteral("path"), documentPath_}}));
        leaving.flush();
        leaving.abort();
    }
    // Time for the server to run the request and see the disconnect mid-load.
    QTest::qWait(200);

    QLocalSocket next;
    QVERIFY(connectClient(next));
    next.write(line(2, QStringLiteral("status")));
    next.flush();
    const QList<QJsonObject> replies = readReplies(next, 1);
    QCOMPARE(replies.size(), 1);
    QVERIFY(replies.first().value(QStringLiteral("ok")).toBool());
}

QTEST_MAIN(ControlSerialTest)
#include "tst_control_serial.moc"
