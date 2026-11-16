// File > Scripts > Load Files into Stack: the Load Layers dialog, source
// selection, stack construction, sizing, failures, Auto-Align, and the
// embedded smart-object option.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/layers_api.h"
#include "api/view_bridge.h"
#include "frame.h"
#include "load_layers_dialog.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>

#include "qt_test_support.h"

#include <memory>

namespace {

template <typename T>
T* child(QObject& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

} // namespace

class LoadFilesIntoStackTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void dialogShowsControls();
    void dialogDefaults();
    void dialogUseMenu();
    void dialogOkNeedsFiles();
    void dialogCancelChangesNothing();
    void removeSelected();
    void addOpenFiles();
    void duplicatesIgnored();
    void folderListsImages();
    void loadsOneLayerPerFile();
    void canvasIsLargestSource();
    void firstFileIsTopmost();
    void unreadableFileIsSkipped();
    void allFailedCreatesNothing();
    void worksWithoutDocument();
    void smartObjectCombinesLayers();
    void noSmartObjectKeepsLayers();

private:
    QString makeImage(const QString& name, int width, int height, const QColor& colour);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LoadFilesIntoStackTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LoadFilesIntoStackTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

QString LoadFilesIntoStackTest::makeImage(const QString& name, int width, int height,
                                          const QColor& colour)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(colour);
    {
        QPainter painter(&image);
        painter.fillRect(0, 0, width / 2, height / 2,
                         QColor(255 - colour.red(), 255 - colour.green(), 255 - colour.blue()));
    }
    const QString path = dir_.filePath(name);
    return image.save(path, "PNG") ? path : QString();
}

void LoadFilesIntoStackTest::dialogShowsControls()
{
    pictura::LoadLayersDialog dialog({}, dir_.path());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Load Layers"));
    QVERIFY(child<QGroupBox>(dialog, "loadLayersSourceGroup"));
    QVERIFY(child<QComboBox>(dialog, "loadLayersUse"));
    QVERIFY(child<QListWidget>(dialog, "loadLayersFiles"));
    QVERIFY(child<QPushButton>(dialog, "loadLayersBrowse"));
    QVERIFY(child<QPushButton>(dialog, "loadLayersRemove"));
    QVERIFY(child<QPushButton>(dialog, "loadLayersAddOpen"));
    QVERIFY(child<QCheckBox>(dialog, "loadLayersAlign"));
    QVERIFY(child<QCheckBox>(dialog, "loadLayersSmartObject"));
    QVERIFY(child<QPushButton>(dialog, "loadLayersOk"));
    QVERIFY(child<QPushButton>(dialog, "loadLayersCancel"));
}

void LoadFilesIntoStackTest::dialogDefaults()
{
    pictura::LoadLayersDialog dialog({}, dir_.path());
    auto* use = child<QComboBox>(dialog, "loadLayersUse");
    auto* files = child<QListWidget>(dialog, "loadLayersFiles");
    auto* ok = child<QPushButton>(dialog, "loadLayersOk");
    QVERIFY(use && files && ok);
    QCOMPARE(use->currentData().toString(), QStringLiteral("files"));
    QVERIFY(!child<QCheckBox>(dialog, "loadLayersAlign")->isChecked());
    QVERIFY(!child<QCheckBox>(dialog, "loadLayersSmartObject")->isChecked());
    QCOMPARE(files->count(), 0);
    QVERIFY(!ok->isEnabled());
}

void LoadFilesIntoStackTest::dialogUseMenu()
{
    pictura::LoadLayersDialog dialog({}, dir_.path());
    auto* use = child<QComboBox>(dialog, "loadLayersUse");
    QVERIFY(use);
    QCOMPARE(use->count(), 2);
    QCOMPARE(use->itemData(0).toString(), QStringLiteral("files"));
    QCOMPARE(use->itemData(1).toString(), QStringLiteral("folder"));
}

void LoadFilesIntoStackTest::dialogOkNeedsFiles()
{
    const QString image = makeImage(QStringLiteral("ok.png"), 8, 8, Qt::red);
    QVERIFY(!image.isEmpty());
    pictura::LoadLayersDialog dialog({}, dir_.path());
    auto* ok = child<QPushButton>(dialog, "loadLayersOk");
    QVERIFY(ok);
    QVERIFY(!ok->isEnabled());
    dialog.addFiles({image});
    QVERIFY(ok->isEnabled());
}

void LoadFilesIntoStackTest::dialogCancelChangesNothing()
{
    QVERIFY(window_->newDocument(QStringLiteral("Keep"), 16, 12, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    auto* view = window_->activeView();
    QVERIFY(view);
    const bool dirty = view->is_dirty();
    const int history = view->history_count();
    const int layers = pictura::layers_api::rowCount(*view);

    pictura::LoadLayersDialog dialog({}, dir_.path());
    dialog.reject();

    QCOMPARE(view->is_dirty(), dirty);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::layers_api::rowCount(*view), layers);
}

void LoadFilesIntoStackTest::removeSelected()
{
    const QString first = makeImage(QStringLiteral("remove-a.png"), 8, 8, Qt::red);
    const QString second = makeImage(QStringLiteral("remove-b.png"), 8, 8, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    pictura::LoadLayersDialog dialog({}, dir_.path());
    dialog.addFiles({first, second});
    auto* list = child<QListWidget>(dialog, "loadLayersFiles");
    auto* remove = child<QPushButton>(dialog, "loadLayersRemove");
    QVERIFY(list && remove);
    list->item(0)->setSelected(true);
    remove->click();
    QCOMPARE(dialog.paths(), QStringList{second});
}

void LoadFilesIntoStackTest::addOpenFiles()
{
    const QString image = makeImage(QStringLiteral("open.png"), 8, 8, Qt::green);
    QVERIFY(!image.isEmpty());
    pictura::LoadLayersDialog dialog({image}, dir_.path());
    auto* addOpen = child<QPushButton>(dialog, "loadLayersAddOpen");
    QVERIFY(addOpen);
    QVERIFY(addOpen->isEnabled());
    addOpen->click();
    QCOMPARE(dialog.paths(), QStringList{image});
}

void LoadFilesIntoStackTest::duplicatesIgnored()
{
    const QString image = makeImage(QStringLiteral("dup.png"), 8, 8, Qt::red);
    QVERIFY(!image.isEmpty());
    pictura::LoadLayersDialog dialog({}, dir_.path());
    dialog.addFiles({image, image});
    QCOMPARE(dialog.paths(), QStringList{image});
}

void LoadFilesIntoStackTest::folderListsImages()
{
    const QString folder = dir_.filePath(QStringLiteral("folder"));
    QVERIFY(QDir().mkpath(folder));
    const QString first = makeImage(QStringLiteral("folder/a.png"), 8, 8, Qt::red);
    const QString second = makeImage(QStringLiteral("folder/b.jpg"), 8, 8, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    QFile notes(folder + QStringLiteral("/notes.txt"));
    QVERIFY(notes.open(QIODevice::WriteOnly));
    notes.write("not an image");
    notes.close();

    pictura::LoadLayersDialog dialog({}, dir_.path());
    auto* use = child<QComboBox>(dialog, "loadLayersUse");
    QVERIFY(use);
    use->setCurrentIndex(1);
    dialog.setFolder(folder);
    const QStringList paths = dialog.paths();
    QCOMPARE(paths.size(), 2);
    QVERIFY(paths.contains(first));
    QVERIFY(paths.contains(second));
    for (const QString& path : paths) {
        QVERIFY(!path.endsWith(QStringLiteral(".txt")));
    }
}

void LoadFilesIntoStackTest::loadsOneLayerPerFile()
{
    const QString first = makeImage(QStringLiteral("one-a.png"), 16, 12, Qt::red);
    const QString second = makeImage(QStringLiteral("one-b.png"), 16, 12, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    pictura::PictureView view;
    const QStringList failures =
        pictura::document_api::loadFilesIntoStack(view, {first, second}, false, false);
    QVERIFY2(failures.isEmpty(), qPrintable(failures.join(QLatin1Char(','))));
    QVERIFY(view.has_document());
    QCOMPARE(pictura::layers_api::rowCount(view), 2);
    QVERIFY(!view.is_dirty());
    QCOMPARE(view.history_count(), 1);
    QCOMPARE(view.history_label(0), QStringLiteral("Load Files into Stack"));
}

void LoadFilesIntoStackTest::canvasIsLargestSource()
{
    const QString big = makeImage(QStringLiteral("big.png"), 64, 48, Qt::red);
    const QString small = makeImage(QStringLiteral("small.png"), 32, 20, Qt::blue);
    QVERIFY(!big.isEmpty() && !small.isEmpty());
    pictura::PictureView view;
    QVERIFY(pictura::document_api::loadFilesIntoStack(view, {big, small}, false, false).isEmpty());
    const pictura::layers_api::Snapshot snapshot = pictura::layers_api::snapshot(view, 0, true);
    QCOMPARE(snapshot.documentWidth, 64);
    QCOMPARE(snapshot.documentHeight, 48);
}

void LoadFilesIntoStackTest::firstFileIsTopmost()
{
    const QString first = makeImage(QStringLiteral("first.png"), 16, 12, Qt::red);
    const QString second = makeImage(QStringLiteral("second.png"), 16, 12, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    pictura::PictureView view;
    QVERIFY(pictura::document_api::loadFilesIntoStack(view, {first, second}, false, false).isEmpty());
    QCOMPARE(pictura::layers_api::rowAt(view, 0).name, QStringLiteral("first.png"));
    QCOMPARE(pictura::layers_api::rowAt(view, 1).name, QStringLiteral("second.png"));
}

void LoadFilesIntoStackTest::unreadableFileIsSkipped()
{
    const QString image = makeImage(QStringLiteral("present.png"), 8, 8, Qt::red);
    QVERIFY(!image.isEmpty());
    const QString missing = dir_.filePath(QStringLiteral("missing.png"));
    pictura::PictureView view;
    const QStringList failures =
        pictura::document_api::loadFilesIntoStack(view, {image, missing}, false, false);
    QCOMPARE(failures, QStringList{missing});
    QVERIFY(view.has_document());
    QCOMPARE(pictura::layers_api::rowCount(view), 1);
}

void LoadFilesIntoStackTest::allFailedCreatesNothing()
{
    const QString missing = dir_.filePath(QStringLiteral("none.png"));
    pictura::PictureView view;
    const QStringList failures =
        pictura::document_api::loadFilesIntoStack(view, {missing}, false, false);
    QCOMPARE(failures, QStringList{missing});
    QVERIFY(!view.has_document());
}

void LoadFilesIntoStackTest::worksWithoutDocument()
{
    const QString first = makeImage(QStringLiteral("no-doc-a.png"), 8, 8, Qt::red);
    const QString second = makeImage(QStringLiteral("no-doc-b.png"), 8, 8, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    QCOMPARE(window_->documentCount(), 0);
    pictura::PictureView view;
    QVERIFY(pictura::document_api::loadFilesIntoStack(view, {first, second}, false, false).isEmpty());
    QVERIFY(view.has_document());
}

void LoadFilesIntoStackTest::smartObjectCombinesLayers()
{
    const QString first = makeImage(QStringLiteral("smart-a.png"), 16, 12, Qt::red);
    const QString second = makeImage(QStringLiteral("smart-b.png"), 16, 12, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    pictura::PictureView view;
    QVERIFY(pictura::document_api::loadFilesIntoStack(view, {first, second}, false, true).isEmpty());
    QCOMPARE(pictura::layers_api::rowCount(view), 1);
    const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(view, 0);
    QVERIFY(row.smartObject);
    QVERIFY(view.layer_can_edit_smart_object_contents(row.path));
}

void LoadFilesIntoStackTest::noSmartObjectKeepsLayers()
{
    const QString first = makeImage(QStringLiteral("plain-a.png"), 16, 12, Qt::red);
    const QString second = makeImage(QStringLiteral("plain-b.png"), 16, 12, Qt::blue);
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    pictura::PictureView view;
    QVERIFY(pictura::document_api::loadFilesIntoStack(view, {first, second}, false, false).isEmpty());
    QCOMPARE(pictura::layers_api::rowCount(view), 2);
    QVERIFY(!pictura::layers_api::rowAt(view, 0).smartObject);
    QVERIFY(!pictura::layers_api::rowAt(view, 1).smartObject);
}

QTEST_MAIN(LoadFilesIntoStackTest)
#include "tst_load_files_into_stack.moc"
