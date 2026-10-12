// The session store's single-writer guard (#332): a launch refuses to write a
// store another instance wrote after it started, while its own writes and a
// store from before its launch round-trip.

#include <QtTest/QtTest>

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

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

private:
    void writeStore(int brightness, const QString& writer, qint64 writtenAt);

    pictura::test::ScopedStateHome stateHome_;
};

void SessionStoreTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void SessionStoreTest::writeStore(int brightness, const QString& writer, qint64 writtenAt)
{
    const QString path = pictura::sessionFilePath();
    QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
    QJsonObject obj;
    obj.insert(QStringLiteral("brightnessLevel"), brightness);
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
    state.brightnessLevel = 2;
    QVERIFY(pictura::saveSession(state));
    state.brightnessLevel = 3;
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().brightnessLevel, 3);
}

void SessionStoreTest::anotherInstancesNewerWriteIsRefused()
{
    writeStore(3, QStringLiteral("another-instance"), QDateTime::currentMSecsSinceEpoch());
    pictura::SessionState state;
    state.brightnessLevel = 2;
    QVERIFY(!pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().brightnessLevel, 3);
}

void SessionStoreTest::aStoreFromBeforeTheLaunchIsAdopted()
{
    writeStore(3, QStringLiteral("another-instance"), 1);
    pictura::SessionState state;
    state.brightnessLevel = 2;
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().brightnessLevel, 2);
    QFile file(pictura::sessionFilePath());
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QJsonObject obj = QJsonDocument::fromJson(file.readAll()).object();
    QVERIFY(obj.value(QStringLiteral("writerToken")).toString()
            != QStringLiteral("another-instance"));
}

QTEST_MAIN(SessionStoreTest)
#include "tst_session_store.moc"
