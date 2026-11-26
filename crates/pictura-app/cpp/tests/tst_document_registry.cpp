// The document registry seam: documents and their windows can be driven
// without the main window, and the frame's hooks forward to the registry.

#include <QtTest/QtTest>

#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QWidget>

#include <memory>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "canvas_scrollbars.h"
#include "document_float_window.h"
#include "document_registry.h"
#include "document_tile_host.h"
#include "image_view.h"
#include "frame.h"
#include "qt_test_support.h"

namespace {

pictura::DocumentRegistry::Hooks testHooks(QWidget* shell)
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
    hooks.wireOpenSignals = [](pictura::PictureView*) {};
    hooks.rememberRecent = [](const QString&) {};
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

class DocumentRegistryTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void addActivateFloatDockTileRemove();
    void frameForwardsDocuments();
    void removingADocumentNotifiesOwners();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void DocumentRegistryTest::init() {}
void DocumentRegistryTest::cleanup() {}

void DocumentRegistryTest::addActivateFloatDockTileRemove()
{
    QWidget shell;
    auto registry = std::make_unique<pictura::DocumentRegistry>(testHooks(&shell), &shell);

    int documentsChanged = 0;
    int activeChanged = 0;
    QObject::connect(registry.get(), &pictura::DocumentRegistry::documentsChanged,
                     registry.get(), [&]() { ++documentsChanged; });
    QObject::connect(registry.get(), &pictura::DocumentRegistry::activeDocumentChanged,
                     registry.get(), [&]() { ++activeChanged; });

    for (int i = 0; i < 3; ++i) {
        pictura::PictureView* view = newView(&shell);
        QVERIFY(view);
        QVERIFY(registry->addDocument(view, QString(), true) >= 0);
    }
    QCOMPARE(registry->count(), 3);
    QVERIFY(documentsChanged >= 3);
    QVERIFY(activeChanged >= 1);
    registry->setActiveIndex(1);
    QCOMPARE(registry->activeIndex(), 1);

    // Reorder the tabs: the document list keeps its index order, the window
    // cycle order follows the strip.
    pictura::DocEntry* first = registry->docs().at(0);
    registry->tabs()->tabBar()->moveTab(0, 2);
    QCOMPARE(registry->docs().at(0), first);
    QCOMPARE(registry->windows().size(), 3);
    QCOMPARE(registry->windows().at(2), first->windows.first());

    QVERIFY(registry->floatDocumentForTest(0));
    QVERIFY(registry->docs().at(0)->windows.first()->kind
            == pictura::DocWindow::Host::Float);
    QVERIFY(registry->dockDocumentForTest(0, -1));
    QVERIFY(registry->docs().at(0)->windows.first()->kind
            == pictura::DocWindow::Host::Tab);
    QVERIFY(registry->tileWindow(registry->docs().at(0)->windows.first(), true));
    QVERIFY(registry->docs().at(0)->windows.first()->kind
            == pictura::DocWindow::Host::Tile);
    registry->untileWindow(registry->docs().at(0)->windows.first());

    registry->removeDocument(0);
    QCOMPARE(registry->count(), 2);
}

void DocumentRegistryTest::frameForwardsDocuments()
{
    auto frame = pictura::test::makeMainWindow();
    QVERIFY(frame);
    pictura::DocumentRegistry* registry = frame->documentRegistry();
    QVERIFY(registry);
    const int before = frame->documentCount();

    pictura::PictureView* view = newView(frame.get());
    QVERIFY(view);
    QVERIFY(registry->addDocument(view, QString(), true) >= 0);
    QCOMPARE(frame->documentCount(), before + 1);
    QCOMPARE(frame->viewAt(before), view);
    QCOMPARE(frame->activeView(), view);

    registry->removeDocument(before);
    QCOMPARE(frame->documentCount(), before);
}

void DocumentRegistryTest::removingADocumentNotifiesOwners()
{
    QWidget shell;
    auto registry = std::make_unique<pictura::DocumentRegistry>(testHooks(&shell), &shell);
    pictura::PictureView* view = newView(&shell);
    QVERIFY(view);
    QVERIFY(registry->addDocument(view, QString(), true) >= 0);

    bool notified = false;
    bool aliveDuringSignal = false;
    QObject::connect(registry.get(), &pictura::DocumentRegistry::aboutToRemoveDocument,
                     registry.get(), [&](int index) {
                         notified = true;
                         aliveDuringSignal = QPointer<pictura::PictureView>(view) != nullptr
                                             && registry->docAt(index) != nullptr;
                     });
    registry->removeDocument(0);
    QVERIFY(notified);
    QVERIFY(aliveDuringSignal);
    QCOMPARE(registry->count(), 0);
}

PICTURA_QT_TEST_MAIN(DocumentRegistryTest)
#include "tst_document_registry.moc"
