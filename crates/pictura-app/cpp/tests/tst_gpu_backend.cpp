// The Windows GPU backend chain's cache, marker, candidate selection, and the
// five-state diagnostics label. Pure cache/marker tests run everywhere; the
// probe itself is manual on Windows hardware.

#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QProcess>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "frame.h"
#include "gpu_probe.h"
#include "preferences_dialog.h"
#include "session.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

QStringList candidates()
{
    return pictura::gpuCandidateNames();
}

void writeMarker(const QString& backend, const QString& version, qint64 pid)
{
    QDir().mkpath(QFileInfo(gpuMarkerPath()).absolutePath());
    QJsonObject marker;
    marker.insert(QStringLiteral("schemaVersion"), 1);
    marker.insert(QStringLiteral("appVersion"), version);
    marker.insert(QStringLiteral("backend"), backend);
    marker.insert(QStringLiteral("pid"), double(pid));
    QFile file(gpuMarkerPath());
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(QJsonDocument(marker).toJson(QJsonDocument::Compact));
}

} // namespace

class GpuBackendTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void gpuProbeOutcomeMapsToCacheState();
    void missingFileLoadsAllUnknown();
    void corruptCacheLoadsUnknown();
    void okRoundTrips();
    void failedRoundTrips();
    void crashedRoundTrips();
    void versionBumpInvalidatesEverything();
    void okBackendSkipsTheProbe();
    void failedIsRetriedNextLaunch();
    void crashedIsSkippedUntilReset();
    void corruptCacheDoesNotFailTheLaunch();
    void preferenceReenableClearsCache();
    void selectGpuBackendRecordsOkAndSelectsIt();
    void selectGpuBackendRecordsFailedAndTriesNext();
    void selectGpuBackendRecordsCrashAndTriesNext();
    void selectGpuBackendTreatsTimeoutAsCrashed();

    void markerAttemptingWrittenBeforeProbe();
    void successClearsTheMarker();
    void cleanFailureWritesFailedNotCrashed();
    void cleanParentFailureRecordsFailed();
    void liveInstanceMarkerIsIgnored();
    void staleMarkerMarksTheBackendCrashed();
    void staleMarkerFallsToTheNextCandidate();
    void unknownMarkerIsRemovedWithoutACacheChange();
    void oldVersionMarkerIsIgnored();
    void liveMarkerIsNotOverwritten();
    void malformedMarkerIsRemoved();
    void futureSchemaVersionLoadsUnknown();

    void linuxSkipsTheProbeAndKeepsVulkan();
    void interopProbeStaysVulkanOnly();
    void childProbeLeavesStateUntouched();

    void statusBarShowsTheBackendLabel();
    void aboutTextNamesBackend();
    void toggleIsDisabledOnlyWhenNoBackendExists();
    void statusBackendTokenIsLegacy();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void GpuBackendTest::initTestCase()
{
    QVERIFY2(stateHome_.isValid(), "a throwaway XDG_STATE_HOME is required");
}

void GpuBackendTest::init()
{
    QFile::remove(gpuBackendFilePath());
    QFile::remove(gpuMarkerPath());
}

void GpuBackendTest::cleanup()
{
    QFile::remove(gpuBackendFilePath());
    QFile::remove(gpuMarkerPath());
}

void GpuBackendTest::gpuProbeOutcomeMapsToCacheState()
{
    QCOMPARE(gpuProbeOutcomeToState(0), GpuBackendState::Ok);
    QCOMPARE(gpuProbeOutcomeToState(1), GpuBackendState::Failed);
    QCOMPARE(gpuProbeOutcomeToState(2), GpuBackendState::Crashed);
    QCOMPARE(gpuProbeOutcomeToState(9), GpuBackendState::Failed);
}

void GpuBackendTest::missingFileLoadsAllUnknown()
{
    const GpuBackendCache cache = loadGpuBackendCache();
    QVERIFY(cache.backends.isEmpty());
    QCOMPARE(cache.appVersion, QString::fromLatin1(kGpuCacheAppVersion));
}

void GpuBackendTest::corruptCacheLoadsUnknown()
{
    QDir().mkpath(QFileInfo(gpuBackendFilePath()).absolutePath());
    QFile file(gpuBackendFilePath());
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("{not json");
    file.close();
    QVERIFY(loadGpuBackendCache().backends.isEmpty());
}

void GpuBackendTest::okRoundTrips()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Ok, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Ok);
}

void GpuBackendTest::failedRoundTrips()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Failed, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Failed);
}

void GpuBackendTest::crashedRoundTrips()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Crashed, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Crashed);
}

void GpuBackendTest::versionBumpInvalidatesEverything()
{
    GpuBackendCache cache;
    cache.appVersion = QStringLiteral("0.1.0");
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Ok, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    QVERIFY(loadGpuBackendCache(QStringLiteral("0.2.0")).backends.isEmpty());
}

void GpuBackendTest::okBackendSkipsTheProbe()
{
    const QStringList names = candidates();
    if (names.isEmpty()) {
        QSKIP("no compiled candidates");
    }
    GpuBackendCache cache;
    for (const QString& name : names) {
        cache.backends.insert(name, {GpuBackendState::Ok, QStringLiteral("now"), {}});
    }
    bool probed = false;
    const QString chosen = selectGpuBackend(cache, [&](const QString&) {
        probed = true;
        return 1;
    });
    QVERIFY(!probed);
    QCOMPARE(chosen, names.first());
}

void GpuBackendTest::failedIsRetriedNextLaunch()
{
    const QStringList names = candidates();
    if (names.isEmpty()) {
        QSKIP("no compiled candidates");
    }
    GpuBackendCache cache;
    for (const QString& name : names) {
        cache.backends.insert(name, {GpuBackendState::Failed, QStringLiteral("now"), {}});
    }
    const QString chosen = selectGpuBackend(cache, [&](const QString& backend) {
        return backend == names.last() ? 0 : 1;
    });
    QCOMPARE(chosen, names.last());
    QCOMPARE(gpuBackendState(cache, names.last()), GpuBackendState::Ok);
}

void GpuBackendTest::crashedIsSkippedUntilReset()
{
    GpuBackendCache cache;
    for (const QString& name : candidates()) {
        cache.backends.insert(name, {GpuBackendState::Crashed, QStringLiteral("now"), {}});
    }
    bool probed = false;
    const QString chosen = selectGpuBackend(cache, [&](const QString&) {
        probed = true;
        return 0;
    });
    QVERIFY(chosen.isEmpty());
    QVERIFY(!probed);
}

void GpuBackendTest::corruptCacheDoesNotFailTheLaunch()
{
    QDir().mkpath(QFileInfo(gpuBackendFilePath()).absolutePath());
    QFile file(gpuBackendFilePath());
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("garbage");
    file.close();
    GpuBackendCache cache = loadGpuBackendCache();
    QVERIFY(cache.backends.isEmpty());
    QVERIFY(selectGpuBackend(cache, [](const QString&) { return 1; }).isEmpty());
}

void GpuBackendTest::preferenceReenableClearsCache()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Crashed, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));

    window_ = pictura::test::makeMainWindow();
    window_->showPreferences(PreferencesDialog::kPerformance);
    auto* box = window_->findChild<QCheckBox*>(QStringLiteral("preferences_useGpuCompute"));
    QVERIFY(box != nullptr);
    box->setChecked(false);
    box->setChecked(true);
    QVERIFY(!QFile::exists(gpuBackendFilePath()));
}

void GpuBackendTest::selectGpuBackendRecordsOkAndSelectsIt()
{
    GpuBackendCache cache;
    const QString chosen = selectGpuBackend(cache, [](const QString&) { return 0; });
    QVERIFY(!chosen.isEmpty());
    QCOMPARE(gpuBackendState(cache, chosen), GpuBackendState::Ok);
}

void GpuBackendTest::selectGpuBackendRecordsFailedAndTriesNext()
{
    const QStringList names = candidates();
    if (names.isEmpty()) {
        QSKIP("no compiled candidates");
    }
    GpuBackendCache cache;
    const QString chosen = selectGpuBackend(cache, [&](const QString& backend) {
        return backend == names.first() ? 1 : 0;
    });
    QCOMPARE(gpuBackendState(cache, names.first()), GpuBackendState::Failed);
    if (names.size() > 1) {
        QCOMPARE(chosen, names.at(1));
    } else {
        QVERIFY(chosen.isEmpty());
    }
}

void GpuBackendTest::selectGpuBackendRecordsCrashAndTriesNext()
{
    const QStringList names = candidates();
    if (names.isEmpty()) {
        QSKIP("no compiled candidates");
    }
    GpuBackendCache cache;
    const QString chosen = selectGpuBackend(cache, [&](const QString& backend) {
        return backend == names.first() ? 2 : 0;
    });
    QCOMPARE(gpuBackendState(cache, names.first()), GpuBackendState::Crashed);
    if (names.size() > 1) {
        QCOMPARE(chosen, names.at(1));
    } else {
        QVERIFY(chosen.isEmpty());
    }
}

void GpuBackendTest::selectGpuBackendTreatsTimeoutAsCrashed()
{
    const QStringList names = candidates();
    if (names.isEmpty()) {
        QSKIP("no compiled candidates");
    }
    GpuBackendCache cache;
    selectGpuBackend(cache, [](const QString&) { return 2; });
    QCOMPARE(gpuBackendState(cache, names.first()), GpuBackendState::Crashed);
}

void GpuBackendTest::markerAttemptingWrittenBeforeProbe()
{
    QVERIFY(beginDeviceInit(QStringLiteral("vulkan")));
    const QJsonObject marker = QJsonDocument::fromJson(
                                   [&] {
                                       QFile file(gpuMarkerPath());
                                       return file.open(QIODevice::ReadOnly) ? file.readAll()
                                                                             : QByteArray();
                                   }())
                                   .object();
    QCOMPARE(marker.value(QStringLiteral("backend")).toString(), QStringLiteral("vulkan"));
    endDeviceInit(true, QStringLiteral("vulkan"));
    QVERIFY(!QFile::exists(gpuMarkerPath()));
}

void GpuBackendTest::successClearsTheMarker()
{
    QVERIFY(beginDeviceInit(QStringLiteral("vulkan")));
    endDeviceInit(true, QStringLiteral("vulkan"));
    QVERIFY(!QFile::exists(gpuMarkerPath()));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Ok);
}

void GpuBackendTest::cleanFailureWritesFailedNotCrashed()
{
    QVERIFY(beginDeviceInit(QStringLiteral("vulkan")));
    endDeviceInit(false, QStringLiteral("vulkan"));
    QVERIFY(!QFile::exists(gpuMarkerPath()));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Failed);
}

void GpuBackendTest::cleanParentFailureRecordsFailed()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Ok, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    endDeviceInit(false, QStringLiteral("vulkan"));
    QCOMPARE(gpuBackendState(loadGpuBackendCache(), QStringLiteral("vulkan")),
             GpuBackendState::Failed);
}

void GpuBackendTest::liveInstanceMarkerIsIgnored()
{
    QVERIFY(beginDeviceInit(QStringLiteral("vulkan")));
    GpuBackendCache cache = loadGpuBackendCache();
    recoverGpuInitMarker(cache);
    QCOMPARE(gpuBackendState(cache, QStringLiteral("vulkan")), GpuBackendState::Unknown);
    QVERIFY(QFile::exists(gpuMarkerPath()));
}

void GpuBackendTest::staleMarkerMarksTheBackendCrashed()
{
    writeMarker(QStringLiteral("vulkan"), QString::fromLatin1(kGpuCacheAppVersion), 0);
    GpuBackendCache cache = loadGpuBackendCache();
    recoverGpuInitMarker(cache);
    QCOMPARE(gpuBackendState(cache, QStringLiteral("vulkan")), GpuBackendState::Crashed);
    QVERIFY(!QFile::exists(gpuMarkerPath()));
}

void GpuBackendTest::staleMarkerFallsToTheNextCandidate()
{
    const QStringList names = candidates();
    if (names.size() < 2) {
        QSKIP("needs two compiled candidates");
    }
    GpuBackendCache cache;
    cache.backends.insert(names.first(),
                          {GpuBackendState::Crashed, QStringLiteral("now"), {}});
    QCOMPARE(selectGpuBackend(cache, [](const QString&) { return 0; }), names.at(1));
}

void GpuBackendTest::unknownMarkerIsRemovedWithoutACacheChange()
{
    writeMarker(QStringLiteral("metal"), QString::fromLatin1(kGpuCacheAppVersion), 0);
    GpuBackendCache cache = loadGpuBackendCache();
    recoverGpuInitMarker(cache);
    QVERIFY(cache.backends.isEmpty());
    QVERIFY(!QFile::exists(gpuMarkerPath()));
}

void GpuBackendTest::oldVersionMarkerIsIgnored()
{
    writeMarker(QStringLiteral("vulkan"), QStringLiteral("0.0.1"), 0);
    GpuBackendCache cache = loadGpuBackendCache();
    recoverGpuInitMarker(cache);
    QVERIFY(cache.backends.isEmpty());
    QVERIFY(!QFile::exists(gpuMarkerPath()));
}

void GpuBackendTest::liveMarkerIsNotOverwritten()
{
#ifdef Q_OS_WIN
    QSKIP("the sleeping child helper is Unix-only");
#else
    QProcess sleeper;
    sleeper.start(QStringLiteral("sleep"), {QStringLiteral("60")});
    QVERIFY(sleeper.waitForStarted());
    const qint64 pid = sleeper.processId();
    QVERIFY(pid > 0);
    writeMarker(QStringLiteral("vulkan"), QString::fromLatin1(kGpuCacheAppVersion), pid);
    QVERIFY(!beginDeviceInit(QStringLiteral("vulkan")));
    sleeper.kill();
    sleeper.waitForFinished();
#endif
}

void GpuBackendTest::malformedMarkerIsRemoved()
{
    QDir().mkpath(QFileInfo(gpuMarkerPath()).absolutePath());
    {
        QFile file(gpuMarkerPath());
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("{not json");
    }
    GpuBackendCache cache = loadGpuBackendCache();
    recoverGpuInitMarker(cache);
    QVERIFY(!QFile::exists(gpuMarkerPath()));
    QVERIFY(cache.backends.isEmpty());

    QJsonObject future;
    future.insert(QStringLiteral("schemaVersion"), 2);
    future.insert(QStringLiteral("appVersion"), QString::fromLatin1(kGpuCacheAppVersion));
    future.insert(QStringLiteral("backend"), QStringLiteral("vulkan"));
    future.insert(QStringLiteral("pid"), double(0));
    {
        QFile file(gpuMarkerPath());
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(QJsonDocument(future).toJson(QJsonDocument::Compact));
    }
    GpuBackendCache futureCache = loadGpuBackendCache();
    recoverGpuInitMarker(futureCache);
    QVERIFY(!QFile::exists(gpuMarkerPath()));
    QVERIFY(futureCache.backends.isEmpty());
}

void GpuBackendTest::futureSchemaVersionLoadsUnknown()
{
    QDir().mkpath(QFileInfo(gpuBackendFilePath()).absolutePath());
    QJsonObject entry;
    entry.insert(QStringLiteral("state"), QStringLiteral("ok"));
    QJsonObject backends;
    backends.insert(QStringLiteral("vulkan"), entry);
    QJsonObject root;
    root.insert(QStringLiteral("schemaVersion"), 2);
    root.insert(QStringLiteral("appVersion"), QString::fromLatin1(kGpuCacheAppVersion));
    root.insert(QStringLiteral("backends"), backends);
    QFile file(gpuBackendFilePath());
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(QJsonDocument(root).toJson(QJsonDocument::Compact));
    file.close();
    QVERIFY(loadGpuBackendCache().backends.isEmpty());
}

void GpuBackendTest::linuxSkipsTheProbeAndKeepsVulkan()
{
#ifdef Q_OS_WIN
    QSKIP("Windows runs the chain");
#else
    prepareGpuBackend();
    QVERIFY(selectedGpuBackend().isEmpty());
    QVERIFY(!QFile::exists(gpuBackendFilePath()));
    QVERIFY(!QFile::exists(gpuMarkerPath()));
#endif
}

void GpuBackendTest::interopProbeStaysVulkanOnly()
{
    PictureView view;
    if (!view.gpu_interop_prepare()) {
        QSKIP("no Vulkan device");
    }
    QVERIFY(view.gpu_vk_device() != 0);
}

void GpuBackendTest::childProbeLeavesStateUntouched()
{
    GpuBackendCache cache;
    cache.backends.insert(QStringLiteral("vulkan"),
                          {GpuBackendState::Crashed, QStringLiteral("now"), {}});
    QVERIFY(saveGpuBackendCache(cache));
    const QByteArray before = [&] {
        QFile file(gpuBackendFilePath());
        return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
    }();
    QCOMPARE(runGpuProbeChild("bogus"), 1);
    const QByteArray after = [&] {
        QFile file(gpuBackendFilePath());
        return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
    }();
    QCOMPARE(after, before);
}

void GpuBackendTest::statusBarShowsTheBackendLabel()
{
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_->newDocument(QStringLiteral("Gpu"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCoreApplication::processEvents();
    auto* label = window_->findChild<QLabel*>(QStringLiteral("statusBackendLabel"));
    QVERIFY(label != nullptr);
    const QString text = label->text();
    QVERIFY2(text == QStringLiteral("GPU (DX12)") || text == QStringLiteral("GPU (Vulkan)")
                 || text == QStringLiteral("CPU") || text == QStringLiteral("CPU (no GPU)")
                 || text == QStringLiteral("CPU (safe mode)"),
             qPrintable(text));
}

void GpuBackendTest::aboutTextNamesBackend()
{
    QVERIFY(aboutText(QStringLiteral("GPU (Vulkan)")).contains(QStringLiteral("GPU (Vulkan)")));
}

void GpuBackendTest::toggleIsDisabledOnlyWhenNoBackendExists()
{
    PictureView probe;
    const bool available = probe.gpu_available();
    window_ = pictura::test::makeMainWindow();
    window_->showPreferences(PreferencesDialog::kPerformance);
    auto* box = window_->findChild<QCheckBox*>(QStringLiteral("preferences_useGpuCompute"));
    QVERIFY(box != nullptr);
    QCOMPARE(box->isEnabled(), available || gpuChainHasResetPath());
}

void GpuBackendTest::statusBackendTokenIsLegacy()
{
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_->newDocument(QStringLiteral("Gpu"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCoreApplication::processEvents();
    PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QString token = view->active_backend();
    QVERIFY2(token == QStringLiteral("GPU") || token == QStringLiteral("CPU")
                 || token == QStringLiteral("CPU (no GPU)"),
             qPrintable(token));
}

QTEST_MAIN(GpuBackendTest)
#include "tst_gpu_backend.moc"
