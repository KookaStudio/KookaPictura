// Control server limits: the live-endpoint lock, the image budget, input caps,
// and a save that would need a dialog.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtNetwork/QLocalSocket>

#ifndef _WIN32
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#endif

#include "control_server.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "qt_test_support.h"

namespace {

QString errorCode(const QJsonObject& response)
{
    return response.value(QStringLiteral("error"))
        .toObject()
        .value(QStringLiteral("code"))
        .toString();
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

} // namespace

class ControlLimitsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void saveNeedingAFormatDialogIsRefused();
    void secondServerRefusesALiveEndpoint();
    void staleEndpointIsReclaimed();
    void oversizedDocumentIsRefused();
    void documentOverTheByteLimitIsRefused();
    void lassoPointsAreCapped();
    void longNamesAreRefused();
    void uiTreeLimitsAreEnforced();

private:
    QJsonObject call(const QString& method, const QJsonObject& params = {});
    void expectRefusedNew(int width, int height);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlLimitsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Limits"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    server_ = std::make_unique<pictura::ControlServer>(window_.get(), QString());
}

QJsonObject ControlLimitsTest::call(const QString& method, const QJsonObject& params)
{
    return server_->dispatch(method, params);
}

void ControlLimitsTest::expectRefusedNew(int width, int height)
{
    const int count = window_->documentCount();
    const QJsonObject response =
        call(QStringLiteral("document"), {{QStringLiteral("op"), QStringLiteral("new")},
                                          {QStringLiteral("width"), width},
                                          {QStringLiteral("height"), height}});
    QCOMPARE(errorCode(response), QStringLiteral("invalid_param"));
    QCOMPARE(window_->documentCount(), count);
}

void ControlLimitsTest::saveNeedingAFormatDialogIsRefused()
{
    const QString path = dir_.filePath(QStringLiteral("flat.png"));
    QVERIFY(window_->saveActiveAs(path));
    QVERIFY(call(QStringLiteral("layer_op"),
                 {{QStringLiteral("op"), QStringLiteral("add")}, {QStringLiteral("index"), 0}})
                .value(QStringLiteral("ok"))
                .toBool());
    const QByteArray before = fileBytes(path);
    QVERIFY(!before.isEmpty());

    const QJsonObject response =
        call(QStringLiteral("document"), {{QStringLiteral("op"), QStringLiteral("save")}});
    QCOMPARE(errorCode(response), QStringLiteral("refused"));
    QCOMPARE(fileBytes(path), before);
}

void ControlLimitsTest::secondServerRefusesALiveEndpoint()
{
    const QString socketPath = dir_.filePath(QStringLiteral("live.sock"));
    pictura::ControlServer first(window_.get(), socketPath);
    QVERIFY(first.listen());
    const QByteArray token = fileBytes(first.tokenPath());
    QVERIFY(!token.isEmpty());

    pictura::ControlServer second(window_.get(), socketPath);
    QVERIFY(!second.listen());
    QCOMPARE(fileBytes(first.tokenPath()), token);

    QLocalSocket client;
    client.connectToServer(socketPath);
    QVERIFY(client.waitForConnected(5000));
    const QJsonObject request{{QStringLiteral("id"), 1},
                              {QStringLiteral("method"), QStringLiteral("status")},
                              {QStringLiteral("token"), QString::fromLatin1(first.token())}};
    client.write(QJsonDocument(request).toJson(QJsonDocument::Compact) + '\n');
    client.flush();
    QTRY_VERIFY_WITH_TIMEOUT(client.canReadLine(), 5000);
    QVERIFY(QJsonDocument::fromJson(client.readLine())
                .object()
                .value(QStringLiteral("ok"))
                .toBool());
}

void ControlLimitsTest::staleEndpointIsReclaimed()
{
#ifdef _WIN32
    QSKIP("a named pipe leaves no file behind");
#else
    const QString socketPath = dir_.filePath(QStringLiteral("stale.sock"));
    const QByteArray native = socketPath.toLocal8Bit();
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    QVERIFY(size_t(native.size()) < sizeof(address.sun_path));
    std::memcpy(address.sun_path, native.constData(), size_t(native.size()));
    const int fd = ::socket(AF_UNIX, SOCK_STREAM, 0);
    QVERIFY(fd >= 0);
    QCOMPARE(::bind(fd, reinterpret_cast<const sockaddr*>(&address), sizeof(address)), 0);
    ::close(fd);
    QVERIFY(QFile::exists(socketPath));

    pictura::ControlServer server(window_.get(), socketPath);
    QVERIFY(server.listen());
#endif
}

void ControlLimitsTest::oversizedDocumentIsRefused()
{
    expectRefusedNew(100000, 100000);
}

void ControlLimitsTest::documentOverTheByteLimitIsRefused()
{
    expectRefusedNew(30000, 30000);
}

void ControlLimitsTest::lassoPointsAreCapped()
{
    QVERIFY(call(QStringLiteral("selection"), {{QStringLiteral("op"), QStringLiteral("lasso_begin")}})
                .value(QStringLiteral("ok"))
                .toBool());
    const QJsonObject point{{QStringLiteral("op"), QStringLiteral("lasso_point")},
                            {QStringLiteral("x"), 1},
                            {QStringLiteral("y"), 1}};
    for (int i = 0; i < pictura::ControlServer::kMaxLassoPoints; ++i) {
        QVERIFY(call(QStringLiteral("selection"), point).value(QStringLiteral("ok")).toBool());
    }
    QCOMPARE(errorCode(call(QStringLiteral("selection"), point)), QStringLiteral("invalid_param"));
    window_->activeView()->cancel_lasso();
}

void ControlLimitsTest::longNamesAreRefused()
{
    const QString longName(pictura::ControlServer::kMaxNameChars + 1, QLatin1Char('n'));
    const int count = window_->documentCount();
    QCOMPARE(errorCode(call(QStringLiteral("document"),
                            {{QStringLiteral("op"), QStringLiteral("new")},
                             {QStringLiteral("width"), 8},
                             {QStringLiteral("height"), 8},
                             {QStringLiteral("name"), longName}})),
             QStringLiteral("invalid_param"));
    QCOMPARE(window_->documentCount(), count);

    const QString before = window_->activeView()->layer_name(0);
    QCOMPARE(errorCode(call(QStringLiteral("layer_op"), {{QStringLiteral("op"), QStringLiteral("set_name")},
                                                         {QStringLiteral("index"), 0},
                                                         {QStringLiteral("name"), longName}})),
             QStringLiteral("invalid_param"));
    QCOMPARE(window_->activeView()->layer_name(0), before);
}

void ControlLimitsTest::uiTreeLimitsAreEnforced()
{
    QCOMPARE(errorCode(call(QStringLiteral("ui_tree"),
                            {{QStringLiteral("max_depth"), pictura::ControlServer::kMaxUiTreeDepth + 1}})),
             QStringLiteral("invalid_param"));
    QCOMPARE(errorCode(call(QStringLiteral("ui_tree"),
                            {{QStringLiteral("max_children"),
                              pictura::ControlServer::kMaxUiTreeChildren + 1}})),
             QStringLiteral("invalid_param"));
    QVERIFY(call(QStringLiteral("ui_tree"), {{QStringLiteral("max_depth"), 1}})
                .value(QStringLiteral("ok"))
                .toBool());
}

QTEST_MAIN(ControlLimitsTest)
#include "tst_control_limits.moc"
