#pragma once

#include <QtCore/QCoreApplication>
#include <QtCore/QElapsedTimer>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtNetwork/QLocalSocket>

#include "control_server.h"

namespace pictura::test {

// A client of a real ControlServer socket, for what in-process dispatch() skips.
class ControlClient {
public:
    static constexpr int kTimeoutMs = 10000;

    explicit ControlClient(ControlServer& server)
        : token_(QString::fromLatin1(server.token()))
    {
        socket_.connectToServer(server.socketPath());
        socket_.waitForConnected(kTimeoutMs);
    }

    bool connected() const { return socket_.state() == QLocalSocket::ConnectedState; }
    void abort() { socket_.abort(); }

    void send(int id, const QString& method, const QJsonObject& params = {})
    {
        const QJsonObject request{{QStringLiteral("id"), id},
                                  {QStringLiteral("method"), method},
                                  {QStringLiteral("params"), params},
                                  {QStringLiteral("token"), token_}};
        socket_.write(QJsonDocument(request).toJson(QJsonDocument::Compact) + '\n');
        socket_.flush();
    }

    void sendRaw(const QByteArray& bytes)
    {
        socket_.write(bytes);
        socket_.flush();
    }

    // One reply, or an empty object after `timeoutMs`.
    QJsonObject read(int timeoutMs = kTimeoutMs)
    {
        QElapsedTimer timer;
        timer.start();
        while (!socket_.canReadLine() && timer.elapsed() < timeoutMs) {
            QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        }
        return socket_.canReadLine() ? QJsonDocument::fromJson(socket_.readLine()).object()
                                     : QJsonObject();
    }

    QJsonObject call(const QString& method, const QJsonObject& params = {})
    {
        send(++lastId_, method, params);
        return read();
    }

private:
    QLocalSocket socket_;
    QString token_;
    int lastId_ = 1000;
};

inline QString errorCode(const QJsonObject& response)
{
    return response.value(QStringLiteral("error"))
        .toObject()
        .value(QStringLiteral("code"))
        .toString();
}

inline bool succeeded(const QJsonObject& response)
{
    return response.value(QStringLiteral("ok")).toBool();
}

} // namespace pictura::test
