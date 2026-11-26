#include <cstdio>

#include <QtWidgets/QAbstractButton>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QTimer>
#include <QtTest/QtTest>
#include <QtWidgets/QMessageBox>

#include "preferences.h"
#include "profile_startup.h"
#include "session.h"
#include "settings_backup.h"
#include "workspace_store.h"
#include "prefs_test_support.h"
#include "qt_test_support.h"

using namespace pictura;
using namespace pictura::test;

namespace {

constexpr auto kBackupId = "interrupted-test";
constexpr auto kMarker = ".profile-reset.json";
constexpr auto kDark = "schema_version = 1\n[interface]\nbrightnessLevel = 3\n";

QString rootOf(const QString& file)
{
    return QFileInfo(file).absolutePath();
}

QString resetBackup(const QString& root)
{
    return QDir(root).filePath(QStringLiteral("reset-backup-") + QString::fromLatin1(kBackupId));
}

bool stageInterruptedReset(const QString& config, const QString& state)
{
    const QJsonObject transaction{
        {QStringLiteral("id"), QString::fromLatin1(kBackupId)},
        {QStringLiteral("configRoot"), config},
        {QStringLiteral("stateRoot"), state},
        {QStringLiteral("missing"), QJsonArray{}},
    };
    return writeProfileJson(QDir(config).filePath(QString::fromLatin1(kMarker)), transaction)
        && writeProfileJson(QDir(state).filePath(QString::fromLatin1(kMarker)), transaction)
        && writeProfileFile(QDir(resetBackup(state)).filePath(QStringLiteral(".published")), "published\n")
        && writeProfileFile(QDir(resetBackup(config)).filePath(QStringLiteral("prefs.toml")), kDark)
        && writeProfileJson(QDir(resetBackup(state)).filePath(QStringLiteral("state.json")),
                            {{QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/before.psd")}}})
        && writeProfileFile(QDir(config).filePath(QStringLiteral("prefs.toml")), "schema_version = 1\n")
        && writeProfileJson(QDir(state).filePath(QStringLiteral("state.json")),
                            {{QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/partial.psd")}}});
}

void answerReset(QMessageBox::StandardButton button)
{
    QTimer::singleShot(0, [button] {
        auto* prompt = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
        if (prompt && prompt->button(button)) {
            prompt->button(button)->click();
        }
    });
}

} // namespace

class PreferencesResetTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void startupFlagCancelKeepsProfile();
    void startupFlagConfirmRestoresFactoryProfile();
    void resetRefusesBusyProfileOrWriteFailure();
    void interruptedResetRecoversBeforeLoad();
    void concurrentInterruptedResetRecoverySerializes();
    void incompleteRegistrationPreservesOverlappingProfile();
    void completedRollbackDoesNotReplay();

private:
    std::unique_ptr<ScopedStateHome> home_;
};

void PreferencesResetTest::init()
{
    home_ = std::make_unique<ScopedStateHome>();
    QVERIFY(home_->isValid());
}

void PreferencesResetTest::cleanup()
{
    home_.reset();
}

void PreferencesResetTest::startupFlagCancelKeepsProfile()
{
    QVERIFY(writeProfileFile(preferencesFilePath(), kDark));
    answerReset(QMessageBox::Cancel);
    QString problem;
    auto profile = prepareProfile({QStringLiteral("--reset-preferences")}, true, &problem);
    QVERIFY2(profile, qPrintable(problem));
    QCOMPARE(loadPreferences().brightnessLevel, 3);
    QCOMPARE(readProfileFile(preferencesFilePath()), QByteArray(kDark));
}

void PreferencesResetTest::startupFlagConfirmRestoresFactoryProfile()
{
    QVERIFY(writeProfileFile(preferencesFilePath(), QByteArray(kDark)
        + "[color]\ncolorPolicy = 1\n[keyboard.shortcuts]\n'file.new' = ['Ctrl+Alt+N']\n"));
    QVERIFY(writeProfileFile(preferencesFilePath() + QStringLiteral(".bak"), kDark));
    QVERIFY(writeProfileJson(sessionFilePath(), {
        {QStringLiteral("brightnessLevel"), 3},
        {QStringLiteral("layout"), QStringLiteral("YWJj")},
        {QStringLiteral("windowGeometry"), QStringLiteral("eHl6")},
        {QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/keep.psd")}},
    }));
    QVERIFY(writeProfileFile(QDir(workspaceStoreDir()).filePath(QStringLiteral("index.json")), "{}"));
    QVERIFY(writeProfileFile(pendingSettingsImportPath(), "{}"));
    const auto recovery = QDir(rootOf(sessionFilePath())).filePath(QStringLiteral("recovery/snapshot"));
    QVERIFY(writeProfileFile(recovery, "recovery bytes"));

    answerReset(QMessageBox::Ok);
    QString problem;
    auto profile = prepareProfile({QStringLiteral("--reset-preferences")}, true, &problem);
    QVERIFY2(profile, qPrintable(problem));
    QVERIFY(problem.isEmpty());
    QVERIFY(initializePreferences());
    const auto preferences = loadPreferences();
    QCOMPARE(preferences.brightnessLevel, 1);
    QCOMPARE(preferences.colorPolicy, 0);
    QVERIFY(preferences.shortcuts.isEmpty());
    const auto state = loadSession();
    QVERIFY(state.layout.isEmpty());
    QCOMPARE(state.windowGeometry, QByteArray("xyz"));
    QCOMPARE(state.recent, QStringList{QStringLiteral("/tmp/keep.psd")});
    QVERIFY(!QFileInfo::exists(workspaceStoreDir()));
    QVERIFY(!hasPendingSettingsImport());
    QCOMPARE(readProfileFile(recovery), QByteArray("recovery bytes"));
    QVERIFY(!QDir(rootOf(preferencesFilePath())).entryList(
        {QStringLiteral("reset-backup-*")}, QDir::Dirs).isEmpty());
    profile.reset();
    QVERIFY(prepareProfile({}, false, &problem));
    QCOMPARE(loadPreferences().brightnessLevel, 1);
}

void PreferencesResetTest::resetRefusesBusyProfileOrWriteFailure()
{
    QVERIFY(writeProfileFile(preferencesFilePath(), kDark));
    const auto state = rootOf(sessionFilePath());
    QVERIFY(QDir().mkpath(state));
    QProcess holder;
    holder.start(QCoreApplication::applicationFilePath(),
                 {QStringLiteral("--suite"), QStringLiteral("tst_preferences_reset"),
                  QStringLiteral("--hold-profile")});
    QVERIFY(holder.waitForStarted());
    QTRY_VERIFY_WITH_TIMEOUT(holder.bytesAvailable() > 0, 10000);
    QCOMPARE(holder.readAllStandardOutput(), QByteArray("ready\n"));
    QString error;
    QVERIFY(!resetProfile(&error));
    QVERIFY(!error.isEmpty());
    QCOMPARE(readProfileFile(preferencesFilePath()), QByteArray(kDark));
    holder.kill();
    QVERIFY(holder.waitForFinished());
    QVERIFY(QDir().mkdir(sessionFilePath()));
    QVERIFY(!resetProfile(&error));
    QCOMPARE(readProfileFile(preferencesFilePath()), QByteArray(kDark));
}

void PreferencesResetTest::interruptedResetRecoversBeforeLoad()
{
    QVERIFY(stageInterruptedReset(rootOf(preferencesFilePath()), rootOf(sessionFilePath())));
    QString error;
    QVERIFY2(prepareProfile({}, false, &error), qPrintable(error));
    QCOMPARE(loadPreferences().brightnessLevel, 3);
    QCOMPARE(loadSession().recent, QStringList{QStringLiteral("/tmp/before.psd")});
}

void PreferencesResetTest::concurrentInterruptedResetRecoverySerializes()
{
    const auto config = rootOf(preferencesFilePath());
    const auto state = rootOf(sessionFilePath());
    QVERIFY(stageInterruptedReset(config, state));
    QTemporaryDir alternative;
    QVERIFY(alternative.isValid());
    QProcess first;
    QProcess second;
    auto environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("XDG_STATE_HOME"), alternative.path());
    second.setProcessEnvironment(environment);
    for (QProcess* process : {&first, &second}) {
        process->start(QCoreApplication::applicationFilePath(),
                       {QStringLiteral("--suite"), QStringLiteral("tst_preferences_reset"),
                        QStringLiteral("--recover-profile")});
    }
    for (QProcess* process : {&first, &second}) {
        if (process->state() != QProcess::NotRunning) {
            QVERIFY(process->waitForFinished(15000));
        }
        QCOMPARE(process->exitStatus(), QProcess::NormalExit);
        QCOMPARE(process->exitCode(), 0);
        QCOMPARE(process->readAllStandardOutput(), QByteArray("3\n"));
    }
    QVERIFY(!QFileInfo::exists(QDir(config).filePath(QString::fromLatin1(kMarker))));
    QVERIFY(!QFileInfo::exists(QDir(state).filePath(QString::fromLatin1(kMarker))));
    QCOMPARE(loadSession().recent, QStringList{QStringLiteral("/tmp/before.psd")});
}

void PreferencesResetTest::incompleteRegistrationPreservesOverlappingProfile()
{
    const auto config = rootOf(preferencesFilePath());
    const auto state = rootOf(sessionFilePath());
    const QJsonObject transaction{
        {QStringLiteral("id"), QString::fromLatin1(kBackupId)},
        {QStringLiteral("configRoot"), config},
        {QStringLiteral("stateRoot"), state},
        {QStringLiteral("missing"), QJsonArray{QStringLiteral("state.json")}},
    };
    QVERIFY(writeProfileJson(QDir(config).filePath(QString::fromLatin1(kMarker)), transaction));
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/new.psd")}}}));
    QString error;
    QVERIFY2(prepareProfile({}, false, &error), qPrintable(error));
    QCOMPARE(loadSession().recent, QStringList{QStringLiteral("/tmp/new.psd")});
}

void PreferencesResetTest::completedRollbackDoesNotReplay()
{
    const auto config = rootOf(preferencesFilePath());
    const auto state = rootOf(sessionFilePath());
    QVERIFY(stageInterruptedReset(config, state));
    QVERIFY(writeProfileFile(QDir(resetBackup(state)).filePath(QStringLiteral(".rolled-back")), "rolled back\n"));
    QVERIFY(QFile::remove(QDir(state).filePath(QString::fromLatin1(kMarker))));
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/new.psd")}}}));
    QString error;
    QVERIFY2(prepareProfile({}, false, &error), qPrintable(error));
    QCOMPARE(loadSession().recent, QStringList{QStringLiteral("/tmp/new.psd")});
}

namespace picturaQtTest_PreferencesResetTest {
int main(int argc, char** argv)
{
    QApplication app(argc, argv);
    if (app.arguments().contains(QStringLiteral("--hold-profile"))
        || app.arguments().contains(QStringLiteral("--recover-profile"))) {
        QString error;
        auto profile = prepareProfile({}, false, &error);
        if (!profile) {
            qWarning().noquote() << error;
            return 2;
        }
        if (app.arguments().contains(QStringLiteral("--hold-profile"))) {
            std::puts("ready");
            std::fflush(stdout);
            return app.exec();
        }
        std::printf("%d\n", loadPreferences().brightnessLevel);
        return 0;
    }
    PreferencesResetTest test;
    return QTest::qExec(&test, argc, argv);
}
PICTURA_QT_TEST_REGISTER(PreferencesResetTest)
}

#include "tst_preferences_reset.moc"
