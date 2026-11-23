// Shared file-export batch lifecycle: preflight, ordering, progress,
// cancellation, view lifetime, and stop-on-failure.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "api/layer_comps_api.h"
#include "api/layers_bridge.h"
#include "export_batch.h"
#include "export_layers_dialog.h"
#include "frame.h"
#include "layer_comps_export.h"
#include "qt_test_support.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QVector>

#include <memory>

namespace {

pictura::PictureView* newDoc(pictura::PicturaMainWindow& window, const QString& name)
{
    if (!window.newDocument(name, 64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window.activeView();
}

QString addLayer(pictura::PictureView& view, const QString& name)
{
    const QString path = view.add_layer_in(QString());
    if (!path.isEmpty() && !name.isEmpty()) {
        view.set_layer_name_path(path, name);
    }
    return path;
}

int makeComp(pictura::PictureView& view, const QString& name)
{
    return pictura::layer_comps_api::create(view, name, QString(), 1);
}

QStringList files(const QString& folder)
{
    return QDir(folder).entryList(QDir::Files | QDir::NoDotAndDotDot);
}

bool writeMarker(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write("x") == 1;
}

pictura::ExportBatchSpec fakeBatch(const QString& destination, const QString& targetsText)
{
    pictura::ExportBatchSpec spec;
    spec.title = QStringLiteral("Export Batch");
    spec.destination = destination;
    spec.suffix = QStringLiteral(".psd");
    spec.targetsText = targetsText;
    spec.destinationFailureText = QStringLiteral("The destination folder is not writable.");
    spec.failureText = [](const pictura::ExportTarget& target) {
        return QStringLiteral("Could not write %1.").arg(target.path);
    };
    return spec;
}

} // namespace

class ExportBatchTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void badDestinationWritesNothing();
    void emptyTargetsWriteNothing();
    void directoryTargetWritesNothing();
    void writesTargetsInOrder();
    void writeFailureStopsBatch();
    void progressOnlyIncreases();
    void finishedBatchReportsCompletion();
    void cancelBetweenFiles();
    void closedDocumentStopsBatch();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ExportBatchTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ExportBatchTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void ExportBatchTest::badDestinationWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-bad-dest"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QVERIFY(!addLayer(*view, QStringLiteral("B")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString destination = dir.filePath(QStringLiteral("missing"));
    auto spec = pictura::layersExportBatchSpec(*view, nullptr, destination,
                                               QStringLiteral("x"), false,
                                               QStringLiteral("PSD"), false);
    int writes = 0;
    spec.write = [&writes](pictura::PictureView&, const pictura::ExportTarget&) {
        ++writes;
        return true;
    };
    pictura::ExportBatchRun run;
    QVERIFY(!pictura::exportBatch(spec, run));
    QCOMPARE(writes, 0);
    QCOMPARE(run.written, 0);
    QCOMPARE(spec.destinationFailureText,
             QStringLiteral("The destination folder is not writable."));
    const pictura::ExportTarget sample {QStringLiteral("0"), QStringLiteral("/tmp/out.psd")};
    QCOMPARE(spec.failureText(sample), QStringLiteral("Could not write layer 0 to /tmp/out.psd."));
}

void ExportBatchTest::emptyTargetsWriteNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-empty"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = pictura::layersExportBatchSpec(*view, nullptr, dir.path(), QStringLiteral("x"),
                                               false, QStringLiteral("PSD"), false);
    bool wrote = false;
    spec.write = [&wrote](pictura::PictureView&, const pictura::ExportTarget&) {
        wrote = true;
        return true;
    };
    spec.targetsText.clear();
    pictura::ExportBatchRun run;
    QVERIFY(!pictura::exportBatch(spec, run));
    QVERIFY(!wrote);
    QCOMPARE(run.total, 0);
    QCOMPARE(run.written, 0);
    QCOMPARE(run.progressPermille(), 1000);
    QVERIFY(files(dir.path()).isEmpty());
}

void ExportBatchTest::directoryTargetWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-dir-target"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = fakeBatch(dir.path(), QStringLiteral("k1\ta\nk2\tb\nk3\tc\n"));
    spec.view = view;
    QVERIFY(QDir(dir.path()).mkpath(QStringLiteral("b.psd")));
    QString failing;
    spec.failureText = [&failing](const pictura::ExportTarget& target) {
        failing = target.path;
        return QStringLiteral("Could not write %1.").arg(target.path);
    };
    int writes = 0;
    spec.write = [&writes](pictura::PictureView&, const pictura::ExportTarget&) {
        ++writes;
        return true;
    };
    pictura::ExportBatchRun run;
    QVERIFY(!pictura::exportBatch(spec, run));
    QCOMPARE(writes, 0);
    QCOMPARE(run.written, 0);
    QCOMPARE(failing, dir.path() + QStringLiteral("/b.psd"));
    QVERIFY(files(dir.path()).isEmpty());
}

void ExportBatchTest::writesTargetsInOrder()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-order"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = fakeBatch(dir.path(), QStringLiteral("k1\tone\nk2\ttwo\nk3\tthree\n"));
    spec.view = view;
    QStringList seen;
    spec.write = [&seen](pictura::PictureView&, const pictura::ExportTarget& target) {
        seen.append(target.path);
        return writeMarker(target.path);
    };
    pictura::ExportBatchRun run;
    QVERIFY(pictura::exportBatch(spec, run));
    QCOMPARE(seen, QStringList({dir.path() + QStringLiteral("/one.psd"),
                                dir.path() + QStringLiteral("/two.psd"),
                                dir.path() + QStringLiteral("/three.psd")}));
    QCOMPARE(files(dir.path()).size(), 3);
    QVERIFY(files(dir.path()).contains(QStringLiteral("one.psd")));
    QCOMPARE(run.written, 3);
}

void ExportBatchTest::writeFailureStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-write-fail"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QVERIFY(makeComp(*view, QStringLiteral("A")) >= 0);
    QVERIFY(makeComp(*view, QStringLiteral("B")) >= 0);
    QVERIFY(makeComp(*view, QStringLiteral("C")) >= 0);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = pictura::layerCompsExportBatchSpec(*view, nullptr, dir.path(),
                                                   QStringLiteral("x"), QStringLiteral("PSD"),
                                                   false);
    const auto productionFailure = spec.failureText;
    pictura::ExportTarget reported;
    bool reportedSeen = false;
    spec.failureText = [&productionFailure, &reported, &reportedSeen](
                           const pictura::ExportTarget& target) {
        reported = target;
        reportedSeen = true;
        return productionFailure(target);
    };
    QVector<pictura::ExportTarget> seen;
    spec.write = [&seen](pictura::PictureView&, const pictura::ExportTarget& target) {
        seen.append(target);
        return seen.size() == 1 ? writeMarker(target.path) : false;
    };
    pictura::ExportBatchRun run;
    QVERIFY(!pictura::exportBatch(spec, run));
    QCOMPARE(seen.size(), 2);
    QCOMPARE(run.written, 1);
    QVERIFY(run.failed);
    QVERIFY(!run.cancelled);
    QCOMPARE(files(dir.path()).size(), 1);
    QVERIFY(reportedSeen);
    QCOMPARE(reported.path, seen.at(1).path);
    QCOMPARE(productionFailure(reported), QStringLiteral("Could not write %1.").arg(reported.path));
}

void ExportBatchTest::progressOnlyIncreases()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-progress"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = fakeBatch(dir.path(), QStringLiteral("k1\ta\nk2\tb\nk3\tc\nk4\td\n"));
    spec.view = view;
    pictura::ExportBatchRun run;
    QVector<int> observed;
    spec.write = [&observed, &run](pictura::PictureView&, const pictura::ExportTarget& target) {
        observed.append(run.progressPermille());
        return writeMarker(target.path);
    };
    QVERIFY(pictura::exportBatch(spec, run));
    QCOMPARE(observed.size(), 4);
    QCOMPARE(observed.at(0), 0);
    QCOMPARE(observed.at(1), 250);
    QCOMPARE(observed.at(2), 500);
    QCOMPARE(observed.at(3), 750);
    for (int i = 1; i < observed.size(); ++i) {
        QVERIFY(observed.at(i) >= observed.at(i - 1));
    }
}

void ExportBatchTest::finishedBatchReportsCompletion()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-complete"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = fakeBatch(dir.path(), QStringLiteral("k1\ta\nk2\tb\nk3\tc\n"));
    spec.view = view;
    spec.write = [](pictura::PictureView&, const pictura::ExportTarget& target) {
        return writeMarker(target.path);
    };
    pictura::ExportBatchRun run;
    QVERIFY(pictura::exportBatch(spec, run));
    QCOMPARE(run.total, 3);
    QCOMPARE(run.written, 3);
    QCOMPARE(run.progressPermille(), 1000);
    QVERIFY(!run.cancelled);
    QVERIFY(!run.failed);
}

void ExportBatchTest::cancelBetweenFiles()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-cancel"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QVERIFY(makeComp(*view, QStringLiteral("A")) >= 0);
    QVERIFY(makeComp(*view, QStringLiteral("B")) >= 0);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = pictura::layerCompsExportBatchSpec(*view, nullptr, dir.path(),
                                                   QStringLiteral("x"), QStringLiteral("PSD"),
                                                   false);
    pictura::ExportBatchRun run;
    int writes = 0;
    spec.write = [&run, &writes](pictura::PictureView& targetView,
                                const pictura::ExportTarget& target) {
        ++writes;
        const bool ok = pictura::layer_comps_api::exportFile(
            targetView, target.key.toInt(), QStringLiteral("PSD"), false, target.path);
        if (ok && writes == 1) {
            run.cancel();
        }
        return ok;
    };
    QVERIFY(!pictura::exportBatch(spec, run));
    QCOMPARE(writes, 1);
    QCOMPARE(run.written, 1);
    QVERIFY(run.cancelled);
    QVERIFY(!run.failed);
    QCOMPARE(files(dir.path()).size(), 1);
}

void ExportBatchTest::closedDocumentStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("batch-close"));
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("A")).isEmpty());
    QVERIFY(makeComp(*view, QStringLiteral("A")) >= 0);
    QVERIFY(makeComp(*view, QStringLiteral("B")) >= 0);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto spec = pictura::layerCompsExportBatchSpec(*view, nullptr, dir.path(),
                                                   QStringLiteral("x"), QStringLiteral("PSD"),
                                                   false);
    const auto productionFailure = spec.failureText;
    pictura::ExportTarget reported;
    bool reportedSeen = false;
    spec.failureText = [&productionFailure, &reported, &reportedSeen](
                           const pictura::ExportTarget& target) {
        reported = target;
        reportedSeen = true;
        return productionFailure(target);
    };
    int writes = 0;
    spec.write = [this, &writes](pictura::PictureView& targetView,
                                 const pictura::ExportTarget& target) {
        ++writes;
        const bool ok = pictura::layer_comps_api::exportFile(
            targetView, target.key.toInt(), QStringLiteral("PSD"), false, target.path);
        if (ok && writes == 1) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
        return ok;
    };
    pictura::ExportBatchRun run;
    QVERIFY(!pictura::exportBatch(spec, run));
    QVERIFY(run.failed);
    QVERIFY(!run.cancelled);
    QCOMPARE(run.written, 1);
    QCOMPARE(writes, 1);
    QVERIFY(reportedSeen);
    QCOMPARE(reported.path, dir.path() + QStringLiteral("/xB.psd"));
    QCOMPARE(productionFailure(reported), QStringLiteral("Could not write %1.").arg(reported.path));
    QCOMPARE(files(dir.path()), QStringList {QStringLiteral("xA.psd")});
}

QTEST_MAIN(ExportBatchTest)

#include "tst_export_batch.moc"
