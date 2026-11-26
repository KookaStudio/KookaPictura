// The session store's single-writer guard (#332): a launch refuses to write a
// store another instance wrote after it started, while its own writes and a
// store from before its launch round-trip.

#include <QtTest/QtTest>

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

#include "frame.h"
#include "panels/panel_column.h"
#include "preferences.h"
#include "prefs_test_support.h"
#include "session.h"

#include "qt_test_support.h"

class SessionStoreTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    // Order matters: the refusal case must be the first save this process makes,
    // so a lazily-captured launch time would fail it.
    void anotherInstancesNewerWriteIsRefused();
    void aStoreFromBeforeTheLaunchIsAdopted();
    void ownWritesRoundTrip();
    void sessionAndPreferencesRestoreIndependently();
    void missingOrCorruptSessionKeepsPreferences();
    void failedSessionWriteKeepsPreviousFile();
    void legacyPanelLayoutDefaultsSurviveSplit();
    void columnRailModePersistsIndependently();
    void freePanRoundTrips();
    void snapCanvasToWorkspaceRoundTrips();
    void swatchesAbsentByDefault();
    void swatchesRoundTrip();
    void swatchesSurviveAnUnrelatedSave();

private:
    void writeStore(int railWidth, const QString& writer, qint64 writtenAt);

    pictura::test::ScopedStateHome stateHome_;
};

void SessionStoreTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void SessionStoreTest::writeStore(int railWidth, const QString& writer, qint64 writtenAt)
{
    const QString path = pictura::sessionFilePath();
    QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
    QJsonObject obj;
    obj.insert(QStringLiteral("railWidth"), railWidth);
    obj.insert(QStringLiteral("writerToken"), writer);
    obj.insert(QStringLiteral("writtenAt"), writtenAt);
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Compact));
    file.close();
}

void SessionStoreTest::ownWritesRoundTrip()
{
    pictura::SessionState state;
    QVERIFY(pictura::test::writeProfileJson(pictura::sessionFilePath(), {}));
    state.railWidth = 2;
    QVERIFY(pictura::saveSession(state));
    QVERIFY(!pictura::test::readProfileJson(pictura::sessionFilePath())
                .value(QStringLiteral("writerToken")).toString().isEmpty());
    state.railWidth = 3;
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().railWidth, 3);
}

void SessionStoreTest::anotherInstancesNewerWriteIsRefused()
{
    writeStore(3, QStringLiteral("another-instance"), QDateTime::currentMSecsSinceEpoch());
    pictura::SessionState state;
    state.railWidth = 2;
    const auto original = pictura::test::readProfileFile(pictura::sessionFilePath());
    QVERIFY(!pictura::saveSession(state));
    QCOMPARE(pictura::test::readProfileFile(pictura::sessionFilePath()), original);
    QCOMPARE(pictura::loadSession().railWidth, 3);
}

void SessionStoreTest::aStoreFromBeforeTheLaunchIsAdopted()
{
    writeStore(3, QStringLiteral("another-instance"), 1);
    pictura::SessionState state;
    state.railWidth = 2;
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().railWidth, 2);
    QFile file(pictura::sessionFilePath());
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QJsonObject obj = QJsonDocument::fromJson(file.readAll()).object();
    QVERIFY(obj.value(QStringLiteral("writerToken")).toString()
            != QStringLiteral("another-instance"));
}

void SessionStoreTest::freePanRoundTrips()
{
    pictura::SessionState state;
    state.freePan = false;
    QVERIFY(pictura::saveSession(state));
    QVERIFY(!pictura::loadSession().freePan);

    state.freePan = true;
    QVERIFY(pictura::saveSession(state));
    QVERIFY(pictura::loadSession().freePan);
}

void SessionStoreTest::snapCanvasToWorkspaceRoundTrips()
{
    pictura::SessionState state;
    QVERIFY(!pictura::loadSession().snapCanvasToWorkspace);
    state.snapCanvasToWorkspace = true;
    QVERIFY(pictura::saveSession(state));
    QVERIFY(pictura::loadSession().snapCanvasToWorkspace);

    state.snapCanvasToWorkspace = false;
    QVERIFY(pictura::saveSession(state));
    QVERIFY(!pictura::loadSession().snapCanvasToWorkspace);
}

void SessionStoreTest::swatchesAbsentByDefault()
{
    const pictura::SessionState state = pictura::loadSession();
    QVERIFY2(!state.swatchesLoaded, "an older store has no swatch key");
    QVERIFY(state.swatches.isEmpty());
}

void SessionStoreTest::swatchesRoundTrip()
{
    QJsonObject entry;
    entry.insert(QStringLiteral("name"), QStringLiteral("Swatch 1"));
    entry.insert(QStringLiteral("color"), QStringLiteral("#010203"));

    pictura::SessionState state;
    state.swatches = QJsonArray{entry};
    state.swatchesLoaded = true;
    QVERIFY(pictura::saveSession(state));

    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(loaded.swatchesLoaded);
    QCOMPARE(loaded.swatches.size(), 1);
    QCOMPARE(loaded.swatches.at(0).toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("Swatch 1"));
    QCOMPARE(loaded.swatches.at(0).toObject().value(QStringLiteral("color")).toString(),
             QStringLiteral("#010203"));
}

void SessionStoreTest::swatchesSurviveAnUnrelatedSave()
{
    pictura::SessionState state = pictura::loadSession();
    QVERIFY(state.swatchesLoaded);
    state.freePan = !state.freePan;
    QVERIFY(pictura::saveSession(state));

    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(loaded.swatchesLoaded);
    QCOMPARE(loaded.swatches.size(), 1);
    QCOMPARE(loaded.swatches.at(0).toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("Swatch 1"));
}

void SessionStoreTest::sessionAndPreferencesRestoreIndependently()
{
    pictura::test::ScopedStateHome isolated;
    QVERIFY(isolated.isValid());
    auto preferences = pictura::loadPreferences();
    auto updated = preferences;
    updated.brightnessLevel = 3;
    QVERIFY(pictura::savePreferences(preferences, updated));
    pictura::SessionState state;
    state.layout = "session layout";
    state.recent = {QStringLiteral("/tmp/kept.psd")};
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().layout, state.layout);
    QCOMPARE(pictura::loadSession().recent, state.recent);
    QCOMPARE(pictura::loadPreferences().brightnessLevel, 3);
    QVERIFY(!pictura::test::readProfileJson(pictura::sessionFilePath())
                .contains(QStringLiteral("brightnessLevel")));
}

void SessionStoreTest::missingOrCorruptSessionKeepsPreferences()
{
    pictura::test::ScopedStateHome isolated;
    QVERIFY(isolated.isValid());
    const auto before = pictura::loadPreferences();
    auto after = before;
    after.brightnessLevel = 3;
    QVERIFY(pictura::savePreferences(before, after));
    QVERIFY(pictura::loadSession().layout.isEmpty());
    QCOMPARE(pictura::loadPreferences().brightnessLevel, 3);
    QVERIFY(pictura::test::writeProfileFile(pictura::sessionFilePath(), "{bad"));
    QVERIFY(pictura::loadSession().layout.isEmpty());
    QCOMPARE(pictura::loadPreferences().brightnessLevel, 3);
}

void SessionStoreTest::failedSessionWriteKeepsPreviousFile()
{
    pictura::test::ScopedStateHome isolated;
    QVERIFY(isolated.isValid());
    const auto path = pictura::sessionFilePath();
    QVERIFY(pictura::test::writeProfileFile(path, "{\"recent\":[\"/tmp/original.psd\"]}"));
    const auto previous = pictura::test::readProfileFile(path);
    pictura::setSessionCommitFailureForTest(true);
    const bool saved = pictura::saveSession(pictura::SessionState{});
    pictura::setSessionCommitFailureForTest(false);
    QVERIFY(!saved);
    QCOMPARE(pictura::test::readProfileFile(path), previous);
}

void SessionStoreTest::legacyPanelLayoutDefaultsSurviveSplit()
{
    const auto groupsFor = [](const QString& panel) {
        const QJsonArray panels{panel};
        return QJsonArray{QJsonObject{{QStringLiteral("order"), panels},
                                      {QStringLiteral("visible"), panels}}};
    };
    const auto layers = QStringLiteral("layersPanel");
    const auto styles = QStringLiteral("stylesPanel");
    const auto primaryGroups = groupsFor(layers);
    const QJsonArray columns{
        QJsonObject{{QStringLiteral("side"), QStringLiteral("right")},
                    {QStringLiteral("groups"), primaryGroups}},
        QJsonObject{{QStringLiteral("side"), QStringLiteral("left")},
                    {QStringLiteral("groups"), groupsFor(styles)}},
    };
    for (const int schema : {5, 6, 7}) {
        pictura::test::ScopedStateHome isolated;
        QVERIFY(isolated.isValid());
        QJsonObject legacy{{QStringLiteral("schemaVersion"), schema},
                           {QStringLiteral("brightnessLevel"), 3},
                           {QStringLiteral("railWidth"), 334},
                           {QStringLiteral("panelRailMode"), QStringLiteral("iconic")},
                           {QStringLiteral("unknown"), 7}};
        legacy.insert(schema == 5 ? QStringLiteral("panelGroups") : QStringLiteral("panelColumns"),
                      schema == 5 ? primaryGroups : columns);
        QVERIFY(pictura::test::writeProfileJson(pictura::sessionFilePath(), legacy));
        QVERIFY(pictura::initializePreferences());
        QCOMPARE(pictura::loadPreferences().brightnessLevel, 3);
        auto window = pictura::test::makeMainWindow();
        auto* primary = window->panelColumn();
        QVERIFY(primary);
        QCOMPARE(window->columnForPanel(layers), primary);
        QVERIFY(primary->railMode());
        QCOMPARE(primary->persistedWidth(), 334);
        if (schema >= 6) {
            auto* secondary = window->columnForPanel(styles);
            QVERIFY(secondary && secondary != primary);
            QVERIFY(secondary->railMode());
            QCOMPARE(secondary->persistedWidth(), pictura::PanelColumn::defaultNormalWidth());
        }
        QCOMPARE(pictura::test::readProfileJson(pictura::sessionFilePath())
                     .value(QStringLiteral("unknown")).toInt(), 7);
    }
}

void SessionStoreTest::columnRailModePersistsIndependently()
{
    pictura::test::ScopedStateHome isolated;
    QVERIFY(isolated.isValid());
    pictura::SessionState state;
    state.panelColumns = {QJsonObject{{QStringLiteral("side"), QStringLiteral("right")},
                                      {QStringLiteral("railMode"), QStringLiteral("normal")}},
                          QJsonObject{{QStringLiteral("side"), QStringLiteral("left")},
                                      {QStringLiteral("railMode"), QStringLiteral("iconic")}}};
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().panelColumns, state.panelColumns);
}

PICTURA_QT_TEST_MAIN(SessionStoreTest)
#include "tst_session_store.moc"
