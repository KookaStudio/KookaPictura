#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QElapsedTimer>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QStatusBar>

#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/filters_bridge.h"
#include "api/view_bridge.h"
#include "dialogs.h"
#include "file_info_dialog.h"
#include "preferences_dialog.h"
#include "frame.h"
#include "qt_test_support.h"
#include "save_for_web_dialog.h"

namespace {

constexpr int metadata = 0;
constexpr int textFile = 1;
constexpr int both = 2;
constexpr int sessions = 0;
constexpr int concise = 1;
constexpr int detailed = 2;

QString packet(const QString& history, const QString& extra = {})
{
    return QStringLiteral(
               "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">"
               "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
               "<rdf:Description rdf:about=\"\" xmlns:p=\"http://ns.adobe.com/photoshop/1.0/\""
               " xmlns:fixture=\"urn:fixture\">%1%2</rdf:Description></rdf:RDF></x:xmpmeta>")
        .arg(history.isNull() ? QString() : QStringLiteral("<p:History>%1</p:History>").arg(history),
             extra);
}

bool setPacket(pictura::PictureView& view, const QString& value)
{
    return view.apply_metadata_edits({QStringLiteral("xmp-packet\t") + value});
}

QByteArray bytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool createDocument(pictura::PicturaMainWindow& window)
{
    return window.newDocument(QStringLiteral("History fixture"), 12, 12,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
}

bool resize(pictura::PictureView& view, int width, int height = 12)
{
    return view.resize_image(QStringLiteral("nearest"), width, height);
}

bool paint(pictura::PictureView& view, bool commit)
{
    if (!view.begin_paint(0xffff0000, 0xffffffff, 4, 100, 100, 0, 100, 100, 25,
                          QStringLiteral("normal"), false, false)) {
        return false;
    }
    const bool dab = view.paint_dab(6.0, 6.0, 1.0);
    if (!commit) {
        view.cancel_paint();
        return dab;
    }
    return dab && view.end_paint();
}

QString reopenedXmp(const QString& path)
{
    auto window = pictura::test::makeMainWindow();
    if (!window->openDocumentAtPath(path, false)) {
        return {};
    }
    return window->activeView()->xmp_packet();
}

struct SavePicker {
    explicit SavePicker(pictura::SaveFileNameProvider provider)
    {
        pictura::setSaveFileNameProviderForTest(std::move(provider));
    }
    ~SavePicker() { pictura::setSaveFileNameProviderForTest({}); }
};

struct LogFixture {
    pictura::test::ScopedStateHome state;
    QTemporaryDir directory;
    std::unique_ptr<pictura::PicturaMainWindow> window = pictura::test::makeMainWindow();

    ~LogFixture()
    {
        pictura::history_log_shutdown();
        pictura::history_log_configure(false, metadata, sessions, {});
    }

    QString path(const QString& name) const { return directory.filePath(name); }
    QString logPath() const { return path(QStringLiteral("history.txt")); }
    pictura::PictureView& view() const { return *window->activeView(); }
};

QImage preview()
{
    QImage picture(12, 12, QImage::Format_RGBA8888);
    picture.fill(Qt::white);
    return picture;
}

void webOptions(pictura::SaveForWebDialog& dialog, int format, int level)
{
    dialog.findChild<QComboBox*>(QStringLiteral("saveForWebFormat"))->setCurrentIndex(format);
    dialog.findChild<QComboBox*>(QStringLiteral("saveForWebMetadata"))->setCurrentIndex(level);
}

} // namespace

class HistoryLogTest : public QObject {
    Q_OBJECT

private slots:
    void preferencesPersistAndCancelCleanly();
    void disablingStopsNewRecords();
    void verbosityFollowsCommittedActivity();
    void capturesOnlyCommittedEdits();
    void routesSelectedDestinations();
    void failedTextSinkKeepsMetadata();
    void reportsTextQueueAndDrainsOnQuit();
    void reopensHistoryAcrossFormats();
    void saveSnapshotsKeepPendingRecords();
    void metadataAppendFailureKeepsImageSaveable();
    void fileInfoEditsDuringSave();
    void undoExplicitHistoryPreservesRestoredValue();
    void refusesImageAsTextLog();
    void webExportHonorsMetadataChoice();
    void webJpegOverflowKeepsSourceRecords();
};

void HistoryLogTest::undoExplicitHistoryPreservesRestoredValue()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("original"))));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 14));
    const auto path = fixture.path(QStringLiteral("undo.psd"));
    QVERIFY(fixture.view().save(path));
    pictura::history_log_configure(false, metadata, concise, {});
    QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("replacement"))));
    QVERIFY(fixture.view().undo());
    QVERIFY(fixture.view().save(path));
    const auto result = reopenedXmp(path);
    QVERIFY(result.contains(QStringLiteral("original")));
    QVERIFY(result.contains(QStringLiteral("Image Size")));
    QVERIFY(!result.contains(QStringLiteral("replacement")));
    QVERIFY(fixture.view().redo());
    QVERIFY(fixture.view().save(path));
    QVERIFY(reopenedXmp(path).contains(QStringLiteral("replacement")));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 15));
    pictura::save_set_delay(fixture.view(), 100);
    QVERIFY(pictura::save_start(fixture.view(), path));
    QVERIFY(fixture.view().undo());
    QVERIFY(fixture.view().undo());
    QVERIFY(pictura::save_finish(fixture.view()));
    QVERIFY(fixture.view().save(path));
    const auto restored = reopenedXmp(path);
    QVERIFY(restored.contains(QStringLiteral("original")));
    QVERIFY(!restored.contains(QStringLiteral("replacement")));
}

void HistoryLogTest::refusesImageAsTextLog()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    const auto image = fixture.path(QStringLiteral("image.psd"));
    QVERIFY(fixture.view().save(image));
    const auto original = bytes(image);
    const auto alias = fixture.path(QStringLiteral("alias.txt"));
    QVERIFY(QFile::link(image, alias));
    pictura::history_log_configure(true, both, concise, alias);
    QVERIFY(resize(fixture.view(), 14));
    pictura::history_log_shutdown();
    QCOMPARE(bytes(image), original);
    QVERIFY(fixture.view().save(image));
    const auto textPath = fixture.path(QStringLiteral("valid-log.psd"));
    QFile text(textPath);
    QVERIFY(text.open(QIODevice::WriteOnly));
    QCOMPARE(text.write("existing text\n"), qint64(14));
    text.close();
    const auto originalText = bytes(textPath);
    pictura::history_log_configure(true, both, concise, textPath);
    QVERIFY(!fixture.view().save(textPath));
    QCOMPARE(bytes(textPath), originalText);
}

void HistoryLogTest::preferencesPersistAndCancelCleanly()
{
    LogFixture fixture;
    fixture.window->showPreferences(pictura::PreferencesDialog::kGeneral);
    auto* dialog = fixture.window->preferencesDialog();
    auto* enable = dialog->findChild<QCheckBox*>(QStringLiteral("preferencesHistoryLog"));
    auto* destination = dialog->findChild<QComboBox*>(QStringLiteral("historyLogDestination"));
    auto* verbosity = dialog->findChild<QComboBox*>(QStringLiteral("historyLogVerbosity"));
    QVERIFY(enable && destination && verbosity);
    QVERIFY(!enable->isChecked());
    QCOMPARE(destination->currentIndex(), metadata);
    QCOMPARE(verbosity->currentIndex(), sessions);
    {
        SavePicker cancel([](const auto&, const auto&, const auto&, const auto&, auto*) {
            return QString();
        });
        destination->setCurrentIndex(both);
        QCOMPARE(destination->currentIndex(), metadata);
    }
    {
        SavePicker choose([&](const auto&, const auto&, const auto&, const auto&, auto*) {
            return fixture.logPath();
        });
        destination->setCurrentIndex(both);
    }
    verbosity->setCurrentIndex(detailed);
    enable->setChecked(true);
    fixture.window->saveSessionNow();
    fixture.window = pictura::test::makeMainWindow();
    fixture.window->showPreferences(pictura::PreferencesDialog::kGeneral);
    dialog = fixture.window->preferencesDialog();
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("preferencesHistoryLog"))->isChecked());
    QCOMPARE(dialog->findChild<QComboBox*>(QStringLiteral("historyLogDestination"))->currentIndex(), both);
    QCOMPARE(dialog->findChild<QComboBox*>(QStringLiteral("historyLogVerbosity"))->currentIndex(), detailed);
    QVERIFY(createDocument(*fixture.window));
    QVERIFY(resize(fixture.view(), 22));
    QTRY_VERIFY(bytes(fixture.logPath()).contains("Image Size"));
}

void HistoryLogTest::disablingStopsNewRecords()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("original history"))));
    pictura::history_log_configure(true, both, concise, fixture.logPath());
    QVERIFY(resize(fixture.view(), 14));
    pictura::history_log_configure(false, both, concise, fixture.logPath());
    QVERIFY(resize(fixture.view(), 15));
    const auto path = fixture.path(QStringLiteral("disabled.psd"));
    QVERIFY(fixture.view().save(path));
    const QByteArray output = bytes(path);
    QVERIFY(output.contains("original history"));
    QCOMPARE(output.count("Image Size"), 1);
    QTRY_COMPARE(bytes(fixture.logPath()).count("Image Size"), 1);
}

void HistoryLogTest::verbosityFollowsCommittedActivity()
{
    for (int level : {sessions, concise, detailed}) {
        LogFixture fixture;
        QVERIFY(createDocument(*fixture.window));
        const auto input = fixture.path(QStringLiteral("source.psd"));
        QVERIFY(fixture.view().save(input));
        QVERIFY(fixture.window->closeActiveDocument(false));
        pictura::history_log_configure(true, textFile, level, fixture.logPath());
        pictura::history_log_start();
        QVERIFY(fixture.window->openDocumentAtPath(input, false));
        QVERIFY(paint(fixture.view(), true));
        QVERIFY(pictura::filter_commit_start(fixture.view(), QStringLiteral("gaussian-blur"),
                                             QList<double>{2.5}, QStringLiteral("Filter: Gaussian Blur")));
        QVERIFY(pictura::filter_commit_finish(fixture.view()));
        QVERIFY(resize(fixture.view(), 19));
        QVERIFY(fixture.window->closeActiveDocument(false));
        pictura::history_log_quit();
        pictura::history_log_shutdown();
        const auto log = bytes(fixture.logPath());
        for (const auto* event : {"Start", "Open", "Close", "Quit"}) {
            QVERIFY2(log.contains(event), event);
        }
        QCOMPARE(log.count("Image Size"), level == sessions ? 0 : 1);
        QCOMPARE(log.count("Brush"), level == sessions ? 0 : 1);
        QCOMPARE(log.count("Filter: Gaussian Blur"), level == sessions ? 0 : 1);
        QCOMPARE(log.contains("filter.blur.gaussian.blur"), level == detailed);
        QCOMPARE(log.contains("image.image.size"), level == detailed);
        QCOMPARE(log.contains("width=19"), level == detailed);
    }
}

void HistoryLogTest::capturesOnlyCommittedEdits()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    pictura::history_log_configure(true, metadata, concise, {});
    const auto depth = fixture.view().history_depth();
    QVERIFY(paint(fixture.view(), false));
    QVERIFY(!resize(fixture.view(), 0));
    QCOMPARE(fixture.view().history_depth(), depth);
    QVERIFY(paint(fixture.view(), true));
    QCOMPARE(fixture.view().history_depth(), depth + 1);
    QVERIFY(fixture.view().undo());
    const auto path = fixture.path(QStringLiteral("undo.psd"));
    QVERIFY(fixture.view().save(path));
    QCOMPARE(bytes(path).count("Brush"), 1);
}

void HistoryLogTest::routesSelectedDestinations()
{
    for (int destination : {metadata, textFile, both}) {
        LogFixture fixture;
        QVERIFY(createDocument(*fixture.window));
        QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("existing XMP"))));
        QFile log(fixture.logPath());
        QVERIFY(log.open(QIODevice::WriteOnly));
        QCOMPARE(log.write("previous text\n"), 14);
        log.close();
        pictura::history_log_configure(true, destination, concise, fixture.logPath());
        QVERIFY(resize(fixture.view(), 14));
        const auto path = fixture.path(QStringLiteral("dest.psd"));
        QVERIFY(fixture.view().save(path));
        pictura::history_log_shutdown();
        const auto output = bytes(path);
        QVERIFY(output.contains("existing XMP"));
        QCOMPARE(output.contains("Image Size"), destination != textFile);
        QVERIFY(bytes(fixture.logPath()).startsWith("previous text\n"));
        QCOMPARE(bytes(fixture.logPath()).contains("Image Size"), destination != metadata);
    }
}

void HistoryLogTest::failedTextSinkKeepsMetadata()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    pictura::history_log_configure(true, both, concise,
                                  fixture.path(QStringLiteral("missing/history.txt")));
    QVERIFY(resize(fixture.view(), 14));
    QTRY_VERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("History Log")));
    const auto path = fixture.path(QStringLiteral("failure.psd"));
    QVERIFY(fixture.view().save(path));
    QVERIFY(bytes(path).contains("Image Size"));
}

void HistoryLogTest::reportsTextQueueAndDrainsOnQuit()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    pictura::history_log_configure(true, both, concise, fixture.logPath());
    pictura::history_log_stall_for_test(true);
    for (int i = 0; i < 300; ++i) {
        QVERIFY(resize(fixture.view(), 13 + i % 2));
    }
    QTRY_VERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("History Log")));
    const auto path = fixture.path(QStringLiteral("queue.psd"));
    QVERIFY(fixture.view().save(path));
    QVERIFY(bytes(path).contains("Image Size"));
    QElapsedTimer shutdown;
    shutdown.start();
    pictura::history_log_shutdown();
    QVERIFY(shutdown.elapsed() < 5000);
    pictura::history_log_configure(true, both, concise, fixture.path(QStringLiteral("healthy.txt")));
    QVERIFY(fixture.window->closeActiveDocument(false));
    pictura::history_log_quit();
    pictura::history_log_shutdown();
    const auto log = bytes(fixture.path(QStringLiteral("healthy.txt")));
    QVERIFY(log.indexOf("Close") >= 0);
    QVERIFY(log.indexOf("Quit") > log.indexOf("Close"));
}

void HistoryLogTest::reopensHistoryAcrossFormats()
{
    for (const auto& suffix : {QStringLiteral(".psd"), QStringLiteral(".psb"),
                               QStringLiteral(".jpg"), QStringLiteral(".tif")}) {
        LogFixture fixture;
        QVERIFY(createDocument(*fixture.window));
        QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("previous entries"),
            QStringLiteral("<fixture:other>preserved</fixture:other>"))));
        pictura::history_log_configure(true, metadata, concise, {});
        QVERIFY(resize(fixture.view(), 14));
        auto path = fixture.path(QStringLiteral("history") + suffix);
        QVERIFY(fixture.view().save(path));
        QVERIFY(fixture.window->closeActiveDocument(false));
        QVERIFY2(fixture.window->openDocumentAtPath(path, false), qPrintable(path));
        pictura::FileInfoDialog fileInfo(fixture.view().xmp_rows(),
                                         fixture.view().file_info_edit_fields(),
                                         fixture.view().xmp_packet());
        const auto fileInfoRows = fileInfo.rowsForTest(QStringLiteral("Photoshop"))
            .join(QLatin1Char('\n'));
        QVERIFY(fileInfoRows.contains(QStringLiteral("previous entries")));
        QVERIFY(fileInfoRows.contains(QStringLiteral("Image Size")));
        QVERIFY(resize(fixture.view(), 16));
        QVERIFY(fixture.view().save(path));
        const auto output = bytes(path);
        QCOMPARE(output.count("Image Size"), 2);
        QVERIFY(output.contains("<fixture:other>preserved</fixture:other>"));
    }
}

void HistoryLogTest::saveSnapshotsKeepPendingRecords()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 14));
    const auto path = fixture.path(QStringLiteral("source.psd"));
    pictura::save_set_delay(fixture.view(), 100);
    QVERIFY(pictura::save_start(fixture.view(), path));
    QVERIFY(resize(fixture.view(), 15));
    QVERIFY(pictura::save_finish(fixture.view()));
    QCOMPARE(bytes(path).count("Image Size"), 1);
    QVERIFY(!fixture.view().save(fixture.path(QStringLiteral("missing/fail.psd"))));
    auto copy = fixture.path(QStringLiteral("copy.psd"));
    QVERIFY(fixture.view().save_copy(copy));
    QCOMPARE(bytes(copy).count("Image Size"), 2);
    QVERIFY(fixture.view().save(path));
    QCOMPARE(bytes(path).count("Image Size"), 2);
}

void HistoryLogTest::metadataAppendFailureKeepsImageSaveable()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    const auto base = packet(QStringLiteral("previous"), QStringLiteral("<fixture:pad></fixture:pad>"));
    const auto pad = QString(65500 - base.toUtf8().size(), QLatin1Char('x'));
    QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("previous"),
                                           QStringLiteral("<fixture:pad>%1</fixture:pad>").arg(pad))));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 14));
    const auto jpeg = fixture.path(QStringLiteral("limited.jpg"));
    QVERIFY(fixture.view().save(jpeg));
    QVERIFY(bytes(jpeg).contains("previous"));
    QVERIFY(!bytes(jpeg).contains("Image Size"));
    const auto psd = fixture.path(QStringLiteral("retry.psd"));
    QVERIFY(fixture.view().save(psd));
    QCOMPARE(bytes(psd).count("Image Size"), 1);
    const auto structured = packet({}, QStringLiteral(
        "<p:History><rdf:Seq><rdf:li>old structured history</rdf:li></rdf:Seq></p:History>"));
    QVERIFY(setPacket(fixture.view(), structured));
    QVERIFY(resize(fixture.view(), 15));
    const auto unsupported = fixture.path(QStringLiteral("unsupported.psd"));
    QVERIFY(fixture.view().save(unsupported));
    QVERIFY(bytes(unsupported).contains("old structured history"));
    QVERIFY(!bytes(unsupported).contains("Image Size"));
}

void HistoryLogTest::fileInfoEditsDuringSave()
{
    for (int mode = 0; mode < 4; ++mode) {
        LogFixture fixture;
        QVERIFY(createDocument(*fixture.window));
        QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("baseline"))));
        pictura::history_log_configure(true, metadata, concise, {});
        QVERIFY(resize(fixture.view(), 14));
        const auto path = fixture.path(QStringLiteral("rebase.psd"));
        pictura::save_set_delay(fixture.view(), 100);
        QVERIFY(pictura::save_start(fixture.view(), path));
        const auto updated = QStringLiteral("<fixture:other>new value</fixture:other>");
        const auto desired = mode == 0 ? QStringLiteral("baseline")
                          : mode == 1 ? QStringLiteral("replacement") : QString();
        QVERIFY(setPacket(fixture.view(), packet(desired, updated)));
        if (mode == 3) {
            QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("baseline"), updated)));
        }
        QVERIFY(pictura::save_finish(fixture.view()));
        QVERIFY(resize(fixture.view(), 15));
        QVERIFY(fixture.view().save(path));
        const auto output = reopenedXmp(path);
        QCOMPARE(output.count(QStringLiteral("Image Size")), mode == 0 ? 2 : 1);
        QVERIFY(output.contains(QStringLiteral("new value")));
        QCOMPARE(output.contains(QStringLiteral("replacement")), mode == 1);
    }
}

void HistoryLogTest::webExportHonorsMetadataChoice()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 14));
    const auto source = fixture.path(QStringLiteral("web-source.psd"));
    QVERIFY(fixture.view().save(source));
    QVERIFY(resize(fixture.view(), 15));
    pictura::SaveForWebDialog dialog(preview(), pictura::web_metadata(fixture.view()));
    for (int format : {pictura::SaveForWebDialog::Jpeg, pictura::SaveForWebDialog::Png8,
                       pictura::SaveForWebDialog::Png24}) {
        for (int level = 0; level <= 4; ++level) {
            webOptions(dialog, format, level);
            auto encoded = dialog.encoded();
            QVERIFY(!encoded.isEmpty());
            QCOMPARE(encoded.count("Image Size"), level >= 3 ? 2 : 0);
        }
    }
    QVERIFY(fixture.view().save(source));
    QCOMPARE(bytes(source).count("Image Size"), 2);
}

void HistoryLogTest::webJpegOverflowKeepsSourceRecords()
{
    LogFixture fixture;
    QVERIFY(createDocument(*fixture.window));
    auto base = packet(QStringLiteral("previous"), QStringLiteral("<fixture:pad></fixture:pad>"));
    auto pad = QString(65500 - base.toUtf8().size(), QLatin1Char('x'));
    QVERIFY(setPacket(fixture.view(), packet(QStringLiteral("previous"),
                                           QStringLiteral("<fixture:pad>%1</fixture:pad>").arg(pad))));
    pictura::history_log_configure(true, metadata, concise, {});
    QVERIFY(resize(fixture.view(), 14));
    pictura::SaveForWebDialog dialog(preview(), pictura::web_metadata(fixture.view()));
    webOptions(dialog, pictura::SaveForWebDialog::Jpeg, 4);
    const auto jpeg = dialog.encoded();
    QVERIFY(jpeg.contains("previous"));
    QVERIFY(!jpeg.contains("Image Size"));
    QVERIFY(dialog.historyLogOmitted());
    const QVector<pictura::ExportSlice> slices{{0, 0, 12, 12, 1, 0}};
    auto cells = pictura::SaveForWebDialog::buildSliceCells(
        slices, QSize(12, 12), QSize(12, 12), QStringLiteral("slice"), QStringLiteral("jpg"));
    QVERIFY(!cells.isEmpty());
    QVERIFY(dialog.writeSlices(fixture.path(QStringLiteral("slices.html")), 0, 0, cells,
                              QStringLiteral("Slices")));
    const auto slice = bytes(fixture.path(QStringLiteral("images/") + cells.first().name));
    QVERIFY(slice.contains("previous"));
    QVERIFY(!slice.contains("Image Size"));
    QVERIFY(dialog.historyLogOmitted());
    const auto output = fixture.path(QStringLiteral("source.psd"));
    QVERIFY(fixture.view().save(output));
    QCOMPARE(bytes(output).count("Image Size"), 1);
}

QTEST_MAIN(HistoryLogTest)
#include "tst_history_log.moc"
