// File > Open accepts a multi-file selection (#1118): every chosen path opens
// as its own document, and a file that fails to open does not stop the rest.

#include <QtTest/QtTest>

#include <optional>

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QPointer>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QVector>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QFileSystemModel>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>

#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "api/paint_bridge.h"
#include "api/view_bridge.h"
#include "color_settings_dialog.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "frame_place.h"
#include "image_view.h"
#include "preferences_dialog.h"
#include "profile_warning_dialog.h"
#include "tools.h"

#include "multi_file_test_support.h"
#include "qt_test_support.h"

using namespace pictura;

namespace {

QStringList documentPaths(const PicturaMainWindow& window)
{
    QStringList paths;
    for (int i = 0; i < window.documentCount(); ++i) {
        paths.append(window.documentPath(i));
    }
    paths.sort();
    return paths;
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
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);

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
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);

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
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);

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
    auto window = makeShownWindow();
    QVERIFY(window != nullptr);

    queueOpenDialogInteraction(window.get(), {first, broken, last}, &probe);
    window->showOpenDialog();

    QVERIFY2(probe.error.isEmpty(), qPrintable(probe.error));
    QCOMPARE(int(probe.selected.size()), 3);
    QTRY_COMPARE(window->documentCount(), 2);
    QTRY_VERIFY(!window->isLoadingDocument(0) && !window->isLoadingDocument(1));
    const QStringList expected = {first, last};
    QCOMPARE(documentPaths(*window), expected);
}

PICTURA_QT_TEST_MAIN(MultiFileOpenTest)
#include "tst_multi_file_open.moc"
