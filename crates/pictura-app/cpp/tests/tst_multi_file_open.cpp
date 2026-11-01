// File > Open accepts a multi-file selection (#1118): every chosen path opens
// as its own document, and a file that fails to open does not stop the rest.

#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QFileSystemModel>
#include <QtGui/QImage>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QTabBar>

#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "preferences_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

struct OpenDialogProbe {
    bool seen = false;
    QFileDialog::FileMode mode = QFileDialog::AnyFile;
    QStringList selected;
    QString error;
};

QFileDialog* visibleFileDialog()
{
    // The dialog is parented to the frame, so it is not in topLevelWidgets().
    for (QWidget* widget : QApplication::allWidgets()) {
        if (auto* dialog = qobject_cast<QFileDialog*>(widget); dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

// QFileSystemModel populates a folder asynchronously, so wait until the view
// shows the chosen folder before selecting in it.
bool waitForFiles(QFileSystemModel* model, QAbstractItemView* list, const QStringList& paths)
{
    for (int waited = 0; waited < 5000; waited += 20) {
        bool ready = list->rootIndex().isValid() && model->rowCount(list->rootIndex()) > 0;
        for (int i = 0; ready && i < paths.size(); ++i) {
            ready = model->index(paths.at(i)).isValid();
        }
        if (ready) {
            return true;
        }
        QTest::qWait(20); // QFileSystemModel populates the folder asynchronously.
    }
    return false;
}

// Select `paths` in the open dialog's file list and accept it.
void chooseFiles(QFileDialog* dialog, const QStringList& paths, OpenDialogProbe* probe)
{
    dialog->setDirectory(QFileInfo(paths.first()).absolutePath());
    auto* list = dialog->findChild<QAbstractItemView*>(QStringLiteral("listView"));
    auto* model = list ? qobject_cast<QFileSystemModel*>(list->model()) : nullptr;
    if (!list || !model || !list->selectionModel()) {
        probe->error = QStringLiteral("the open dialog has no file list");
        dialog->reject();
        return;
    }
    if (!waitForFiles(model, list, paths)) {
        probe->error = QStringLiteral("the open dialog did not show the folder");
        dialog->reject();
        return;
    }
    QItemSelection selection;
    for (const QString& path : paths) {
        const QModelIndex index = model->index(path);
        if (!index.isValid()) {
            probe->error = QStringLiteral("no index for %1").arg(path);
            dialog->reject();
            return;
        }
        selection.select(index, index);
    }
    list->selectionModel()->select(selection,
                                   QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
    probe->selected = dialog->selectedFiles();
    // QFileDialog overrides accept() as protected; access is checked on the
    // static type while virtual dispatch still runs the dialog's override.
    static_cast<QDialog*>(dialog)->accept();
}

// Schedule the interaction with the dialog that File > Open is about to show.
// The watchdog rejects a dialog that never gets chosen, so a failed probe fails
// the test instead of hanging it; `context` cancels both timers with the window.
void queueOpenDialogInteraction(QWidget* context, const QStringList& paths,
                                OpenDialogProbe* probe)
{
    QTimer::singleShot(0, context, [paths, probe] {
        // showOpenDialog() shows the dialog after this timer is queued; poll so
        // a slow platform cannot lose the interaction before it is visible.
        for (int waited = 0; waited < 5000; waited += 10) {
            QFileDialog* dialog = visibleFileDialog();
            if (dialog) {
                probe->seen = true;
                probe->mode = dialog->fileMode();
                chooseFiles(dialog, paths, probe);
                return;
            }
            QTest::qWait(10); // Let the dialog finish being shown.
        }
        probe->error = QStringLiteral("no open dialog appeared");
    });
    QTimer::singleShot(10000, context, [probe] {
        if (QFileDialog* dialog = visibleFileDialog()) {
            probe->error = QStringLiteral("the open dialog did not accept");
            dialog->reject();
        }
    });
}

QStringList documentPaths(const PicturaMainWindow& window)
{
    QStringList paths;
    for (int i = 0; i < window.documentCount(); ++i) {
        paths.append(window.documentPath(i));
    }
    paths.sort();
    return paths;
}

bool writePng(const QString& path, const QColor& color)
{
    QImage image(8, 8, QImage::Format_ARGB32);
    image.fill(color);
    return image.save(path, "PNG");
}

bool writeBrokenPsd(const QString& path)
{
    QFile file(path);
    const QByteArray bytes("not a Photoshop file");
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

} // namespace

class MultiFileOpenTest : public QObject {
    Q_OBJECT

private slots:
    void multiSelectOpensOneDocumentPerFile();
    void multipleSelectionFollowsTheTabsPreference();
    void singleSelectionOpensOneDocument();
    void failedFileDoesNotStopTheRest();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void MultiFileOpenTest::multiSelectOpensOneDocumentPerFile()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString red = dir.filePath(QStringLiteral("red.png"));
    const QString green = dir.filePath(QStringLiteral("green.png"));
    const QString blue = dir.filePath(QStringLiteral("blue.png"));
    QVERIFY(writePng(red, Qt::red));
    QVERIFY(writePng(green, Qt::green));
    QVERIFY(writePng(blue, Qt::blue));

    OpenDialogProbe probe;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QTest::qWait(0);

    queueOpenDialogInteraction(window.get(), {red, green, blue}, &probe);
    window->showOpenDialog();

    QVERIFY2(probe.error.isEmpty(), qPrintable(probe.error));
    QVERIFY(probe.seen);
    QCOMPARE(int(probe.mode), int(QFileDialog::ExistingFiles));
    QCOMPARE(int(probe.selected.size()), 3);
    QTRY_COMPARE(window->documentCount(), 3);
    QTRY_VERIFY(!window->isLoadingDocument(0) && !window->isLoadingDocument(1)
                && !window->isLoadingDocument(2));
    const QStringList expected = {blue, green, red};
    QCOMPARE(documentPaths(*window), expected);
    auto* tabs = window->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(tabs != nullptr);
    QCOMPARE(tabs->count(), 3);
}

void MultiFileOpenTest::multipleSelectionFollowsTheTabsPreference()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("float-first.png"));
    const QString second = dir.filePath(QStringLiteral("float-second.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writePng(second, Qt::blue));

    OpenDialogProbe probe;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QTest::qWait(0);

    window->registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesInterface));
    QCoreApplication::processEvents();
    PreferencesDialog* prefs = window->preferencesDialog();
    QVERIFY(prefs != nullptr);
    QVERIFY(prefs->setCheckboxForTest(QStringLiteral("openDocumentsAsTabs"), false));
    QVERIFY(!window->openDocumentsAsTabs());

    queueOpenDialogInteraction(window.get(), {first, second}, &probe);
    window->showOpenDialog();

    QVERIFY2(probe.error.isEmpty(), qPrintable(probe.error));
    QTRY_COMPARE(window->documentCount(), 2);
    QTRY_VERIFY(!window->isLoadingDocument(0) && !window->isLoadingDocument(1));
    QTRY_VERIFY(window->floatWindowForTest(0) != nullptr
                && window->floatWindowForTest(1) != nullptr);
    auto* tabs = window->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(tabs != nullptr);
    QCOMPARE(tabs->count(), 0);
}

void MultiFileOpenTest::singleSelectionOpensOneDocument()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("single.png"));
    QVERIFY(writePng(path, Qt::red));

    OpenDialogProbe probe;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QTest::qWait(0);

    queueOpenDialogInteraction(window.get(), {path}, &probe);
    window->showOpenDialog();

    QVERIFY2(probe.error.isEmpty(), qPrintable(probe.error));
    QCOMPARE(int(probe.selected.size()), 1);
    QTRY_COMPARE(window->documentCount(), 1);
    QTRY_VERIFY(!window->isLoadingDocument(0));
    QCOMPARE(window->documentPath(0), path);
}

void MultiFileOpenTest::failedFileDoesNotStopTheRest()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = dir.filePath(QStringLiteral("first.png"));
    const QString broken = dir.filePath(QStringLiteral("broken.psd"));
    const QString last = dir.filePath(QStringLiteral("last.png"));
    QVERIFY(writePng(first, Qt::red));
    QVERIFY(writeBrokenPsd(broken));
    QVERIFY(writePng(last, Qt::blue));

    OpenDialogProbe probe;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QTest::qWait(0);

    queueOpenDialogInteraction(window.get(), {first, broken, last}, &probe);
    window->showOpenDialog();

    QVERIFY2(probe.error.isEmpty(), qPrintable(probe.error));
    QCOMPARE(int(probe.selected.size()), 3);
    QTRY_COMPARE(window->documentCount(), 2);
    QTRY_VERIFY(!window->isLoadingDocument(0) && !window->isLoadingDocument(1));
    const QStringList expected = {first, last};
    QCOMPARE(documentPaths(*window), expected);
}

QTEST_MAIN(MultiFileOpenTest)
#include "tst_multi_file_open.moc"
