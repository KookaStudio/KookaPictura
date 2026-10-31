#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <memory>

#include "control_policy.h"

namespace pictura {

class ControlServer;
class PicturaMainWindow;

struct AgentSettings {
    bool enabled = false;
    bool allowEdit = false;
    bool allowSave = false;
    ApprovalMode approval = ApprovalMode::AutoApprove;
    QStringList roots;

    bool operator==(const AgentSettings& other) const
    {
        return enabled == other.enabled && allowEdit == other.allowEdit
            && allowSave == other.allowSave && approval == other.approval && roots == other.roots;
    }
};

// The control server of a window a person is using, off until they enable it.
// Lives beside the frame rather than in it: saveSession() patches the stored
// state, so these keys survive without the frame knowing them.
class AgentControl : public QObject {
public:
    // `endpoint` overrides liveEndpoint() for tests.
    explicit AgentControl(PicturaMainWindow* frame, const QString& endpoint = QString());
    ~AgentControl() override;

    // Per user, so an MCP client can find it without being told. Empty, with
    // `problem` set, when no safe directory exists for it.
    static QString liveEndpoint(QString* problem);
    static bool isSandboxed();
    // `pictura --mcp`: serve MCP on stdio, and return the exit code. Runs before
    // QApplication. `PICTURA_CONTROL_SOCKET` overrides the endpoint.
    static int runBridge(const QStringList& args);

    const AgentSettings& settings() const { return settings_; }
    // Persist `settings`, then start, stop, or re-apply them to a running server.
    void apply(const AgentSettings& settings);

    bool isRunning() const { return server_ != nullptr; }
    // Why the server is not running although it is enabled; empty otherwise.
    QString problem() const { return problem_; }
    ControlServer* server() const { return server_.get(); }

private:
    void sync();
    ControlPolicy policy(const QString& endpoint) const;

    PicturaMainWindow* frame_;
    QString endpoint_;
    AgentSettings settings_;
    QString problem_;
    std::unique_ptr<ControlServer> server_;
};

} // namespace pictura
