#include <QtCore/QLocale>
#include <QtCore/QLockFile>
#include <QtCore/QProcess>
#include <QtCore/QScopeGuard>
#include <QtTest/QtTest>
#include <QtWidgets/QDoubleSpinBox>

#include "color_settings_dialog.h"
#include "preferences.h"
#include "profile_startup.h"
#include "session.h"
#include "settings_backup.h"
#include "prefs_test_support.h"
#include "qt_test_support.h"

using namespace pictura;
using namespace pictura::test;

class PreferencesStoreTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void corruptPreferencesCannotRestoreAgentGrants();
    void freshProfileUsesSeparateRoots();
    void preferencesReachConsumersAfterRelaunch();
    void localeDoesNotChangeStoredNumbers();
    void mixedLegacyProfileSplits();
    void interruptedMigrationRetries();
    void busyLegacyMigrationDefersPatch();
    void deletedPreferencesStayReset();
    void corruptPreferencesRecoverWithOneNotice();
    void isolatedLaunchesProtectRealConfig();
    void legacyGpuDefaultSurvivesMigration();
    void shiftSwitchDefaultsToOn();
    void shiftSwitchPersistsInPrefs();
    void legacyShiftSwitchDefaultsToOn();

private:
    std::unique_ptr<ScopedStateHome> home_;
};

void PreferencesStoreTest::init()
{
    home_ = std::make_unique<ScopedStateHome>();
    QVERIFY(home_->isValid());
}

void PreferencesStoreTest::cleanup()
{
    home_.reset();
}

void PreferencesStoreTest::freshProfileUsesSeparateRoots()
{
    QVERIFY(initializePreferences());
    QVERIFY(readProfileFile(preferencesFilePath()).startsWith("schema_version = 1"));
    QVERIFY(QFileInfo(preferencesFilePath()).absolutePath()
            != QFileInfo(sessionFilePath()).absolutePath());
    SessionState state;
    state.recent = {QStringLiteral("/tmp/document.psd")};
    QVERIFY(saveSession(state));
    QCOMPARE(loadSession().recent, state.recent);
    QVERIFY(!readProfileFile(preferencesFilePath()).contains("document.psd"));
}

void PreferencesStoreTest::preferencesReachConsumersAfterRelaunch()
{
    auto first = makeMainWindow();
    first->setUiScalePercent(130);
    auto before = loadPreferences();
    auto after = before;
    after.memoryUsagePercent = 75;
    QVERIFY(savePreferences(before, after));
    first.reset();
    auto second = makeMainWindow();
    QCOMPARE(second->uiScalePercent(), 130);
    QCOMPARE(loadPreferences().memoryUsagePercent, 75);
    QVERIFY(!readProfileJson(sessionFilePath()).contains(QStringLiteral("uiScalePercent")));
}

void PreferencesStoreTest::localeDoesNotChangeStoredNumbers()
{
    const auto oldLocale = QLocale();
    const auto restore = qScopeGuard([&] { QLocale::setDefault(oldLocale); });
    QLocale::setDefault(QLocale(QLocale::German, QLocale::Germany));
    ColorSettingsDialog dialog(ColorSettingsValues{});
    auto* gamma = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("colorSettingsTextGamma"));
    QVERIFY(gamma);
    gamma->setValue(1.65);
    QCOMPARE(gamma->cleanText(), QStringLiteral("1,65"));
    auto before = loadPreferences();
    auto after = before;
    after.textGamma = dialog.textGamma();
    QVERIFY(savePreferences(before, after));
    QVERIFY(readProfileFile(preferencesFilePath()).contains("textGamma = 1.65"));
    QCOMPARE(loadPreferences().textGamma, 1.65);
}

void PreferencesStoreTest::mixedLegacyProfileSplits()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {
        {QStringLiteral("schemaVersion"), 12},
        {QStringLiteral("brightnessLevel"), 3},
        {QStringLiteral("gpuCompute"), false},
        {QStringLiteral("layout"), QStringLiteral("YWJj")},
        {QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/recent.psd")}},
        {QStringLiteral("futureStateKey"), 17},
        {QStringLiteral("shortcuts"), QJsonObject{
            {QStringLiteral("file.new"), QJsonArray{QStringLiteral("Ctrl+Alt+N")}}}},
    }));
    QVERIFY(initializePreferences());
    const auto prefs = loadPreferences();
    QCOMPARE(prefs.brightnessLevel, 3);
    QVERIFY(!prefs.gpuCompute);
    QCOMPARE(prefs.shortcuts.value(QStringLiteral("file.new")), QStringList{QStringLiteral("Ctrl+Alt+N")});
    QCOMPARE(loadSession().layout, QByteArray("abc"));
    const auto state = readProfileJson(sessionFilePath());
    QCOMPARE(state.value(QStringLiteral("futureStateKey")).toInt(), 17);
    QVERIFY(state.contains(QStringLiteral("recent")));
    QVERIFY(!state.contains(QStringLiteral("brightnessLevel")));
    QVERIFY(!state.contains(QStringLiteral("shortcuts")));
}

void PreferencesStoreTest::interruptedMigrationRetries()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("brightnessLevel"), 2}}));
    const auto source = readProfileFile(sessionFilePath());
    QVERIFY(QDir().mkpath(preferencesFilePath()));
    QVERIFY(!initializePreferences());
    QCOMPARE(readProfileFile(sessionFilePath()), source);
    QVERIFY(QDir().rmdir(preferencesFilePath()));
    QVERIFY(writeProfileFile(preferencesFilePath(), "schema_version = 1\n[interface]\nbrightnessLevel = 3\n"));
    QVERIFY(initializePreferences());
    QCOMPARE(loadPreferences().brightnessLevel, 3);
    QVERIFY(!readProfileJson(sessionFilePath()).contains(QStringLiteral("brightnessLevel")));
}

void PreferencesStoreTest::busyLegacyMigrationDefersPatch()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {
        {QStringLiteral("brightnessLevel"), 3},
        {QStringLiteral("gpuCompute"), false},
    }));
    QLockFile lock(sessionFilePath() + QStringLiteral(".lock"));
    QVERIFY(lock.tryLock());
    const auto before = loadPreferences();
    auto after = before;
    after.uiScalePercent = 120;
    QVERIFY(!savePreferences(before, after));
    QVERIFY(!QFileInfo::exists(preferencesFilePath()));
    QCOMPARE(loadPreferences().uiScalePercent, 120);
    lock.unlock();
    QVERIFY(savePreferences(before, after));
    const auto restored = loadPreferences();
    QCOMPARE(restored.brightnessLevel, 3);
    QVERIFY(!restored.gpuCompute);
    QCOMPARE(restored.uiScalePercent, 120);
}

void PreferencesStoreTest::deletedPreferencesStayReset()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("brightnessLevel"), 3}}));
    QVERIFY(initializePreferences());
    QVERIFY(writeProfileFile(preferencesFilePath() + QStringLiteral(".bak"),
                             "schema_version = 1\n[interface]\nbrightnessLevel = 3\n"));
    auto state = readProfileJson(sessionFilePath());
    state.insert(QStringLiteral("brightnessLevel"), 2);
    state.insert(QStringLiteral("recent"), QJsonArray{QStringLiteral("/tmp/recent.psd")});
    QVERIFY(writeProfileJson(sessionFilePath(), state));
    QVERIFY(QFile::remove(preferencesFilePath()));
    QVERIFY(initializePreferences());
    QCOMPARE(loadPreferences().brightnessLevel, 1);
    QCOMPARE(loadSession().recent.size(), 1);
}

void PreferencesStoreTest::corruptPreferencesRecoverWithOneNotice()
{
    const QByteArray corrupt = "schema_version = 1\n[broken";
    for (const bool haveBackup : {true, false}) {
        if (haveBackup) {
            QVERIFY(writeProfileFile(preferencesFilePath() + QStringLiteral(".bak"),
                                     "schema_version = 1\n[interface]\nbrightnessLevel = 2\n"));
        } else {
            QFile::remove(preferencesFilePath() + QStringLiteral(".bak"));
        }
        QVERIFY(writeProfileFile(preferencesFilePath(), corrupt));
        QString notice;
        QVERIFY(initializePreferences(&notice));
        QVERIFY(!notice.isEmpty());
        QCOMPARE(loadPreferences().brightnessLevel, haveBackup ? 2 : 1);
        const QDir dir(QFileInfo(preferencesFilePath()).absolutePath());
        const auto files = dir.entryList({QStringLiteral("prefs.toml.corrupt-*")}, QDir::Files);
        QVERIFY(!files.isEmpty());
        for (const auto& name : files) {
            QCOMPARE(readProfileFile(dir.filePath(name)), corrupt);
        }
        notice.clear();
        QVERIFY(initializePreferences(&notice));
        QVERIFY(notice.isEmpty());
    }
}

void PreferencesStoreTest::corruptPreferencesCannotRestoreAgentGrants()
{
    const auto restoreFailure = qScopeGuard([] {
        setSettingsImportFailureForTest(SettingsImportFailure::None);
    });
    for (const int version : {1, 2}) {
        const QByteArray elevated = "schema_version = 1\n[agent]\nagentControlEnabled = true\n"
                                    "agentAllowEdit = true\nagentAllowSave = true\n"
                                    "agentApprovalMode = 2\nagentRoots = ['/tmp']\n";
        QVERIFY(writeProfileFile(preferencesFilePath(), elevated));
        auto enabled = loadPreferences();
        auto disabled = enabled;
        disabled.agentControlEnabled = false;
        QVERIFY(savePreferences(enabled, disabled));
        QJsonObject pending{{QStringLiteral("format"), QStringLiteral("pictura-settings")},
                            {QStringLiteral("formatVersion"), version}};
        if (version == 1) {
            pending.insert(QStringLiteral("session"),
                           QJsonObject{{QStringLiteral("agentControlEnabled"), true},
                                       {QStringLiteral("agentAllowSave"), true},
                                       {QStringLiteral("brightnessLevel"), 3}});
        } else {
            pending.insert(QStringLiteral("preferences"), QString::fromUtf8(elevated)
                + QStringLiteral("[interface]\nbrightnessLevel = 3\n"));
        }
        QVERIFY(writePendingSettingsImport(pending));
        QVERIFY(writeProfileFile(preferencesFilePath(), "invalid TOML"));
        QVERIFY(initializePreferences());
        setSettingsImportFailureForTest(SettingsImportFailure::AfterPreferences);
        QVERIFY(!applyPendingSettingsImport());
        QVERIFY(hasPendingSettingsImport());
        QVERIFY(!loadPreferences().agentControlEnabled);
        setSettingsImportFailureForTest(SettingsImportFailure::None);
        QVERIFY(applyPendingSettingsImport());
        auto recovered = loadPreferences();
        QVERIFY(!recovered.agentControlEnabled);
        QVERIFY(!recovered.agentAllowEdit);
        QVERIFY(!recovered.agentAllowSave);
        QVERIFY(recovered.agentRoots.isEmpty());
        QCOMPARE(loadPreferences().brightnessLevel, 3);
        QString resetError;
        QVERIFY2(resetProfile(&resetError), qPrintable(resetError));
        QVERIFY(writePendingSettingsImport(pending));
        QVERIFY(applyPendingSettingsImport());
        recovered = loadPreferences();
        QVERIFY(!recovered.agentControlEnabled);
        auto granted = recovered;
        granted.agentControlEnabled = true;
        QVERIFY(savePreferences(recovered, granted, true));
        QVERIFY(loadPreferences().agentControlEnabled);
    }
}

void PreferencesStoreTest::isolatedLaunchesProtectRealConfig()
{
    const QString outerFile = preferencesFilePath();
    const QByteArray sentinel = "schema_version = 1\n[interface]\nbrightnessLevel = 3\n";
    QVERIFY(writeProfileFile(outerFile, sentinel));
    {
        ScopedStateHome nested;
        QVERIFY(nested.isValid());
        QVERIFY(initializePreferences());
        QVERIFY(outerFile != preferencesFilePath());
        QCOMPARE(loadPreferences().brightnessLevel, 1);
    }
    QCOMPARE(preferencesFilePath(), outerFile);
    QCOMPARE(readProfileFile(outerFile), sentinel);

    QTemporaryDir isolated;
    QVERIFY(isolated.isValid());
    QProcess process;
    process.start(QString::fromUtf8(PICTURA_TEST_EXECUTABLE),
                  {QStringLiteral("--headless"), QStringLiteral("--control"),
                   QStringLiteral("--state-home"), isolated.path(),
                   QStringLiteral("--control-socket"), isolated.filePath(QStringLiteral("prefs.sock"))});
    QVERIFY(process.waitForStarted());
    QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(isolated.filePath(QStringLiteral("config/kooka-pictura/prefs.toml"))), 15000);
    process.kill();
    QVERIFY(process.waitForFinished());
    QCOMPARE(readProfileFile(outerFile), sentinel);

    QTemporaryDir selfTestOverride;
    QVERIFY(selfTestOverride.isValid());
    QProcess selfTest;
    auto environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("TMPDIR"), selfTestOverride.path());
    selfTest.setProcessEnvironment(environment);
    selfTest.start(QString::fromUtf8(PICTURA_TEST_EXECUTABLE),
                   {QStringLiteral("--headless"), QStringLiteral("--self-test"),
                    QStringLiteral("--state-home"), selfTestOverride.path()});
    QVERIFY(selfTest.waitForStarted());
    QByteArray report;
    const auto reachedSelfTest = [&] {
        report += selfTest.readAllStandardError();
        return report.contains("pictura self-test: PASS core platform_headless");
    };
    QTRY_VERIFY_WITH_TIMEOUT(reachedSelfTest(), 30000);
    if (selfTest.state() != QProcess::NotRunning) {
        selfTest.kill();
        QVERIFY(selfTest.waitForFinished());
    }
    QVERIFY(!QFileInfo::exists(selfTestOverride.filePath(
        QStringLiteral("config/kooka-pictura/prefs.toml"))));
    QCOMPARE(readProfileFile(outerFile), sentinel);
}

void PreferencesStoreTest::legacyGpuDefaultSurvivesMigration()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("schemaVersion"), 1}}));
    QVERIFY(initializePreferences());
    QVERIFY(loadPreferences().gpuCompute);
}

void PreferencesStoreTest::shiftSwitchDefaultsToOn()
{
    QVERIFY(loadPreferences().useShiftKeyForToolSwitch);
}

void PreferencesStoreTest::shiftSwitchPersistsInPrefs()
{
    auto old = loadPreferences();
    auto edited = old;
    edited.useShiftKeyForToolSwitch = false;
    QVERIFY(savePreferences(old, edited));
    QVERIFY(!loadPreferences().useShiftKeyForToolSwitch);
    QVERIFY(!readProfileJson(sessionFilePath()).contains(QStringLiteral("useShiftKeyForToolSwitch")));
}

void PreferencesStoreTest::legacyShiftSwitchDefaultsToOn()
{
    QVERIFY(writeProfileJson(sessionFilePath(), {{QStringLiteral("schemaVersion"), 3}}));
    QVERIFY(initializePreferences());
    QVERIFY(loadPreferences().useShiftKeyForToolSwitch);
    QCOMPARE(loadSession().toolsColumns, 1);
}

QTEST_MAIN(PreferencesStoreTest)
#include "tst_preferences_store.moc"
