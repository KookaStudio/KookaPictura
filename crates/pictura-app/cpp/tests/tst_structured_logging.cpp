#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLoggingCategory>
#include <QtCore/QProcess>
#include <QtCore/QTemporaryDir>

#include "api/view_bridge.h"
#include "api/engine_bridge.h"
#include "diagnostic_logging.h"
#include "qt_test_support.h"

Q_LOGGING_CATEGORY(logFixture, "pictura.fixture")
Q_LOGGING_CATEGORY(filteredFixture, "pictura.filtered")

namespace {

QByteArray fileRecords(const QString& root)
{
    const QDir logs(root + QStringLiteral("/kooka-pictura/logs"));
    QByteArray records;
    for (const auto& name : logs.entryList({QStringLiteral("*.jsonl")}, QDir::Files)) {
        QFile file(logs.filePath(name));
        if (file.open(QIODevice::ReadOnly)) {
            records += file.readAll();
        }
    }
    return records;
}

bool hasEvent(const QByteArray& stream, const QString& event, const QString& level,
              const QString& category = {})
{
    for (const QByteArray& line : stream.split('\n')) {
        const auto record = QJsonDocument::fromJson(line).object();
        if (record.value(QStringLiteral("event")) == event
            && record.value(QStringLiteral("level")) == level
            && (category.isEmpty() || record.value(QStringLiteral("category")) == category)
            && record.value(QStringLiteral("timestamp")).toString().endsWith(QLatin1Char('Z'))) {
            return true;
        }
    }
    return false;
}

void start(QProcess& process, const QString& executable, const QStringList& args,
           const QString& stateHome, const QString& filter = {},
           const QString& platform = QStringLiteral("offscreen"))
{
    auto env = QProcessEnvironment::systemEnvironment();
    env.insert(QStringLiteral("XDG_STATE_HOME"), stateHome);
    env.insert(QStringLiteral("QT_QPA_PLATFORM"), platform);
    env.insert(QStringLiteral("PICTURA_DISABLE_UPDATE_CHECK"), QStringLiteral("1"));
    env.insert(QStringLiteral("RUST_LOG"), filter);
    env.insert(QStringLiteral("QT_LOGGING_RULES"), QStringLiteral("pictura.filtered=false"));
    process.setProcessEnvironment(env);
    process.start(executable, args);
}

int runChild(const QString& mode, const QString& root)
{
    pictura::DiagnosticLogging logging(root, true, mode == QStringLiteral("stall"));
    if (mode == QStringLiteral("fatal")) {
        qFatal("diagnostic-fixture-fatal");
    }
    qCDebug(logFixture) << "diagnostic-fixture-private";
    qCInfo(logFixture) << "diagnostic-fixture-private";
    qCWarning(logFixture) << "diagnostic-fixture-private";
    qCCritical(logFixture) << "diagnostic-fixture-private";
    qCWarning(filteredFixture) << "diagnostic-fixture-private";
    pictura::diagnostics_dependency_for_test("diagnostic-fixture-private");
    if (mode == QStringLiteral("stall")) {
        return 0;
    }
    auto window = pictura::test::makeMainWindow();
    if (!window->newDocument(QStringLiteral("diagnostic-fixture-private"), 8, 8,
                             QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return 3;
    }
    pictura::history_log_configure(true, 0, 1, {});
    window->activeView()->apply_metadata_edits(
        {QStringLiteral("2:5\tdiagnostic-fixture-private")});
    return window->activeView()->save(root + QStringLiteral("/diagnostic-fixture-private.psd"))
        ? 0 : 4;
}
} // namespace

class StructuredLoggingTest : public QObject {
    Q_OBJECT

private slots:
    void routesRustAndQtRecords();
    void appliesTargetAndCategoryRules();
    void keepsPrivateFieldsOutOfFile();
    void isolatesStartupModes();
    void flushesAndPreservesFatalExit();
};

void StructuredLoggingTest::routesRustAndQtRecords()
{
    QTemporaryDir root;
    QVERIFY(root.isValid());
    QProcess child;
    start(child, QCoreApplication::applicationFilePath(),
          {QStringLiteral("--logging-child"), QStringLiteral("records")}, root.path(),
          QStringLiteral("info,pictura::qt=debug"));
    QVERIFY(child.waitForFinished(30000));
    QCOMPARE(child.exitCode(), 0);
    const auto records = fileRecords(root.path());
    QVERIFY(hasEvent(records, QStringLiteral("document.save"), QStringLiteral("INFO")));
    for (const auto& level : {QStringLiteral("DEBUG"), QStringLiteral("INFO"),
                              QStringLiteral("WARN"), QStringLiteral("ERROR")}) {
        QVERIFY(hasEvent(records, QStringLiteral("qt.message"), level,
                         QStringLiteral("pictura.fixture")));
    }
}

void StructuredLoggingTest::appliesTargetAndCategoryRules()
{
    QTemporaryDir root;
    QVERIFY(root.isValid());
    QProcess child;
    start(child, QCoreApplication::applicationFilePath(),
          {QStringLiteral("--logging-child"), QStringLiteral("records")}, root.path(),
          QStringLiteral("warn,pictura::qt=debug"));
    QVERIFY(child.waitForFinished(30000));
    QCOMPARE(child.exitCode(), 0);
    const auto records = fileRecords(root.path());
    QVERIFY(hasEvent(records, QStringLiteral("qt.message"), QStringLiteral("DEBUG"),
                     QStringLiteral("pictura.fixture")));
    QVERIFY(!records.contains("pictura.filtered"));
    QVERIFY(!hasEvent(records, QStringLiteral("document.save"), QStringLiteral("INFO")));
}

void StructuredLoggingTest::keepsPrivateFieldsOutOfFile()
{
    QTemporaryDir root;
    QVERIFY(root.isValid());
    QProcess child;
    start(child, QCoreApplication::applicationFilePath(),
          {QStringLiteral("--logging-child"), QStringLiteral("records")}, root.path());
    QVERIFY(child.waitForFinished(30000));
    QCOMPARE(child.exitCode(), 0);
    const auto records = fileRecords(root.path());
    QVERIFY(hasEvent(records, QStringLiteral("qt.message"), QStringLiteral("WARN"),
                     QStringLiteral("pictura.fixture")));
    QVERIFY(hasEvent(records, QStringLiteral("dependency.log"), QStringLiteral("WARN")));
    QVERIFY(!records.contains("File Info"));
    QVERIFY(!records.contains("diagnostic-fixture-private"));
}

void StructuredLoggingTest::isolatesStartupModes()
{
    QTemporaryDir root;
    QTemporaryDir explicitState;
    QVERIFY(root.isValid() && explicitState.isValid());
    const QString executable = QString::fromUtf8(PICTURA_LOGGING_TEST_EXECUTABLE);
    QProcess theme;
    start(theme, executable, {QStringLiteral("--dump-theme")}, root.path());
    QVERIFY(theme.waitForFinished(30000));
    QVERIFY(QJsonDocument::fromJson(theme.readAllStandardOutput()).isObject());

    QProcess probe;
    start(probe, executable, {QStringLiteral("--gpu-probe"), QStringLiteral("invalid")}, root.path());
    QVERIFY(probe.waitForFinished(30000));
    QCOMPARE(probe.exitCode(), 1);
    QVERIFY(probe.readAllStandardOutput().isEmpty());

    QProcess mcp;
    start(mcp, executable, {QStringLiteral("--mcp")}, root.path());
    QVERIFY(mcp.waitForStarted());
    mcp.closeWriteChannel();
    QVERIFY(mcp.waitForFinished(30000));
    for (const auto& line : mcp.readAllStandardOutput().split('\n')) {
        QVERIFY(line.isEmpty() || QJsonDocument::fromJson(line).isObject());
    }
    QVERIFY(!QFileInfo::exists(root.filePath(QStringLiteral("kooka-pictura/logs"))));

    QProcess isolated;
    start(isolated, executable, {QStringLiteral("--headless"), QStringLiteral("--self-test"),
                                 QStringLiteral("--state-home"), root.path()},
          root.path(), {}, QStringLiteral("minimal"));
    QVERIFY(isolated.waitForFinished(30000));
    QVERIFY(isolated.readAllStandardError().contains("pictura self-test: FAIL"));
    QVERIFY(!QFileInfo::exists(root.filePath(QStringLiteral("kooka-pictura/logs"))));

    QProcess editor;
    start(editor, executable,
          {QStringLiteral("--headless"), QStringLiteral("--control"), QStringLiteral("--state-home"),
           explicitState.path()}, root.path());
    QVERIFY(editor.waitForStarted());
    QTRY_VERIFY_WITH_TIMEOUT(
        QFileInfo::exists(explicitState.filePath(QStringLiteral("kooka-pictura/logs"))), 10000);
    editor.kill();
    QVERIFY(editor.waitForFinished());
    QVERIFY(!QFileInfo::exists(root.filePath(QStringLiteral("kooka-pictura/logs"))));
}

void StructuredLoggingTest::flushesAndPreservesFatalExit()
{
    QTemporaryDir healthyRoot;
    QTemporaryDir stalledRoot;
    QTemporaryDir fatalRoot;
    QTemporaryDir unavailableRoot;
    QVERIFY(healthyRoot.isValid() && stalledRoot.isValid() && fatalRoot.isValid()
            && unavailableRoot.isValid());
    const auto exe = QCoreApplication::applicationFilePath();
    QProcess healthy;
    start(healthy, exe, {QStringLiteral("--logging-child"), QStringLiteral("records")}, healthyRoot.path());
    QVERIFY(healthy.waitForFinished(30000));
    QCOMPARE(healthy.exitCode(), 0);
    QVERIFY(hasEvent(fileRecords(healthyRoot.path()), QStringLiteral("document.save"), QStringLiteral("INFO")));

    QVERIFY(QDir(unavailableRoot.path()).mkpath(QStringLiteral("kooka-pictura")));
    QFile blocked(unavailableRoot.filePath(QStringLiteral("kooka-pictura/logs")));
    QVERIFY(blocked.open(QIODevice::WriteOnly));
    blocked.close();
    QProcess unavailable;
    start(unavailable, exe, {QStringLiteral("--logging-child"), QStringLiteral("records")},
          unavailableRoot.path());
    QVERIFY(unavailable.waitForFinished(30000));
    QCOMPARE(unavailable.exitCode(), 0);
    const auto fallback = unavailable.readAllStandardError();
    QCOMPARE(fallback.count("diagnostic file sink unavailable"), 1);
    QVERIFY(fallback.contains("\"event\":\"document.save\""));

    QProcess stalled;
    start(stalled, exe, {QStringLiteral("--logging-child"), QStringLiteral("stall")}, stalledRoot.path());
    QVERIFY(stalled.waitForFinished(7000));
    QCOMPARE(stalled.exitCode(), 0);
    QVERIFY(stalled.readAllStandardError().contains("diagnostic drain incomplete"));
    QVERIFY(stalled.readAllStandardOutput().isEmpty());

    QProcess fatal;
    start(fatal, exe, {QStringLiteral("--logging-child"), QStringLiteral("fatal")}, fatalRoot.path());
    QVERIFY(fatal.waitForFinished(10000));
    QCOMPARE(fatal.exitStatus(), QProcess::CrashExit);
    const auto stderrText = fatal.readAllStandardError();
    QVERIFY(stderrText.contains("ERROR"));
    QVERIFY(stderrText.contains("fatal"));
    QVERIFY(stderrText.contains("diagnostic-fixture-fatal"));
}

int main(int argc, char** argv)
{
    QApplication app(argc, argv);
    const auto args = app.arguments();
    if (args.size() == 3 && args.at(1) == QStringLiteral("--logging-child")) {
        return runChild(args.at(2), qEnvironmentVariable("XDG_STATE_HOME"));
    }
    StructuredLoggingTest suite;
    return QTest::qExec(&suite, argc, argv);
}

#include "tst_structured_logging.moc"
