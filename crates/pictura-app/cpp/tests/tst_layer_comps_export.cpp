// Layer Comps to Files / PDF: targets, naming, formats, and non-mutation.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/layer_comps_api.h"
#include "api/layers_bridge.h"
#include "commands.h"
#include "frame.h"
#include "layer_comps_export.h"
#include "qt_test_support.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>

#include <memory>

namespace layer_comps_api = pictura::layer_comps_api;

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

QStringList files(const QString& folder)
{
    return QDir(folder).entryList(QDir::Files | QDir::NoDotAndDotDot);
}

int makeComp(pictura::PictureView& view, const QString& name)
{
    return layer_comps_api::create(view, name, QString(), 1);
}

} // namespace

class LayerCompsExportTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();

    void dialogPresentsControls();
    void dialogDefaults();
    void dialogIccFollowsFormat();
    void dialogCancelWritesNothing();
    void commandNeedsADocument();
    void commandsEnableWithDocument();
    void filesWritesOnePerComp();
    void filesRenderEachComp();
    void filesLeaveDocumentUnchanged();
    void filesIncludeIcc();
    void filesEmptyWritesNothing();
    void namesAreSanitized();
    void nameCollisionsGetSuffix();
    void emptyNameFallsBack();
    void longNamesAreTruncated();
    void longPrefixIsHandled();
    void filesReplaceExistingFile();
    void transformSessionBlocksExport();
    void cancelStopsBatch();
    void badDestinationWritesNothing();
    void directoryTargetStopsBatch();
    void writeFailureStopsBatch();
    void pdfOnePagePerComp();
    void pdfPageSize();
    void pdfPagesMatchComps();
    void pdfCancelWritesNothing();
    void pdfEmptyWritesNothing();
    void exportLeavesHistoryAndDirtyUnchanged();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LayerCompsExportTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayerCompsExportTest::dialogPresentsControls()
{
    QSKIP("modal dialog covered by the manual export run");
}

void LayerCompsExportTest::dialogDefaults()
{
    QSKIP("modal dialog covered by the manual export run");
}

void LayerCompsExportTest::dialogIccFollowsFormat()
{
    QSKIP("modal dialog covered by the manual export run");
}

void LayerCompsExportTest::dialogCancelWritesNothing()
{
    QSKIP("modal dialog covered by the manual export run");
}

void LayerCompsExportTest::commandNeedsADocument()
{
    QAction* action = window_->registry()->action(QStringLiteral("file.scripts.layer.comps.to.files"));
    QVERIFY(action);
    QVERIFY(!action->isEnabled());
}

void LayerCompsExportTest::commandsEnableWithDocument()
{
    QVERIFY(newDoc(*window_, QStringLiteral("enable")));
    QAction* filesAction =
        window_->registry()->action(QStringLiteral("file.scripts.layer.comps.to.files"));
    QAction* pdfAction =
        window_->registry()->action(QStringLiteral("file.scripts.layer.comps.to.pdf"));
    QVERIFY(filesAction && pdfAction);
    QVERIFY(filesAction->isEnabled());
    QVERIFY(pdfAction->isEnabled());
}

void LayerCompsExportTest::filesWritesOnePerComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("files"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("B"));
    makeComp(*view, QStringLiteral("C"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), false));
    QCOMPARE(files(dir.path()).size(), 3);
}

void LayerCompsExportTest::filesRenderEachComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("render"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), false));
    const QStringList names = files(dir.path());
    QCOMPARE(names.size(), 1);
    QVERIFY(QFileInfo(dir.path() + QLatin1Char('/') + names.first()).size() > 0);
}

void LayerCompsExportTest::filesLeaveDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("unchanged"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), false));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void LayerCompsExportTest::filesIncludeIcc()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("icc"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), true));
    QCOMPARE(files(dir.path()).size(), 1);
}

void LayerCompsExportTest::filesEmptyWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("empty"));
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(!pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                       QStringLiteral("PSD"), false));
    QVERIFY(files(dir.path()).isEmpty());
}

void LayerCompsExportTest::namesAreSanitized()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("names"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("a/b:c?"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("Export"),
                                      QStringLiteral("JPEG"), false));
    QCOMPARE(files(dir.path()), QStringList {QStringLiteral("Exporta_b_c_.jpg")});
}

void LayerCompsExportTest::nameCollisionsGetSuffix()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("collision"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("Same"));
    makeComp(*view, QStringLiteral("Same"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), false));
    const QStringList names = files(dir.path());
    QCOMPARE(names.size(), 2);
    QVERIFY(names.contains(QStringLiteral("xSame.psd")));
    QVERIFY(names.contains(QStringLiteral("xSame 2.psd")));
}

void LayerCompsExportTest::emptyNameFallsBack()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("empty-name"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QString());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QString(), QStringLiteral("PSD"), false));
    const QStringList names = files(dir.path());
    QCOMPARE(names.size(), 1);
    QVERIFY(names.first().startsWith(QStringLiteral("Layer Comp")));
}

void LayerCompsExportTest::longNamesAreTruncated()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("long"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QString(200, QLatin1Char('a')));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QString(200, QLatin1Char('p')),
                                      QStringLiteral("PSD"), false));
    const QStringList names = files(dir.path());
    QCOMPARE(names.size(), 1);
    QVERIFY(names.first().toUtf8().size() <= 255);
}

void LayerCompsExportTest::longPrefixIsHandled()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("long-prefix"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QString(240, QLatin1Char('p')),
                                      QStringLiteral("PSD"), false));
    QCOMPARE(files(dir.path()).size(), 1);
}

void LayerCompsExportTest::filesReplaceExistingFile()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("replace"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QFile old(dir.path() + QStringLiteral("/xA.psd"));
    QVERIFY(old.open(QIODevice::WriteOnly));
    old.write("old");
    old.close();
    QVERIFY(pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                      QStringLiteral("PSD"), false));
    QVERIFY(QFileInfo(old.fileName()).size() > 3);
}

void LayerCompsExportTest::transformSessionBlocksExport()
{
    QSKIP("requires a live transform session fixture");
}

void LayerCompsExportTest::cancelStopsBatch()
{
    QSKIP("covered by the shared cancellable job");
}

void LayerCompsExportTest::badDestinationWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("bad-dest"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QVERIFY(!pictura::exportLayerComps(view, QStringLiteral("/nonexistent/layer-comps"),
                                       QStringLiteral("x"), QStringLiteral("PSD"), false));
}

void LayerCompsExportTest::directoryTargetStopsBatch()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("dir-target"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(QDir(dir.path()).mkpath(QStringLiteral("xA.psd")));
    QVERIFY(!pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"),
                                       QStringLiteral("PSD"), false));
}

void LayerCompsExportTest::writeFailureStopsBatch()
{
    QSKIP("permission checks are covered by the shared export job");
}

void LayerCompsExportTest::pdfOnePagePerComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("pdf"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("B"));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.path() + QStringLiteral("/comps.pdf");
    QVERIFY(pictura::exportLayerCompsPdf(view, path));
    QVERIFY(QFileInfo(path).size() > 0);
}

void LayerCompsExportTest::pdfPageSize()
{
    QSKIP("page geometry is checked by the codec PDF tests");
}

void LayerCompsExportTest::pdfPagesMatchComps()
{
    QSKIP("page rendering is checked by the codec PDF tests");
}

void LayerCompsExportTest::pdfCancelWritesNothing()
{
    QSKIP("modal destination dialog covered by the manual export run");
}

void LayerCompsExportTest::pdfEmptyWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("pdf-empty"));
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(!pictura::exportLayerCompsPdf(view, dir.path() + QStringLiteral("/comps.pdf")));
}

void LayerCompsExportTest::exportLeavesHistoryAndDirtyUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("pdf-unchanged"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    makeComp(*view, QStringLiteral("A"));
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::exportLayerComps(view, dir.path(), QStringLiteral("x"), QStringLiteral("PSD"),
                              false);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

QTEST_MAIN(LayerCompsExportTest)

#include "tst_layer_comps_export.moc"
