// Whole-document commands as cancellable jobs (#1278): on a large document
// Rotate, Flatten and a depth conversion run on a worker behind the progress
// window, Cancel leaves the document and history unchanged, and a small
// document's command still applies at once. The fills run the same way
// (#1300).

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QPushButton>

#include <algorithm>
#include <functional>

#include "commands.h"
#include "document_command.h"
#include "api/document_api.h"
#include "fill_dialog.h"
#include "selftest_paint_fixture.h"

#include "api/document_bridge.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

// 4.32 MP, over the 4 MP job threshold, and not square, so a rotation shows.
constexpr int kWide = 2400;
constexpr int kTall = 1800;

QImage seed(int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// What the job window showed while a command ran.
struct Seen {
    bool window = false;
    bool busy = false;
    int value = 0;
};

} // namespace

class DocumentJobsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void aRotateRunsAsAJob();
    void anArbitraryRotateRunsAsAJob();
    void cancellingAFlattenChangesNothing();
    void aDepthConversionShowsABusyBar();
    void aSmallDocumentAppliesAtOnce();
    void aLargeGradientRunsAsAJob();
    void cancellingAFillChangesNothing();
    void aSmallGradientFillsAtOnce();
    void rotationPanicLeavesDocumentAndHistoryIntact();
    void validationRefusalClearsPreviousResult();
    void staleOrdinaryWorkerDoesNotPublish();
    void deferredTransformSettlesBeforeWorkerSnapshot();

private:
    bool dispatch(const char* id);
    // Run command `id`, watching the job window; with `cancel`, press its
    // Cancel button as soon as it shows.
    Seen run(const char* id, bool cancel = false);
    // Run `action`, watching the job window as `run` does. A Fill dialog it
    // opens is accepted as it is, and an angle prompt answered with 15°.
    Seen watch(const std::function<void()>& action, bool cancel = false);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void DocumentJobsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

bool DocumentJobsTest::dispatch(const char* id)
{
    return window_->registry()->dispatch(QString::fromLatin1(id));
}

Seen DocumentJobsTest::run(const char* id, bool cancel)
{
    return watch([this, id] { dispatch(id); }, cancel);
}

Seen DocumentJobsTest::watch(const std::function<void()>& action, bool cancel)
{
    Seen seen;
    QTimer probe;
    probe.setInterval(20);
    connect(&probe, &QTimer::timeout, this, [&] {
        if (auto* fill = window_->findChild<pictura::FillDialog*>();
            fill && fill->isVisible()) {
            fill->accept();
            return;
        }
        auto* prompt = window_->findChild<QDialog*>(QStringLiteral("rotateCanvasDialog"));
        if (prompt && prompt->isVisible()) {
            if (auto* angle = prompt->findChild<QLineEdit*>(QStringLiteral("rotateCanvasAngle"))) {
                angle->setText(QStringLiteral("15"));
                prompt->accept();
            } else {
                prompt->reject();
            }
            return;
        }
        auto* d = window_->findChild<QProgressDialog*>(QStringLiteral("jobProgress"));
        if (!d || !d->isVisible()) {
            return;
        }
        seen.window = true;
        seen.busy = seen.busy || d->maximum() == 0;
        seen.value = std::max(seen.value, d->value());
        if (cancel) {
            if (auto* button = d->findChild<QPushButton*>()) {
                cancel = false;
                button->click();
            }
        }
    });
    probe.start();
    action();
    probe.stop();
    return seen;
}

// [lop_whole_image_job] [lop_counted_progress]
void DocumentJobsTest::aRotateRunsAsAJob()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_rotate_job"));
    QVERIFY2(f.ok(), "rotate fixture");
    const int count = f.view->history_count();
    pictura::document_job_set_delay(*f.view, 800);
    const Seen seen = run(pictura::command_ids::ImageRotate90Cw);
    QVERIFY2(seen.window, "the progress window appeared");
    QVERIFY2(!seen.busy && seen.value > 0, "the bar counted the layers");
    QVERIFY(!pictura::document_job_pending(*f.view));
    QCOMPARE(f.view->document_width(), kTall);
    QCOMPARE(f.view->document_height(), kWide);
    QCOMPARE(f.view->history_count(), count + 1);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Rotate"));
}

// [lop_whole_image_cancel]
// [lop_arbitrary_rotate_job]
void DocumentJobsTest::anArbitraryRotateRunsAsAJob()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_arbitrary_job"));
    QVERIFY2(f.ok(), "rotate fixture");
    const int count = f.view->history_count();
    pictura::document_job_set_delay(*f.view, 800);
    QSize held;
    QTimer during;
    during.setInterval(20);
    connect(&during, &QTimer::timeout, this, [&] {
        auto* d = window_->findChild<QProgressDialog*>(QStringLiteral("jobProgress"));
        if (d && d->isVisible()) {
            held = QSize(f.view->document_width(), f.view->document_height());
        }
    });
    during.start();
    const Seen seen = run(pictura::command_ids::ImageRotateArbitrary);
    during.stop();
    QVERIFY2(seen.window, "the progress window appeared");
    QCOMPARE(held, QSize(kWide, kTall));
    QVERIFY(!pictura::document_job_pending(*f.view));
    // 15° grows the canvas to the rotated bounds.
    QVERIFY(f.view->document_width() > kWide);
    QVERIFY(f.view->document_height() > kTall);
    QCOMPARE(f.view->history_count(), count + 1);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Rotate"));
}

void DocumentJobsTest::cancellingAFlattenChangesNothing()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_flatten_job"));
    QVERIFY2(f.ok(), "flatten fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    const int layers = f.view->layer_count();
    const int count = f.view->history_count();
    const int index = f.view->history_index();
    const QImage before = f.view->display_image(2, 0, 0, 1200, 900);

    pictura::document_job_set_delay(*f.view, 30000);
    QElapsedTimer clock;
    clock.start();
    const Seen seen = run(pictura::command_ids::LayerFlattenImage, true);
    QVERIFY2(seen.window, "the progress window offered Cancel");
    QVERIFY2(clock.elapsed() < 20000, "the cancel ended the held job");
    QCOMPARE(pictura::document_job_status(*f.view), 4u);
    QVERIFY(pictura::document_op_result(*f.view).isEmpty());
    QVERIFY(!pictura::document_job_pending(*f.view));
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->history_count(), count);
    QCOMPARE(f.view->history_index(), index);
    QVERIFY2(f.view->display_image(2, 0, 0, 1200, 900) == before, "the document is unchanged");

    // The same command then applies as one state.
    pictura::document_job_set_delay(*f.view, 0);
    QVERIFY(dispatch(pictura::command_ids::LayerFlattenImage));
    QCOMPARE(f.view->layer_count(), 1);
    QCOMPARE(f.view->history_count(), count + 1);
}

// [lop_busy_bar]
void DocumentJobsTest::aDepthConversionShowsABusyBar()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_depth_job"));
    QVERIFY2(f.ok(), "depth fixture");
    const int count = f.view->history_count();
    pictura::document_job_set_delay(*f.view, 800);
    const Seen seen = run(pictura::command_ids::ImageMode16Bits);
    QVERIFY2(seen.window, "the progress window appeared");
    QVERIFY2(seen.busy, "a conversion that cannot count shows a busy bar");
    QCOMPARE(pictura::image_depth_bits(*f.view), 16);
    QCOMPARE(f.view->history_count(), count + 1);
}

// [lop_small_sync]
void DocumentJobsTest::aSmallDocumentAppliesAtOnce()
{
    Fixture f(*window_, seed(64, 48), QStringLiteral("pictura_small_rotate"));
    QVERIFY2(f.ok(), "small fixture");
    const int count = f.view->history_count();
    pictura::document_job_set_delay(*f.view, 800);
    QElapsedTimer clock;
    clock.start();
    const Seen seen = run(pictura::command_ids::ImageRotate90Cw);
    QVERIFY2(!seen.window, "no progress window");
    QVERIFY2(clock.elapsed() < 800, "the delay seam is never reached");
    QCOMPARE(f.view->document_width(), 48);
    QCOMPARE(f.view->history_count(), count + 1);
}

// [ocf_fill_job]
void DocumentJobsTest::aLargeGradientRunsAsAJob()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_gradient_job"));
    QVERIFY2(f.ok(), "gradient fixture");
    window_->setActiveTool(pictura::ToolId::Gradient);
    const int base = f.view->history_index();
    const QRgb before = f.view->sample_argb(0, kTall / 2);
    pictura::document_job_set_delay(*f.view, 800);
    const Seen seen = watch([&] { f.drag({QPointF(0, kTall / 2), QPointF(kWide - 1, kTall / 2)}); });
    QVERIFY2(seen.window, "the progress window appeared");
    QVERIFY2(seen.busy, "a fill that cannot count shows a busy bar");
    QVERIFY(!pictura::document_job_pending(*f.view));
    QVERIFY(f.committedOnce(base, "Gradient"));
    QVERIFY2(f.view->sample_argb(0, kTall / 2) != before, "the gradient landed");
}

// [ocf_fill_cancel]
void DocumentJobsTest::cancellingAFillChangesNothing()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("pictura_fill_job"));
    QVERIFY2(f.ok(), "fill fixture");
    const int count = f.view->history_count();
    const int index = f.view->history_index();
    const QImage before = f.view->display_image(2, 0, 0, 1200, 900);

    pictura::document_job_set_delay(*f.view, 30000);
    QElapsedTimer clock;
    clock.start();
    const Seen seen = run(pictura::command_ids::EditFill, true);
    QVERIFY2(seen.window, "the progress window offered Cancel");
    QVERIFY2(clock.elapsed() < 20000, "the cancel ended the held job");
    QVERIFY(!pictura::document_job_pending(*f.view));
    QCOMPARE(f.view->history_count(), count);
    QCOMPARE(f.view->history_index(), index);
    QVERIFY2(f.view->display_image(2, 0, 0, 1200, 900) == before, "the document is unchanged");
}

// [ocf_fill_small_sync]
void DocumentJobsTest::aSmallGradientFillsAtOnce()
{
    Fixture f(*window_, seed(64, 48), QStringLiteral("pictura_small_gradient"));
    QVERIFY2(f.ok(), "small fixture");
    window_->setActiveTool(pictura::ToolId::Gradient);
    const int base = f.view->history_index();
    pictura::document_job_set_delay(*f.view, 800);
    QElapsedTimer clock;
    clock.start();
    const Seen seen = watch([&] { f.drag({QPointF(0, 24), QPointF(63, 24)}); });
    QVERIFY2(!seen.window, "no progress window");
    QVERIFY2(clock.elapsed() < 800, "the delay seam is never reached");
    QVERIFY(!pictura::document_job_pending(*f.view));
    QVERIFY(f.committedOnce(base, "Gradient"));
}

void DocumentJobsTest::rotationPanicLeavesDocumentAndHistoryIntact()
{
    for (const auto size : {QSize(64, 48), QSize(kWide, kTall)}) {
        Fixture f(*window_, seed(size.width(), size.height()), QStringLiteral("rotate-panic"));
        QVERIFY(f.ok());
        const int history = f.view->history_count();
        const bool dirty = pictura::document_api::isDirty(*f.view);
        const auto pixel = f.view->composite_argb(0, 0);
        pictura::document_job_panic_for_test(*f.view);
        run(pictura::command_ids::ImageRotate90Cw);
        QCOMPARE(pictura::document_job_status(*f.view), 6u);
        QVERIFY(!pictura::document_job_pending(*f.view));
        QCOMPARE(f.view->document_width(), size.width());
        QCOMPARE(f.view->document_height(), size.height());
        QCOMPARE(f.view->composite_argb(0, 0), pixel);
        QCOMPARE(f.view->history_count(), history);
        QCOMPARE(pictura::document_api::isDirty(*f.view), dirty);
        run(pictura::command_ids::ImageRotate90Cw);
        QCOMPARE(pictura::document_job_status(*f.view), 0u);
        QCOMPARE(f.view->document_width(), size.height());
        QCOMPARE(f.view->history_count(), history + 1);
    }
}

void DocumentJobsTest::validationRefusalClearsPreviousResult()
{
    Fixture f(*window_, seed(64, 48), QStringLiteral("invalid-rotation"));
    QVERIFY(f.ok());
    QVERIFY(f.view->duplicate_layer(0) >= 0);
    QVERIFY(f.view->flatten_image() > 0);
    QVERIFY(!pictura::document_op_result(*f.view).isEmpty());
    const int history = f.view->history_count();
    const bool dirty = pictura::document_api::isDirty(*f.view);
    QVERIFY(!pictura::runDocumentCommand(window_.get(), *f.view, QStringLiteral("Rotate"),
                                        [&] { f.view->rotate_doc(0); }));
    QCOMPARE(pictura::document_job_status(*f.view), 3u);
    QVERIFY(pictura::document_op_result(*f.view).isEmpty());
    QCOMPARE(f.view->history_count(), history);
    QCOMPARE(f.view->document_width(), 64);
    QCOMPARE(pictura::document_api::isDirty(*f.view), dirty);
}

void DocumentJobsTest::staleOrdinaryWorkerDoesNotPublish()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("stale-rotation"));
    QVERIFY(f.ok());
    pictura::document_job_arm(*f.view, true);
    const auto stale = pictura::document_job_attempt(*f.view);
    pictura::document_job_set_delay(*f.view, 150);
    QVERIFY(f.view->rotate_doc(1));
    pictura::document_job_arm(*f.view, false);
    pictura::document_job_edit_source_for_test(*f.view);
    const auto cancelled = pictura::document_job_finish_for(*f.view, stale);
    QCOMPARE(cancelled.code, 4u);
    QVERIFY(!cancelled.applied);
    QCOMPARE(f.view->document_width(), kWide);
    QCOMPARE(f.view->layer_name(0), QStringLiteral("Edited while worker ran"));

    pictura::document_job_arm(*f.view, true);
    const auto replaced = pictura::document_job_attempt(*f.view);
    QVERIFY(f.view->rotate_doc(1));
    pictura::document_job_arm(*f.view, false);
    QVERIFY(pictura::document_api::newDocument(*f.view, 22, 15, QStringLiteral("rgb"), 8,
                                                QStringLiteral("white")));
    QVERIFY(f.view->rotate_doc(1));
    const int history = f.view->history_count();
    QCOMPARE(pictura::document_job_finish_for(*f.view, replaced).code, 4u);
    QCOMPARE(pictura::document_job_status(*f.view), 0u);
    QCOMPARE(f.view->document_width(), 15);
    QCOMPARE(f.view->history_count(), history);
}

void DocumentJobsTest::deferredTransformSettlesBeforeWorkerSnapshot()
{
    Fixture f(*window_, seed(kWide, kTall), QStringLiteral("transform-before-rotation"));
    QVERIFY(f.ok());
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(f.view->begin_free_transform(QStringLiteral("1")));
    QVERIFY(pictura::transform_set_angle(*f.view, 15.0));
    pictura::transform_set_delay(*f.view, 150);
    QVERIFY(f.view->commit_transform());
    QVERIFY(pictura::transform_pending(*f.view));
    const int before = f.view->history_count();
    pictura::document_job_arm(*f.view, true);
    const auto attempt = pictura::document_job_attempt(*f.view);
    QVERIFY(f.view->rotate_doc(1));
    pictura::document_job_arm(*f.view, false);
    const auto result = pictura::document_job_finish_for(*f.view, attempt);
    QCOMPARE(result.code, 0u);
    QVERIFY(result.applied);
    QCOMPARE(f.view->history_count(), before + 2);
    QCOMPARE(f.view->history_label(before), QStringLiteral("Free Transform"));
    QCOMPARE(f.view->document_width(), kTall);
}

PICTURA_QT_TEST_MAIN(DocumentJobsTest)
#include "tst_document_jobs.moc"
