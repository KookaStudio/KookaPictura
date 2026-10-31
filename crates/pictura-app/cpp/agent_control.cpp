#include "agent_control.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QRegularExpression>

#ifndef _WIN32
#include <sys/stat.h>
#include <unistd.h>
#endif

#include "control_server.h"
#include "frame.h"
#include "session.h"

#include "pictura_app/src/mcp_entry.cxxqt.h"

namespace pictura {

namespace {

#ifndef _WIN32
// A world-writable fallback can be planted, so the directory must be ours alone.
bool isPrivateDirectory(const QByteArray& path)
{
    ::mkdir(path.constData(), 0700);
    struct stat info{};
    return ::lstat(path.constData(), &info) == 0 && S_ISDIR(info.st_mode)
        && info.st_uid == ::getuid() && (info.st_mode & 0077) == 0;
}
#endif

} // namespace

AgentControl::AgentControl(PicturaMainWindow* frame, const QString& endpoint)
    : QObject(frame)
    , frame_(frame)
    , endpoint_(endpoint)
{
    const SessionState state = loadSession();
    settings_.enabled = state.agentControlEnabled;
    settings_.allowEdit = state.agentAllowEdit;
    settings_.allowSave = state.agentAllowSave;
    settings_.approval = static_cast<ApprovalMode>(state.agentApprovalMode);
    settings_.roots = state.agentRoots;
    sync();
}

AgentControl::~AgentControl() = default;

bool AgentControl::isSandboxed()
{
    return qEnvironmentVariableIsSet("FLATPAK_ID") || qEnvironmentVariableIsSet("SNAP");
}

QString AgentControl::liveEndpoint(QString* problem)
{
#ifdef _WIN32
    Q_UNUSED(problem);
    QString user = qEnvironmentVariable("USERNAME");
    user.remove(QRegularExpression(QStringLiteral("[^A-Za-z0-9_-]")));
    return QStringLiteral("pictura-live-%1").arg(user);
#else
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QString directory = !runtime.isEmpty()
        ? runtime + QStringLiteral("/pictura")
        : QDir::tempPath() + QStringLiteral("/pictura-%1").arg(::getuid());
    if (!isPrivateDirectory(directory.toLocal8Bit())) {
        if (problem) {
            *problem = QStringLiteral("%1 is not a private directory").arg(directory);
        }
        return {};
    }
    return directory + QStringLiteral("/live.sock");
#endif
}

int AgentControl::runBridge(const QStringList& args)
{
    QString socket = qEnvironmentVariable("PICTURA_CONTROL_SOCKET");
    if (socket.isEmpty()) {
        socket = liveEndpoint(nullptr);
    }
    return mcp_run(socket, ControlServer::tokenPathFor(socket), args);
}

void AgentControl::apply(const AgentSettings& settings)
{
    if (settings == settings_) {
        return;
    }
    settings_ = settings;
    SessionState state = loadSession();
    state.agentControlEnabled = settings.enabled;
    state.agentAllowEdit = settings.allowEdit;
    state.agentAllowSave = settings.allowSave;
    state.agentApprovalMode = static_cast<int>(settings.approval);
    state.agentRoots = settings.roots;
    saveSession(state);
    sync();
}

ControlPolicy AgentControl::policy(const QString& endpoint) const
{
    ControlPolicy policy;
    policy.restricted = true;
    policy.live = true;
    policy.allowEdit = settings_.allowEdit;
    policy.allowSave = settings_.allowSave;
    policy.approval = settings_.approval;
    for (const QString& root : settings_.roots) {
        const QString canonical = QFileInfo(root).canonicalFilePath();
        if (!canonical.isEmpty()) {
            policy.roots.append(canonical);
        }
    }
    policy.denied = protectedDirectories(sessionFilePath(), endpoint);
    return policy;
}

void AgentControl::sync()
{
    problem_.clear();
    if (!settings_.enabled) {
        server_.reset();
        return;
    }
    if (isSandboxed()) {
        problem_ = tr("Agent control is not available in this sandboxed build.");
        server_.reset();
        return;
    }
    if (server_) {
        server_->setPolicy(policy(server_->socketPath()));
        return;
    }
    const QString endpoint = endpoint_.isEmpty() ? liveEndpoint(&problem_) : endpoint_;
    if (endpoint.isEmpty()) {
        return;
    }
    auto server = std::make_unique<ControlServer>(frame_, endpoint);
    server->setPolicy(policy(endpoint));
    if (!server->listen()) {
        problem_ = tr("Another Kooka Pictura window is serving agents.");
        return;
    }
    server_ = std::move(server);
}

} // namespace pictura
