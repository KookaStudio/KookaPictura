// Crash recovery and autosave (#184): the per-instance recovery store, the
// autosave tick, recovering an orphaned session, and the File Handling
// preference.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QLockFile>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QComboBox>

#include "commands.h"
#include "frame.h"
#include "preferences_dialog.h"
#include "recovery_store.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_core/recovery.cxxqt.h"
#include "pictura_app/src/cxxqt_object/path_list.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"

#include "qt_test_support.h"

namespace {

// A closed triangle on the Work Path: a history step, so the document is dirty.
void drawTriangle(pictura::PictureView& v)
{
    for (const QPointF& p : {QPointF(10, 10), QPointF(50, 12), QPointF(30, 40)}) {
        pictura::path_append_corner(v, p.x(), p.y(), false);
        pictura::path_commit_anchor(v);
    }
    pictura::path_close(v);
}

QStringList snapshots(const QString& dir)
{
    return QDir(dir).entryList({QStringLiteral("*.psd")}, QDir::Files, QDir::Name);
}

// Finish every in-flight snapshot write of `window`'s documents.
void waitForWrites(pictura::PicturaMainWindow& window)
{
    for (int i = 0; i < window.documentCount(); ++i) {
        pictura::recovery_wait(*window.viewAt(i));
    }
}

// Move `from`'s snapshots and manifests into a new lock-less session directory,
// as a crashed instance leaves them.
QString fakeCrashedSession(const QString& from)
{
    const QString dir = pictura::RecoveryStore::rootPath() + QStringLiteral("/999999-1");
    QDir().mkpath(dir);
    for (const QString& name :
         QDir(from).entryList({QStringLiteral("*.psd"), QStringLiteral("*.json")}, QDir::Files)) {
        QFile::copy(from + QLatin1Char('/') + name, dir + QLatin1Char('/') + name);
    }
    return dir;
}

} // namespace

class RecoveryTest : public QObject {
    Q_OBJECT

private slots:
    void recoveredNames();
    void autosaveSnapshotsOnlyUnsavedChanges();
    void saveAndCloseDropTheSnapshot();
    void cleanShutdownRemovesTheSession();
    void liveSessionsAreNotOrphans();
    void recoverReopensAnOrphanedDocument();
    void partialRecoveryQuarantinesFailedDocument();
    void fileHandlingPreferenceTogglesAutosave();
};

void RecoveryTest::recoveredNames()
{
    QCOMPARE(pictura::recoveredDocumentName(QStringLiteral("photo.psd")),
             QStringLiteral("photo-Recovered.psd"));
    QCOMPARE(pictura::recoveredDocumentName(QStringLiteral("Untitled-1")),
             QStringLiteral("Untitled-1-Recovered"));
    QCOMPARE(pictura::recoveredDocumentName(QStringLiteral("scan.v2.png")),
             QStringLiteral("scan.v2-Recovered.png"));
}

void RecoveryTest::autosaveSnapshotsOnlyUnsavedChanges()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->enableRecovery();
    const QString dir = window->recoverySessionDirForTest();
    QVERIFY2(!dir.isEmpty() && QFileInfo::exists(dir + QStringLiteral("/lock")),
             "the session directory is created and locked");

    QVERIFY(window->newDocument(QString(), 64, 48, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    window->autosaveRecovery();
    waitForWrites(*window);
    QVERIFY2(snapshots(dir).isEmpty(), "an unmodified document is not snapshotted");

    drawTriangle(*window->activeView());
    QVERIFY(window->isDocumentDirty(0));
    window->autosaveRecovery();
    waitForWrites(*window);
    QCOMPARE(snapshots(dir).size(), 1);
    const QString snapshot = dir + QLatin1Char('/') + snapshots(dir).first();

    // Nothing changed since: the next tick does not write it again.
    QFile::remove(snapshot);
    window->autosaveRecovery();
    waitForWrites(*window);
    QVERIFY2(snapshots(dir).isEmpty(), "an unchanged document is not rewritten");
    window->shutdownRecovery();
}

void RecoveryTest::saveAndCloseDropTheSnapshot()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir out;
    QVERIFY(out.isValid());
    auto window = pictura::test::makeMainWindow();
    window->enableRecovery();
    const QString dir = window->recoverySessionDirForTest();

    for (int i = 0; i < 2; ++i) {
        QVERIFY(window->newDocument(QString(), 32, 32, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        drawTriangle(*window->activeView());
    }
    window->autosaveRecovery();
    waitForWrites(*window);
    QCOMPARE(snapshots(dir).size(), 2);

    QVERIFY(window->saveActiveAs(out.filePath(QStringLiteral("saved.psd"))));
    QCOMPARE(snapshots(dir).size(), 1);
    QVERIFY(window->closeDocument(0, false));
    QVERIFY2(snapshots(dir).isEmpty(), "closing a document deletes its snapshot");
    QVERIFY2(QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files).isEmpty(),
             "and its manifest");
    window->shutdownRecovery();
}

void RecoveryTest::cleanShutdownRemovesTheSession()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QString dir;
    {
        auto window = pictura::test::makeMainWindow();
        window->enableRecovery();
        dir = window->recoverySessionDirForTest();
        QVERIFY(window->newDocument(QString(), 32, 32, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        drawTriangle(*window->activeView());
        window->autosaveRecovery();
        QVERIFY(QFileInfo::exists(dir));
    }
    QVERIFY2(!QFileInfo::exists(dir), "destroying the window removes the session");
    QVERIFY(pictura::RecoveryStore::orphans(QString()).isEmpty());
}

void RecoveryTest::liveSessionsAreNotOrphans()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    pictura::RecoveryStore own;
    QVERIFY(own.open());
    const QString other = pictura::RecoveryStore::rootPath() + QStringLiteral("/other");
    QVERIFY(QDir().mkpath(other));
    QLockFile live(other + QStringLiteral("/lock"));
    QVERIFY(live.tryLock(0));

    QVERIFY2(pictura::RecoveryStore::orphans(own.sessionDir()).isEmpty(),
             "neither this session nor a locked one is an orphan");
    live.unlock();
    QCOMPARE(pictura::RecoveryStore::orphans(own.sessionDir()).size(), 1);
    own.close();
}

void RecoveryTest::recoverReopensAnOrphanedDocument()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QString crashed;
    {
        auto before = pictura::test::makeMainWindow();
        before->enableRecovery();
        QVERIFY(before->newDocument(QString(), 64, 48, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        drawTriangle(*before->activeView());
        before->autosaveRecovery();
        waitForWrites(*before);
        crashed = fakeCrashedSession(before->recoverySessionDirForTest());
    }
    const QList<pictura::OrphanSession> orphans = pictura::RecoveryStore::orphans(QString());
    QCOMPARE(orphans.size(), 1);
    QCOMPARE(orphans.first().documents.size(), 1);
    QCOMPARE(orphans.first().documents.first().name, QStringLiteral("Untitled-1"));

    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(after->documentCount(), 1);
    QCOMPARE(after->documentName(0), QStringLiteral("Untitled-1-Recovered"));
    QVERIFY2(after->isDocumentDirty(0), "a recovered document has unsaved changes");
    QVERIFY2(after->documentPath(0).isEmpty(), "and no file, so Save asks where");
    pictura::PictureView* view = after->activeView();
    QCOMPARE(view->document_width(), 64);
    QCOMPARE(view->document_height(), 48);
    QVERIFY2(pictura::paths_has_work_path(*view), "the Work Path survives recovery");
    QVERIFY2(!QFileInfo::exists(crashed), "the orphaned session is discarded");
    QCOMPARE(snapshots(after->recoverySessionDirForTest()).size(), 1);
    after->shutdownRecovery();
}

void RecoveryTest::partialRecoveryQuarantinesFailedDocument()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QString crashed;
    QString failedSnapshot;
    {
        auto before = pictura::test::makeMainWindow();
        before->enableRecovery();
        for (int i = 0; i < 2; ++i) {
            QVERIFY(before->newDocument(QString(), 64, 48, QStringLiteral("rgb"), 8,
                                        QStringLiteral("white")));
            drawTriangle(*before->activeView());
        }
        before->autosaveRecovery();
        waitForWrites(*before);
        crashed = fakeCrashedSession(before->recoverySessionDirForTest());
    }

    const QList<pictura::OrphanSession> orphans = pictura::RecoveryStore::orphans(QString());
    QCOMPARE(orphans.size(), 1);
    QCOMPARE(orphans.first().documents.size(), 2);
    failedSnapshot = orphans.first().documents.at(1).snapshot;
    {
        QFile file(failedSnapshot);
        QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
        file.write("not a psd");
    }

    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(after->documentCount(), 1);

    const QString quarantine = pictura::RecoveryStore::quarantineRoot() + QLatin1Char('/')
                               + QFileInfo(crashed).fileName();
    const QString snapshotDest =
        quarantine + QLatin1Char('/') + QFileInfo(failedSnapshot).fileName();
    const QString manifestDest = quarantine + QLatin1Char('/')
                                 + QFileInfo(failedSnapshot).completeBaseName()
                                 + QStringLiteral(".json");
    QVERIFY2(QFileInfo::exists(snapshotDest), "the failed snapshot is quarantined");
    QVERIFY2(QFileInfo::exists(manifestDest), "its manifest is quarantined");
    QVERIFY2(!QFileInfo::exists(crashed), "the orphaned session is discarded");
    QVERIFY2(pictura::RecoveryStore::orphans(QString()).isEmpty(),
             "the quarantine directory is not an orphan session");
    QVERIFY2(QFileInfo::exists(snapshotDest), "a later scan does not delete the quarantine");
    QCOMPARE(snapshots(after->recoverySessionDirForTest()).size(), 1);
    after->shutdownRecovery();
}

void RecoveryTest::fileHandlingPreferenceTogglesAutosave()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->enableRecovery();
    const QString dir = window->recoverySessionDirForTest();
    QVERIFY(window->newDocument(QString(), 32, 32, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    drawTriangle(*window->activeView());
    window->autosaveRecovery();
    waitForWrites(*window);
    QCOMPARE(snapshots(dir).size(), 1);

    window->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditPreferencesGeneral));
    QCoreApplication::processEvents();
    pictura::PreferencesDialog* prefs = window->preferencesDialog();
    QVERIFY(prefs != nullptr);
    prefs->openOn(pictura::PreferencesDialog::kFileHandling);
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kFileHandling);
    QVERIFY(prefs->checkboxForTest(QStringLiteral("autoSaveRecovery")));
    auto* minutes = prefs->findChild<QComboBox*>(QStringLiteral("preferencesAutoSaveMinutes"));
    QVERIFY(minutes != nullptr);
    QCOMPARE(minutes->currentData().toInt(), 10);

    minutes->setCurrentIndex(minutes->findData(30));
    QCOMPARE(window->autoSaveMinutes(), 30);
    QCOMPARE(pictura::loadSession().autoSaveMinutes, 30);

    prefs->setCheckboxForTest(QStringLiteral("autoSaveRecovery"), false);
    QVERIFY(!window->autoSaveRecovery());
    QVERIFY(!pictura::loadSession().autoSaveRecovery);
    QVERIFY2(snapshots(dir).isEmpty(), "turning autosave off drops the snapshots");
    window->autosaveRecovery();
    waitForWrites(*window);
    QVERIFY2(snapshots(dir).isEmpty(), "and no tick writes new ones");
    prefs->close();
    window->shutdownRecovery();
}

QTEST_MAIN(RecoveryTest)

#include "tst_recovery.moc"
