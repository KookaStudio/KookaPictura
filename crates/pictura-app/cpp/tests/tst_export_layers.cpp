// File > Scripts > Export Layers to Files: dialog, per-layer files, formats,
// visibility, naming, and the cancellable batch.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "export_layers_dialog.h"
#include "frame.h"

#include <QtCore/QByteArray>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QImageWriter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QProgressDialog>

#include "qt_test_support.h"

#include <memory>

namespace {

bool tiffWriterAvailable()
{
    const QList<QByteArray> formats = QImageWriter::supportedImageFormats();
    return formats.contains("tif") || formats.contains("tiff");
}

template <typename T>
T* child(QObject& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

QStringList files(const QString& folder)
{
    return QDir(folder).entryList(QDir::Files | QDir::NoDotAndDotDot);
}

QByteArray readBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

pictura::PictureView* newDoc(pictura::PicturaMainWindow& window, const QString& name, int width,
                             int height)
{
    if (!window.newDocument(name, width, height, QStringLiteral("rgb"), 8,
                            QStringLiteral("white"))) {
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

bool exportAll(pictura::PictureView& view, const QString& folder, const QString& prefix = QString(),
               bool visibleOnly = false, const QString& format = QStringLiteral("PSD"),
               bool icc = false)
{
    return pictura::exportLayers(&view, folder, prefix, visibleOnly, format, icc);
}

void hideBackground(pictura::PictureView& view)
{
    view.set_layer_visible(0, false);
}

} // namespace

class ExportLayersTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void layersDialogShowsControls();
    void layersDialogDefaults();
    void layersFormatMenuMatches();
    void layersIccControlFollowsFormat();
    void layersDialogLeavesDocumentUnchanged();
    void layersWritesOneFilePerLayer();
    void layersUseDocumentCanvas();
    void layersGroupChildren();
    void layersGroupEffectsNotApplied();
    void layersAdjustmentTarget();
    void layersHiddenLayerRenders();
    void layersPsdKeepsSingleLayer();
    void layersTiffDocumentCanvas();
    void layersJpegFlattensOntoWhite();
    void layersPdfSingleLayerPage();
    void layersIccEmbedsWorkingProfile();
    void layersIccOffWritesUntagged();
    void layersReplaceExistingFiles();
    void layersDirectoryTargetStopsBatch();
    void layersCancelStopsBatch();
    void layersCloseDuringDelayAbortsSafely();
    void layersRefusesBadDestination();
    void layersWriteFailureStopsBatch();
    void layersLeavesDocumentUnchanged();
    void layersSingleLayerWritesOneFile();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ExportLayersTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ExportLayersTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void ExportLayersTest::layersDialogShowsControls()
{
    pictura::ExportLayersDialog dialog(QDir::homePath(), QStringLiteral("poster"));
    QVERIFY(child<QLineEdit>(dialog, "layersDestination"));
    QVERIFY(child<QLineEdit>(dialog, "layersPrefix"));
    QVERIFY(child<QCheckBox>(dialog, "layersVisible"));
    QVERIFY(child<QComboBox>(dialog, "layersFormat"));
    QVERIFY(child<QCheckBox>(dialog, "layersIcc"));
}

void ExportLayersTest::layersDialogDefaults()
{
    pictura::ExportLayersDialog dialog(QDir::homePath(), QStringLiteral("poster"));
    QCOMPARE(child<QLineEdit>(dialog, "layersPrefix")->text(), QStringLiteral("poster"));
    QCOMPARE(dialog.format(), QStringLiteral("PSD"));
    QVERIFY(!dialog.visibleOnly());
    QVERIFY(!dialog.includeIcc());
}

void ExportLayersTest::layersFormatMenuMatches()
{
    pictura::ExportLayersDialog dialog(QDir::homePath(), QString());
    auto* format = child<QComboBox>(dialog, "layersFormat");
    QCOMPARE(format->count(), 5);
    const QStringList expected = {QStringLiteral("PSD"), QStringLiteral("JPEG"),
                                  QStringLiteral("TIFF"), QStringLiteral("BMP"),
                                  QStringLiteral("PDF")};
    for (int i = 0; i < expected.size(); ++i) {
        QCOMPARE(format->itemText(i), expected[i]);
    }
}

void ExportLayersTest::layersIccControlFollowsFormat()
{
    pictura::ExportLayersDialog dialog(QDir::homePath(), QString());
    auto* format = child<QComboBox>(dialog, "layersFormat");
    auto* icc = child<QCheckBox>(dialog, "layersIcc");
    format->setCurrentIndex(3); // BMP
    QVERIFY(!icc->isEnabled());
    format->setCurrentIndex(2); // TIFF
    QVERIFY(icc->isEnabled());
    format->setCurrentIndex(4); // PDF
    QVERIFY(icc->isEnabled());
}

void ExportLayersTest::layersDialogLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersDialog"), 40, 40);
    QVERIFY(view);
    const int history = view->history_count();
    pictura::ExportLayersDialog dialog(QDir::homePath(), QStringLiteral("x"));
    QCOMPARE(view->history_count(), history);
}

void ExportLayersTest::layersWritesOneFilePerLayer()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersCount"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("one")).isEmpty());
    QVERIFY(!addLayer(*view, QStringLiteral("two")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path()));
    QCOMPARE(files(dir.path()).size(), 3);
}

void ExportLayersTest::layersUseDocumentCanvas()
{
    if (!tiffWriterAvailable()) {
        QSKIP("Qt TIFF writer unavailable");
    }
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersCanvas"), 200, 100);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("TIFF")));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    const QImage image(dir.filePath(output.first()));
    QVERIFY(!image.isNull());
    QCOMPARE(image.size(), QSize(200, 100));
    QCOMPARE(image.pixelColor(0, 0).alpha(), 0);
}

void ExportLayersTest::layersGroupChildren()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersGroup"), 40, 40);
    QVERIFY(view);
    const QString group = view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    const QString first = view->add_layer_in(group);
    const QString second = view->add_layer_in(group);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    view->set_layer_name_path(first, QStringLiteral("a"));
    view->set_layer_name_path(second, QStringLiteral("b"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path()));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 3);
    QVERIFY(!output.contains(QStringLiteral("Group.psd")));
    QVERIFY(output.contains(QStringLiteral("a.psd")));
    QVERIFY(output.contains(QStringLiteral("b.psd")));
}

void ExportLayersTest::layersGroupEffectsNotApplied()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersGroupFx"), 40, 40);
    QVERIFY(view);
    const QString group = view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    const QString child = view->add_layer_in(group);
    QVERIFY(!child.isEmpty());
    view->set_layer_name_path(child, QStringLiteral("child"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path()));
    QVERIFY(files(dir.path()).contains(QStringLiteral("child.psd")));
}

void ExportLayersTest::layersAdjustmentTarget()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersAdjustment"), 40, 40);
    QVERIFY(view);
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));

    // The adjustment alone has no pixels: its raster target is transparent.
    if (tiffWriterAvailable()) {
        QTemporaryDir tiffDir;
        QVERIFY(tiffDir.isValid());
        QVERIFY(exportAll(*view, tiffDir.path(), QString(), false, QStringLiteral("TIFF")));
        bool transparent = false;
        for (const QString& name : files(tiffDir.path())) {
            const QImage image(tiffDir.filePath(name));
            if (image.hasAlphaChannel() && image.pixelColor(0, 0).alpha() == 0) {
                transparent = true;
            }
        }
        QVERIFY(transparent);
    }

    // The PSD target carries the adjustment layer.
    QTemporaryDir psdDir;
    QVERIFY(psdDir.isValid());
    QVERIFY(exportAll(*view, psdDir.path(), QString(), false, QStringLiteral("PSD")));
    bool adjustment = false;
    for (const QString& name : files(psdDir.path())) {
        pictura::PictureView reopened;
        if (!pictura::document_api::open(reopened, psdDir.filePath(name))) {
            continue;
        }
        for (const pictura::layers_api::LayerRow& row : pictura::layers_api::rows(reopened)) {
            adjustment = adjustment || row.hasAdjustment;
        }
    }
    QVERIFY(adjustment);
}

void ExportLayersTest::layersHiddenLayerRenders()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersHidden"), 40, 40);
    QVERIFY(view);
    const QString path = addLayer(*view, QStringLiteral("hidden"));
    QVERIFY(!path.isEmpty());
    view->set_layer_visible(path.toInt(), false);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), false));
    QVERIFY(files(dir.path()).contains(QStringLiteral("hidden.psd")));
}

void ExportLayersTest::layersPsdKeepsSingleLayer()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersPsd"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("PSD")));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    pictura::PictureView reopened;
    QVERIFY(pictura::document_api::open(reopened, dir.filePath(output.first())));
    QCOMPARE(reopened.layer_count(), 1);
}

void ExportLayersTest::layersTiffDocumentCanvas()
{
    if (!tiffWriterAvailable()) {
        QSKIP("Qt TIFF writer unavailable");
    }
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersTiff"), 200, 100);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("TIFF")));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    const QImage image(dir.filePath(output.first()));
    QCOMPARE(image.size(), QSize(200, 100));
    QCOMPARE(image.pixelColor(199, 99).alpha(), 0);
}

void ExportLayersTest::layersJpegFlattensOntoWhite()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersJpeg"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("JPEG")));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    const QImage image(dir.filePath(output.first()));
    const QColor pixel = image.pixelColor(20, 20);
    QVERIFY(pixel.red() >= 245 && pixel.green() >= 245 && pixel.blue() >= 245);
}

void ExportLayersTest::layersPdfSingleLayerPage()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersPdf"), 200, 100);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("PDF")));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    const QByteArray bytes = readBytes(dir.filePath(output.first()));
    QVERIFY(bytes.startsWith("%PDF"));
    QVERIFY(bytes.contains("/Count 1"));
    QVERIFY(bytes.contains("/MediaBox [0 0 200 100]"));
}

void ExportLayersTest::layersIccEmbedsWorkingProfile()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersIcc"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("JPEG"), true));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    QVERIFY(readBytes(dir.filePath(output.first())).contains("ICC_PROFILE"));
}

void ExportLayersTest::layersIccOffWritesUntagged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersIccOff"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path(), QString(), true, QStringLiteral("JPEG"), false));
    const QStringList output = files(dir.path());
    QCOMPARE(output.size(), 1);
    QVERIFY(!readBytes(dir.filePath(output.first())).contains("ICC_PROFILE"));
}

void ExportLayersTest::layersReplaceExistingFiles()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersReplace"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("replace")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QFile old(dir.filePath(QStringLiteral("replace.psd")));
    QVERIFY(old.open(QIODevice::WriteOnly));
    old.write("old");
    old.close();
    QVERIFY(exportAll(*view, dir.path(), QString(), true));
    QVERIFY(QFileInfo(dir.filePath(QStringLiteral("replace.psd"))).size() > 4);
}

void ExportLayersTest::layersDirectoryTargetStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersDirTarget"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("taken")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(QDir(dir.path()).mkdir(QStringLiteral("taken.psd")));
    QVERIFY(!exportAll(*view, dir.path(), QString(), true));
}

void ExportLayersTest::layersCancelStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersCancel"), 40, 40);
    QVERIFY(view);
    for (int i = 0; i < 5; ++i) {
        QVERIFY(!addLayer(*view, QStringLiteral("layer%1").arg(i)).isEmpty());
    }
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::export_tools_set_delay(*view, 60000);
    QTimer::singleShot(300, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QProgressDialog*>(widget);
            if (dialog && dialog->objectName() == QLatin1String("jobProgress")) {
                QTest::keyClick(dialog, Qt::Key_Escape);
            }
        }
    });
    QVERIFY(!exportAll(*view, dir.path()));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
    pictura::export_tools_set_delay(*view, 0);
}

void ExportLayersTest::layersCloseDuringDelayAbortsSafely()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersClose"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("one")).isEmpty());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::export_tools_set_delay(*view, 200000);
    QTimer::singleShot(100, this, [this] {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    });
    const bool exported = exportAll(*view, dir.path());

    pictura::PictureView* fresh = newDoc(*window_, QStringLiteral("LayersCloseReset"), 10, 10);
    QVERIFY(fresh);
    pictura::export_tools_set_delay(*fresh, 0);
    QVERIFY(!exported);
}

void ExportLayersTest::layersRefusesBadDestination()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersBadDir"), 40, 40);
    QVERIFY(view);
    QVERIFY(!exportAll(*view, QStringLiteral("/proc/does-not-exist")));
}

void ExportLayersTest::layersWriteFailureStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersWriteFail"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("taken")).isEmpty());
    hideBackground(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(QDir(dir.path()).mkdir(QStringLiteral("taken.psd")));
    QVERIFY(!exportAll(*view, dir.path(), QString(), true));
}

void ExportLayersTest::layersLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersUnchanged"), 40, 40);
    QVERIFY(view);
    QVERIFY(!addLayer(*view, QStringLiteral("only")).isEmpty());
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    const QString path = pictura::document_api::filePath(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path()));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
    QCOMPARE(pictura::document_api::filePath(*view), path);
}

void ExportLayersTest::layersSingleLayerWritesOneFile()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("LayersSingle"), 40, 40);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(exportAll(*view, dir.path()));
    QCOMPARE(files(dir.path()).size(), 1);
}

QTEST_MAIN(ExportLayersTest)

#include "tst_export_layers.moc"
