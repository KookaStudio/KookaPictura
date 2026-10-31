// Registering with MCP clients writes to files that belong to other apps, so the
// tests pin what must survive and what must never be written.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QPushButton>

#include "client_registration.h"
#include "client_registration_box.h"

using namespace pictura;

namespace {

const QString kCommand = QStringLiteral("/opt/pictura");

QByteArray readFile(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

void writeFile(const QString& path, const QByteArray& bytes)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(bytes);
}

QJsonObject servers(const QString& path)
{
    return QJsonDocument::fromJson(readFile(path)).object()
        .value(QStringLiteral("mcpServers")).toObject();
}

McpClient cursorAt(const QString& path)
{
    return {QStringLiteral("Cursor"), path, {}, false};
}

} // namespace

class ClientRegistrationTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void newFileIsCreated();
    void otherServersSurvive();
    void invalidFilesAreRefused();
    void decliningChangesNothing();
    void differentEntryNeedsItsOwnAcceptance();
    void unregisteringRemovesOnlyTheEntry();
    void symbolicLinkIsKept();
    void cliIsUsed();
    void missingCliIsReported();

private:
    bool commit(const ConfigChange& change, const QString& path);

    std::unique_ptr<QTemporaryDir> dir_;
    QString config_;
};

void ClientRegistrationTest::init()
{
    dir_ = std::make_unique<QTemporaryDir>();
    QVERIFY(dir_->isValid());
    config_ = dir_->filePath(QStringLiteral("cursor/mcp.json"));
}

bool ClientRegistrationTest::commit(const ConfigChange& change, const QString& path)
{
    return applyChange(path, change, nullptr);
}

void ClientRegistrationTest::newFileIsCreated()
{
    QCOMPARE(registrationState(config_, kCommand), EntryState::Absent);
    QVERIFY(commit(planRegister(config_, kCommand), config_));
    const QJsonObject entry = servers(config_).value(QStringLiteral("pictura")).toObject();
    QCOMPARE(entry.value(QStringLiteral("command")).toString(), kCommand);
    QCOMPARE(entry.value(QStringLiteral("args")).toArray().first().toString(), QStringLiteral("--mcp"));
    QCOMPARE(registrationState(config_, kCommand), EntryState::Same);
}

void ClientRegistrationTest::otherServersSurvive()
{
    const QByteArray original =
        R"({"theme":"dark","mcpServers":{"other":{"command":"x","env":{"K":"V"}}}})";
    writeFile(config_, original);
    QVERIFY(commit(planRegister(config_, kCommand), config_));

    const QJsonObject root = QJsonDocument::fromJson(readFile(config_)).object();
    QCOMPARE(root.value(QStringLiteral("theme")).toString(), QStringLiteral("dark"));
    QVERIFY(servers(config_).contains(QStringLiteral("pictura")));
    QCOMPARE(servers(config_).value(QStringLiteral("other")).toObject()
                 .value(QStringLiteral("env")).toObject()
                 .value(QStringLiteral("K")).toString(),
             QStringLiteral("V"));

    const QStringList backups = QDir(QFileInfo(config_).absolutePath())
                                    .entryList({QStringLiteral("mcp.json.pictura-backup-*")});
    QCOMPARE(backups.size(), 1);
    QCOMPARE(readFile(QFileInfo(config_).absolutePath() + QLatin1Char('/') + backups.first()),
             original);
}

void ClientRegistrationTest::invalidFilesAreRefused()
{
    for (const QByteArray& bytes :
         {QByteArray("{\n  // a comment\n  \"mcpServers\": {}\n}\n"),
          QByteArray(R"({"mcpServers":["nope"]})"), QByteArray("[]")}) {
        writeFile(config_, bytes);
        const ConfigChange change = planRegister(config_, kCommand);
        QVERIFY(!change.problem.isEmpty());
        QVERIFY(!commit(change, config_));
        QCOMPARE(readFile(config_), bytes);
        QCOMPARE(registrationState(config_, kCommand), EntryState::Unreadable);
    }
}

void ClientRegistrationTest::decliningChangesNothing()
{
    ClientRegistrationBox box({cursorAt(config_)}, kCommand);
    box.setConfirm([](const QString&, const QString&) { return false; });
    box.rowButton(0)->click();
    QVERIFY(!QFile::exists(config_));

    box.setConfirm([](const QString&, const QString&) { return true; });
    box.rowButton(0)->click();
    QVERIFY(QFile::exists(config_));
    QCOMPARE(box.rowButton(0)->text(), QStringLiteral("Unregister"));
}

void ClientRegistrationTest::differentEntryNeedsItsOwnAcceptance()
{
    writeFile(config_, R"({"mcpServers":{"pictura":{"command":"/elsewhere","args":["--mcp"]}}})");
    QCOMPARE(registrationState(config_, kCommand), EntryState::Different);
    ClientRegistrationBox box({cursorAt(config_)}, kCommand);
    QString asked;
    box.setConfirm([&asked](const QString& question, const QString&) {
        asked = question;
        return false;
    });
    box.rowButton(0)->click();
    QVERIFY(asked.contains(QStringLiteral("Replace")));
    QCOMPARE(box.rowButton(0)->text(), QStringLiteral("Replace"));
    QCOMPARE(registrationState(config_, kCommand), EntryState::Different);

    box.setConfirm([](const QString&, const QString&) { return true; });
    box.rowButton(0)->click();
    QCOMPARE(registrationState(config_, kCommand), EntryState::Same);
    QCOMPARE(box.rowButton(0)->text(), QStringLiteral("Unregister"));
}

void ClientRegistrationTest::unregisteringRemovesOnlyTheEntry()
{
    writeFile(config_, R"({"mcpServers":{"other":{"command":"x"},"pictura":{"command":"y"}}})");
    QVERIFY(commit(planUnregister(config_), config_));
    QVERIFY(!servers(config_).contains(QStringLiteral("pictura")));
    QVERIFY(servers(config_).contains(QStringLiteral("other")));
}

void ClientRegistrationTest::symbolicLinkIsKept()
{
#ifdef _WIN32
    QSKIP("needs a symbolic link");
#else
    const QString target = dir_->filePath(QStringLiteral("dotfiles/mcp.json"));
    writeFile(target, "{}");
    QVERIFY(QDir().mkpath(QFileInfo(config_).absolutePath()));
    QVERIFY(QFile::link(target, config_));
    QVERIFY(commit(planRegister(config_, kCommand), config_));
    QVERIFY(QFileInfo(config_).isSymLink());
    QVERIFY(readFile(target).contains("pictura"));
#endif
}

void ClientRegistrationTest::cliIsUsed()
{
#ifdef _WIN32
    QSKIP("needs a shell script");
#else
    const QString log = dir_->filePath(QStringLiteral("args.log"));
    const QString program = dir_->filePath(QStringLiteral("claude"));
    writeFile(program, QStringLiteral("#!/bin/sh\necho \"$@\" > %1\n").arg(log).toUtf8());
    QVERIFY(QFile::setPermissions(program, QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner));

    ClientRegistrationBox box({{QStringLiteral("Claude Code"), config_, program, false}}, kCommand);
    QString shown;
    box.setConfirm([&shown](const QString&, const QString& details) {
        shown = details;
        return true;
    });
    box.rowButton(0)->click();
    QTRY_VERIFY(QFile::exists(log));
    const QByteArray arguments = readFile(log).trimmed();
    QCOMPARE(arguments, QByteArray("mcp add --scope user pictura -- /opt/pictura --mcp"));
    QVERIFY(shown.endsWith(QString::fromUtf8(arguments)));
    QTRY_VERIFY(box.rowButton(0)->isEnabled());
#endif
}

void ClientRegistrationTest::missingCliIsReported()
{
    ClientRegistrationBox box({{QStringLiteral("Claude Code"), config_, {}, true}}, kCommand);
    QVERIFY(!box.rowButton(0)->isEnabled());
    QVERIFY(box.rowStatus(0).contains(QStringLiteral("not found")));
}

QTEST_MAIN(ClientRegistrationTest)
#include "tst_client_registration.moc"
