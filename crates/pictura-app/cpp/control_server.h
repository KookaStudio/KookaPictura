#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QObject>
#include <QtCore/QString>

class QLocalServer;
class QLocalSocket;

namespace pictura {

class PicturaMainWindow;

// Opt-in JSON control endpoint for agentic testing. Serves newline-delimited
// requests on a Unix-domain socket and dispatches on the Qt GUI thread.
//
// ponytail: requests block the GUI thread while a method runs; fine for a test
// surface. Upgrade path is app-wide off-GUI-thread compute.
class ControlServer : public QObject {
    Q_OBJECT

public:
    ControlServer(PicturaMainWindow* frame, const QString& socketPath,
                  QObject* parent = nullptr);
    ~ControlServer() override;

    // Path used when the constructor is given an empty path.
    static QString defaultSocketPath();

    // Upper bound on an unterminated request line held per socket. A larger
    // buffer is rejected with `bad_request` and the socket is disconnected.
    static constexpr int kMaxRequestBytes = 1024 * 1024;

    // ponytail: caller-supplied image ceilings at the trust boundary.
    // `layer_thumbnail` allocates size^2 in the engine (u32 overflow/OOB for
    // size >= 65536), so cap it; `max_dim` bounds the screenshot response size
    // so a hostile value cannot bypass the guard.
    static constexpr int kMaxThumbnailDim = 1024;
    static constexpr int kMaxScreenshotDim = 4096;

    // Listen on the socket and write a fresh session token to tokenPath(),
    // readable by the owner only. Fails, listening on nothing, when either
    // cannot be created.
    bool listen();
    QString socketPath() const { return socketPath_; }

    // Every socket request must carry this session token as its `token`
    // member, so another process of the same user that can reach the socket
    // but cannot read the owner-only token file gets no file read/write
    // oracle (#844 M5). `dispatch()` is in-process and needs none.
    QString tokenPath() const;
    QByteArray token() const { return token_; }

    // Dispatch one method call; returns the full {ok,result} or {ok,error}
    // wrapper. Also drives the in-process self-test block.
    QJsonObject dispatch(const QString& method, const QJsonObject& params);

public slots:
    // Close the server and remove the socket. Idempotent.
    void shutdown();

private slots:
    void onNewConnection();
    void onReadyRead();
    void onDisconnected();

private:
    QJsonObject ok(const QJsonObject& result) const;
    QJsonObject error(const QString& code, const QString& message) const;
    QJsonObject methodStatus(const QJsonObject& params);
    QJsonObject methodGetPixel(const QJsonObject& params);
    QJsonObject methodListLayers(const QJsonObject& params);
    QJsonObject methodListCommands(const QJsonObject& params);
    QJsonObject methodDispatchCommand(const QJsonObject& params);
    QJsonObject methodDocument(const QJsonObject& params);
    QJsonObject methodEdit(const QJsonObject& params);
    QJsonObject methodSetUnsavedPolicy(const QJsonObject& params);
    QJsonObject methodScreenshot(const QJsonObject& params);
    QJsonObject methodUiTree(const QJsonObject& params);
    QJsonObject methodLayerThumbnail(const QJsonObject& params);
    QJsonObject methodSetTool(const QJsonObject& params);
    QJsonObject methodToolOptions(const QJsonObject& params);
    QJsonObject methodSelection(const QJsonObject& params);
    QJsonObject methodFilter(const QJsonObject& params);
    QJsonObject methodAdjustment(const QJsonObject& params);
    QJsonObject methodLayerOp(const QJsonObject& params);
    QJsonObject methodSetGpuCompute(const QJsonObject& params);
    QJsonObject methodPointer(const QJsonObject& params);
    QJsonObject methodKey(const QJsonObject& params);

    void handleLine(QLocalSocket* socket, const QByteArray& line);
    bool writeTokenFile();
    bool authorized(const QJsonObject& request) const;
    void writeLine(QLocalSocket* socket, const QJsonObject& response);

    PicturaMainWindow* frame_;
    QLocalServer* server_ = nullptr;
    QString socketPath_;
    QByteArray token_;
    bool tokenWritten_ = false;
    QHash<QLocalSocket*, QByteArray> buffers_;
    // Tracked unsaved-prompt policy, mirrored from the global set in dialogs.cpp.
    bool unsavedInteractive_ = false;
    QString unsavedChoice_ = QStringLiteral("discard");
    // The button a `pointer` `down` holds until its `up`, carried by `move`s
    // in between so a client can time each move of a drag on its own.
    Qt::MouseButtons heldButtons_ = Qt::NoButton;
};

} // namespace pictura
