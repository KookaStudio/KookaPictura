#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

enum class ControlTier { Read, Edit, Save, Admin };

// When a restricted session asks before touching a file. See `judgePath`.
enum class ApprovalMode { Manual, AutoApprove, AllowAll };

enum class PathVerdict { Allow, Ask, Refuse };

// What one control session may do. The default is bare `--control`: no limits,
// because a process started that way has no person's work to protect.
struct ControlPolicy {
    bool restricted = false;
    // A person is at this window: requests can ask them, and must not disturb them.
    bool live = false;
    bool allowEdit = false;
    bool allowSave = false;
    ApprovalMode approval = ApprovalMode::AutoApprove;
    QStringList roots;
    QStringList denied;

    bool grants(ControlTier tier) const;
};

// What no session may touch in any mode: the app's state, which holds these
// permissions, and the endpoint's directory, which holds the token.
QStringList protectedDirectories(const QString& stateFile, const QString& endpoint);

// The canonical form of `path`, so a symlinked directory is judged by where it
// lands. A path that does not exist yet resolves through its parent.
QString canonicalTarget(const QString& path);

// `target` is canonical. `known` is a file the agent made, or the person
// approved, this session. A session that cannot ask refuses instead.
//
// ponytail: a same-user race between this check and the use, and hardlinks, are
// out of reach of a path check.
PathVerdict judgePath(const ControlPolicy& policy, const QString& target, bool writes,
                      bool exists, bool known);

// Commands a restricted session may dispatch: dialog-free, and touching neither
// files nor the app's settings. Default-deny, so a new command stays out until
// someone lists it.
bool commandAllowed(const QString& id);
QStringList allowedCommands();

} // namespace pictura
