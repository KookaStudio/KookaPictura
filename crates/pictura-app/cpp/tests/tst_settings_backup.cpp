// Settings backup/restore (#1155): the single pictura.json export, the staged
// import that applies before the next launch, validation and sanitization, and
// the Preferences entry points.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QPushButton>

#include "frame.h"
#include "preferences_dialog.h"
#include "session.h"
#include "settings_backup.h"
#include "workspace_store.h"

#include "qt_test_support.h"

namespace {

class ScopedDataHome {
public:
    ScopedDataHome()
        : valid_(dir_.isValid())
        , hadPrior_(qEnvironmentVariableIsSet("XDG_DATA_HOME"))
        , prior_(hadPrior_ ? qgetenv("XDG_DATA_HOME") : QByteArray())
    {
        if (valid_) {
            qputenv("XDG_DATA_HOME", dir_.path().toUtf8());
        }
    }

    ~ScopedDataHome()
    {
        if (!valid_) {
            return;
        }
        if (hadPrior_) {
            qputenv("XDG_DATA_HOME", prior_);
        } else {
            qunsetenv("XDG_DATA_HOME");
        }
    }

    ScopedDataHome(const ScopedDataHome&) = delete;
    ScopedDataHome& operator=(const ScopedDataHome&) = delete;

    bool isValid() const { return valid_; }
    QString path() const { return dir_.path(); }

private:
    QTemporaryDir dir_;
    bool valid_;
    bool hadPrior_;
    QByteArray prior_;
};

void clearDirectory(const QString& path)
{
    QDir dir(path);
    if (dir.exists()) {
        dir.removeRecursively();
    }
    QDir().mkpath(path);
}

bool writeBytes(const QString& path, const QByteArray& bytes)
{
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        return false;
    }
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    return file.write(bytes) == bytes.size();
}

bool writeJson(const QString& path, const QJsonObject& object)
{
    return writeBytes(path, QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QJsonObject readJson(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    return QJsonDocument::fromJson(file.readAll()).object();
}

QByteArray readBytes(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    return file.readAll();
}

QJsonObject minimalBackup(const QJsonObject& session)
{
    QJsonObject backup;
    backup.insert(QStringLiteral("format"), QStringLiteral("pictura-settings"));
    backup.insert(QStringLiteral("formatVersion"), 1);
    backup.insert(QStringLiteral("session"), session);
    return backup;
}

} // namespace

class SettingsBackupTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void preferencesOffersBothButtons();
    void exportCarriesEveryStore();
    void unknownSessionKeysSurvive();
    void exportExcludesCaches();
    void malformedFileIsRefused();
    void newerFormatIsRefused();
    void unsafeNamesAreRefused();
    void importBacksUpCurrentSettings();
    void importSanitizesMachineSpecificValues();
    void validImportIsStaged();
    void pendingImportAppliesEverySection();
    void absentSectionsAreLeftAlone();
    void corruptPendingIsQuarantined();
    void roundTripRestoresEveryStore();
    void frameStagesImportAndRequestsRestart();

private:
    QString stateDir() const;
    QString adjustmentDir() const;
    QString actionsDir() const;
    QString pdfDir() const;
    QString palettePath() const;
    QString backupPath() const;

    pictura::test::ScopedStateHome stateHome_;
    ScopedDataHome dataHome_;
    QTemporaryDir backupDir_;
};

QString SettingsBackupTest::stateDir() const
{
    return QFileInfo(pictura::sessionFilePath()).absolutePath();
}

QString SettingsBackupTest::adjustmentDir() const
{
    return dataHome_.path() + QStringLiteral("/kooka-pictura/Presets/Adjustments");
}

QString SettingsBackupTest::actionsDir() const
{
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
        + QStringLiteral("/Presets/Actions");
}

QString SettingsBackupTest::pdfDir() const
{
    return stateDir() + QStringLiteral("/PDF Presets");
}

QString SettingsBackupTest::palettePath() const
{
    return stateDir() + QStringLiteral("/Actions Palette.psp");
}

QString SettingsBackupTest::backupPath() const
{
    return backupDir_.path() + QStringLiteral("/pictura.json");
}

void SettingsBackupTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dataHome_.isValid());
    QVERIFY(backupDir_.isValid());
}

void SettingsBackupTest::init()
{
    clearDirectory(stateDir());
    clearDirectory(dataHome_.path());
    clearDirectory(backupDir_.path());
}

void SettingsBackupTest::cleanup()
{
}

void SettingsBackupTest::preferencesOffersBothButtons()
{
    pictura::PreferencesDialog dialog;
    auto* exportButton = dialog.findChild<QPushButton*>(
        QStringLiteral("preferencesExportSettings"));
    auto* importButton = dialog.findChild<QPushButton*>(
        QStringLiteral("preferencesImportSettings"));
    QVERIFY(exportButton != nullptr);
    QVERIFY(importButton != nullptr);

    QSignalSpy exportSpy(&dialog, &pictura::PreferencesDialog::exportSettingsRequested);
    QSignalSpy importSpy(&dialog, &pictura::PreferencesDialog::importSettingsRequested);
    exportButton->click();
    importButton->click();
    QCOMPARE(exportSpy.count(), 1);
    QCOMPARE(importSpy.count(), 1);
}

void SettingsBackupTest::exportCarriesEveryStore()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 2},
                                  {QStringLiteral("unknownKey"), QStringLiteral("keep")}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/index.json"),
                      QJsonObject{{QStringLiteral("schemaVersion"), 1},
                                  {QStringLiteral("activeWorkspace"), QStringLiteral("w1")},
                                  {QStringLiteral("order"), QJsonArray{QStringLiteral("w1")}}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/w1.json"),
                      QJsonObject{{QStringLiteral("id"), QStringLiteral("w1")},
                                  {QStringLiteral("name"), QStringLiteral("One")},
                                  {QStringLiteral("kind"), QStringLiteral("user")}}));
    QVERIFY(writeBytes(adjustmentDir() + QStringLiteral("/levels.presets"),
                       QByteArrayLiteral("# presets\n")));
    const QByteArray actionBytes("\x00\x01\x02", 3);
    QVERIFY(writeBytes(actionsDir() + QStringLiteral("/set.atn"), actionBytes));
    QVERIFY(writeBytes(pdfDir() + QStringLiteral("/press.joboptions"),
                       QByteArrayLiteral("press options")));
    const QByteArray paletteBytes("\x00\x10\x20", 3);
    QVERIFY(writeBytes(palettePath(), paletteBytes));

    const QJsonObject backup = pictura::collectSettingsBackup();
    QCOMPARE(backup.value(QStringLiteral("format")).toString(),
             QStringLiteral("pictura-settings"));
    QCOMPARE(backup.value(QStringLiteral("formatVersion")).toInt(), 1);
    QVERIFY(!backup.value(QStringLiteral("appVersion")).toString().isEmpty());
    QVERIFY(!backup.value(QStringLiteral("exportedAt")).toString().isEmpty());

    const QJsonObject session = backup.value(QStringLiteral("session")).toObject();
    QCOMPARE(session.value(QStringLiteral("brightnessLevel")).toInt(), 2);

    const QJsonObject workspaces = backup.value(QStringLiteral("workspaces")).toObject();
    QCOMPARE(workspaces.value(QStringLiteral("index"))
                 .toObject()
                 .value(QStringLiteral("activeWorkspace"))
                 .toString(),
             QStringLiteral("w1"));
    QVERIFY(workspaces.value(QStringLiteral("files"))
                .toObject()
                .contains(QStringLiteral("w1.json")));

    const QJsonObject presets = backup.value(QStringLiteral("presets")).toObject();
    const QJsonArray adjustments = presets.value(QStringLiteral("adjustments")).toArray();
    QCOMPARE(adjustments.size(), 1);
    QCOMPARE(adjustments.at(0).toObject().value(QStringLiteral("file")).toString(),
             QStringLiteral("levels.presets"));
    QVERIFY(adjustments.at(0).toObject().contains(QStringLiteral("text")));

    const QJsonArray actions = presets.value(QStringLiteral("actions")).toArray();
    QCOMPARE(actions.size(), 1);
    QCOMPARE(actions.at(0).toObject().value(QStringLiteral("file")).toString(),
             QStringLiteral("set.atn"));
    QVERIFY(actions.at(0).toObject().contains(QStringLiteral("base64")));

    const QJsonArray pdf = presets.value(QStringLiteral("pdf")).toArray();
    QCOMPARE(pdf.size(), 1);
    QCOMPARE(pdf.at(0).toObject().value(QStringLiteral("file")).toString(),
             QStringLiteral("press.joboptions"));

    QVERIFY(presets.value(QStringLiteral("actionsPalette")).toObject().contains(
        QStringLiteral("base64")));

    QVERIFY(pictura::writeSettingsBackup(backupPath(), backup, nullptr));
    const QJsonObject reread = readJson(backupPath());
    QCOMPARE(reread.value(QStringLiteral("format")).toString(),
             QStringLiteral("pictura-settings"));
}

void SettingsBackupTest::unknownSessionKeysSurvive()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 4},
                                  {QStringLiteral("unknownKey"), QStringLiteral("keep")}}));
    const QJsonObject backup = pictura::collectSettingsBackup();
    QVERIFY(pictura::writeSettingsBackup(backupPath(), backup, nullptr));

    QVERIFY(writeJson(pictura::sessionFilePath(), QJsonObject{}));
    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));
    QVERIFY(pictura::applyPendingSettingsImport(nullptr));

    const QJsonObject restored = readJson(pictura::sessionFilePath());
    QCOMPARE(restored.value(QStringLiteral("unknownKey")).toString(),
             QStringLiteral("keep"));
}

void SettingsBackupTest::exportExcludesCaches()
{
    QVERIFY(writeJson(stateDir() + QStringLiteral("/gpu-backend.json"), QJsonObject{}));
    QVERIFY(writeJson(stateDir() + QStringLiteral("/gpu-init.marker"), QJsonObject{}));
    QVERIFY(QDir().mkpath(stateDir() + QStringLiteral("/recovery/session")));

    const QJsonObject backup = pictura::collectSettingsBackup();
    const QByteArray json = QJsonDocument(backup).toJson(QJsonDocument::Compact);
    QVERIFY(!json.contains("gpu-backend"));
    QVERIFY(!json.contains("gpu-init"));
    QVERIFY(!json.contains("recovery"));
}

void SettingsBackupTest::malformedFileIsRefused()
{
    QVERIFY(writeBytes(backupPath(), QByteArrayLiteral("not json")));
    QString error;
    QVERIFY(!pictura::stageSettingsImport(backupPath(), &error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!pictura::hasPendingSettingsImport());

    QVERIFY(writeJson(backupPath(),
                      QJsonObject{{QStringLiteral("format"), QStringLiteral("other")}}));
    error.clear();
    QVERIFY(!pictura::stageSettingsImport(backupPath(), &error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!pictura::hasPendingSettingsImport());
}

void SettingsBackupTest::newerFormatIsRefused()
{
    QVERIFY(writeJson(backupPath(),
                      QJsonObject{{QStringLiteral("format"), QStringLiteral("pictura-settings")},
                                  {QStringLiteral("formatVersion"), 2}}));
    QString error;
    QVERIFY(!pictura::stageSettingsImport(backupPath(), &error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!pictura::hasPendingSettingsImport());
}

void SettingsBackupTest::unsafeNamesAreRefused()
{
    QJsonObject backup = minimalBackup(QJsonObject{{QStringLiteral("brightnessLevel"), 1}});
    backup.insert(QStringLiteral("workspaces"),
                  QJsonObject{{QStringLiteral("index"), QJsonObject{}},
                              {QStringLiteral("files"),
                               QJsonObject{{QStringLiteral("../evil.json"), QJsonObject{}}}}});
    QVERIFY(writeJson(backupPath(), backup));
    QString error;
    QVERIFY(!pictura::stageSettingsImport(backupPath(), &error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!pictura::hasPendingSettingsImport());

    backup.remove(QStringLiteral("workspaces"));
    backup.insert(QStringLiteral("presets"),
                  QJsonObject{{QStringLiteral("adjustments"),
                               QJsonArray{QJsonObject{
                                   {QStringLiteral("file"), QStringLiteral("../evil.presets")},
                                   {QStringLiteral("text"), QStringLiteral("x")}}}}});
    QVERIFY(writeJson(backupPath(), backup));
    error.clear();
    QVERIFY(!pictura::stageSettingsImport(backupPath(), &error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!pictura::hasPendingSettingsImport());
}

void SettingsBackupTest::importBacksUpCurrentSettings()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 2}}));
    QString error;
    const QString path = pictura::backupCurrentSettings(&error);
    QVERIFY2(!path.isEmpty(), qPrintable(error));
    QVERIFY(QFileInfo::exists(path));
    QCOMPARE(QFileInfo(path).absolutePath(), pictura::settingsBackupDirectory());
    const QJsonObject backup = readJson(path);
    QCOMPARE(backup.value(QStringLiteral("session"))
                 .toObject()
                 .value(QStringLiteral("brightnessLevel"))
                 .toInt(),
             2);
}

void SettingsBackupTest::importSanitizesMachineSpecificValues()
{
    const QString existing = stateDir() + QStringLiteral("/existing.png");
    QVERIFY(writeBytes(existing, QByteArrayLiteral("image")));
    const QString missing = stateDir() + QStringLiteral("/missing.png");

    QJsonObject session{
        {QStringLiteral("recent"), QJsonArray{existing, missing}},
        {QStringLiteral("windowGeometry"), QStringLiteral("AAAA")},
        {QStringLiteral("windowMaximized"), true},
    };
    QVERIFY(writeJson(backupPath(), minimalBackup(session)));
    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));

    const QJsonObject pending = readJson(pictura::pendingSettingsImportPath());
    const QJsonObject staged = pending.value(QStringLiteral("session")).toObject();
    const QJsonArray recent = staged.value(QStringLiteral("recent")).toArray();
    QCOMPARE(recent.size(), 1);
    QCOMPARE(recent.at(0).toString(), existing);
    QVERIFY(!staged.contains(QStringLiteral("windowGeometry")));
    QVERIFY(!staged.contains(QStringLiteral("windowMaximized")));
}

void SettingsBackupTest::validImportIsStaged()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 1}}));
    QVERIFY(writeJson(backupPath(),
                      minimalBackup(QJsonObject{{QStringLiteral("brightnessLevel"), 7}})));

    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));
    QVERIFY(pictura::hasPendingSettingsImport());
    QCOMPARE(pictura::loadSession().brightnessLevel, 1);

    const QJsonObject pending = readJson(pictura::pendingSettingsImportPath());
    QCOMPARE(pending.value(QStringLiteral("session"))
                 .toObject()
                 .value(QStringLiteral("brightnessLevel"))
                 .toInt(),
             7);
}

void SettingsBackupTest::pendingImportAppliesEverySection()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 5}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/index.json"),
                      QJsonObject{{QStringLiteral("activeWorkspace"), QStringLiteral("w1")}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/w1.json"),
                      QJsonObject{{QStringLiteral("id"), QStringLiteral("w1")}}));
    QVERIFY(writeBytes(adjustmentDir() + QStringLiteral("/levels.presets"),
                       QByteArrayLiteral("levels")));
    const QByteArray actionBytes("\x01\x02", 2);
    QVERIFY(writeBytes(actionsDir() + QStringLiteral("/set.atn"), actionBytes));
    QVERIFY(writeBytes(pdfDir() + QStringLiteral("/press.joboptions"),
                       QByteArrayLiteral("pdf")));
    const QByteArray paletteBytes("\x03\x04", 2);
    QVERIFY(writeBytes(palettePath(), paletteBytes));

    const QJsonObject backup = pictura::collectSettingsBackup();
    QVERIFY(pictura::writeSettingsBackup(backupPath(), backup, nullptr));

    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 1}}));
    QFile::remove(pictura::workspaceStoreDir() + QStringLiteral("/w1.json"));
    QFile::remove(adjustmentDir() + QStringLiteral("/levels.presets"));
    QFile::remove(actionsDir() + QStringLiteral("/set.atn"));
    QFile::remove(pdfDir() + QStringLiteral("/press.joboptions"));
    QFile::remove(palettePath());

    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));
    QVERIFY(pictura::applyPendingSettingsImport(nullptr));
    QVERIFY(!pictura::hasPendingSettingsImport());

    QCOMPARE(pictura::loadSession().brightnessLevel, 5);
    QVERIFY(QFileInfo::exists(pictura::workspaceStoreDir() + QStringLiteral("/w1.json")));
    QCOMPARE(readBytes(adjustmentDir() + QStringLiteral("/levels.presets")),
             QByteArrayLiteral("levels"));
    QCOMPARE(readBytes(actionsDir() + QStringLiteral("/set.atn")), actionBytes);
    QCOMPARE(readBytes(pdfDir() + QStringLiteral("/press.joboptions")),
             QByteArrayLiteral("pdf"));
    QCOMPARE(readBytes(palettePath()), paletteBytes);
}

void SettingsBackupTest::absentSectionsAreLeftAlone()
{
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/index.json"),
                      QJsonObject{{QStringLiteral("activeWorkspace"), QStringLiteral("keep")}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/keep.json"),
                      QJsonObject{{QStringLiteral("id"), QStringLiteral("keep")}}));
    QVERIFY(writeBytes(adjustmentDir() + QStringLiteral("/keep.presets"),
                       QByteArrayLiteral("keep")));

    QVERIFY(writeJson(backupPath(),
                      minimalBackup(QJsonObject{{QStringLiteral("brightnessLevel"), 3}})));
    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));
    QVERIFY(pictura::applyPendingSettingsImport(nullptr));

    QVERIFY(QFileInfo::exists(pictura::workspaceStoreDir() + QStringLiteral("/keep.json")));
    QCOMPARE(readBytes(adjustmentDir() + QStringLiteral("/keep.presets")),
             QByteArrayLiteral("keep"));
}

void SettingsBackupTest::corruptPendingIsQuarantined()
{
    QVERIFY(writeBytes(pictura::pendingSettingsImportPath(), QByteArrayLiteral("not json")));
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 9}}));

    QString error;
    QVERIFY(!pictura::applyPendingSettingsImport(&error));
    QVERIFY(!error.isEmpty());
    QVERIFY(!QFileInfo::exists(pictura::pendingSettingsImportPath()));
    const QStringList quarantined =
        QDir(stateDir()).entryList({QStringLiteral("pending-settings.json.corrupt-*")},
                                   QDir::Files);
    QCOMPARE(quarantined.size(), 1);
    QCOMPARE(pictura::loadSession().brightnessLevel, 9);
}

void SettingsBackupTest::roundTripRestoresEveryStore()
{
    QVERIFY(writeJson(pictura::sessionFilePath(),
                      QJsonObject{{QStringLiteral("brightnessLevel"), 6},
                                  {QStringLiteral("unknownKey"), QStringLiteral("keep")}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/index.json"),
                      QJsonObject{{QStringLiteral("activeWorkspace"), QStringLiteral("w2")}}));
    QVERIFY(writeJson(pictura::workspaceStoreDir() + QStringLiteral("/w2.json"),
                      QJsonObject{{QStringLiteral("id"), QStringLiteral("w2")}}));
    QVERIFY(writeBytes(adjustmentDir() + QStringLiteral("/curve.presets"),
                       QByteArrayLiteral("curve")));
    const QByteArray actionBytes("\x10\x11", 2);
    QVERIFY(writeBytes(actionsDir() + QStringLiteral("/round.atn"), actionBytes));
    QVERIFY(writeBytes(pdfDir() + QStringLiteral("/round.joboptions"),
                       QByteArrayLiteral("round")));
    const QByteArray paletteBytes("\x20\x21", 2);
    QVERIFY(writeBytes(palettePath(), paletteBytes));

    const QJsonObject backup = pictura::collectSettingsBackup();
    QVERIFY(pictura::writeSettingsBackup(backupPath(), backup, nullptr));

    QVERIFY(writeJson(pictura::sessionFilePath(), QJsonObject{}));
    QFile::remove(pictura::workspaceStoreDir() + QStringLiteral("/w2.json"));
    QFile::remove(adjustmentDir() + QStringLiteral("/curve.presets"));
    QFile::remove(actionsDir() + QStringLiteral("/round.atn"));
    QFile::remove(pdfDir() + QStringLiteral("/round.joboptions"));
    QFile::remove(palettePath());

    QVERIFY(pictura::stageSettingsImport(backupPath(), nullptr));
    QVERIFY(pictura::applyPendingSettingsImport(nullptr));

    const QJsonObject restored = readJson(pictura::sessionFilePath());
    QCOMPARE(restored.value(QStringLiteral("brightnessLevel")).toInt(), 6);
    QCOMPARE(restored.value(QStringLiteral("unknownKey")).toString(),
             QStringLiteral("keep"));
    QVERIFY(QFileInfo::exists(pictura::workspaceStoreDir() + QStringLiteral("/w2.json")));
    QCOMPARE(readBytes(adjustmentDir() + QStringLiteral("/curve.presets")),
             QByteArrayLiteral("curve"));
    QCOMPARE(readBytes(actionsDir() + QStringLiteral("/round.atn")), actionBytes);
    QCOMPARE(readBytes(pdfDir() + QStringLiteral("/round.joboptions")),
             QByteArrayLiteral("round"));
    QCOMPARE(readBytes(palettePath()), paletteBytes);
}

void SettingsBackupTest::frameStagesImportAndRequestsRestart()
{
    QVERIFY(writeJson(backupPath(),
                      minimalBackup(QJsonObject{{QStringLiteral("brightnessLevel"), 8}})));
    pictura::setSettingsRestartPromptInteractive(false);

    QString error;
    QVERIFY2(pictura::importSettingsFile(backupPath(), nullptr, &error), qPrintable(error));
    QVERIFY(pictura::hasPendingSettingsImport());
    QCOMPARE(pictura::settingsRestartPromptCountForTest(), 1);
    QCOMPARE(QDir(pictura::settingsBackupDirectory())
                 .entryList({QStringLiteral("pictura-*.json")}, QDir::Files)
                 .size(),
             1);
}

QTEST_MAIN(SettingsBackupTest)
#include "tst_settings_backup.moc"
