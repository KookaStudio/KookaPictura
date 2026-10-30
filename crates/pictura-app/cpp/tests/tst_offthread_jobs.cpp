// Off-thread previews, applies, and native saves (#972): the GUI keeps
// processing events while the worker runs, and the landed result is the one the
// synchronous path would produce.

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QEventLoop>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

#include <functional>
#include <memory>
#include <vector>

#include "api/document_api.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "image_view.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/channels.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_core/save_jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "qt_test_support.h"

namespace {

::rust::Slice<const std::uint8_t> slice(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), std::size_t(bytes.size())};
}

QByteArray blockOf(const char* kind)
{
    const ::rust::Vec<std::uint8_t> v = pictura::image_adjustment_default(
        QString::fromLatin1(kind), 0x000000u, 0xffffffu);
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

QByteArray lightnessBlock(double value)
{
    const ::rust::Vec<std::uint8_t> v = pictura::image_adjustment_set(
        slice(blockOf("hue-saturation")), QStringLiteral("lightness"), value);
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

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
    void aCompletedSaveWritesAndClearsModified();
    void aFailedSaveLeavesTheDocumentUntouched();
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
    TimerProbe probe;
    probe.start();
    QVERIFY(pictura::save_start(*view, path));
    QVERIFY(pictura::save_running(*view));

    pumpUntil([&] { return !pictura::save_running(*view); });
    probe.stop();
    QVERIFY2(probe.ticks >= 5, "the event loop kept ticking during the save");
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
    QVERIFY(pictura::save_start(*view, path));
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
    QVERIFY(pictura::save_start(*view, QStringLiteral("/proc/does-not-exist/x.psd")));
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
    QVERIFY(pictura::save_start(*view, path));
    QTimer::singleShot(50, view, [view] {
        pictura::image_adjust_direct(*view, QStringLiteral("desaturate"));
    });
    pumpUntil([&] { return !pictura::save_running(*view); });
    QVERIFY(pictura::save_finish(*view));
    QVERIFY(QFileInfo::exists(path));
    QCOMPARE(pictura::document_api::filePath(*view), path);
    QVERIFY(pictura::document_api::isDirty(*view));
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

QTEST_MAIN(OffthreadJobsTest)
#include "tst_offthread_jobs.moc"
