// The file seam: an ImportSaveCoordinator can save and reopen a document with
// a DocumentRegistry and no PicturaMainWindow.

#include <QtTest/QtTest>

#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QApplication>
#include <QtWidgets/QWidget>

#include <memory>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "canvas_scrollbars.h"
#include "document_registry.h"
#include "image_view.h"
#include "import_save_coordinator.h"
#include "qt_test_support.h"

namespace {

pictura::DocumentRegistry::Hooks testHooks(QWidget* shell,
                                           pictura::ImportSaveCoordinator** coordinator)
{
    pictura::DocumentRegistry::Hooks hooks;
    hooks.shellWidget = [shell]() -> QWidget* { return shell; };
    hooks.openDocumentsAsTabs = []() { return true; };
    hooks.floatingDocumentDocking = []() { return true; };
    hooks.fullScreen = []() { return false; };
    hooks.requestRefresh = []() {};
    hooks.requestPanelRefresh = []() {};
    hooks.stopPanelRefreshTimer = []() {};
    hooks.activateShellWindow = []() {};
    hooks.raisePanelOverlays = []() {};
    hooks.closeWindowRequested = [](pictura::DocWindow*) {};
    hooks.setupCanvas = [shell](pictura::DocWindow& window) {
        window.canvas = new pictura::ImageView(shell);
        window.host = new pictura::CanvasScrollBars(shell);
        window.host->setView(window.canvas);
        pictura::PictureView* view = window.doc->view;
        if (view->has_document()) {
            window.canvas->setDocument(
                QSize(view->document_width(), view->document_height()));
        } else {
            window.canvas->replaceImage(view->image());
        }
    };
    hooks.wireCanvas = [](pictura::DocWindow&) {};
    hooks.wireDocument = [](pictura::DocEntry&) {};
    hooks.applyViewSettings = [](pictura::PictureView*) {};
    hooks.wireOpenSignals = [coordinator](pictura::PictureView* view) {
        if (*coordinator) {
            (*coordinator)->wireOpenSignals(view);
        }
    };
    hooks.rememberRecent = [coordinator](const QString& path) {
        if (*coordinator) {
            (*coordinator)->rememberRecent(path);
        }
    };
    hooks.forgetToolView = [](pictura::PictureView*) {};
    hooks.forgetCanvas = [](pictura::ImageView*) {};
    hooks.openDelayForTest = []() -> quint64 { return 0; };
    hooks.loadingPageColor = []() { return QColor(Qt::white); };
    hooks.scrollAllWindows = []() { return false; };
    hooks.toolCanvas = []() -> pictura::ImageView* { return nullptr; };
    return hooks;
}

pictura::PictureView* newView(QWidget* parent)
{
    auto* view = new pictura::PictureView(parent);
    if (!pictura::document_api::newDocument(*view, 8, 6, QStringLiteral("rgb"), 8,
                                            QStringLiteral("white"))) {
        delete view;
        return nullptr;
    }
    return view;
}

} // namespace

class ImportSaveCoordinatorTest : public QObject {
    Q_OBJECT

private slots:
    void saveOpenAndRemember();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ImportSaveCoordinatorTest::saveOpenAndRemember()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("coordinator.psd"));

    QWidget shell;
    pictura::ImportSaveCoordinator* coordinator = nullptr;
    auto registry = std::make_unique<pictura::DocumentRegistry>(
        testHooks(&shell, &coordinator), &shell);

    pictura::ImportSaveCoordinator::Hooks hooks;
    hooks.applyColorSettings = [](pictura::PictureView*) {};
    hooks.requestRefresh = []() {};
    hooks.updateWindowTitle = []() {};
    hooks.showStatus = [](const QString&, int) {};
    hooks.recordCommand = [](const QString&, const QJsonObject&) {};
    auto save = std::make_unique<pictura::ImportSaveCoordinator>(*registry, &shell, hooks);
    coordinator = save.get();

    pictura::PictureView* view = newView(&shell);
    QVERIFY(view);
    QVERIFY(registry->addDocument(view, QString(), true) >= 0);
    QCOMPARE(registry->count(), 1);

    QVERIFY(save->saveActiveAs(path));
    QVERIFY2(QFileInfo::exists(path), "the PSD is written");
    QCOMPARE(save->recentFiles().value(0), path);

    const QString shown = QFileInfo(path).fileName();
    QVERIFY2(save->openPath(path), "the saved PSD reopens");
    QCOMPARE(registry->count(), 2);
    QCOMPARE(registry->documentName(1), shown);
    QVERIFY(save->recentFiles().contains(path));
    QCOMPARE(save->recentFiles().value(0), path);
}

QTEST_MAIN(ImportSaveCoordinatorTest)
#include "tst_import_save_coordinator.moc"
