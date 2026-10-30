#include "control_policy.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>

namespace pictura {

namespace {

Qt::CaseSensitivity pathCase()
{
#if defined(_WIN32) || defined(__APPLE__)
    return Qt::CaseInsensitive;
#else
    return Qt::CaseSensitive;
#endif
}

// At a separator boundary, so a root `/a/b` does not hold `/a/b2/x`.
bool isInside(const QString& target, const QString& directory)
{
    QString root = QDir::cleanPath(directory);
    if (target.compare(root, pathCase()) == 0) {
        return true;
    }
    if (!root.endsWith(QLatin1Char('/'))) {
        root += QLatin1Char('/');
    }
    return target.startsWith(root, pathCase());
}

bool isInsideAny(const QString& target, const QStringList& directories)
{
    for (const QString& directory : directories) {
        if (isInside(target, directory)) {
            return true;
        }
    }
    return false;
}

// UNC paths and NTFS streams name something other than a plain local file.
bool isUnsafeSpelling(const QString& target)
{
#ifdef _WIN32
    if (target.indexOf(QLatin1Char(':'), 2) >= 0) {
        return true;
    }
#endif
    return target.startsWith(QStringLiteral("//"));
}

} // namespace

bool ControlPolicy::grants(ControlTier tier) const
{
    if (!restricted) {
        return true;
    }
    switch (tier) {
    case ControlTier::Read:
        return true;
    case ControlTier::Edit:
        return allowEdit;
    case ControlTier::Save:
        return allowEdit && allowSave;
    case ControlTier::Admin:
        return false;
    }
    return false;
}

QStringList protectedDirectories(const QString& stateFile, const QString& endpoint)
{
    QStringList directories{QFileInfo(QFileInfo(stateFile).absolutePath()).canonicalFilePath()};
#ifndef _WIN32
    // A Windows endpoint is a pipe name, with no directory.
    directories.append(QFileInfo(QFileInfo(endpoint).absolutePath()).canonicalFilePath());
#else
    Q_UNUSED(endpoint);
#endif
    directories.removeAll(QString());
    return directories;
}

QString canonicalTarget(const QString& path)
{
    const QFileInfo info(path);
    if (info.exists()) {
        return info.canonicalFilePath();
    }
    const QString parent = info.dir().canonicalPath();
    return parent.isEmpty() ? QDir::cleanPath(path) : parent + QLatin1Char('/') + info.fileName();
}

PathVerdict judgePath(const ControlPolicy& policy, const QString& target, bool writes,
                      bool exists, bool known)
{
    if (!policy.restricted) {
        return PathVerdict::Allow;
    }
    if (isUnsafeSpelling(target) || isInsideAny(target, policy.denied)) {
        return PathVerdict::Refuse;
    }
    if (policy.approval == ApprovalMode::AllowAll) {
        return PathVerdict::Allow;
    }
    const bool overwrite = writes && exists;
    const bool exempt = known && policy.approval == ApprovalMode::AutoApprove;
    if (exempt || (isInsideAny(target, policy.roots) && !overwrite)) {
        return PathVerdict::Allow;
    }
    return policy.live ? PathVerdict::Ask : PathVerdict::Refuse;
}

QStringList allowedCommands()
{
    return {
        QStringLiteral("edit.undo"),
        QStringLiteral("edit.redo"),
        QStringLiteral("edit.stepForward"),
        QStringLiteral("edit.stepBackward"),
        QStringLiteral("select.all"),
        QStringLiteral("select.deselect"),
        QStringLiteral("select.reselect"),
        QStringLiteral("select.inverse"),
        QStringLiteral("image.rotate180"),
        QStringLiteral("image.rotate90cw"),
        QStringLiteral("image.rotate90ccw"),
        QStringLiteral("image.flipHorizontal"),
        QStringLiteral("image.flipVertical"),
        QStringLiteral("image.auto.tone"),
        QStringLiteral("image.auto.contrast"),
        QStringLiteral("image.auto.color"),
        QStringLiteral("image.adjustments.invert"),
        QStringLiteral("image.adjustments.desaturate"),
        QStringLiteral("image.adjustments.equalize"),
        QStringLiteral("layer.arrange.front"),
        QStringLiteral("layer.arrange.forward"),
        QStringLiteral("layer.arrange.backward"),
        QStringLiteral("layer.arrange.back"),
    };
}

bool commandAllowed(const QString& id)
{
    return allowedCommands().contains(id);
}

} // namespace pictura
