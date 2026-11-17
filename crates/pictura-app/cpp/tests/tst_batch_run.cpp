// File > Automate > Batch runs (#618; document/batch-processing): the
// per-input pipeline, the Open/Save As overrides, destinations, the error
// policy, cancellation, progress, and recorded Open steps. Inputs are small
// 8x4 PNGs; the test action rotates them 90° so a processed file is 4x8.

#include <QtTest/QtTest>

#include "batch_options.h"
#include "batch_runner.h"
#include "commands.h"
#include "frame.h"
#include "panels/actions_panel.h"
#include "panels/actions_tree_model.h"

#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QImageReader>
#include <QtWidgets/QProgressDialog>

#include <memory>

#include "qt_test_support.h"

namespace {

using Steps = QList<QPair<QString, QJsonObject>>;

QString writePng(const QString& path, int width, int height)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QImage image(width, height, QImage::Format_RGBA8888);
    image.fill(QColor(200, 40, 40));
    return image.save(path) ? path : QString();
}

QSize imageSize(const QString& path)
{
    return QImageReader(path).size();
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

// The first value of the first JPEG quantization table: lower is higher quality.
int firstQuantValue(const QString& path)
{
    const QByteArray data = fileBytes(path);
    for (int i = 0; i + 5 < data.size(); ++i) {
        if (uchar(data.at(i)) == 0xFF && uchar(data.at(i + 1)) == 0xDB) {
            return uchar(data.at(i + 5));
        }
    }
    return -1;
}

QPair<QString, QJsonObject> rotateStep()
{
    return {QStringLiteral("image.rotate90cw"), QJsonObject()};
}

QPair<QString, QJsonObject> openStep(const QString& path)
{
    return {QStringLiteral("file.open"), QJsonObject{{QStringLiteral("path"), path}}};
}

QPair<QString, QJsonObject> saveAsJpegStep(const QString& path, int quality)
{
    return {QStringLiteral("file.saveAs"),
            QJsonObject{{QStringLiteral("as"), QStringLiteral("JPG")},
                        {QStringLiteral("in"), path},
                        {QStringLiteral("options"),
                         QJsonObject{{QStringLiteral("quality"), quality}}}}};
}

} // namespace

class BatchRunTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void openedFilesSourceProcessesTabs();
    void destinationNoneLeavesDocumentsOpen();
    void saveAndCloseOverwritesInPlace();
    void folderDestinationWritesNamedFiles();
    void overrideOpenProcessesInputs();
    void withoutOverrideOpenTheRecordedFileIsOpened();
    void overrideSaveAsWritesToTheDestination();
    void overrideSaveAsWithSaveAndCloseKeepsTheInputFolder();
    void withoutOverrideSaveAsTheRecordedPathIsReused();
    void stopForErrorsHaltsOnTheFirstFailure();
    void logErrorsContinuesAndRecords();
    void unwritableLogFallsBackToStopping();
    void cancelStopsAfterTheCurrentStep();
    void progressAdvancesAndCancelStops();
    void loggedErrorsAreReportedAtTheEnd();
    void openWhileRecordingAppendsAnOpenStep();
    void playingAnOpenStepOpensTheFile();
    void openStepForAMissingFileFails();

private:
    QString path(const QString& name) const { return dir_->filePath(name); }
    QString addAction(const Steps& steps);
    pictura::BatchOptions folderOptions(const QString& action, const QString& folder) const;
    bool runBatch(const pictura::BatchOptions& options);
    void writeErrorInputs();
    int documentIndexForPath(const QString& file) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<QTemporaryDir> dir_;
    pictura::ActionsPanel* panel_ = nullptr;
    QString setId_;
    int actionSerial_ = 0;
    QStringList messages_;
};

void BatchRunTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    panel_ = window_->actionsPanel();
    QVERIFY(panel_);
    panel_->setPersistEnabledForTest(false);
    window_->batch().setMessageHandler(
        [this](const QString&, const QString& text) { messages_ << text; });
}

void BatchRunTest::init()
{
    dir_ = std::make_unique<QTemporaryDir>();
    QVERIFY(dir_->isValid());
    messages_.clear();
}

void BatchRunTest::cleanup()
{
    if (window_->batch().runner()) {
        window_->batch().runner()->cancel();
        QTRY_VERIFY(!window_->batch().running());
    }
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
}

QString BatchRunTest::addAction(const Steps& steps)
{
    pictura::ActionsEngineApi* engine = panel_->engineForTest();
    if (setId_.isEmpty()) {
        setId_ = engine->createSet(QStringLiteral("Batch Tests"));
    }
    const QString name = QStringLiteral("Batch %1").arg(++actionSerial_);
    const QString actionId = engine->createAction(setId_, name, 0, false, false, 0);
    if (actionId.isEmpty() || !engine->startRecording(actionId, -1)) {
        return QString();
    }
    for (const auto& step : steps) {
        engine->recordCommand(step.first, step.second);
    }
    engine->stopRecording();
    panel_->refresh();
    return name;
}

pictura::BatchOptions BatchRunTest::folderOptions(const QString& action,
                                                  const QString& folder) const
{
    pictura::BatchOptions options;
    options.set = QStringLiteral("Batch Tests");
    options.action = action;
    options.source = pictura::BatchOptions::Source::Folder;
    options.sourceFolder = folder;
    return options;
}

bool BatchRunTest::runBatch(const pictura::BatchOptions& options)
{
    if (!window_->batch().start(options)) {
        return false;
    }
    return QTest::qWaitFor([this] { return !window_->batch().running(); }, 30000);
}

void BatchRunTest::writeErrorInputs()
{
    QVERIFY(!writePng(path(QStringLiteral("in/a.png")), 8, 4).isEmpty());
    QFile broken(path(QStringLiteral("in/b.png")));
    QVERIFY(broken.open(QIODevice::WriteOnly));
    broken.write("not an image");
    broken.close();
    QVERIFY(!writePng(path(QStringLiteral("in/c.png")), 8, 4).isEmpty());
}

int BatchRunTest::documentIndexForPath(const QString& file) const
{
    for (int i = 0; i < window_->documentCount(); ++i) {
        if (QFileInfo(window_->documentPath(i)) == QFileInfo(file)) {
            return i;
        }
    }
    return -1;
}

void BatchRunTest::openedFilesSourceProcessesTabs()
{
    for (const char* name : {"One", "Two"}) {
        QVERIFY(window_->newDocument(QString::fromLatin1(name), 8, 4, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white")));
    }
    // A document opened by the run itself is not an input.
    const QString action = addAction({rotateStep()});
    pictura::BatchOptions options = folderOptions(action, QString());
    options.source = pictura::BatchOptions::Source::OpenedFiles;
    QVERIFY(runBatch(options));
    QCOMPARE(window_->documentCount(), 2);
    QCOMPARE(window_->viewAt(0)->document_width(), 4);
    QCOMPARE(window_->viewAt(1)->document_width(), 4);
    QCOMPARE(window_->batch().runner()->processedCount(), 2);
}

void BatchRunTest::destinationNoneLeavesDocumentsOpen()
{
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    const QString b = writePng(path(QStringLiteral("in/b.png")), 8, 4);
    const QByteArray before = fileBytes(a);
    QVERIFY(runBatch(folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")))));
    QCOMPARE(window_->documentCount(), 2);
    for (const QString& file : {a, b}) {
        const int index = documentIndexForPath(file);
        QVERIFY(index >= 0);
        QCOMPARE(window_->viewAt(index)->document_width(), 4);
        QVERIFY(window_->isDocumentDirty(index));
    }
    QCOMPARE(fileBytes(a), before);
    QCOMPARE(imageSize(b), QSize(8, 4));
}

void BatchRunTest::saveAndCloseOverwritesInPlace()
{
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    const QString b = writePng(path(QStringLiteral("in/b.png")), 8, 4);
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(a), QSize(4, 8));
    QCOMPARE(imageSize(b), QSize(4, 8));
    QCOMPARE(window_->documentCount(), 0);
}

void BatchRunTest::folderDestinationWritesNamedFiles()
{
    writePng(path(QStringLiteral("in/a.png")), 8, 4);
    writePng(path(QStringLiteral("in/b.png")), 8, 4);
    QVERIFY(QDir().mkpath(path(QStringLiteral("out"))));
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::Folder;
    options.destinationFolder = path(QStringLiteral("out"));
    options.namingFields = {QStringLiteral("Document Name"),
                            QStringLiteral("2 Digit Serial Number")};
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(path(QStringLiteral("out/a01.png"))), QSize(4, 8));
    QCOMPARE(imageSize(path(QStringLiteral("out/b02.png"))), QSize(4, 8));
    QCOMPARE(window_->documentCount(), 0);
}

void BatchRunTest::overrideOpenProcessesInputs()
{
    const QString baked = writePng(path(QStringLiteral("x.png")), 6, 2);
    const QByteArray bakedBefore = fileBytes(baked);
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    const QString b = writePng(path(QStringLiteral("in/b.png")), 8, 4);
    pictura::BatchOptions options =
        folderOptions(addAction({openStep(baked), rotateStep()}), path(QStringLiteral("in")));
    options.overrideOpen = true;
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(a), QSize(4, 8));
    QCOMPARE(imageSize(b), QSize(4, 8));
    QCOMPARE(fileBytes(baked), bakedBefore);
    QCOMPARE(window_->documentCount(), 0);
}

void BatchRunTest::withoutOverrideOpenTheRecordedFileIsOpened()
{
    const QString baked = writePng(path(QStringLiteral("x.png")), 6, 2);
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    QVERIFY(runBatch(
        folderOptions(addAction({openStep(baked), rotateStep()}), path(QStringLiteral("in")))));
    const int bakedIndex = documentIndexForPath(baked);
    const int inputIndex = documentIndexForPath(a);
    QVERIFY(bakedIndex >= 0);
    QVERIFY(inputIndex >= 0);
    QCOMPARE(window_->viewAt(bakedIndex)->document_width(), 2);
    QCOMPARE(window_->viewAt(inputIndex)->document_width(), 8);
}

void BatchRunTest::overrideSaveAsWritesToTheDestination()
{
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    writePng(path(QStringLiteral("in/b.png")), 8, 4);
    QVERIFY(QDir().mkpath(path(QStringLiteral("out"))));
    const QString recorded = path(QStringLiteral("recorded.jpg"));
    pictura::BatchOptions options = folderOptions(
        addAction({rotateStep(), saveAsJpegStep(recorded, 3)}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::Folder;
    options.destinationFolder = path(QStringLiteral("out"));
    options.overrideSave = true;
    options.namingFields = {QStringLiteral("Document Name"), QStringLiteral("extension")};
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(path(QStringLiteral("out/a.jpg"))), QSize(4, 8));
    QCOMPARE(imageSize(path(QStringLiteral("out/b.jpg"))), QSize(4, 8));
    QVERIFY2(!QFileInfo::exists(recorded), "the step's recorded path is not written");
    QCOMPARE(window_->documentCount(), 0);

    // The step's quality 3 is coarser than a quality-12 save of the same input.
    QVERIFY(window_->openDocumentAtPath(a));
    const QString reference = path(QStringLiteral("reference.jpg"));
    pictura::CommandParams params;
    params.values = saveAsJpegStep(reference, 12).second;
    QVERIFY(window_->registry()->executeRecorded(QStringLiteral("file.saveAs"), params));
    QVERIFY(firstQuantValue(path(QStringLiteral("out/a.jpg")))
            > firstQuantValue(reference));
}

void BatchRunTest::overrideSaveAsWithSaveAndCloseKeepsTheInputFolder()
{
    const QString a = writePng(path(QStringLiteral("in/a.png")), 8, 4);
    pictura::BatchOptions options = folderOptions(
        addAction({rotateStep(), saveAsJpegStep(path(QStringLiteral("recorded.jpg")), 3)}),
        path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    options.overrideSave = true;
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(path(QStringLiteral("in/a.jpg"))), QSize(4, 8));
    QVERIFY(!QFileInfo::exists(path(QStringLiteral("recorded.jpg"))));
    QCOMPARE(window_->documentCount(), 0);
    Q_UNUSED(a)
}

void BatchRunTest::withoutOverrideSaveAsTheRecordedPathIsReused()
{
    writePng(path(QStringLiteral("in/a.png")), 8, 4);
    writePng(path(QStringLiteral("in/b.png")), 10, 4);
    const QString recorded = path(QStringLiteral("recorded.jpg"));
    QVERIFY(runBatch(folderOptions(addAction({rotateStep(), saveAsJpegStep(recorded, 8)}),
                                   path(QStringLiteral("in")))));
    QCOMPARE(imageSize(recorded), QSize(4, 10));
}

void BatchRunTest::stopForErrorsHaltsOnTheFirstFailure()
{
    writeErrorInputs();
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(path(QStringLiteral("in/a.png"))), QSize(4, 8));
    QCOMPARE(imageSize(path(QStringLiteral("in/c.png"))), QSize(8, 4));
    QVERIFY(window_->batch().runner()->stopError().contains(QStringLiteral("b.png")));
    QVERIFY(!messages_.isEmpty());
    QVERIFY(messages_.last().contains(QStringLiteral("b.png")));
    QCOMPARE(documentIndexForPath(path(QStringLiteral("in/c.png"))), -1);
}

void BatchRunTest::logErrorsContinuesAndRecords()
{
    writeErrorInputs();
    const QString log = path(QStringLiteral("errors.txt"));
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    options.errors = pictura::BatchOptions::Errors::Log;
    options.errorLog = log;
    QVERIFY(runBatch(options));
    QCOMPARE(imageSize(path(QStringLiteral("in/a.png"))), QSize(4, 8));
    QCOMPARE(imageSize(path(QStringLiteral("in/c.png"))), QSize(4, 8));
    const QStringList lines =
        QString::fromUtf8(fileBytes(log)).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    QCOMPARE(lines.size(), 1);
    QVERIFY(lines.first().startsWith(path(QStringLiteral("in/b.png")) + QStringLiteral(": ")));
    QCOMPARE(window_->batch().runner()->errorCount(), 1);
    QCOMPARE(window_->documentCount(), 0);
}

void BatchRunTest::unwritableLogFallsBackToStopping()
{
    writeErrorInputs();
    const QString log = path(QStringLiteral("missing/errors.txt"));
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    options.errors = pictura::BatchOptions::Errors::Log;
    options.errorLog = log;
    QVERIFY(runBatch(options));
    QVERIFY(!messages_.isEmpty());
    QVERIFY(messages_.first().contains(QDir::toNativeSeparators(log)));
    QCOMPARE(imageSize(path(QStringLiteral("in/c.png"))), QSize(8, 4));
    QVERIFY(!window_->batch().runner()->stopError().isEmpty());
}

void BatchRunTest::cancelStopsAfterTheCurrentStep()
{
    for (const char* name : {"a", "b", "c"}) {
        writePng(path(QStringLiteral("in/%1.png").arg(QLatin1String(name))), 8, 4);
    }
    QVERIFY(QDir().mkpath(path(QStringLiteral("out"))));
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep(), rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::Folder;
    options.destinationFolder = path(QStringLiteral("out"));
    QVERIFY(window_->batch().start(options));
    int opened = 0;
    // Connected after the runner's own open handler, which runs first and
    // starts the action; this one then cancels while the second input's action
    // is playing.
    const auto connection =
        connect(window_.get(), &pictura::PicturaMainWindow::documentOpenFinished, this,
                [this, &opened] {
                    if (++opened == 2) {
                        // Cancel arrives from the event loop, as from the
                        // progress dialog's button.
                        QTimer::singleShot(0, this, [this] {
                            QVERIFY(panel_->isPlaying());
                            window_->batch().runner()->cancel();
                        });
                    }
                });
    QVERIFY(QTest::qWaitFor([this] { return !window_->batch().running(); }, 30000));
    disconnect(connection);
    QVERIFY(window_->batch().runner()->wasCancelled());
    QCOMPARE(QDir(path(QStringLiteral("out"))).entryList(QDir::Files),
             QStringList{QStringLiteral("a.png")});
    QCOMPARE(window_->documentCount(), 0);
}

void BatchRunTest::progressAdvancesAndCancelStops()
{
    for (const char* name : {"a", "b", "c"}) {
        writePng(path(QStringLiteral("in/%1.png").arg(QLatin1String(name))), 8, 4);
    }
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.destination = pictura::BatchOptions::Destination::SaveAndClose;
    QVERIFY(window_->batch().start(options));
    pictura::BatchRunner* runner = window_->batch().runner();
    QVERIFY(runner && runner->progressDialog());
    QStringList labels;
    connect(runner, &pictura::BatchRunner::progressChanged, this,
            [&labels, runner](int index, int, const QString&) {
                labels << runner->progressDialog()->labelText();
                if (index == 1) {
                    // What the Cancel button sends.
                    emit runner->progressDialog()->canceled();
                }
            });
    QVERIFY(QTest::qWaitFor([this] { return !window_->batch().running(); }, 30000));
    QCOMPARE(labels.size(), 2);
    QVERIFY(labels.at(0).contains(QStringLiteral("1 of 3")));
    QVERIFY(labels.at(1).contains(QStringLiteral("2 of 3")));
    QCOMPARE(imageSize(path(QStringLiteral("in/a.png"))), QSize(4, 8));
    QCOMPARE(imageSize(path(QStringLiteral("in/c.png"))), QSize(8, 4));
}

void BatchRunTest::loggedErrorsAreReportedAtTheEnd()
{
    writeErrorInputs();
    const QString log = path(QStringLiteral("errors.txt"));
    pictura::BatchOptions options =
        folderOptions(addAction({rotateStep()}), path(QStringLiteral("in")));
    options.errors = pictura::BatchOptions::Errors::Log;
    options.errorLog = log;
    QVERIFY(runBatch(options));
    QCOMPARE(messages_.size(), 1);
    QVERIFY(messages_.first().contains(QStringLiteral("1 error")));
    QVERIFY(messages_.first().contains(QDir::toNativeSeparators(log)));
}

void BatchRunTest::openWhileRecordingAppendsAnOpenStep()
{
    const QString a = writePng(path(QStringLiteral("a.png")), 8, 4);
    QList<QPair<QString, QJsonObject>> recorded;
    window_->registry()->setRecordingSink(
        [&recorded](const QString& id, const pictura::CommandParams& params) {
            recorded << qMakePair(id, params.values);
        });
    window_->openUserPaths({a});
    window_->waitForOpensForTest();
    window_->registry()->setRecordingSink(pictura::CommandRecordingSink());
    QCOMPARE(recorded.size(), 1);
    QCOMPARE(recorded.first().first, QStringLiteral("file.open"));
    QCOMPARE(recorded.first().second.value(QStringLiteral("path")).toString(),
             QFileInfo(a).absoluteFilePath());
}

void BatchRunTest::playingAnOpenStepOpensTheFile()
{
    const QString a = writePng(path(QStringLiteral("a.png")), 8, 4);
    QSignalSpy finished(panel_, &pictura::ActionsPanel::playbackFinished);
    QVERIFY(panel_->playActionForBatch(QStringLiteral("Batch Tests"),
                                       addAction({openStep(a), rotateStep()}),
                                       pictura::ActionsPanel::BatchPlayback{}));
    QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 20000);
    QVERIFY(finished.first().at(0).toBool());
    QCOMPARE(QFileInfo(window_->activeFilePath()), QFileInfo(a));
    QCOMPARE(window_->activeView()->document_width(), 4);
}

void BatchRunTest::openStepForAMissingFileFails()
{
    QSignalSpy finished(panel_, &pictura::ActionsPanel::playbackFinished);
    QVERIFY(panel_->playActionForBatch(
        QStringLiteral("Batch Tests"),
        addAction({openStep(path(QStringLiteral("gone.png"))), rotateStep()}),
        pictura::ActionsPanel::BatchPlayback{}));
    QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 20000);
    QVERIFY(!finished.first().at(0).toBool());
    QVERIFY(finished.first().at(1).toString().contains(QStringLiteral("file.open")));
    QCOMPARE(window_->documentCount(), 0);
}

QTEST_MAIN(BatchRunTest)
#include "tst_batch_run.moc"
