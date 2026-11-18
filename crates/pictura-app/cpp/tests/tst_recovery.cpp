// Crash recovery and autosave (#184): the per-instance recovery store, the
// autosave tick, recovering an orphaned session, and the File Handling
// preference.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QEventLoop>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QLockFile>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtCore/QtEndian>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStatusBar>

#include "commands.h"
#include "dialogs.h"
#include "command_ids.h"
#include "vp_dialog.h"
#include "api/filters_bridge.h"
#include "api/document_api.h"
#include "frame.h"
#include "preferences_dialog.h"
#include "recovery_store.h"
#include "session.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/proofing_bridge.h"

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
    return QDir(dir).entryList({QStringLiteral("*.psd"), QStringLiteral("*.psb")}, QDir::Files, QDir::Name);
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
    const QString dir = QFileInfo(from).absolutePath() + QStringLiteral("/999999-1");
    QDir().mkpath(dir);
    for (const QString& name : QDir(from).entryList(QDir::Files)) {
        if (name != QStringLiteral("lock")) {
            QFile::copy(QDir(from).filePath(name), QDir(dir).filePath(name));
        }
    }
    return dir;
}

// The informative text of `window`'s recovery failure dialog, closing it; an
// empty string when none is shown.
QString takeFailureDialog(pictura::PicturaMainWindow& window)
{
    auto* box = window.findChild<QMessageBox*>(QStringLiteral("recoveryFailures"));
    if (!box) {
        return QString();
    }
    const QString text = box->informativeText();
    box->close();
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    return text;
}

} // namespace

class RecoveryTest : public QObject {
    Q_OBJECT

private slots:
    void recoveredNames();
    void largeCheckpointUsesPsb();
    void journalRecoversCommittedDocumentState();
    void journalSkipsUncommittedChanges();
    void disablingRecoveryDrainsPendingWrites();
    void journalFailureReportsDegradedProtection();
    void journalTailFailureRecoversValidPrefix();
    void failedRecoveryTransferKeepsOrphanChain();
    void staleBackgroundSaveLeavesJournalRecoverable();
    void checkpointResourcesMatchTheContainer();
    void generatedResourcesNeverDriveRecoveryOrLeak();
    void recoveredResourcesPreserveImportedBytes();
    void metadataOnlyEditSurvivesRecovery();
    void legacyScannerCannotDeleteVersionedOrphan();
    void firstStoreSyncsAncestorEntries();
    void autosaveSnapshotsOnlyUnsavedChanges();
    void saveAndCloseDropTheSnapshot();
    void cleanShutdownRemovesTheSession();
    void liveSessionsAreNotOrphans();
    void recoverReopensAnOrphanedDocument();
    void partialRecoveryQuarantinesFailedDocument();
    void memoryRefusedRecoveryIsOfferedAgain();
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
    const auto before = QDir(dir).entryList(QDir::Files);

    // Nothing changed since: the next tick does not write it again.
    window->autosaveRecovery();
    waitForWrites(*window);
    QCOMPARE(QDir(dir).entryList(QDir::Files), before);
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
    const auto discard = pictura::UnsavedChoice::Discard;
    pictura::setRequestUnsavedChoice(&discard);
    drawTriangle(*window->viewAt(0));
    const bool closed = window->closeDocument(0, true);
    pictura::setRequestUnsavedChoice(nullptr);
    QVERIFY(closed);
    QVERIFY2(snapshots(dir).isEmpty(), "closing a document deletes its snapshot");
    QVERIFY(QDir(dir).entryList({QStringLiteral("*.journal")}, QDir::Files).isEmpty());
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
    const QString crashed = pictura::RecoveryStore::rootPath() + QStringLiteral("/legacy");
    QVERIFY(QDir().mkpath(crashed));
    {
        auto before = pictura::test::makeMainWindow();
        QVERIFY(before->newDocument(QString(), 64, 48, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        drawTriangle(*before->activeView());
        QVERIFY(before->saveActiveAs(QDir(crashed).filePath(QStringLiteral("1.psd"))));
        QFile legacy(QDir(crashed).filePath(QStringLiteral("1.json")));
        QVERIFY(legacy.open(QIODevice::WriteOnly));
        QVERIFY(legacy.write("{\"name\":\"Untitled-1\"}") > 0);
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
    view->select_all();
    QVERIFY(view->can_undo());
    QVERIFY(view->undo());
    QVERIFY2(after->isDocumentDirty(0),
             "undo back to the recovered start cannot make it clean");
    QVERIFY2(!QFileInfo::exists(crashed), "the orphaned session is discarded");
    QCOMPARE(snapshots(after->recoverySessionDirForTest()).size(), 1);
    QVERIFY2(takeFailureDialog(*after).isEmpty(), "a full recovery shows no failure dialog");
    after->shutdownRecovery();
}

void RecoveryTest::partialRecoveryQuarantinesFailedDocument()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QString crashed;
    QString failedSnapshot;
    QString failedManifest;
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
    failedManifest = orphans.first().documents.at(1).manifest;
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
                                 + QFileInfo(failedManifest).fileName();
    QVERIFY2(QFileInfo::exists(snapshotDest), "the failed snapshot is quarantined");
    QVERIFY2(QFileInfo::exists(manifestDest), "its manifest is quarantined");
    QVERIFY2(!QFileInfo::exists(crashed), "the orphaned session is discarded");
    QVERIFY2(pictura::RecoveryStore::orphans(QString()).isEmpty(),
             "the quarantine directory is not an orphan session");
    QVERIFY2(QFileInfo::exists(snapshotDest), "a later scan does not delete the quarantine");
    QCOMPARE(snapshots(after->recoverySessionDirForTest()).size(), 1);
    const QString failure = takeFailureDialog(*after);
    QVERIFY2(failure.contains(QStringLiteral("Untitled-2")), qPrintable(failure));
    QVERIFY2(failure.contains(QDir::toNativeSeparators(quarantine)), qPrintable(failure));
    after->shutdownRecovery();
}

// [cr_memory_refused] A snapshot this machine cannot open for lack of memory
// stays in its session for a later launch instead of being quarantined.
void RecoveryTest::memoryRefusedRecoveryIsOfferedAgain()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QString crashed;
    {
        auto before = pictura::test::makeMainWindow();
        before->enableRecovery();
        for (const int side : {64, 2048}) {
            QVERIFY(before->newDocument(QString(), side, side * 3 / 4, QStringLiteral("rgb"), 8,
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
    const pictura::RecoverableDocument small = orphans.first().documents.at(0);
    const pictura::RecoverableDocument large = orphans.first().documents.at(1);
    QCOMPARE(large.name, QStringLiteral("Untitled-2"));
    const QString largeManifest = large.manifest;

    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    // About 4 MiB of room and no scratch: the 64×48 snapshot decodes, the
    // 2048×1536 one (12 MiB of tiles) is refused for memory.
    pictura::set_available_memory_for_test(260LL * 1024 * 1024);
    pictura::set_scratch_available_for_test(0);
    const int recovered = after->recoverOrphans();
    pictura::set_available_memory_for_test(-1);
    pictura::set_scratch_available_for_test(-1);
    QCOMPARE(recovered, 1);
    QCOMPARE(after->documentCount(), 1);
    QCOMPARE(after->documentName(0), QStringLiteral("Untitled-1-Recovered"));

    QVERIFY2(!QFileInfo::exists(small.snapshot), "the recovered snapshot leaves the session");
    QVERIFY2(QFileInfo::exists(large.snapshot), "the refused snapshot stays");
    QVERIFY2(QFileInfo::exists(largeManifest), "with its manifest");
    QVERIFY2(QFileInfo::exists(crashed), "the session is kept");
    QVERIFY2(!QFileInfo::exists(pictura::RecoveryStore::quarantineRoot() + QLatin1Char('/')
                                + QFileInfo(crashed).fileName()),
             "nothing is quarantined");
    const QString failure = takeFailureDialog(*after);
    QVERIFY2(failure.contains(QStringLiteral("Untitled-2")), qPrintable(failure));
    QVERIFY2(failure.contains(QStringLiteral("memory")), qPrintable(failure));
    QVERIFY2(failure.contains(QDir::toNativeSeparators(large.snapshot)), qPrintable(failure));
    QVERIFY2(!failure.contains(QStringLiteral("Untitled-1")), qPrintable(failure));

    // With room again, the next offer lists only the refused document, and
    // Recover opens it.
    QString offered;
    bool clicked = false;
    QTimer::singleShot(0, [&offered, &clicked] {
        auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
        if (!box) {
            return;
        }
        offered = box->informativeText();
        for (QAbstractButton* button : box->buttons()) {
            if (button->text() == QStringLiteral("Recover")) {
                button->click();
                clicked = true;
            }
        }
    });
    after->offerRecovery();
    QVERIFY2(clicked, "the next launch offers the refused document");
    QVERIFY2(offered.contains(QStringLiteral("Untitled-2")), qPrintable(offered));
    QVERIFY2(!offered.contains(QStringLiteral("Untitled-1")), qPrintable(offered));
    QCOMPARE(after->documentCount(), 2);
    QCOMPARE(after->documentName(1), QStringLiteral("Untitled-2-Recovered"));
    QCOMPARE(after->activeView()->document_width(), 2048);
    QVERIFY2(!QFileInfo::exists(crashed), "the session goes once it is recovered");
    QVERIFY2(takeFailureDialog(*after).isEmpty(), "and nothing failed");
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

namespace {

using pictura::PicturaMainWindow;

bool newRecoveryDoc(PicturaMainWindow& window, int width = 64, int height = 48)
{
    window.enableRecovery();
    return window.newDocument(QStringLiteral("Recoverable"), width, height,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
}

void recoveryInvert(PicturaMainWindow& window)
{
    auto* view = window.activeView();
    QVERIFY(view);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
}

QByteArray readBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool writeBytes(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly | QIODevice::Truncate)
        && file.write(bytes) == bytes.size();
}

QString artifact(const QString& dir, const QString& pattern)
{
    const auto entries = QDir(dir).entryList({pattern}, QDir::Files);
    return entries.isEmpty() ? QString() : QDir(dir).filePath(entries.first());
}

QByteArray resources(const QByteArray& psd)
{
    const auto number = [&psd](int offset) {
        return qFromBigEndian<quint32>(psd.constData() + offset);
    };
    if (psd.size() < 34) {
        return {};
    }
    const qsizetype offset = 30 + static_cast<qsizetype>(number(26));
    if (offset > psd.size() - 4) {
        return {};
    }
    return psd.mid(offset + 4, number(offset));
}

QByteArray block(quint16 id, const QByteArray& payload)
{
    QByteArray result("8BIM");
    char prefix[2];
    qToBigEndian(id, prefix);
    result.append(prefix, 2);
    result.append(2, '\0');
    char size[4];
    qToBigEndian(static_cast<quint32>(payload.size()), size);
    result.append(size, 4);
    result.append(payload);
    if (payload.size() % 2 != 0) {
        result.append('\0');
    }
    return result;
}

QByteArray withResources(QByteArray file, const QByteArray& original)
{
    const auto offset = 30 + qFromBigEndian<quint32>(file.constData() + 26);
    const auto size = qFromBigEndian<quint32>(file.constData() + offset);
    char length[4];
    qToBigEndian(static_cast<quint32>(original.size()), length);
    file.replace(offset, size + 4, QByteArray(length, 4) + original);
    return file;
}

void createVanishingPlane(pictura::PictureView& view)
{
    pictura::VanishingPointDialog dialog(&view, QStringLiteral("Recovery"));
    QVERIFY(dialog.ready());
    const double corners[] = {8, 8, 24, 8, 24, 24, 8, 24};
    QCOMPARE(pictura::vp_create(rust::Slice<const double>(corners, 8)), 0);
    dialog.accept();
}

} // namespace

void RecoveryTest::largeCheckpointUsesPsb()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window, 30001, 2));
    recoveryInvert(*window);
    waitForWrites(*window);
    const QString dir = window->recoverySessionDirForTest();
    const QByteArray file = readBytes(artifact(dir, QStringLiteral("*.psb")));
    QCOMPARE(file.left(6), QByteArray("8BPS\0\2", 6));
    fakeCrashedSession(dir);
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(after->activeView()->document_width(), 30001);
    QCOMPARE(QColor(after->activeView()->composite_argb(1, 1)), QColor(Qt::black));
}

void RecoveryTest::journalRecoversCommittedDocumentState()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    auto& view = *window->activeView();
    drawTriangle(view);
    QVERIFY(!view.add_solid_fill(0xff2244aau).isEmpty());
    QVERIFY(view.undo());
    QVERIFY(view.redo());
    QVERIFY(view.undo());
    view.set_active_layer(QStringLiteral("0"));
    QVERIFY(view.begin_paint(0xFF000000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false));
    QVERIFY(view.paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view.end_paint());
    const auto pixel = view.composite_argb(16, 16);
    waitForWrites(*window);
    fakeCrashedSession(window->recoverySessionDirForTest());
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QVERIFY(pictura::paths_has_work_path(*after->activeView()));
    QCOMPARE(after->activeView()->composite_argb(16, 16), pixel);
    QVERIFY(!after->activeView()->can_undo());
}

void RecoveryTest::journalSkipsUncommittedChanges()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    auto& view = *window->activeView();
    recoveryInvert(*window);
    waitForWrites(*window);
    const QString dir = window->recoverySessionDirForTest();
    const QString manifest = artifact(dir, QStringLiteral("*.json"));
    const QByteArray before = readBytes(manifest);
    QVERIFY(view.begin_paint(0xFFFFFFFFu, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false));
    QVERIFY(view.paint_dab(16.0, 16.0, 1.0));
    QVERIFY(!pictura::image_adjust_direct(view, QStringLiteral("unknown")));
    waitForWrites(*window);
    fakeCrashedSession(dir);
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(QColor(after->activeView()->composite_argb(16, 16)), QColor(Qt::black));
    QVERIFY(!before.isEmpty());
    QCOMPARE(readBytes(manifest), before);
}

void RecoveryTest::disablingRecoveryDrainsPendingWrites()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    recoveryInvert(*window);
    window->setAutoSaveRecovery(false, 10);
    waitForWrites(*window);
    recoveryInvert(*window);
    waitForWrites(*window);
    QCOMPARE(QDir(window->recoverySessionDirForTest()).entryList(QDir::Files),
             QStringList{QStringLiteral("lock")});
}

void RecoveryTest::journalFailureReportsDegradedProtection()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    recoveryInvert(*window);
    waitForWrites(*window);
    const QString dir = window->recoverySessionDirForTest();
    pictura::recovery_fail_next_for_test(*window->activeView(), 1);
    recoveryInvert(*window);
    QVERIFY(!pictura::recovery_wait(*window->activeView()));
    QTRY_VERIFY(window->statusBar()->currentMessage().contains(QStringLiteral("recovery"),
                                                              Qt::CaseInsensitive));
    fakeCrashedSession(dir);
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(QColor(after->activeView()->composite_argb(16, 16)), QColor(Qt::black));
}

void RecoveryTest::journalTailFailureRecoversValidPrefix()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    recoveryInvert(*window);
    waitForWrites(*window);
    recoveryInvert(*window);
    waitForWrites(*window);
    const QString crashed = fakeCrashedSession(window->recoverySessionDirForTest());
    QFile journal(artifact(crashed, QStringLiteral("*.journal")));
    QVERIFY(journal.size() > 0);
    QVERIFY(journal.open(QIODevice::Append));
    QCOMPARE(journal.write("torn"), 4);
    journal.close();
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(QColor(after->activeView()->composite_argb(16, 16)), QColor(Qt::white));
    QVERIFY(!takeFailureDialog(*after).isEmpty());
}

void RecoveryTest::failedRecoveryTransferKeepsOrphanChain()
{
    pictura::test::ScopedStateHome state;
    auto first = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*first));
    recoveryInvert(*first);
    waitForWrites(*first);
    const QString crashed = fakeCrashedSession(first->recoverySessionDirForTest());
    const auto files = QDir(crashed).entryList(QDir::Files);
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    const QString transferDir = after->recoverySessionDirForTest();
    const QString heldDir = transferDir + QStringLiteral(".held");
    QVERIFY(QDir().rename(transferDir, heldDir));
    QVERIFY(writeBytes(transferDir, "block recovery writes"));
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(QDir(crashed).entryList(QDir::Files), files);
    QVERIFY(QFile::remove(transferDir));
    QVERIFY(QDir().rename(heldDir, transferDir));
    QVERIFY(after->recoverOrphans() >= 1);
    QVERIFY(!QFileInfo::exists(crashed));
}

void RecoveryTest::staleBackgroundSaveLeavesJournalRecoverable()
{
    const QString childPath = qEnvironmentVariable("PICTURA_TEST_RECOVERY_CHILD");
    if (!childPath.isEmpty()) {
        auto window = pictura::test::makeMainWindow();
        QVERIFY(newRecoveryDoc(*window));
        QVERIFY(window->saveActiveAs(childPath + QStringLiteral(".psd")));
        auto& view = *window->activeView();
        pictura::save_set_delay(view, 500);
        window->registry()->refresh();
        window->registry()->action(QString::fromLatin1(pictura::command_ids::FileSave))->trigger();
        QVERIFY(pictura::save_running(view));
        recoveryInvert(*window);
        waitForWrites(*window);
        QTRY_COMPARE(pictura::save_progress(view), -1);
        QVERIFY(writeBytes(childPath + QStringLiteral(".ready"), "ready"));
        QEventLoop loop;
        loop.exec();
        return;
    }

    pictura::test::ScopedStateHome state;
    QTemporaryDir marker;
    const QString path = marker.filePath(QStringLiteral("user"));
    QProcess child;
    auto env = QProcessEnvironment::systemEnvironment();
    env.insert(QStringLiteral("PICTURA_TEST_RECOVERY_CHILD"), path);
    child.setProcessEnvironment(env);
    child.start(QCoreApplication::applicationFilePath(), {QStringLiteral("staleBackgroundSaveLeavesJournalRecoverable")});
    QVERIFY(child.waitForStarted());
    QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(path + QStringLiteral(".ready")), 10000);
    child.kill();
    QVERIFY(child.waitForFinished());
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QCOMPARE(QColor(after->activeView()->composite_argb(16, 16)), QColor(Qt::black));
    pictura::PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, path + QStringLiteral(".psd")));
    QCOMPARE(QColor(reopened.composite_argb(16, 16)), QColor(Qt::white));
}

void RecoveryTest::checkpointResourcesMatchTheContainer()
{
    for (const auto width : {64, 30001}) {
        pictura::test::ScopedStateHome state;
        auto window = pictura::test::makeMainWindow();
        QVERIFY(newRecoveryDoc(*window, width, 2));
        recoveryInvert(*window);
        waitForWrites(*window);
        const bool psb = width > 30000;
        const QString file = artifact(window->recoverySessionDirForTest(),
                                      psb ? QStringLiteral("*.psb") : QStringLiteral("*.psd"));
        const QByteArray section = resources(readBytes(file));
        QCOMPARE(section.count(QByteArray("8BIM\x04\x3e", 6)), 1);
        QCOMPARE(section.count(QByteArray("8BIM\x04\x3f", 6)), 1);
        QVERIFY(section.contains(psb ? QByteArray("\0P\0S\0B", 6) : QByteArray("\0P\0S\0D", 6)));
    }
}

void RecoveryTest::generatedResourcesNeverDriveRecoveryOrLeak()
{
    pictura::test::ScopedStateHome state;
    QTemporaryDir output;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    drawTriangle(*window->activeView());
    waitForWrites(*window);
    const QString crashed = fakeCrashedSession(window->recoverySessionDirForTest());
    const QString file = artifact(crashed, QStringLiteral("*.psd"));
    const QByteArray checkpoint = readBytes(file);
    QVERIFY(!checkpoint.isEmpty());
    QByteArray resourceSection = resources(checkpoint);
    const int pathId = resourceSection.indexOf(QByteArray("8BIM\x04\x3e", 6));
    const int formatId = resourceSection.indexOf(QByteArray("8BIM\x04\x3f", 6));
    QVERIFY(pathId >= 0 && formatId >= 0);
    const auto replaceBlock = [&](int offset, const QByteArray& replacement) {
        const auto length = qFromBigEndian<quint32>(resourceSection.constData() + offset + 8);
        resourceSection.replace(offset, 12 + length + length % 2, replacement);
    };
    QVERIFY(formatId > pathId);
    replaceBlock(formatId, block(1087, "wrong format"));
    replaceBlock(pathId, block(1086, "/missing"));
    QVERIFY(writeBytes(file, withResources(checkpoint, resourceSection)));
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QVERIFY(pictura::paths_has_work_path(*after->activeView()));
    const QString user = output.filePath(QStringLiteral("user.psd"));
    QVERIFY(after->saveActiveAs(user));
    const QByteArray section = resources(readBytes(user));
    QVERIFY(!section.contains(QByteArray("8BIM\x04\x3e", 6)));
    QVERIFY(!section.contains(QByteArray("8BIM\x04\x3f", 6)));
}

void RecoveryTest::recoveredResourcesPreserveImportedBytes()
{
    pictura::test::ScopedStateHome state;
    QTemporaryDir output;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    const QString input = output.filePath(QStringLiteral("foreign.psd"));
    QVERIFY(window->saveActiveAs(input));
    const QByteArray original = block(1086, "foreign") + block(1087, "arbitrary")
        + block(1086, "duplicate");
    QVERIFY(writeBytes(input, withResources(readBytes(input), original)));
    QVERIFY(window->openDocumentAtPath(input));
    recoveryInvert(*window);
    waitForWrites(*window);
    fakeCrashedSession(window->recoverySessionDirForTest());
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    const QString result = output.filePath(QStringLiteral("resaved.psd"));
    QVERIFY(after->saveActiveAs(result));
    QVERIFY(resources(readBytes(result)).startsWith(original));
}

void RecoveryTest::metadataOnlyEditSurvivesRecovery()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    const auto history = window->activeView()->history_index();
    createVanishingPlane(*window->activeView());
    QVERIFY(pictura::recovery_wait(*window->activeView()));
    QVERIFY(pictura::print_document_settings_update(*window->activeView(), QStringLiteral("{}"),
              QStringLiteral("{\"border\":4}"), true).isEmpty());
    QCOMPARE(window->activeView()->history_index(), history);
    QVERIFY(pictura::recovery_wait(*window->activeView()));
    fakeCrashedSession(window->recoverySessionDirForTest());
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
    QVERIFY(after->isDocumentDirty(0));
    const auto settings = QJsonDocument::fromJson(
        pictura::print_document_settings(*after->activeView()).toUtf8()).object();
    QCOMPARE(settings.value(QStringLiteral("border")).toDouble(), 4.0);
    pictura::VanishingPointDialog dialog(after->activeView(), QStringLiteral("Recovered"));
    QVERIFY(dialog.ready());
    QCOMPARE(pictura::vp_plane_count(), 1);
    dialog.reject();
}

void RecoveryTest::legacyScannerCannotDeleteVersionedOrphan()
{
    pictura::test::ScopedStateHome state;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(newRecoveryDoc(*window));
    recoveryInvert(*window);
    waitForWrites(*window);
    const QString crash = fakeCrashedSession(window->recoverySessionDirForTest());
    const QStringList files = QDir(crash).entryList(QDir::Files);
    const QDir legacy(pictura::RecoveryStore::rootPath());
    for (const auto& session : legacy.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        if (session == QStringLiteral("quarantine")) {
            continue;
        }
        QDir candidate(legacy.filePath(session));
        bool recognized = false;
        for (const auto& file : candidate.entryList({QStringLiteral("*.json")}, QDir::Files)) {
            recognized |= candidate.exists(QFileInfo(file).completeBaseName() + QStringLiteral(".psd"));
        }
        if (!recognized) {
            QVERIFY(candidate.removeRecursively());
        }
    }
    QCOMPARE(QDir(crash).entryList(QDir::Files), files);
    auto after = pictura::test::makeMainWindow();
    after->enableRecovery();
    QCOMPARE(after->recoverOrphans(), 1);
}

void RecoveryTest::firstStoreSyncsAncestorEntries()
{
    for (int fault = 2; fault <= 4; ++fault) {
        pictura::test::ScopedStateHome state;
        auto window = pictura::test::makeMainWindow();
        QVERIFY(newRecoveryDoc(*window));
        pictura::recovery_fail_next_for_test(*window->activeView(), fault);
        recoveryInvert(*window);
        QVERIFY(!pictura::recovery_wait(*window->activeView()));
        const QString dir = window->recoverySessionDirForTest();
        QVERIFY(QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files).isEmpty());
        recoveryInvert(*window);
        QVERIFY(pictura::recovery_wait(*window->activeView()));
        QVERIFY(!QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files).isEmpty());
    }
}

QTEST_MAIN(RecoveryTest)

#include "tst_recovery.moc"
