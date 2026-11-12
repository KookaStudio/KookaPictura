// Whole-document commands as cancellable jobs (#1278): on a large document
// Rotate, Flatten and a depth conversion run on a worker behind the progress
// window, Cancel leaves the document and history unchanged, and a small
// document's command still applies at once.

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QPushButton>

#include <algorithm>

#include "commands.h"
#include "selftest_paint_fixture.h"

#include "api/document_bridge.h"

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
    void cancellingAFlattenChangesNothing();
    void aDepthConversionShowsABusyBar();
    void aSmallDocumentAppliesAtOnce();

private:
    bool dispatch(const char* id);
    // Run command `id`, watching the job window; with `cancel`, press its
    // Cancel button as soon as it shows.
    Seen run(const char* id, bool cancel = false);

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
    Seen seen;
    QTimer probe;
    probe.setInterval(20);
    connect(&probe, &QTimer::timeout, this, [&] {
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
    dispatch(id);
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

QTEST_MAIN(DocumentJobsTest)
#include "tst_document_jobs.moc"
