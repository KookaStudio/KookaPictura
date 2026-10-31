#include "client_registration.h"

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QObject>
#include <QtCore/QSaveFile>
#include <QtCore/QStandardPaths>

#include <functional>

namespace pictura {

namespace {

const QString kServers = QStringLiteral("mcpServers");
const QString kName = QStringLiteral("pictura");

QJsonObject serverEntry(const QString& command)
{
    return {{QStringLiteral("command"), command},
            {QStringLiteral("args"), QJsonArray{QStringLiteral("--mcp")}}};
}

// The root object, or a reason to leave the file alone. A missing or empty file
// is an empty root.
QString readRoot(const QString& path, QByteArray* bytes, QJsonObject* root)
{
    QFile file(path);
    if (file.exists()) {
        if (!file.open(QIODevice::ReadOnly)) {
            return QObject::tr("%1 cannot be read.").arg(path);
        }
        *bytes = file.readAll();
    }
    if (bytes->trimmed().isEmpty()) {
        return {};
    }
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(*bytes, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return QObject::tr("%1 is not plain JSON, so it was left alone.").arg(path);
    }
    *root = document.object();
    if (root->contains(kServers) && !root->value(kServers).isObject()) {
        return QObject::tr("%1 has an unexpected mcpServers value, so it was left alone.")
            .arg(path);
    }
    return {};
}

ConfigChange withServers(const QString& path,
                         const std::function<void(QJsonObject&)>& edit)
{
    ConfigChange change;
    QJsonObject root;
    change.problem = readRoot(path, &change.before, &root);
    if (!change.problem.isEmpty()) {
        return change;
    }
    QJsonObject servers = root.value(kServers).toObject();
    edit(servers);
    root.insert(kServers, servers);
    change.after = QJsonDocument(root).toJson(QJsonDocument::Indented);
    return change;
}

QString findClaude(const QString& home)
{
    const QString found = QStandardPaths::findExecutable(QStringLiteral("claude"));
    if (!found.isEmpty()) {
        return found;
    }
    // A desktop launcher's PATH often lacks what a login shell adds.
    return QStandardPaths::findExecutable(QStringLiteral("claude"),
                                          {home + QStringLiteral("/.local/bin")});
}

} // namespace

QList<McpClient> mcpClients(const QString& home)
{
    QList<McpClient> clients;

    McpClient claudeCode;
    claudeCode.name = QStringLiteral("Claude Code");
    const QString configDir = qEnvironmentVariable("CLAUDE_CONFIG_DIR", home);
    claudeCode.configPath = configDir + QStringLiteral("/.claude.json");
    claudeCode.cliProgram = findClaude(home);
    claudeCode.cliMissing = claudeCode.cliProgram.isEmpty();
    clients.append(claudeCode);

    clients.append({QStringLiteral("Cursor"), home + QStringLiteral("/.cursor/mcp.json"), {}, false});

    // There is no official Linux build of Claude Desktop.
#if defined(__APPLE__)
    clients.append({QStringLiteral("Claude Desktop"),
                    home + QStringLiteral("/Library/Application Support/Claude/"
                                          "claude_desktop_config.json"),
                    {}, false});
#elif defined(_WIN32)
    // ponytail: a Microsoft Store install reads a packaged copy under
    // %LOCALAPPDATA%, so registering there can land in a file it ignores.
    clients.append({QStringLiteral("Claude Desktop"),
                    QDir::fromNativeSeparators(qEnvironmentVariable("APPDATA"))
                        + QStringLiteral("/Claude/claude_desktop_config.json"),
                    {}, false});
#endif
    return clients;
}

EntryState registrationState(const QString& configPath, const QString& command)
{
    QByteArray bytes;
    QJsonObject root;
    if (!readRoot(configPath, &bytes, &root).isEmpty()) {
        return EntryState::Unreadable;
    }
    const QJsonObject servers = root.value(kServers).toObject();
    if (!servers.contains(kName)) {
        return EntryState::Absent;
    }
    const QJsonObject entry = servers.value(kName).toObject();
    const QJsonObject expected = serverEntry(command);
    return entry.value(QStringLiteral("command")) == expected.value(QStringLiteral("command"))
            && entry.value(QStringLiteral("args")) == expected.value(QStringLiteral("args"))
        ? EntryState::Same
        : EntryState::Different;
}

ConfigChange planRegister(const QString& configPath, const QString& command)
{
    return withServers(configPath,
                       [&command](QJsonObject& servers) {
                           servers.insert(kName, serverEntry(command));
                       });
}

ConfigChange planUnregister(const QString& configPath)
{
    return withServers(configPath, [](QJsonObject& servers) { servers.remove(kName); });
}

bool applyChange(const QString& configPath, const ConfigChange& change, QString* problem)
{
    const auto fail = [problem](const QString& message) {
        if (problem) {
            *problem = message;
        }
        return false;
    };
    if (!change.problem.isEmpty()) {
        return fail(change.problem);
    }
    // Through a link, so the link stays one.
    const QFileInfo info(configPath);
    const QString target = info.exists() ? info.canonicalFilePath() : configPath;
    if (!QDir().mkpath(QFileInfo(target).absolutePath())) {
        return fail(QObject::tr("%1 cannot be created.").arg(target));
    }
    if (info.exists()) {
        const QString stamp =
            QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss-zzz"));
        if (!QFile::copy(target, target + QStringLiteral(".pictura-backup-") + stamp)) {
            return fail(QObject::tr("%1 could not be backed up, so it was left alone.").arg(target));
        }
    }
    QSaveFile file(target);
    if (!file.open(QIODevice::WriteOnly) || file.write(change.after) != change.after.size()
        || !file.commit()) {
        return fail(QObject::tr("%1 could not be written.").arg(target));
    }
    return true;
}

} // namespace pictura
