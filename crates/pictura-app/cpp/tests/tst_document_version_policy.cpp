#include <QtTest/QtTest>

#include "api/document_bridge.h"
#include "commands.h"
#include "control_server.h"
#include "control_test_client.h"
#include "dialogs.h"
#include "file_info_dialog.h"
#include "frame.h"
#include "qt_test_support.h"

#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QtEndian>
#include <QtGui/QImage>
#include <QtWidgets/QStatusBar>

#include <filesystem>

namespace {

QByteArray readFile(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool writeFile(const QString& path, const QByteArray& bytes)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

template <class T>
QByteArray be(T value)
{
    char bytes[sizeof(T)];
    qToBigEndian(value, bytes);
    return QByteArray(bytes, sizeof(T));
}

QByteArray resource(quint16 id, const QByteArray& name, QByteArray data)
{
    QByteArray bytes = "8BIM" + be(id);
    bytes.append(char(name.size()));
    bytes += name;
    if ((name.size() + 1) % 2) {
        bytes.append('\0');
    }
    bytes += be(quint32(data.size()));
    bytes += data;
    if (data.size() % 2) {
        bytes.append('\0');
    }
    return bytes;
}

QByteArray stamp(quint16 version = 2, quint16 reader = 2)
{
    return resource(4973, "KookaPicturaDocumentVersion",
                    "KPVR" + be(quint16(1)) + be(version) + be(reader) + be(quint16(4)) + "test");
}

QByteArray versionInfo(bool merged = true)
{
    QByteArray body = be(quint32(1)) + char(merged);
    for (const QString& text : {QStringLiteral("Adobe Photoshop"), QStringLiteral("Reader")}) {
        body += be(quint32(text.size() + 1));
        for (const QChar unit : text) {
            body += be(quint16(unit.unicode()));
        }
        body += be(quint16(0));
    }
    return resource(1057, {}, body + be(quint32(73)));
}

qsizetype layerSectionOffset(const QByteArray& file)
{
    const qsizetype colorEnd = 30 + qFromBigEndian<quint32>(file.constData() + 26);
    return colorEnd + 4 + qFromBigEndian<quint32>(file.constData() + colorEnd);
}

QByteArray withResources(QByteArray file, const QByteArray& resources)
{
    const qsizetype colorEnd = 30 + qFromBigEndian<quint32>(file.constData() + 26);
    return file.replace(colorEnd, layerSectionOffset(file) - colorEnd,
                        be(quint32(resources.size())) + resources);
}

qsizetype firstLayerBlend(const QByteArray& file)
{
    const qsizetype record = layerSectionOffset(file) + 10;
    const auto channels = qFromBigEndian<quint16>(file.constData() + record + 16);
    return record + 18 + 6 * channels;
}

QByteArray withLayerTag(QByteArray file, const QByteArray& key)
{
    const qsizetype section = layerSectionOffset(file);
    const qsizetype extra = firstLayerBlend(file) + 12;
    const auto extraLength = qFromBigEndian<quint32>(file.constData() + extra);
    const QByteArray block = "8BIM" + key + be(quint32(0));
    file.insert(extra + 4 + extraLength, block);
    for (qsizetype offset : {section, section + 4, extra}) {
        const auto old = qFromBigEndian<quint32>(file.constData() + offset);
        file.replace(offset, 4, be(quint32(old + block.size())));
    }
    return file;
}

} // namespace

class DocumentVersionPolicyTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void reportsUnsupportedNativeVersion();
    void warnsWhenVersionMirrorCannotBeWritten();
    void opensLegacyAndCurrentFilesForEditing();
    void keepsCompatibleFutureStampOnSave();
    void restrictsUnsupportedPrivateVersions();
    void editsFileWithAdvisoryWriterMetadata();
    void reportsUnsupportedPhotoshopFeatures();
    void showsVersionInfoInFileInfo();
    void protectsSourceAcrossOutputPaths();
    void genericExportRejectsReadOnly();
    void revertHistoryPreservesCompatibility();
    void reportsSourceDecodeFailure();
    void restrictsAmbiguousVersionInfo();
    void restrictsIncompleteResources();
    void readOnlyRejectsEditsAndSaves();
    void usesOriginalCompositeForCompatibility();
    void refreshesAccessOnReplacement();
    void writesFlattenedCompatibilityCopy();
    void refusesUnsafeCompatibilityCopies();

private:
    QString install(const QByteArray& resources, const QString& name = QStringLiteral("source.psd"));
    void chooseCopy(const QString& path, bool confirm = true);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<QTemporaryDir> dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QByteArray baseline_;
};

void DocumentVersionPolicyTest::init()
{
    dir_ = std::make_unique<QTemporaryDir>();
    QVERIFY(dir_->isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_->newDocument(QStringLiteral("Original"), 4, 4, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const auto baseline = dir_->filePath(QStringLiteral("baseline.psd"));
    QVERIFY(window_->saveActiveAs(baseline));
    baseline_ = readFile(baseline);
    QVERIFY(baseline_.size() > 30);
}

void DocumentVersionPolicyTest::cleanup()
{
    pictura::setSaveFileNameProviderForTest({});
    window_.reset();
}

QString DocumentVersionPolicyTest::install(const QByteArray& resources, const QString& name)
{
    const auto path = dir_->filePath(name);
    if (!writeFile(path, withResources(baseline_, resources)) || !window_->openDocumentAtPath(path)) {
        return {};
    }
    return path;
}

void DocumentVersionPolicyTest::chooseCopy(const QString& path, bool confirm)
{
    window_->setWarningDriverForTest([confirm](const QString&, const QString&) { return confirm; });
    pictura::setSaveFileNameProviderForTest(
        [path](QWidget*, const QString&, const QString&, const QStringList&, QString*) { return path; });
}

void DocumentVersionPolicyTest::reportsUnsupportedNativeVersion()
{
    QVERIFY(!install({}, QStringLiteral("psd-with-psb-suffix.psb")).isEmpty());
    const auto psb = dir_->filePath(QStringLiteral("psb-with-psd-suffix.psb"));
    QVERIFY(window_->saveActiveAs(psb));
    const auto swapped = dir_->filePath(QStringLiteral("psb-with-psd-suffix.psd"));
    QVERIFY(QFile::copy(psb, swapped));
    QVERIFY(window_->openDocumentAtPath(swapped));
    auto malformed = baseline_;
    malformed[5] = '\3';
    const auto rejected = dir_->filePath(QStringLiteral("unsupported.psd"));
    QVERIFY(writeFile(rejected, malformed));
    QVERIFY(!window_->openDocumentAtPath(rejected));
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("PSD version 3")));
}

void DocumentVersionPolicyTest::warnsWhenVersionMirrorCannotBeWritten()
{
    const auto path = install(resource(1060, {}, "<malformed"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->saveActive());
    const auto bytes = readFile(path);
    QVERIFY(bytes.contains("<malformed"));
    QVERIFY(bytes.contains("KookaPicturaDocumentVersion"));
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("XMP")));
}

void DocumentVersionPolicyTest::opensLegacyAndCurrentFilesForEditing()
{
    QVERIFY(!install({}).isEmpty());
    QVERIFY(pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(window_->activeView()->set_layer_opacity(0, 64));
    QVERIFY(!install(stamp(1, 1), QStringLiteral("current.psd")).isEmpty());
    QVERIFY(pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(window_->activeView()->set_layer_opacity(0, 64));
}

void DocumentVersionPolicyTest::keepsCompatibleFutureStampOnSave()
{
    const auto marker = resource(4000, "Inert plug-in metadata", "opaque-data");
    const auto path = install(stamp(2, 1) + marker);
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->activeView()->open_notice().contains(QStringLiteral("newer")));
    QVERIFY(window_->activeView()->set_layer_opacity(0, 64));
    QVERIFY(window_->saveActive());
    const auto saved = readFile(path);
    QVERIFY(saved.contains(stamp(2, 1)));
    QVERIFY(saved.contains(marker));
}

void DocumentVersionPolicyTest::restrictsUnsupportedPrivateVersions()
{
    const auto malformed = resource(4973, "KookaPicturaDocumentVersion", "bad");
    const auto conflict = resource(1060, {}, R"(<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:p="https://kookastudio.github.io/KookaPictura/ns/document-version/1.0/" p:opDocumentVersion="2" p:minReaderVersion="1"/></rdf:RDF>)");
    const QList<QByteArray> cases{stamp(), malformed, stamp(1, 1) + stamp(1, 1),
                                  stamp(1, 1) + conflict, conflict};
    for (qsizetype i = 0; i < cases.size(); ++i) {
        QVERIFY(!install(cases.at(i), QStringLiteral("private%1.psd").arg(i)).isEmpty());
        QVERIFY(!pictura::compatibility_reason(*window_->activeView()).isEmpty());
    }
}

void DocumentVersionPolicyTest::editsFileWithAdvisoryWriterMetadata()
{
    const auto exif = resource(1058, {}, "opaque-exif");
    const auto path = install(versionInfo() + exif);
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(window_->activeView()->set_layer_opacity(0, 128));
    QVERIFY(window_->saveActive());
    const auto saved = readFile(path);
    QVERIFY(saved.contains(versionInfo()));
    QVERIFY(saved.contains(exif));
}

void DocumentVersionPolicyTest::reportsUnsupportedPhotoshopFeatures()
{
    const QByteArray base = withResources(baseline_, versionInfo());
    const auto section = layerSectionOffset(base);
    const auto length = qFromBigEndian<quint32>(base.constData() + section);
    for (const QByteArray& key : {QByteArray("artb"), QByteArray("lnkE"), QByteArray("Dvpt")}) {
        auto file = base;
        const auto extra = "8BIM" + key + be(quint32(4)) + QByteArray(4, '\0');
        file.insert(section + 4 + length, extra);
        file.replace(section, 4, be(quint32(length + extra.size())));
        const auto path = dir_->filePath(QString::fromLatin1(key) + QStringLiteral(".psd"));
        QVERIFY(writeFile(path, file));
        QVERIFY(window_->openDocumentAtPath(path));
        QVERIFY(!pictura::compatibility_reason(*window_->activeView()).isEmpty());
        QVERIFY(!window_->activeView()->rotate_doc(1));
    }
    for (const QByteArray& key : {QByteArray("brit"), QByteArray("Dvlr"), QByteArray("vscg")}) {
        const auto path = dir_->filePath(QString::fromLatin1(key) + QStringLiteral(".psd"));
        QVERIFY(writeFile(path, withLayerTag(base, key)));
        QVERIFY(window_->openDocumentAtPath(path));
        auto* view = window_->activeView();
        QVERIFY(!pictura::compatibility_reason(*view).isEmpty());
        QVERIFY(!view->rotate_doc(1));
        QVERIFY(!view->set_layer_opacity(0, 128));
    }
    QVERIFY(!install(resource(1058, {}, "inert-exif")).isEmpty());
    QVERIFY(pictura::compatibility_reason(*window_->activeView()).isEmpty());
}

void DocumentVersionPolicyTest::showsVersionInfoInFileInfo()
{
    QVERIFY(!install(versionInfo()).isEmpty());
    auto* view = window_->activeView();
    pictura::FileInfoDialog dialog(view->xmp_rows(), view->file_info_edit_fields(), view->xmp_packet());
    const auto rows = dialog.rowsForTest(QStringLiteral("Photoshop")).join(u'\n');
    QVERIFY(rows.contains(QStringLiteral("Adobe Photoshop")));
    QVERIFY(rows.contains(QStringLiteral("Reader")));
    QVERIFY(rows.contains(QStringLiteral("73")));
}

void DocumentVersionPolicyTest::protectsSourceAcrossOutputPaths()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    const auto before = readFile(path);
    QVERIFY(window_->newDocument(QStringLiteral("Other tab"), 4, 4, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::ControlServer server(window_.get(), dir_->filePath(QStringLiteral("control.sock")));
    const auto exportPng = [&](const QString& destination) {
        return pictura::test::succeeded(server.dispatch(QStringLiteral("export_png"),
                                                        {{QStringLiteral("path"), destination}}));
    };
    QVERIFY(!exportPng(path));
    const auto alias = dir_->filePath(QStringLiteral("alias.png"));
    std::error_code linkError;
    std::filesystem::create_hard_link(path.toStdString(), alias.toStdString(), linkError);
    QVERIFY(!linkError);
    QVERIFY(!exportPng(alias));
    QVERIFY(!window_->saveActiveAs(path));
    QCOMPARE(readFile(path), before);
    QVERIFY(exportPng(dir_->filePath(QStringLiteral("other.png"))));
}

void DocumentVersionPolicyTest::genericExportRejectsReadOnly()
{
    QVERIFY(!install(stamp() + versionInfo()).isEmpty());
    pictura::ControlServer server(window_.get(), dir_->filePath(QStringLiteral("control.sock")));
    const auto destination = dir_->filePath(QStringLiteral("generic.png"));
    QVERIFY(!pictura::test::succeeded(server.dispatch(QStringLiteral("export_png"),
                                                      {{QStringLiteral("path"), destination}})));
    QVERIFY(!QFileInfo::exists(destination));
}

void DocumentVersionPolicyTest::revertHistoryPreservesCompatibility()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    QVERIFY(writeFile(path, withResources(baseline_, {})));
    QVERIFY(window_->revertActive());
    auto* view = window_->activeView();
    QVERIFY(view->undo());
    QVERIFY(!pictura::compatibility_reason(*view).isEmpty());
    QVERIFY(!pictura::compatibility_preview_available(*view));
    QVERIFY(view->redo());
    QVERIFY(pictura::compatibility_reason(*view).isEmpty());
    QVERIFY(view->set_layer_opacity(0, 100));
    QVERIFY(writeFile(path, withResources(baseline_, stamp() + versionInfo())));
    QVERIFY(window_->revertActive());
    QVERIFY(!pictura::compatibility_reason(*view).isEmpty());
    QVERIFY(view->undo());
    QVERIFY(pictura::compatibility_reason(*view).isEmpty());
    QVERIFY(view->redo());
    QVERIFY(!pictura::compatibility_reason(*view).isEmpty());
}

void DocumentVersionPolicyTest::reportsSourceDecodeFailure()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    QVERIFY(writeFile(path, "changed"));
    const auto copy = dir_->filePath(QStringLiteral("copy.png"));
    QVERIFY(writeFile(copy, "previous"));
    chooseCopy(copy);
    QVERIFY(!window_->saveCopyWithDialog());
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("changed since")));
    QCOMPARE(readFile(copy), QByteArray("previous"));
}

void DocumentVersionPolicyTest::restrictsAmbiguousVersionInfo()
{
    const QList<QByteArray> cases{versionInfo() + versionInfo(false),
                                  resource(1057, {}, be(quint32(2))), resource(1057, {}, "bad")};
    for (qsizetype i = 0; i < cases.size(); ++i) {
        QVERIFY(!install(cases.at(i), QStringLiteral("info%1.psd").arg(i)).isEmpty());
        QVERIFY(pictura::compatibility_reason(*window_->activeView()).contains(QStringLiteral("1057")));
    }
}

void DocumentVersionPolicyTest::restrictsIncompleteResources()
{
    const QList<QByteArray> cases{stamp().chopped(2), "????" + stamp(), stamp(1, 1) + "tail"};
    for (qsizetype i = 0; i < cases.size(); ++i) {
        QVERIFY(!install(cases.at(i), QStringLiteral("incomplete%1.psd").arg(i)).isEmpty());
        QVERIFY(pictura::compatibility_reason(*window_->activeView()).contains(QStringLiteral("incomplete")));
    }
}

void DocumentVersionPolicyTest::readOnlyRejectsEditsAndSaves()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    auto* view = window_->activeView();
    const auto before = readFile(path);
    const auto history = view->history_count();
    QVERIFY(!view->set_layer_opacity(0, 128));
    QVERIFY(!view->begin_paint(0xffff0000, 0xffffffff, 8, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false));
    QVERIFY(!view->rotate_doc(1));
    auto* registry = window_->registry();
    QVERIFY(registry);
    QVERIFY(!registry->executeRecorded(QString::fromLatin1(pictura::command_ids::ImageRotate90Cw),
                                       pictura::CommandParams{}));
    pictura::ControlServer server(window_.get(), dir_->filePath(QStringLiteral("control.sock")));
    QVERIFY(!pictura::test::succeeded(server.dispatch(QStringLiteral("adjust_apply"),
                                    {{QStringLiteral("kind"), QStringLiteral("posterize")}})));
    QVERIFY(!window_->saveActive());
    QVERIFY(!window_->saveActiveAs(dir_->filePath(QStringLiteral("refused.psd"))));
    QVERIFY(!view->save(dir_->filePath(QStringLiteral("refused.png"))));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(window_->activeFilePath(), path);
    QVERIFY(!window_->isDocumentDirty(window_->activeDocumentIndex()));
    QCOMPARE(readFile(path), before);
}

void DocumentVersionPolicyTest::usesOriginalCompositeForCompatibility()
{
    auto distinct = withResources(baseline_, stamp() + versionInfo());
    // Change the layer's opacity without changing the persisted white merge.
    distinct[firstLayerBlend(distinct) + 8] = '\0';
    const auto path = dir_->filePath(QStringLiteral("original-merge.psd"));
    QVERIFY(writeFile(path, distinct));
    QVERIFY(window_->openDocumentAtPath(path));
    QVERIFY(pictura::compatibility_preview_available(*window_->activeView()));
    QCOMPARE(window_->activeView()->image().pixelColor(0, 0), QColor(Qt::white));
    QVERIFY(!install(stamp() + versionInfo(false), QStringLiteral("no-composite.psd")).isEmpty());
    QVERIFY(!pictura::compatibility_preview_available(*window_->activeView()));
    QVERIFY(window_->activeView()->open_notice().contains(QStringLiteral("preview")));
}

void DocumentVersionPolicyTest::refreshesAccessOnReplacement()
{
    const auto path = install(stamp());
    QVERIFY(!path.isEmpty());
    QVERIFY(!pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(writeFile(path, withResources(baseline_, {})));
    QVERIFY(window_->revertActive());
    QVERIFY(pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(window_->activeView()->set_layer_opacity(0, 100));
    QVERIFY(window_->saveActive());
    QVERIFY(window_->activeView()->history_jump(0));
    QVERIFY(!pictura::compatibility_reason(*window_->activeView()).isEmpty());
    QVERIFY(!window_->saveActive());
}

void DocumentVersionPolicyTest::writesFlattenedCompatibilityCopy()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    const auto before = readFile(path);
    const auto history = window_->activeView()->history_count();
    const auto copy = dir_->filePath(QStringLiteral("copy.png"));
    chooseCopy(copy);
    QVERIFY(window_->saveCopyWithDialog());
    QCOMPARE(QImage(copy).pixelColor(0, 0), QColor(Qt::white));
    QCOMPARE(readFile(path), before);
    QCOMPARE(window_->activeFilePath(), path);
    QCOMPARE(window_->activeView()->history_count(), history);
    QVERIFY(!window_->isDocumentDirty(window_->activeDocumentIndex()));
}

void DocumentVersionPolicyTest::refusesUnsafeCompatibilityCopies()
{
    const auto path = install(stamp() + versionInfo());
    QVERIFY(!path.isEmpty());
    const auto source = readFile(path);
    const auto copy = dir_->filePath(QStringLiteral("refused.png"));
    chooseCopy(copy, false);
    QVERIFY(!window_->saveCopyWithDialog());
    chooseCopy(path);
    QVERIFY(!window_->saveCopyWithDialog());
    QVERIFY(!window_->activeView()->save_copy(dir_->filePath(QStringLiteral("refused.psd"))));
    QVERIFY(!install(stamp() + versionInfo(false), QStringLiteral("no-copy.psd")).isEmpty());
    chooseCopy(copy);
    QVERIFY(!window_->saveCopyWithDialog());
    QVERIFY(!QFile::exists(copy));
    QCOMPARE(readFile(path), source);
}

QTEST_MAIN(DocumentVersionPolicyTest)
#include "tst_document_version_policy.moc"
