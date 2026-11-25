// Zoomify and Paths to Illustrator: dialogs, output files, and the pumped
// export progress window.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "busy_job.h"
#include "command_ids.h"
#include "commands.h"
#include "export_paths_dialog.h"
#include "frame.h"
#include "zoomify_dialog.h"

#include <QtCore/QByteArray>
#include <QtCore/QDir>
#include <QtCore/QSize>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

#include <cstdint>
#include <memory>

namespace {

template <typename T>
T* child(QObject& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

QString read(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? QString::fromUtf8(file.readAll()) : QString();
}

QByteArray readBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

pictura::PictureView* newDoc(pictura::PicturaMainWindow& window, const QString& name, int width, int height,
                             const QString& fill = QStringLiteral("white"))
{
    if (!window.newDocument(name, width, height, QStringLiteral("rgb"), 8, fill)) {
        return nullptr;
    }
    return window.activeView();
}

} // namespace

class FileExportToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void zoomifyDialogShowsControls();
    void zoomifyDialogDefaults();
    void zoomifyDialogBounds();
    void zoomifyDialogLeavesDocumentUnchanged();
    void zoomifyWritesOneTier();
    void zoomifyWritesEveryTier();
    void zoomifyRollsTileGroups();
    void zoomifyEdgeTileDimensions();
    void zoomifyTilesFlattenOntoWhite();
    void zoomifyWritesTilesAndPage();
    void zoomifyCancelLeavesDocument();
    void zoomifyLeavesDocumentUnchanged();
    void zoomifyRefusesBadDestination();
    void zoomifyLeavesUnrelatedFiles();

    void pathsDialogOffersBothFormats();
    void pathsAppendsAiSuffix();
    void pathsDialogLeavesDocumentUnchanged();
    void pathsNoPathsWritesNothing();
    void pathsCarriesWorkAndSaved();
    void pathsUnwritableDestinationKeepsOldFile();
    void pathsLeavesDocumentUnchanged();

    void exportCommandsNeedADocument();
    void exportProgressShowsTheJobWindow();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FileExportToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void FileExportToolsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    pictura::zoomifySetTestDelay(0);
}

void FileExportToolsTest::zoomifyDialogShowsControls()
{
    pictura::ZoomifyDialog dialog(1200, 800, QDir::homePath());
    QVERIFY(child<QLineEdit>(dialog, "zoomifyFolder"));
    QVERIFY(child<QSpinBox>(dialog, "zoomifyWidth"));
    QVERIFY(child<QSpinBox>(dialog, "zoomifyHeight"));
    QVERIFY(child<QSpinBox>(dialog, "zoomifyQuality"));
    QVERIFY(child<QComboBox>(dialog, "zoomifyTemplate"));
    QCOMPARE(child<QComboBox>(dialog, "zoomifyTemplate")->count(), 1);
}

void FileExportToolsTest::zoomifyDialogDefaults()
{
    pictura::ZoomifyDialog dialog(1200, 800, QString());
    QCOMPARE(dialog.baseWidth(), 1200);
    QCOMPARE(dialog.baseHeight(), 800);
    QCOMPARE(dialog.quality(), 75);
    QCOMPARE(child<QSpinBox>(dialog, "zoomifyQuality")->minimum(), 0);
    QCOMPARE(child<QSpinBox>(dialog, "zoomifyQuality")->maximum(), 100);
}

void FileExportToolsTest::zoomifyDialogBounds()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyBounds"), 100, 50);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(!pictura::zoomifyExport(view, dir.path(), 0, 50, 75));
    QVERIFY(!pictura::zoomifyExport(view, dir.path(), 101, 50, 75));
    pictura::ZoomifyDialog dialog(100, 50, dir.path());
    QCOMPARE(child<QSpinBox>(dialog, "zoomifyWidth")->maximum(), 100);
    QCOMPARE(child<QSpinBox>(dialog, "zoomifyHeight")->maximum(), 50);
}

void FileExportToolsTest::zoomifyDialogLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyDialog"), 40, 40);
    QVERIFY(view);
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    pictura::ZoomifyDialog dialog(40, 40, QDir::homePath());
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void FileExportToolsTest::zoomifyWritesOneTier()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyOne"), 200, 100);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 200, 100, 75));
    QVERIFY(QFileInfo::exists(dir.filePath(QStringLiteral("TileGroup0/0-0-0.jpg"))));
    QVERIFY(read(dir.filePath(QStringLiteral("ImageProperties.xml"))).contains(
        QStringLiteral("NUMTILES=\"1\"")));
}

void FileExportToolsTest::zoomifyWritesEveryTier()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyTiers"), 1000, 700);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 1000, 700, 75));
    QVERIFY(QFileInfo::exists(dir.filePath(QStringLiteral("TileGroup0/2-3-2.jpg"))));
    QVERIFY(read(dir.filePath(QStringLiteral("ImageProperties.xml"))).contains(
        QStringLiteral("NUMTILES=\"17\"")));
}

void FileExportToolsTest::zoomifyRollsTileGroups()
{
    const ::rust::Vec<std::int32_t> flat = pictura::zoomify_plan_flat(4096, 4096);
    QVERIFY(flat.size() >= 4);
    QVERIFY(flat[3] > 256);
    QCOMPARE(pictura::zoomify_group_for(255), 0);
    QCOMPARE(pictura::zoomify_group_for(256), 1);
    QCOMPARE(pictura::zoomify_group_for(512), 2);
}

void FileExportToolsTest::zoomifyEdgeTileDimensions()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyEdge"), 300, 200);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 300, 200, 75));
    const QImage edge(dir.filePath(QStringLiteral("TileGroup0/1-1-0.jpg")));
    QVERIFY(!edge.isNull());
    QCOMPARE(edge.size(), QSize(44, 200));
}

void FileExportToolsTest::zoomifyTilesFlattenOntoWhite()
{
    pictura::PictureView* view =
        newDoc(*window_, QStringLiteral("ZoomifyWhite"), 8, 8, QStringLiteral("transparent"));
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 8, 8, 75));
    const QImage tile(dir.filePath(QStringLiteral("TileGroup0/0-0-0.jpg")));
    QVERIFY(!tile.isNull());
    const QColor pixel = tile.pixelColor(0, 0);
    QVERIFY2(pixel.red() >= 245 && pixel.green() >= 245 && pixel.blue() >= 245,
             qPrintable(pixel.name()));
}

void FileExportToolsTest::zoomifyWritesTilesAndPage()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyPage"), 200, 100);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 200, 100, 75));
    const QString html = read(dir.filePath(QStringLiteral("index.html")));
    QVERIFY(html.contains(QStringLiteral("TileGroup")));
    QVERIFY(html.contains(QStringLiteral("W=200")));
    QVERIFY(!html.contains(QStringLiteral("fetch(")));
    QVERIFY(!html.contains(QStringLiteral("XMLHttpRequest")));
}

void FileExportToolsTest::zoomifyCancelLeavesDocument()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyCancel"), 1000, 700);
    QVERIFY(view);
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::zoomifySetTestDelay(60);
    QTimer::singleShot(300, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QProgressDialog*>(widget);
            if (dialog && dialog->objectName() == QLatin1String("jobProgress")) {
                QTest::keyClick(dialog, Qt::Key_Escape);
            }
        }
    });
    QVERIFY(!pictura::zoomifyExport(view, dir.path(), 1000, 700, 75));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void FileExportToolsTest::zoomifyLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyUnchanged"), 200, 100);
    QVERIFY(view);
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    const QString path = pictura::document_api::filePath(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 200, 100, 75));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
    QCOMPARE(pictura::document_api::filePath(*view), path);
}

void FileExportToolsTest::zoomifyRefusesBadDestination()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyBadDir"), 100, 100);
    QVERIFY(view);
    QVERIFY(!pictura::zoomifyExport(view, QStringLiteral("/proc/does-not-exist/zoomify"), 100, 100,
                                   75));
}

void FileExportToolsTest::zoomifyLeavesUnrelatedFiles()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ZoomifyOther"), 100, 100);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QFile keep(dir.filePath(QStringLiteral("keep.txt")));
    QVERIFY(keep.open(QIODevice::WriteOnly));
    keep.write("keep");
    keep.close();
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 100, 100, 75));
    QCOMPARE(read(dir.filePath(QStringLiteral("keep.txt"))), QStringLiteral("keep"));
}

void FileExportToolsTest::pathsDialogOffersBothFormats()
{
    pictura::ExportPathsDialog dialog(QDir::homePath() + QStringLiteral("/paths.ai"));
    auto* format = child<QComboBox>(dialog, "pathsFormat");
    QVERIFY(format);
    QCOMPARE(format->count(), 2);
    QCOMPARE(format->currentData().toString(), QStringLiteral("EPS"));
    QCOMPARE(format->itemData(1).toString(), QStringLiteral("PDF"));
}

void FileExportToolsTest::pathsAppendsAiSuffix()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsSuffix"), 100, 100);
    QVERIFY(view);
    QVERIFY(pictura::path_append_corner(*view, 10, 10, false));
    QVERIFY(pictura::path_append_corner(*view, 50, 10, false));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportPaths(view, dir.filePath(QStringLiteral("logo")),
                                 QStringLiteral("EPS")));
    QVERIFY(QFileInfo::exists(dir.filePath(QStringLiteral("logo.ai"))));
    QVERIFY(pictura::exportPaths(view, dir.filePath(QStringLiteral("logo.eps")),
                                 QStringLiteral("EPS")));
    QVERIFY(QFileInfo::exists(dir.filePath(QStringLiteral("logo.ai"))));
    QVERIFY(!QFileInfo::exists(dir.filePath(QStringLiteral("logo.eps.ai"))));
}

void FileExportToolsTest::pathsCarriesWorkAndSaved()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsBoth"), 100, 100);
    QVERIFY(view);
    QVERIFY(pictura::path_append_corner(*view, 10, 10, false));
    QVERIFY(pictura::path_append_corner(*view, 50, 10, false));
    QVERIFY(pictura::paths_save_work(*view, QStringLiteral("Outline")) >= 0);
    // Saving a path clears the Work Path; start a fresh one for the pair.
    QVERIFY(pictura::paths_set_active(*view, -2));
    QVERIFY(pictura::path_append_corner(*view, 60, 10, false));
    QVERIFY(pictura::path_append_corner(*view, 80, 10, false));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportPaths(view, dir.filePath(QStringLiteral("both.ai")),
                                 QStringLiteral("EPS")));
    const QByteArray bytes = readBytes(dir.filePath(QStringLiteral("both.ai")));
    QVERIFY(bytes.contains("Work Path"));
    QVERIFY(bytes.contains("Outline"));
}

void FileExportToolsTest::pathsDialogLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsDialog"), 40, 40);
    QVERIFY(view);
    const int history = view->history_count();
    pictura::ExportPathsDialog dialog(QDir::homePath() + QStringLiteral("/paths.ai"));
    QCOMPARE(view->history_count(), history);
}

void FileExportToolsTest::pathsNoPathsWritesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsNone"), 40, 40);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(!pictura::exportPaths(view, dir.filePath(QStringLiteral("none.ai")),
                                  QStringLiteral("EPS")));
    QVERIFY(!QFileInfo::exists(dir.filePath(QStringLiteral("none.ai"))));
}

void FileExportToolsTest::pathsUnwritableDestinationKeepsOldFile()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsUnwritable"), 100, 100);
    QVERIFY(view);
    QVERIFY(pictura::path_append_corner(*view, 10, 10, false));
    QVERIFY(pictura::path_append_corner(*view, 50, 10, false));
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString destination = dir.filePath(QStringLiteral("old.ai"));
    QFile old(destination);
    QVERIFY(old.open(QIODevice::WriteOnly));
    old.write("old");
    old.close();
    const QFileDevice::Permissions permissions = QFile::permissions(dir.path());
    QFile::setPermissions(dir.path(), QFileDevice::ReadOwner | QFileDevice::ExeOwner);
    const bool written = pictura::exportPaths(view, destination, QStringLiteral("EPS"));
    QFile::setPermissions(dir.path(), permissions);
    if (written) {
        QSKIP("permission checks are bypassed (running as root)");
    }
    QCOMPARE(readBytes(destination), QByteArray("old"));
}

void FileExportToolsTest::pathsLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("PathsUnchanged"), 100, 100);
    QVERIFY(view);
    QVERIFY(pictura::path_append_corner(*view, 10, 10, false));
    QVERIFY(pictura::path_append_corner(*view, 50, 10, false));
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(pictura::exportPaths(view, dir.filePath(QStringLiteral("x.ai")),
                                 QStringLiteral("PDF")));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void FileExportToolsTest::exportCommandsNeedADocument()
{
    const char* const ids[] = {pictura::command_ids::FileExportZoomify,
                               pictura::command_ids::FileExportPathsToIllustrator,
                               pictura::command_ids::FileScriptsExportLayersToFiles};
    for (const char* id : ids) {
        QAction* action = window_->registry()->action(QString::fromLatin1(id));
        QVERIFY2(action, id);
        QVERIFY2(!action->isEnabled(), id);
    }
}

void FileExportToolsTest::exportProgressShowsTheJobWindow()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("ExportProgress"), 1000, 700);
    QVERIFY(view);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::zoomifySetTestDelay(30);
    bool shown = false;
    QTimer::singleShot(350, [&shown] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QProgressDialog*>(widget);
            if (dialog && dialog->objectName() == QLatin1String("jobProgress")) {
                shown = true;
            }
        }
    });
    QVERIFY(pictura::zoomifyExport(view, dir.path(), 1000, 700, 75));
    QVERIFY2(shown, "the pumped export reports progress in the job window");
}

PICTURA_QT_TEST_MAIN(FileExportToolsTest)

#include "tst_file_export_tools.moc"
