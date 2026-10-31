#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QLockFile>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <memory>

#include "control_policy.h"

class QLocalServer;
class QLocalSocket;
class QTimer;

namespace pictura {

class ControlApprovalBar;
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

    // Requests a client may have waiting, and clients served at once.
    // ponytail: with kMaxRequestBytes that pins at most 64 MiB per client and
    // 256 MiB in all; lower the line cap if a tighter memory budget matters.
    static constexpr int kMaxQueuedPerConnection = 64;
    static constexpr int kMaxConnections = 4;

    // Caller-supplied sizes that reach the engine unchecked otherwise.
    static constexpr int kMaxLassoPoints = 4096;
    static constexpr int kMaxNameChars = 255;
    static constexpr int kMaxUiTreeDepth = 64;
    static constexpr int kMaxUiTreeChildren = 1024;

    // Kinds `adjust_apply` takes, each with a default block.
    static QStringList adjustmentKinds();

    // Listen on the socket and write a fresh session token to tokenPath(),
    // readable by the owner only. Fails, listening on nothing, when either
    // cannot be created, or while another live process holds the endpoint.
    bool listen();
    QString socketPath() const { return socketPath_; }
    int clientCount() const { return buffers_.size(); }

    // Every socket request must carry this session token as its `token`
    // member, so another process of the same user that can reach the socket
    // but cannot read the owner-only token file gets no file read/write
    // oracle (#844 M5). `dispatch()` is in-process and needs none.
    QString tokenPath() const;
    static QString tokenPathFor(const QString& endpoint);
    QByteArray token() const { return token_; }

    // What socket requests may do; the default is unrestricted.
    void setPolicy(const ControlPolicy& policy);
    const ControlPolicy& policy() const { return policy_; }
    // The method table, and the tier a request needs (some depend on its op).
    static QStringList methodNames();
    ControlTier tierOf(const QString& method, const QJsonObject& params) const;

    static constexpr int kApprovalTimeoutMs = 60000;
    void setApprovalTimeoutMs(int ms) { approvalTimeoutMs_ = ms; }
    ControlApprovalBar* approvalBar() const;

    // Dispatch one method call; returns the full {ok,result} or {ok,error}
    // wrapper. A dialog the call reaches is cancelled and answered `refused`.
    // Also drives the in-process self-test block.
    QJsonObject dispatch(const QString& method, const QJsonObject& params);

signals:
    // The last connected client left.
    void lastClientLeft();
    void clientsChanged();

public slots:
    // Close the server and remove the socket. Idempotent.
    void shutdown();

private slots:
    void onNewConnection();
    void onReadyRead();
    void onDisconnected();

private:
    using MethodHandler = QJsonObject (ControlServer::*)(const QJsonObject&);
    struct MethodRow {
        const char* name;
        MethodHandler handler;
        ControlTier tier;
    };
    struct PendingApproval {
        QPointer<QLocalSocket> socket;
        int id = 0;
        QString method;
        QJsonObject params;
        QString target;
    };
    struct OpeningRequest {
        QPointer<QLocalSocket> socket;
        int id = 0;
        QString path;
        quint64 token = 0;
    };
    struct QueuedLine {
        QPointer<QLocalSocket> socket;
        QByteArray line;
    };

    static const QList<MethodRow>& methodTable();
    static bool isMethod(const QString& method);
    QJsonObject route(const QString& method, const QJsonObject& params);
    QJsonObject documentResult(bool applied) const;
    QJsonObject admit(const QString& method, const QJsonObject& params);
    QString fileTarget(const QString& method, const QJsonObject& params) const;
    PathVerdict verdictFor(const QString& method, const QJsonObject& params,
                           const QString& target) const;
    void serve(const QPointer<QLocalSocket>& socket, int id, const QString& method,
               const QJsonObject& params);
    void run(const QPointer<QLocalSocket>& socket, int id, const QString& method,
             const QJsonObject& params);
    void reply(const QPointer<QLocalSocket>& socket, int id, QJsonObject response);
    void park(const QPointer<QLocalSocket>& socket, int id, const QString& method,
              const QJsonObject& params, const QString& target);
    PendingApproval takePending();
    void onOpenFinished(quint64 token, int outcome);
    void resolveApproval(bool allow);
    void onApprovalTimeout();
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
    QJsonObject applyFilterWithParams(const QString& kind, const QJsonValue& values);
    QJsonObject methodAdjustment(const QJsonObject& params);
    QJsonObject methodLayerOp(const QJsonObject& params);
    QJsonObject methodSetGpuCompute(const QJsonObject& params);
    QJsonObject methodMetrics(const QJsonObject& params);
    QJsonObject methodPointer(const QJsonObject& params);
    QJsonObject methodKey(const QJsonObject& params);
    QJsonObject methodListFilters(const QJsonObject& params);
    QJsonObject methodListAdjustments(const QJsonObject& params);
    QJsonObject methodAdjustApply(const QJsonObject& params);
    QJsonObject methodRunCommand(const QJsonObject& params);
    QJsonObject methodLayerTree(const QJsonObject& params);
    QJsonObject methodSetActiveLayer(const QJsonObject& params);
    QJsonObject methodExportPng(const QJsonObject& params);
    QJsonObject methodHistory(const QJsonObject& params);
    QJsonObject methodCommitCrop(const QJsonObject& params);

    void pump();
    void handleLine(const QPointer<QLocalSocket>& socket, const QByteArray& line);
    void refuseLine(QLocalSocket* socket, const QByteArray& line);
    QString lockPath() const;
    bool writeTokenFile();
    bool authorized(const QJsonObject& request) const;
    bool acquireLock();
    void releaseLock();
    void writeLine(QLocalSocket* socket, const QJsonObject& response);

    PicturaMainWindow* frame_;
    QLocalServer* server_ = nullptr;
    QString socketPath_;
    QByteArray token_;
    bool tokenWritten_ = false;
#ifdef _WIN32
    std::unique_ptr<QLockFile> lock_;
#else
    int lockFd_ = -1;
#endif
    QHash<QLocalSocket*, QByteArray> buffers_;
    QHash<QLocalSocket*, int> queuedCounts_;
    QList<QueuedLine> queue_;
    // Set while pump() runs, so a nested event loop in a request only enqueues.
    bool running_ = false;
    int lassoPoints_ = 0;
    // This session's `pointer` started the pending crop, so it may finish it.
    bool agentCrop_ = false;
    ControlPolicy policy_;
    // Set while a request waits for a person or a load; later ones stay queued.
    bool parked_ = false;
    PendingApproval pending_;
    OpeningRequest opening_;
    // Files the agent made or the person approved, this session.
    QSet<QString> known_;
    // Owned by the window, which can go first during teardown.
    QPointer<ControlApprovalBar> bar_;
    QTimer* approvalTimer_ = nullptr;
    int approvalTimeoutMs_ = kApprovalTimeoutMs;
    // Tracked unsaved-prompt policy, mirrored from the global set in dialogs.cpp.
    bool unsavedInteractive_ = false;
    QString unsavedChoice_ = QStringLiteral("discard");
    // The button a `pointer` `down` holds until its `up`, carried by `move`s
    // in between so a client can time each move of a drag on its own.
    Qt::MouseButtons heldButtons_ = Qt::NoButton;
};

} // namespace pictura
