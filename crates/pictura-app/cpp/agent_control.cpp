#include "agent_control.h"

#include <QtCore/QCoreApplication>
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
#include "preferences.h"

#include "api/engine_bridge.h"

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
    const auto state = loadPreferences();
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
    // Each Flatpak instance has its own runtime directory, except for this
    // subtree, and the window and the bridge are separate instances.
    const QString flatpak = qEnvironmentVariable("FLATPAK_ID");
    const QString shared = flatpak.isEmpty() ? runtime : runtime + QStringLiteral("/app/") + flatpak;
    const QString directory = !runtime.isEmpty()
        ? shared + QStringLiteral("/pictura")
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

QString AgentControl::launchCommand()
{
    if (qEnvironmentVariableIsSet("FLATPAK_ID")) {
        return QStringLiteral("flatpak");
    }
    if (qEnvironmentVariableIsSet("SNAP")) {
        return QStringLiteral("snap");
    }
    const QString image = qEnvironmentVariable("APPIMAGE");
    return image.isEmpty() ? QCoreApplication::applicationFilePath() : image;
}

QStringList AgentControl::launchArguments()
{
    if (qEnvironmentVariableIsSet("FLATPAK_ID")) {
        return {QStringLiteral("run"), qEnvironmentVariable("FLATPAK_ID"), QStringLiteral("--mcp")};
    }
    if (qEnvironmentVariableIsSet("SNAP")) {
        return {QStringLiteral("run"),
                qEnvironmentVariable("SNAP_INSTANCE_NAME", QStringLiteral("kooka-pictura"))
                    + QStringLiteral(".pictura"),
                QStringLiteral("--mcp")};
    }
    return {QStringLiteral("--mcp")};
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
    const auto current = loadPreferences();
    auto previous = current;
    previous.agentControlEnabled = settings_.enabled;
    previous.agentAllowEdit = settings_.allowEdit;
    previous.agentAllowSave = settings_.allowSave;
    previous.agentApprovalMode = static_cast<int>(settings_.approval);
    previous.agentRoots = settings_.roots;
    auto state = previous;
    state.agentControlEnabled = settings.enabled;
    state.agentAllowEdit = settings.allowEdit;
    state.agentAllowSave = settings.allowSave;
    state.agentApprovalMode = static_cast<int>(settings.approval);
    state.agentRoots = settings.roots;
    const bool granting = settings.enabled && !settings_.enabled;
    const bool saved = savePreferences(previous, state, granting);
    const auto merged = loadPreferences();
    settings_.enabled = merged.agentControlEnabled;
    settings_.allowEdit = merged.agentAllowEdit;
    settings_.allowSave = merged.agentAllowSave;
    settings_.approval = static_cast<ApprovalMode>(merged.agentApprovalMode);
    settings_.roots = merged.agentRoots;
    if (!saved && !merged.agentRecoveryGuard) {
        if (settings.enabled != previous.agentControlEnabled) settings_.enabled = settings.enabled;
        if (settings.allowEdit != previous.agentAllowEdit) settings_.allowEdit = settings.allowEdit;
        if (settings.allowSave != previous.agentAllowSave) settings_.allowSave = settings.allowSave;
        if (static_cast<int>(settings.approval) != previous.agentApprovalMode) settings_.approval = settings.approval;
        if (settings.roots != previous.agentRoots) settings_.roots = settings.roots;
    }
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
    policy.denied = protectedDirectories(sessionFilePath(), endpoint, configurationDirectory());
    return policy;
}

void AgentControl::sync()
{
    syncServer();
    emit statusChanged();
}

void AgentControl::syncServer()
{
    problem_.clear();
    if (!settings_.enabled) {
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
    connect(server.get(), &ControlServer::clientsChanged, this, &AgentControl::statusChanged);
    server_ = std::move(server);
}

} // namespace pictura

#include "moc_agent_control.cpp"
