#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QString>

namespace pictura {

// An MCP client Pictura can register with. `cliProgram` set means the client's
// own tool owns the configuration; `cliMissing` means it has one, not found here.
struct McpClient {
    QString name;
    QString configPath;
    QString cliProgram;
    bool cliMissing = false;
};

QList<McpClient> mcpClients(const QString& home);

enum class EntryState { Absent, Same, Different, Unreadable };
EntryState registrationState(const QString& configPath, const QString& command);

// A file's bytes before and after a change, computed without writing anything.
struct ConfigChange {
    QString problem;  // empty when the change can be applied
    QByteArray before;
    QByteArray after;
};

ConfigChange planRegister(const QString& configPath, const QString& command);
ConfigChange planUnregister(const QString& configPath);

// Back up `configPath`, then write `change` atomically. False, with `problem` set,
// leaves the file as it was.
bool applyChange(const QString& configPath, const ConfigChange& change, QString* problem);

} // namespace pictura
