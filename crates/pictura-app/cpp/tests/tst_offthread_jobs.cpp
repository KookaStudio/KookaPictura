// Off-thread previews, applies, and native saves (#972): the GUI keeps
// processing events while the worker runs, and the landed result is the one the
// synchronous path would produce.

#include <QtTest/QtTest>
#include <QtCore/QDir>

#include <QtCore/QElapsedTimer>
#include <QtCore/QEventLoop>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QRegularExpression>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QTabBar>

#include <functional>
#include <memory>
#include <vector>

#include "api/document_api.h"
#include "adjustment_dialog.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "preferences_dialog.h"
#include "vp_dialog.h"
#include "panels/actions_panel.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "panels/history_panel.h"
#include "session.h"
#include "image_view.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/filters_bridge.h"

#include "qt_test_support.h"

namespace {

::rust::Slice<const std::uint8_t> slice(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), std::size_t(bytes.size())};
}

QByteArray blockOf(const char* kind)
{
    const ::rust::Vec<std::uint8_t> v = pictura::image_adjustment_default(
        QString::fromLatin1(kind), 0x000000u, 0xffffffu, false);
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

QByteArray lightnessBlock(double value)
{
    const ::rust::Vec<std::uint8_t> v = pictura::image_adjustment_set(
        slice(blockOf("hue-saturation")), QStringLiteral("lightness"), value, false);
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

class PaintCounter : public QObject {
public:
    int paints = 0;

protected:
    bool eventFilter(QObject*, QEvent* event) override
    {
        if (event->type() == QEvent::Paint) {
            ++paints;
        }
        return false;
    }
};

struct TimerProbe {
    QElapsedTimer elapsed;
    QTimer timer;
    int ticks = 0;
    qint64 maxGap = 0;
    qint64 last = 0;

    TimerProbe()
    {
        timer.setInterval(10);
        QObject::connect(&timer, &QTimer::timeout, &timer, [this] {
            const qint64 now = elapsed.elapsed();
            maxGap = qMax(maxGap, now - last);
            last = now;
            ++ticks;
        });
    }

    void start()
    {
        elapsed.start();
        timer.start();
    }
    void stop() { timer.stop(); }
};

void pumpUntil(const std::function<bool()>& done, int timeoutMs = 8000)
{
    QElapsedTimer elapsed;
    elapsed.start();
    while (!done() && elapsed.elapsed() < timeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
    }
}

const pictura::FilterCommandSpec* wholeLayerSpec()
{
    return pictura::filterCommandForPath(
        {QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Radial Blur")});
}

std::vector<std::uint8_t> maskThumb(pictura::PictureView& view, int maxSide)
{
    int width = 0;
    int height = 0;
    const ::rust::Vec<std::uint8_t> bytes =
        pictura::channels_active_mask_thumbnail(view, maxSide, width, height);
    return std::vector<std::uint8_t>(bytes.begin(), bytes.end());
}

bool makeMaskTarget(pictura::PictureView& view)
{
    const QString layer = view.add_solid_fill(0xff2244aau);
    if (layer.isEmpty()) {
        return false;
    }
    // The fill's automatic default mask is not the target this test wants.
    pictura::layer_mask_delete_path(view, layer);
    if (!view.rasterize_fill_content(layer)) {
        return false;
    }
    if (!pictura::layer_mask_add(view, QStringLiteral("reveal-all"))) {
        return false;
    }
    return pictura::mask_edit_target_set(view, layer);
}

} // namespace

class OffthreadJobsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void saveKeepsTheEventLoopLive();
    void nativeSaveCallersWriteSameBytes();
    void backgroundSavePreferencePersists();
    void backgroundSaveAcceptsInputAndShowsProgress();
    void backgroundSaveAsCompletesOnOrigin();
    void backgroundSaveUpdatesTheSavedHistoryState();
    void backgroundSaveRejectsConflictingWrites();
    void backgroundSaveReportsUnconfirmedPublication();
    void failedBackgroundSaveIgnoresUnrelatedMemoryRefusal();
    void closeRechecksAfterBackgroundSave();
    void destructiveActionsResolveBackgroundSave();
    void uncertainPublicationProtectsCleanState();
    void metadataOnlyEditDuringSaveRemainsDirty();
    void backgroundSaveRecordingKeepsDispatchOrder();
    void aCompletedSaveWritesAndClearsModified();
    void aFailedSaveLeavesTheDocumentUntouched();
    void nativeSaveDistinguishesCodecAndIoErrors();
    void nativeSavePanicBeforeRenameCleansUp();
    void nativeSavePanicAfterRenameProtectsDocument();
    void nativeSaveCopyPanicAfterRename();
    void anEditDuringSaveLeavesTheDocumentModified();
    void largeWholeLayerPreviewKeepsTheEventLoopLive();
    void largeWholeLayerPreviewCancelLeavesTheDocument();
    void largeWholeLayerPreviewCommitReusesTheResult();
    void maskFullZoomPreviewStaysSynchronous();
    void adjustmentPreviewKeepsTheEventLoopLive();
    void latestAdjustmentPreviewWins();
    void adjustmentApplyKeepsTheEventLoopLive();
    void maskAdjustmentPreviewStaysSynchronous();
    void maskAdjustmentApplyStaysOnTheMask();
    void filterAndAdjustmentPanicsPreserveUndo();
    void lockedLayerReportsFilterRefusal();
    void adjustmentDialogOkReportsPanic();

private:
    pictura::PictureView* newDoc(int width, int height);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void OffthreadJobsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    auto* history = window_->findChild<pictura::HistoryPanel*>(QStringLiteral("historyPanel"));
    QVERIFY(history);
    history->setOptionsForTest(false, false, false, false, false);
}

void OffthreadJobsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

pictura::PictureView* OffthreadJobsTest::newDoc(int width, int height)
{
    if (!window_->newDocument(QStringLiteral("Offthread"), width, height, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

void OffthreadJobsTest::saveKeepsTheEventLoopLive()
{
    pictura::PictureView* view = newDoc(64, 64);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("live.psd"));
    pictura::save_set_delay(*view, 400);
    // Drain the new-document startup so its first paint is not mistaken for a
    // save stall; the probe below measures only the save.
    QTest::qWait(20);
    // Start the probe before the call so a regression that encodes in
    // `save_start` would show up as a gap.
    window_->show();
    PaintCounter paint;
    window_->imageView()->installEventFilter(&paint);
    TimerProbe probe;
    QObject::connect(&probe.timer, &QTimer::timeout, window_->imageView(),
                     [this] { window_->imageView()->update(); });
    probe.start();
    QVERIFY(pictura::save_start(*view, path, -1));
    QVERIFY(pictura::save_running(*view));

    pumpUntil([&] { return !pictura::save_running(*view); });
    probe.stop();
    QVERIFY2(probe.ticks >= 5, "the event loop kept ticking during the save");
    QVERIFY(paint.paints > 0);
    window_->imageView()->removeEventFilter(&paint);
    QVERIFY2(probe.maxGap < 50, "the save stalled the event loop");
    QVERIFY(pictura::save_finish(*view));
    QVERIFY(QFileInfo::exists(path));
    QVERIFY(!pictura::document_api::isDirty(*view));
    QCOMPARE(pictura::document_api::filePath(*view), path);
}

void OffthreadJobsTest::aCompletedSaveWritesAndClearsModified()
{
    pictura::PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    QVERIFY(pictura::document_api::isDirty(*view));
    const QString path = dir_.filePath(QStringLiteral("current.psd"));
    QVERIFY(pictura::save_start(*view, path, -1));
    pumpUntil([&] { return !pictura::save_running(*view); });
    QVERIFY(pictura::save_finish(*view));
    QVERIFY(!pictura::document_api::isDirty(*view));
    QCOMPARE(pictura::document_api::filePath(*view), path);

    pictura::PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, path));
    QCOMPARE(pictura::document_api::documentMode(reopened), QStringLiteral("rgb"));
    QCOMPARE(pictura::document_api::filePath(reopened), path);
    QCOMPARE(QColor(reopened.composite_argb(5, 5)), QColor(Qt::black));
}

void OffthreadJobsTest::aFailedSaveLeavesTheDocumentUntouched()
{
    pictura::PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    QVERIFY(pictura::save_start(*view, QStringLiteral("/proc/does-not-exist/x.psd"), -1));
    pumpUntil([&] { return !pictura::save_running(*view); });
    QVERIFY(!pictura::save_finish(*view));
    QVERIFY(pictura::document_api::filePath(*view).isEmpty());
    QVERIFY(pictura::document_api::isDirty(*view));
}

void OffthreadJobsTest::anEditDuringSaveLeavesTheDocumentModified()
{
    pictura::PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    const QString path = dir_.filePath(QStringLiteral("stale.psd"));
    pictura::save_set_delay(*view, 300);
    QVERIFY(pictura::save_start(*view, path, -1));
    QTimer::singleShot(50, view, [view] {
        pictura::image_adjust_direct(*view, QStringLiteral("desaturate"));
    });
    pumpUntil([&] { return !pictura::save_running(*view); });
    QVERIFY(pictura::save_finish(*view));
    QVERIFY(QFileInfo::exists(path));
    QCOMPARE(pictura::document_api::filePath(*view), path);
    QVERIFY(pictura::document_api::isDirty(*view));
}

namespace {

using pictura::PictureView;
using pictura::PicturaMainWindow;

void command(PicturaMainWindow& window, const char* id)
{
    window.registry()->refresh();
    QAction* action = window.registry()->action(QString::fromLatin1(id));
    Q_ASSERT(action);
    action->trigger();
}

void invert(PicturaMainWindow& window)
{
    command(window, "image.adjustments.invert");
}

void chooseSaveAs(PicturaMainWindow& window, const QString& path,
                  const char* id = pictura::command_ids::FileSaveAs)
{
    pictura::setSaveFileNameProviderForTest(
        [&path](QWidget*, const QString&, const QString&, const QStringList&, QString* filter) {
            const QString pattern = QStringLiteral("*.") + QFileInfo(path).suffix();
            for (const QString& candidate : pictura::saveFileFilters()) {
                if (candidate.contains(pattern, Qt::CaseInsensitive)) {
                    *filter = candidate;
                    break;
                }
            }
            return path;
        });
    window.setProfileDialogForTest([](bool, bool&) { return true; });
    command(window, id);
    window.setProfileDialogForTest({});
    pictura::setSaveFileNameProviderForTest({});
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

void addVanishingPlane(PictureView& view)
{
    pictura::VanishingPointDialog dialog(&view, QStringLiteral("Metadata"));
    QVERIFY(dialog.ready());
    const double corners[] = {8, 8, 24, 8, 24, 24, 8, 24};
    QCOMPARE(pictura::vp_create(rust::Slice<const double>(corners, 8)), 0);
    dialog.accept();
}

} // namespace

void OffthreadJobsTest::nativeSaveCallersWriteSameBytes()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    for (const QString& suffix : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        const QString sync = dir_.filePath(QStringLiteral("sync.") + suffix);
        const QString async = dir_.filePath(QStringLiteral("async.") + suffix);
        window_->setProfileDialogForTest([](bool, bool&) { return true; });
        QVERIFY(window_->saveActiveAsWithOptions(sync));
        pictura::save_set_delay(*view, 100);
        chooseSaveAs(*window_, async);
        QTRY_COMPARE(window_->documentPath(0), async);
        QVERIFY(!fileBytes(sync).isEmpty());
        QCOMPARE(fileBytes(sync), fileBytes(async));
    }
}

void OffthreadJobsTest::backgroundSavePreferencePersists()
{
    pictura::test::ScopedStateHome state;
    auto first = pictura::test::makeMainWindow();
    QVERIFY(first->saveInBackground());
    first->setAutoSaveRecovery(true, 30);
    first->showPreferences(QStringLiteral("File Handling"));
    auto* checkbox = first->preferencesDialog()->findChild<QCheckBox*>(QStringLiteral("preferences_saveInBackground"));
    QVERIFY(checkbox && checkbox->isChecked());
    checkbox->click();
    first->saveSessionNow();
    auto second = pictura::test::makeMainWindow();
    QVERIFY(!second->saveInBackground());
    QVERIFY(second->autoSaveRecovery());
    QCOMPARE(second->autoSaveMinutes(), 30);
    QVERIFY(second->newDocument(QStringLiteral("Waiting"), 16, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    const QString file = dir_.filePath(QStringLiteral("waiting.psd"));
    QVERIFY(second->saveActiveAs(file));
    pictura::save_set_delay(*second->activeView(), 400);
    bool busy = false;
    QTimer::singleShot(320, second.get(), [&] {
        busy = second->findChild<QWidget*>(QStringLiteral("jobBusy")) != nullptr;
    });
    command(*second, pictura::command_ids::FileSave);
    QVERIFY(busy);
    QVERIFY(!pictura::save_running(*second->activeView()));
}

void OffthreadJobsTest::backgroundSaveAcceptsInputAndShowsProgress()
{
    PictureView* first = newDoc(32, 32);
    QVERIFY(first);
    const int origin = window_->activeDocumentIndex();
    const QString path = dir_.filePath(QStringLiteral("editing.psd"));
    QVERIFY(window_->saveActiveAs(path));
    PictureView* second = newDoc(32, 32);
    QVERIFY(second);
    const int other = window_->activeDocumentIndex();
    window_->show();
    window_->setActiveDocumentIndex(origin);
    pictura::save_set_delay(*first, 700);
    command(*window_, pictura::command_ids::FileSave);
    QVERIFY(pictura::save_running(*first));
    auto* bar = window_->findChild<QProgressBar*>(QStringLiteral("backgroundSaveProgress"));
    QTRY_VERIFY(bar && bar->isVisible());
    QVERIFY(bar->value() < 100);
    invert(*window_);
    QCOMPARE(QColor(first->composite_argb(8, 8)), QColor(Qt::black));
    window_->setActiveDocumentIndex(other);
    QVERIFY(!bar->isVisible());
    invert(*window_);
    QCOMPARE(QColor(second->composite_argb(8, 8)), QColor(Qt::black));
    auto* tabs = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    const int originTab = window_->tabOfDocumentForTest(origin);
    const QRegularExpression saveSuffix(QStringLiteral("  [0-9]{1,2}%$"));
    QVERIFY(tabs && originTab >= 0);
    QVERIFY(saveSuffix.match(tabs->tabText(originTab)).hasMatch());
    QTRY_VERIFY(!saveSuffix.match(tabs->tabText(originTab)).hasMatch());
}

void OffthreadJobsTest::backgroundSaveAsCompletesOnOrigin()
{
    PictureView* first = newDoc(32, 32);
    QVERIFY(first);
    const int origin = window_->activeDocumentIndex();
    PictureView* second = newDoc(32, 32);
    QVERIFY(second);
    const int other = window_->activeDocumentIndex();
    window_->setActiveDocumentIndex(origin);
    const QString path = dir_.filePath(QStringLiteral("origin.psd"));
    pictura::save_set_delay(*first, 400);
    chooseSaveAs(*window_, path);
    QVERIFY(pictura::save_running(*first));
    invert(*window_);
    window_->setActiveDocumentIndex(other);
    QTRY_COMPARE(window_->documentPath(origin), path);
    QVERIFY(window_->isDocumentDirty(origin));
    QVERIFY(window_->documentPath(other).isEmpty());
    QVERIFY(!window_->isDocumentDirty(other));
    PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, path));
    QCOMPARE(QColor(reopened.composite_argb(8, 8)), QColor(Qt::white));
}

void OffthreadJobsTest::backgroundSaveUpdatesTheSavedHistoryState()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("history.psd"));
    QVERIFY(window_->saveActiveAs(path));
    invert(*window_);
    pictura::save_set_delay(*view, 400);
    command(*window_, pictura::command_ids::FileSave);
    QVERIFY(view->undo());
    QVERIFY(!window_->isDocumentDirty(0));
    QTRY_VERIFY(window_->isDocumentDirty(0));
    QVERIFY(view->redo());
    QVERIFY(!window_->isDocumentDirty(0));
}

void OffthreadJobsTest::backgroundSaveRejectsConflictingWrites()
{
    PictureView* first = newDoc(32, 32);
    QVERIFY(first);
    const QString path = dir_.filePath(QStringLiteral("claimed.psd"));
    pictura::save_set_delay(*first, 700);
    chooseSaveAs(*window_, path);
    QVERIFY(pictura::save_running(*first));
    const QString conflict = dir_.filePath(QStringLiteral("second.psd"));
    chooseSaveAs(*window_, conflict);
    QVERIFY(!QFileInfo::exists(conflict));
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("progress"), Qt::CaseInsensitive));
    QVERIFY(newDoc(32, 32));
    chooseSaveAs(*window_, dir_.filePath(QStringLiteral("./claimed.psd")));
    QVERIFY(window_->documentPath(1).isEmpty());
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("progress"), Qt::CaseInsensitive));
    QTRY_COMPARE(window_->documentPath(0), path);
}

void OffthreadJobsTest::failedBackgroundSaveIgnoresUnrelatedMemoryRefusal()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto path = dir_.filePath(QStringLiteral("unrelated-refusal.psd"));
    QVERIFY(window_->saveActiveAs(path));
    invert(*window_);
    pictura::save_set_fault_for_test(*view, 3);
    command(*window_, pictura::command_ids::FileSave);
    pictura::set_available_memory_for_test(0);
    view->select_all();
    const bool filled = pictura::edit_fill(*view, QColor(Qt::red).rgba(), -1,
                                           QStringLiteral("normal"), 100, false);
    const bool refused = pictura::edit_refused_for_memory();
    pictura::set_available_memory_for_test(-1);
    QVERIFY(!filled);
    QVERIFY(refused);
    QTRY_VERIFY_WITH_TIMEOUT(window_->statusBar()->currentMessage().contains(
                                 QStringLiteral("stopped unexpectedly")), 5000);
    QVERIFY(!window_->statusBar()->currentMessage().contains(QStringLiteral("not enough memory")));
}

void OffthreadJobsTest::backgroundSaveReportsUnconfirmedPublication()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    invert(*window_);
    const QString path = dir_.filePath(QStringLiteral("uncertain.psd"));
    pictura::save_set_sync_failure(*view, true);
    chooseSaveAs(*window_, path);
    QTRY_VERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("could not confirm"),
                                                               Qt::CaseInsensitive));
    QVERIFY(QFileInfo::exists(path));
    QVERIFY(window_->documentPath(0).isEmpty());
    QVERIFY(window_->isDocumentDirty(0));
}

void OffthreadJobsTest::closeRechecksAfterBackgroundSave()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("close-edit.psd"));
    QVERIFY(window_->saveActiveAs(path));
    pictura::save_set_delay(*view, 250);
    command(*window_, pictura::command_ids::FileSave);
    invert(*window_);
    const auto cancel = pictura::UnsavedChoice::Cancel;
    pictura::setRequestUnsavedChoice(&cancel);
    const bool cancelledClose = window_->closeDocument(0, true);
    pictura::setRequestUnsavedChoice(nullptr);
    QVERIFY(!cancelledClose);
    QVERIFY(!pictura::save_running(*view));
    QVERIFY(window_->isDocumentDirty(0));
    const auto save = pictura::UnsavedChoice::Save;
    pictura::setRequestUnsavedChoice(&save);
    const bool savedClose = window_->closeDocument(0, true);
    pictura::setRequestUnsavedChoice(nullptr);
    QVERIFY(savedClose);
    PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, path));
    QCOMPARE(QColor(reopened.composite_argb(8, 8)), QColor(Qt::black));
}

void OffthreadJobsTest::destructiveActionsResolveBackgroundSave()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("destructive.psd"));
    QVERIFY(window_->saveActiveAs(path));
    pictura::save_set_delay(*view, 250);
    for (int operation = 0; operation < 3; ++operation) {
        command(*window_, pictura::command_ids::FileSave);
        invert(*window_);
        const auto cancel = pictura::UnsavedChoice::Cancel;
        pictura::setRequestUnsavedChoice(&cancel);
        if (operation == 0) {
            command(*window_, pictura::command_ids::FileRevert);
        } else if (operation == 1) {
            window_->saveAllDocumentsForRestart();
        } else {
            window_->close();
        }
        pictura::setRequestUnsavedChoice(nullptr);
        QVERIFY(!pictura::save_running(*view));
        QCOMPARE(window_->documentCount(), 1);
        QVERIFY(window_->isDocumentDirty(0));
    }
}

void OffthreadJobsTest::uncertainPublicationProtectsCleanState()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("protected.psd"));
    QVERIFY(window_->saveActiveAs(path));
    window_->enableRecovery();
    invert(*window_);
    pictura::save_set_delay(*view, 400);
    pictura::save_set_sync_failure(*view, true);
    command(*window_, pictura::command_ids::FileSave);
    QVERIFY(view->undo());
    QVERIFY(!window_->isDocumentDirty(0));
    QTRY_VERIFY(window_->isDocumentDirty(0));
    QVERIFY(view->redo() && view->undo());
    QVERIFY(window_->isDocumentDirty(0));
    const auto cancel = pictura::UnsavedChoice::Cancel;
    pictura::setRequestUnsavedChoice(&cancel);
    const bool closed = window_->closeDocument(0, true);
    pictura::setRequestUnsavedChoice(nullptr);
    QVERIFY(!closed);
    pictura::save_set_sync_failure(*view, false);
    QVERIFY(window_->saveActive());
    QVERIFY(!window_->isDocumentDirty(0));
    window_->shutdownRecovery();
}

void OffthreadJobsTest::metadataOnlyEditDuringSaveRemainsDirty()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("metadata.psd"));
    QVERIFY(window_->saveActiveAs(path));
    invert(*window_);
    const int index = view->history_index();
    pictura::save_set_delay(*view, 400);
    command(*window_, pictura::command_ids::FileSave);
    addVanishingPlane(*view);
    QCOMPARE(view->history_index(), index);
    QTRY_COMPARE(pictura::save_progress(*view), -1);
    QVERIFY(window_->isDocumentDirty(0));
    QVERIFY(view->undo() && view->redo());
    QVERIFY(window_->isDocumentDirty(0));
    QVERIFY(window_->saveActive());
    QVERIFY(!window_->isDocumentDirty(0));
}

void OffthreadJobsTest::backgroundSaveRecordingKeepsDispatchOrder()
{
    PictureView* view = newDoc(32, 32);
    QVERIFY(view);
    const QString path = dir_.filePath(QStringLiteral("recorded.psd"));
    QVERIFY(window_->saveActiveAs(path));
    auto* panel = window_->findChild<pictura::ActionsPanel*>();
    QVERIFY(panel);
    auto* recorder = panel->engineForTest();
    QVERIFY(recorder);
    const QString set = recorder->createSet(QStringLiteral("Save tests"));
    const QString first = recorder->createAction(set, QStringLiteral("First"), 0, false, false, 0);
    const QString second = recorder->createAction(set, QStringLiteral("Second"), 0, false, false, 0);
    QVERIFY(recorder->startRecording(first, -1));
    pictura::save_set_delay(*view, 400);
    command(*window_, pictura::command_ids::FileSave);
    const int layers = view->layer_count();
    command(*window_, pictura::command_ids::LayerDuplicateLayer);
    QCOMPARE(view->layer_count(), layers + 1);
    recorder->stopRecording();
    QVERIFY(recorder->startRecording(second, -1));
    QTRY_COMPARE(pictura::save_progress(*view), -1);
    pictura::save_set_sync_failure(*view, true);
    command(*window_, pictura::command_ids::FileSave);
    command(*window_, pictura::command_ids::LayerDuplicateLayer);
    QCOMPARE(view->layer_count(), layers + 2);
    recorder->stopRecording();
    QTRY_COMPARE(pictura::save_progress(*view), -1);
    pictura::save_set_sync_failure(*view, false);

    QStringList firstSteps;
    QStringList firstCommands;
    QStringList secondCommands;
    for (const auto& row : recorder->snapshot()) {
        if (row.commandId.isEmpty()) {
            continue;
        }
        if (row.parentId == first) {
            firstSteps.append(row.id);
            firstCommands.append(row.commandId);
        } else if (row.parentId == second) {
            secondCommands.append(row.commandId);
        }
    }
    const QString duplicate = QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer);
    QCOMPARE(firstCommands, (QStringList{QStringLiteral("file.save"), duplicate}));
    QCOMPARE(secondCommands, QStringList{duplicate});
    recorder->setDocumentFacts(true, true, QStringLiteral("rgb"), false);
    QVERIFY(recorder->startPlayback(firstSteps));
    const auto save = recorder->playbackNext();
    QCOMPARE(save.commandId, QStringLiteral("file.save"));
    QCOMPARE(save.params.value(QStringLiteral("in")).toString(), path);
    const auto edit = recorder->playbackNext();
    QCOMPARE(edit.commandId, duplicate);
    recorder->cancelPlayback();
}

void OffthreadJobsTest::largeWholeLayerPreviewKeepsTheEventLoopLive()
{
    pictura::PictureView* view = newDoc(3000, 2400);
    QVERIFY(view);
    const pictura::FilterCommandSpec* spec = wholeLayerSpec();
    QVERIFY(spec);
    const QImage before = view->image();

    pictura::FilterPreviewView preview;
    preview.canvas = window_->imageView();
    preview.canvasZoom = 1.0;
    preview.devicePixelRatio = 1.0;
    pictura::FilterPreviewDialog dialog(view, *spec, preview);
    pictura::filter_exact_preview_set_delay(*view, 400);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY_WITH_TIMEOUT(pictura::filter_exact_preview_busy(*view), 4000);

    TimerProbe probe;
    // The dialog's 0 ms timer has already started the worker, so this window
    // covers the in-flight job rather than dialog construction.
    probe.start();
    pumpUntil([&] { return !pictura::filter_exact_preview_busy(*view); });
    probe.stop();
    QVERIFY2(probe.ticks >= 5, "the event loop kept ticking during the preview");
    QVERIFY2(probe.maxGap < 50, "the preview stalled the event loop");
    pictura::ImageView* canvas = window_->imageView();
    QTRY_VERIFY_WITH_TIMEOUT(canvas && canvas->filterPreviewShownForTest(), 4000);
    QCOMPARE(view->image(), before);

    dialog.reject();
    QVERIFY(!canvas->filterPreviewShownForTest());
    QCOMPARE(view->image(), before);
}

void OffthreadJobsTest::largeWholeLayerPreviewCancelLeavesTheDocument()
{
    pictura::PictureView* view = newDoc(3000, 2400);
    QVERIFY(view);
    const pictura::FilterCommandSpec* spec = wholeLayerSpec();
    QVERIFY(spec);
    const QImage before = view->image();
    pictura::FilterPreviewView preview;
    preview.canvas = window_->imageView();
    preview.canvasZoom = 1.0;
    preview.devicePixelRatio = 1.0;
    pictura::filter_exact_preview_set_delay(*view, 300);

    {
        pictura::FilterPreviewDialog dialog(view, *spec, preview);
        dialog.show();
        QVERIFY(QTest::qWaitForWindowExposed(&dialog));
        QTRY_VERIFY_WITH_TIMEOUT(pictura::filter_exact_preview_busy(*view), 4000);
        dialog.reject();
        pumpUntil([&] { return !pictura::filter_exact_preview_busy(*view); });
        QCOMPARE(view->image(), before);
        QVERIFY(!window_->imageView()->filterPreviewShownForTest());
    }

    {
        // Cancel again after the worker has landed its picture.
        pictura::FilterPreviewDialog dialog(view, *spec, preview);
        dialog.show();
        QVERIFY(QTest::qWaitForWindowExposed(&dialog));
        QTRY_VERIFY_WITH_TIMEOUT(pictura::filter_exact_preview_busy(*view), 4000);
        pumpUntil([&] { return !pictura::filter_exact_preview_busy(*view); });
        QTRY_VERIFY_WITH_TIMEOUT(window_->imageView()->filterPreviewShownForTest(), 4000);
        dialog.reject();
        QCOMPARE(view->image(), before);
        QVERIFY(!window_->imageView()->filterPreviewShownForTest());
    }
}

void OffthreadJobsTest::largeWholeLayerPreviewCommitReusesTheResult()
{
    pictura::PictureView* view = newDoc(3000, 2400);
    QVERIFY(view);
    const pictura::FilterCommandSpec* spec = wholeLayerSpec();
    QVERIFY(spec);
    const int history = view->history_count();
    pictura::FilterPreviewView preview;
    preview.canvas = window_->imageView();
    preview.canvasZoom = 1.0;
    preview.devicePixelRatio = 1.0;
    pictura::FilterPreviewDialog dialog(view, *spec, preview);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY_WITH_TIMEOUT(pictura::filter_exact_preview_busy(*view), 4000);
    pumpUntil([&] { return !pictura::filter_exact_preview_busy(*view); });
    QTRY_VERIFY_WITH_TIMEOUT(window_->imageView()->filterPreviewShownForTest(), 4000);
    const QList<double> values = dialog.values();
    dialog.accept();
    QVERIFY(pictura::filter_commit_start(*view, spec->kind, values,
                                         QStringLiteral("Filter: Radial Blur")));
    QVERIFY(pictura::filter_commit_finish(*view));
    QCOMPARE(view->history_count(), history + 1);
}

void OffthreadJobsTest::maskFullZoomPreviewStaysSynchronous()
{
    pictura::PictureView* view = newDoc(3000, 2400);
    QVERIFY(view);
    QVERIFY(makeMaskTarget(*view));
    const pictura::FilterCommandSpec* spec =
        pictura::filterCommandForKind(QStringLiteral("clouds"));
    QVERIFY(spec);
    const std::vector<std::uint8_t> before = maskThumb(*view, 32);

    pictura::FilterPreviewView preview;
    preview.canvas = window_->imageView();
    preview.canvasZoom = 1.0;
    preview.devicePixelRatio = 1.0;
    pictura::FilterPreviewDialog dialog(view, *spec, preview);
    const QList<double> values = dialog.values();
    QVERIFY2(!pictura::filter_exact_preview_start(*view, spec->kind, values, 0, 0, 3000, 2400,
                                                  1.0),
             "a mask target keeps the synchronous path");

    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY_WITH_TIMEOUT(maskThumb(*view, 32) != before, 8000);
    QVERIFY2(!pictura::filter_exact_preview_busy(*view),
             "the dialog did not start the exact worker for a mask target");
    dialog.reject();
    QVERIFY(maskThumb(*view, 32) == before);
    QVERIFY(!window_->imageView()->filterPreviewShownForTest());
}

void OffthreadJobsTest::maskAdjustmentApplyStaysOnTheMask()
{
    pictura::PictureView* view = newDoc(128, 128);
    QVERIFY(view);
    QVERIFY(makeMaskTarget(*view));
    const QImage layerBefore = view->layer_thumbnail(0, 64);
    const std::vector<std::uint8_t> maskBefore = maskThumb(*view, 64);
    const int history = view->history_count();

    QVERIFY(pictura::image_adjust_apply_start(*view, slice(lightnessBlock(-100.0)),
                                              QStringLiteral("Hue/Saturation")));
    QVERIFY(pictura::filter_commit_finish(*view));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Hue/Saturation"));
    QCOMPARE(view->layer_thumbnail(0, 64), layerBefore);
    QVERIFY(maskThumb(*view, 64) != maskBefore);
}

void OffthreadJobsTest::adjustmentPreviewKeepsTheEventLoopLive()
{
    pictura::PictureView* view = newDoc(512, 512);
    QVERIFY(view);
    // drains the new-document startup so its first paint is not read as a stall
    QTest::qWait(20);
    const QImage before = view->image();
    const QByteArray block = lightnessBlock(-100.0);
    pictura::image_adjust_preview_set_delay(*view, 400);
    // Start the probe before the call so a synchronous start would show a gap.
    TimerProbe probe;
    probe.start();
    QVERIFY(pictura::image_adjust_preview_start(*view, slice(block)));

    pumpUntil([&] { return !pictura::image_adjust_preview_busy(*view); });
    probe.stop();
    QVERIFY2(probe.ticks >= 5, "the event loop kept ticking during the preview");
    QVERIFY2(probe.maxGap < 50, "the preview stalled the event loop");
    QVERIFY(pictura::image_adjust_preview_poll(*view));
    QVERIFY(view->image() != before);
    pictura::image_adjust_preview_cancel(*view);
    QCOMPARE(view->image(), before);
    QVERIFY(!pictura::document_api::isDirty(*view));
}

void OffthreadJobsTest::latestAdjustmentPreviewWins()
{
    pictura::PictureView* view = newDoc(128, 128);
    QVERIFY(view);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    const QRgb before = view->composite_argb(50, 50);
    pictura::image_adjust_preview_set_delay(*view, 300);
    QVERIFY(pictura::image_adjust_preview_start(*view, slice(lightnessBlock(-100.0))));
    QVERIFY(pictura::image_adjust_preview_start(*view, slice(lightnessBlock(100.0))));
    pumpUntil([&] { return !pictura::image_adjust_preview_busy(*view); });
    QVERIFY(pictura::image_adjust_preview_poll(*view));
    const QRgb pixel = view->composite_argb(50, 50);
    QVERIFY(pixel != before);
    QVERIFY(qRed(pixel) > 128 && qGreen(pixel) > 128 && qBlue(pixel) > 128);
    pictura::image_adjust_preview_cancel(*view);
}

void OffthreadJobsTest::adjustmentApplyKeepsTheEventLoopLive()
{
    pictura::PictureView* view = newDoc(512, 512);
    QVERIFY(view);
    // drains the new-document startup so its first paint is not read as a stall
    QTest::qWait(20);
    const int history = view->history_count();
    const QByteArray block = lightnessBlock(-100.0);
    pictura::image_adjust_apply_set_delay(*view, 400);
    // Start the probe before the call so a synchronous start would show a gap.
    TimerProbe probe;
    probe.start();
    QVERIFY(pictura::image_adjust_apply_start(*view, slice(block),
                                              QStringLiteral("Hue/Saturation")));

    pumpUntil([&] { return !pictura::filter_commit_running(*view); });
    probe.stop();
    QVERIFY2(probe.ticks >= 5, "the event loop kept ticking during the apply");
    QVERIFY2(probe.maxGap < 50, "the apply stalled the event loop");
    QVERIFY(pictura::filter_commit_finish(*view));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Hue/Saturation"));
}

void OffthreadJobsTest::maskAdjustmentPreviewStaysSynchronous()
{
    pictura::PictureView* view = newDoc(128, 128);
    QVERIFY(view);
    QVERIFY(makeMaskTarget(*view));
    const bool dirty = pictura::document_api::isDirty(*view);
    const QByteArray block = lightnessBlock(-100.0);
    QVERIFY2(!pictura::image_adjust_preview_start(*view, slice(block)),
             "a mask target keeps the synchronous path");
    QVERIFY(pictura::image_adjust_preview(*view, slice(block), 0, 0, 128, 128));
    pictura::image_adjust_preview_cancel(*view);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void OffthreadJobsTest::nativeSaveDistinguishesCodecAndIoErrors()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto path = dir_.filePath(QStringLiteral("typed-save.psd"));
    QVERIFY(window_->saveActiveAs(path));
    const auto before = fileBytes(path);
    invert(*window_);
    for (int fault : {1, 2}) {
        pictura::save_set_fault_for_test(*view, fault);
        QVERIFY(!window_->saveActiveAs(path));
        QCOMPARE(pictura::save_last_result(*view).code, fault == 1 ? 100u : 101u);
        QCOMPARE(pictura::save_last_result(*view).publication, 0);
        QCOMPARE(fileBytes(path), before);
    }
}

void OffthreadJobsTest::nativeSavePanicBeforeRenameCleansUp()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto source = dir_.filePath(QStringLiteral("source.psd"));
    const auto destination = dir_.filePath(QStringLiteral("destination.psd"));
    QVERIFY(window_->saveActiveAs(destination));
    QVERIFY(window_->saveActiveAs(source));
    const auto before = fileBytes(destination);
    invert(*window_);
    const int recovery = pictura::recovery_status(*view);
    const auto temps = QDir(dir_.path()).entryList({QStringLiteral("*.tmp")}, QDir::Files);
    pictura::save_set_fault_for_test(*view, 3);
    QVERIFY(!window_->saveActiveAs(destination));
    QCOMPARE(pictura::save_last_result(*view).code, 6u);
    QCOMPARE(pictura::save_last_result(*view).publication, 0);
    QCOMPARE(fileBytes(destination), before);
    QCOMPARE(window_->documentPath(0), source);
    QVERIFY(window_->isDocumentDirty(0));
    QCOMPARE(pictura::recovery_status(*view), recovery);
    QCOMPARE(QDir(dir_.path()).entryList({QStringLiteral("*.tmp")}, QDir::Files), temps);
}

void OffthreadJobsTest::nativeSavePanicAfterRenameProtectsDocument()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto path = dir_.filePath(QStringLiteral("uncertain-save.psd"));
    QVERIFY(window_->saveActiveAs(path));
    invert(*window_);
    pictura::save_set_fault_for_test(*view, 4);
    QVERIFY(!window_->saveActiveAs(path));
    QCOMPARE(pictura::save_last_result(*view).code, 6u);
    QCOMPARE(pictura::save_last_result(*view).publication, 2);
    QCOMPARE(window_->documentPath(0), path);
    QVERIFY(window_->isDocumentDirty(0));
    pictura::PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, path));
    QCOMPARE(reopened.composite_argb(0, 0), view->composite_argb(0, 0));
    const auto cancel = pictura::UnsavedChoice::Cancel;
    pictura::setRequestUnsavedChoice(&cancel);
    const bool closed = window_->closeDocument(0, true);
    pictura::setRequestUnsavedChoice(nullptr);
    QVERIFY(!closed);
    QVERIFY(window_->saveActiveAs(path));
    QVERIFY(!window_->isDocumentDirty(0));
}

void OffthreadJobsTest::nativeSaveCopyPanicAfterRename()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto source = dir_.filePath(QStringLiteral("copy-source.psd"));
    const auto copy = dir_.filePath(QStringLiteral("uncertain-copy.psd"));
    QVERIFY(window_->saveActiveAs(source));
    invert(*window_);
    const int history = view->history_count();
    const int recovery = pictura::recovery_status(*view);
    const auto before = fileBytes(source);
    pictura::save_set_fault_for_test(*view, 4);
    chooseSaveAs(*window_, copy, pictura::command_ids::FileSaveCopy);
    QCOMPARE(pictura::save_last_result(*view).code, 6u);
    QCOMPARE(pictura::save_last_result(*view).publication, 2);
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("copy")));
    QCOMPARE(fileBytes(source), before);
    QCOMPARE(window_->documentPath(0), source);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::recovery_status(*view), recovery);
    QVERIFY(window_->isDocumentDirty(0));
    pictura::PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, copy));
    QCOMPARE(reopened.composite_argb(0, 0), view->composite_argb(0, 0));
}

void OffthreadJobsTest::filterAndAdjustmentPanicsPreserveUndo()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    invert(*window_);
    QVERIFY(view->history_add_snapshot(QStringLiteral("Keep")));
    invert(*window_);
    QVERIFY(view->undo());
    const int count = view->history_count();
    const int index = view->history_index();
    const bool dirty = window_->isDocumentDirty(0);
    const auto pixel = view->composite_argb(0, 0);

    for (int stage : {2, 1}) {
        pictura::filter_set_commit_panic_for_test(*view, stage);
        if (stage == 2) {
            QVERIFY(pictura::filter_commit_start(
                *view, QStringLiteral("gaussian-blur"), {2.0}, QStringLiteral("Blur")));
        } else {
            QVERIFY(pictura::image_adjust_apply_start(
                *view, slice(lightnessBlock(20)), QStringLiteral("Hue/Saturation")));
        }
        QTRY_VERIFY_WITH_TIMEOUT(!pictura::filter_commit_running(*view), 5000);
        QVERIFY(!pictura::filter_commit_finish(*view));
        QCOMPARE(pictura::filter_commit_status(*view), 6u);
        QCOMPARE(view->history_count(), count);
        QCOMPARE(view->history_index(), index);
        QCOMPARE(view->composite_argb(0, 0), pixel);
        QCOMPARE(window_->isDocumentDirty(0), dirty);
        QCOMPARE(pictura::history_snapshot_count(*view), 1);
        QCOMPARE(pictura::history_snapshot_label(*view, 0), QStringLiteral("Keep"));
        QVERIFY(view->redo());
        QVERIFY(view->undo());
    }
    QVERIFY(view->history_restore_snapshot(0));
    QCOMPARE(view->composite_argb(0, 0), pixel);
    const int afterRestore = view->history_count();
    QVERIFY(pictura::image_adjust_apply_start(
        *view, slice(lightnessBlock(20)), QStringLiteral("Hue/Saturation")));
    QVERIFY(pictura::filter_commit_finish(*view));
    QCOMPARE(pictura::filter_commit_status(*view), 0u);
    QCOMPARE(view->history_count(), afterRestore + 1);
}

void OffthreadJobsTest::lockedLayerReportsFilterRefusal()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const QString kind = QStringLiteral("gaussian-blur");
    QVERIFY(pictura::filter_commit_start(*view, kind, {2.0}, QStringLiteral("Blur")));
    QVERIFY(pictura::filter_commit_finish(*view));
    QCOMPARE(view->set_layers_lock({QStringLiteral("0")}, QStringLiteral("pixels"), true), 1);
    const int count = view->history_count();
    const auto pixel = view->composite_argb(0, 0);
    const bool dirty = window_->isDocumentDirty(0);
    QVERIFY(!pictura::filter_commit_start(*view, kind, {2.0}, QStringLiteral("Blur")));
    QCOMPARE(pictura::filter_commit_status(*view), 3u);
    QVERIFY(pictura::filter_last_error(*view).contains(QStringLiteral("pixel-locked")));
    QVERIFY(!pictura::filter_commit_running(*view));
    QCOMPARE(view->history_count(), count);
    QCOMPARE(view->composite_argb(0, 0), pixel);
    QCOMPARE(window_->isDocumentDirty(0), dirty);
}

void OffthreadJobsTest::adjustmentDialogOkReportsPanic()
{
    auto* view = newDoc(32, 32);
    QVERIFY(view);
    const auto pixel = view->composite_argb(0, 0);
    const int count = view->history_count();
    auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("hue-saturation"), view,
        lightnessBlock(20), QRect(0, 0, 32, 32), window_.get());
    QVERIFY(dialog);
    auto* ok = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonOK"));
    QVERIFY(ok);
    const auto title = dialog->windowTitle();
    pictura::filter_set_commit_panic_for_test(*view, 1);
    ok->click();
    QCOMPARE(pictura::filter_commit_status(*view), 6u);
    const auto message = window_->statusBar()->currentMessage();
    QVERIFY(message.contains(title));
    QVERIFY(!message.contains(QStringLiteral("injected")));
    QCOMPARE(view->history_count(), count);
    QCOMPARE(view->composite_argb(0, 0), pixel);
}

PICTURA_QT_TEST_MAIN(OffthreadJobsTest)
#include "tst_offthread_jobs.moc"
