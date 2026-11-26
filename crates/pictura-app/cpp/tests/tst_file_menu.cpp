// File > Browse… (the Linux Bridge replacement), its menu position, target,
// side effects, and the disabled File > Scripts > Browse… row.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"

#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QUrl>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtGui/QKeySequence>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

#include "qt_test_support.h"

class FileMenuTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void browseMenuRow();
    void browseNeedsNoBridge();
    void browseOpensTheDocumentFolder();
    void browseWithoutAPathOpensHome();
    void browseLeavesDocumentsUnchanged();
    void scriptsBrowseIsDisabled();

private:
    QMenu* fileMenu() const;
    QString makeImage(const QString& name);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    QList<QUrl> opened_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FileMenuTest::initTestCase()
{
    qputenv("PICTURA_EDITION", "extended");
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->setUrlOpenerForTest([this](const QUrl& url) { opened_.append(url); });
}

void FileMenuTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    opened_.clear();
}

QMenu* FileMenuTest::fileMenu() const
{
    for (QAction* action : window_->menuBar()->actions()) {
        QString text = action->text();
        text.remove(QLatin1Char('&'));
        if (action->menu() && text == QLatin1String("File")) {
            return action->menu();
        }
    }
    return nullptr;
}

QString FileMenuTest::makeImage(const QString& name)
{
    QImage image(8, 8, QImage::Format_RGB32);
    image.fill(QColor(200, 30, 40));
    const QString path = dir_.filePath(name);
    return image.save(path, "PNG") ? path : QString();
}

void FileMenuTest::browseMenuRow()
{
    QMenu* menu = fileMenu();
    QVERIFY(menu != nullptr);
    const QList<QAction*> actions = menu->actions();
    int open = -1;
    int browse = -1;
    int openAs = -1;
    for (int index = 0; index < actions.size(); ++index) {
        const QString id = actions.at(index)->data().toString();
        if (id == QLatin1String(pictura::command_ids::FileOpen)) {
            open = index;
        } else if (id == QLatin1String(pictura::command_ids::FileBrowse)) {
            browse = index;
        } else if (id == QLatin1String(pictura::command_ids::FileOpenAs)) {
            openAs = index;
        }
    }
    QVERIFY(open >= 0);
    QVERIFY(browse >= 0);
    QVERIFY(openAs >= 0);
    QVERIFY(open < browse);
    QVERIFY(browse < openAs);

    QAction* action = window_->registry()->action(
        QString::fromLatin1(pictura::command_ids::FileBrowse));
    QVERIFY(action != nullptr);
    QCOMPARE(action->text(), QStringLiteral("Browse…"));
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Ctrl+Alt+O")));
    QVERIFY(action->isEnabled());
}

void FileMenuTest::browseNeedsNoBridge()
{
    opened_.clear();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileBrowse)));
    QCOMPARE(opened_.size(), 1);
    QVERIFY(opened_.first().isLocalFile());
}

void FileMenuTest::browseOpensTheDocumentFolder()
{
    const QString path = makeImage(QStringLiteral("photo.png"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openDocumentAtPath(path));
    opened_.clear();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileBrowse)));
    QCOMPARE(opened_.size(), 1);
    QCOMPARE(opened_.first(), QUrl::fromLocalFile(QFileInfo(path).absolutePath()));
}

void FileMenuTest::browseWithoutAPathOpensHome()
{
    opened_.clear();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileBrowse)));
    QCOMPARE(opened_.size(), 1);
    QCOMPARE(opened_.first(), QUrl::fromLocalFile(QDir::homePath()));

    QVERIFY(window_->newDocument(QStringLiteral("Untitled"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    opened_.clear();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileBrowse)));
    QCOMPARE(opened_.size(), 1);
    QCOMPARE(opened_.first(), QUrl::fromLocalFile(QDir::homePath()));
}

void FileMenuTest::browseLeavesDocumentsUnchanged()
{
    const QString path = makeImage(QStringLiteral("unchanged.png"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openDocumentAtPath(path));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const quint32 pixel = view->composite_argb(0, 0);
    const QString documentPath = window_->documentPath(0);
    const bool dirty = window_->isDocumentDirty(0);
    const int active = window_->activeDocumentIndex();
    const int history = view->history_count();

    opened_.clear();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FileBrowse)));

    QCOMPARE(view->composite_argb(0, 0), pixel);
    QCOMPARE(window_->documentPath(0), documentPath);
    QCOMPARE(window_->isDocumentDirty(0), dirty);
    QCOMPARE(window_->activeDocumentIndex(), active);
    QCOMPARE(view->history_count(), history);
}

void FileMenuTest::scriptsBrowseIsDisabled()
{
    QAction* action = window_->registry()->action(QStringLiteral("file.scripts.browse"));
    QVERIFY(action != nullptr);
    QCOMPARE(action->text(), QStringLiteral("Browse…"));
    QVERIFY(!action->isEnabled());
}

PICTURA_QT_TEST_MAIN(FileMenuTest)
#include "tst_file_menu.moc"
