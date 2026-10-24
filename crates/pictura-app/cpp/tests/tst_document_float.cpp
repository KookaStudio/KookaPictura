// Floating document windows (#317): moving a window between the tab strip and
// a float keeps its view, tear-out and dock-back, the header and its menu,
// keys in a top-level float, and how the screen modes and panel controls treat
// floats. Each test function builds its own frame. Geometry checks force the
// in-frame hosting; the checks named "top-level" run with the platform's own.

#include <QtTest/QtTest>

#include <QtCore/QLineF>
#include <QtCore/QTimer>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>

#include "commands.h"
#include "dialogs.h"
#include "document_float_window.h"
#include "frame.h"
#include "image_size_dialog.h"
#include "image_view.h"
#include "panels/panel_column.h"
#include "tool_options.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

using ScreenMode = PicturaMainWindow::ScreenMode;

QAction* invertAction(const PicturaMainWindow& window)
{
    return window.registry()->action(commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Invert")}));
}

void sendMouse(QWidget* target, QEvent::Type type, const QPoint& global, Qt::MouseButton button,
               Qt::MouseButtons held)
{
    QMouseEvent event(type, QPointF(target->mapFromGlobal(global)), QPointF(global), button,
                      held, Qt::NoModifier);
    QApplication::sendEvent(target, &event);
}

// Press on `target` at `from`, move to `to` in ten steps, and release there.
void dragTo(QWidget* target, const QPoint& from, const QPoint& to)
{
    sendMouse(target, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
    for (int i = 1; i <= 10; ++i) {
        sendMouse(target, QEvent::MouseMove, from + (to - from) * i / 10, Qt::NoButton,
                  Qt::LeftButton);
    }
    sendMouse(target, QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
}

// The image point at the canvas centre.
QPointF viewCentre(const ImageView* canvas)
{
    return canvas->widgetToImage(QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
}

QStringList menuEntries(const QMenu* menu)
{
    QStringList entries;
    for (const QAction* action : menu->actions()) {
        entries << (action->isSeparator() ? QStringLiteral("-") : action->text());
    }
    return entries;
}

QAction* menuAction(const QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
    }
    return nullptr;
}

} // namespace

class DocumentFloatTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void hostMoveKeepsView();
    void inactiveFloatRepaints();
    void tearOutFloats();
    void tearOutLastTab();
    void sidewaysDragReorders();
    void dragHeaderDocks();
    void floatHeaderShowsMarker();
    void floatHostingFollowsPlatform();
    void floatCloseClosesDocument();
    void headerMenuEntries();
    void headerMenuTargetsItsWindow();
    void undoShortcutInFloat();
    void menuShortcutInFloat();
    void toolShortcutInFloat();
    void frameKeyInFloat();
    void fullScreenShowsOnlyActive();
    void fullScreenFloatFillsArea();
    void standardModeRestoresLayout();
    void tabKeepsFloatingDocuments();
    void workspaceSwitchKeepsFloats();

private:
    bool newDoc(const QString& name, int width, int height);
    QTabBar* tabBar() const;
    QTabWidget* tabWidget() const;
    QStackedWidget* documentArea() const;
    // Float document `index` in a top-level window and wait until it is the
    // active window; null when the platform does not activate it.
    DocumentFloatWindow* floatTopLevel(int index);
    QMenu* headerMenu(DocumentFloatWindow* floating);
    void closeAll();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

bool DocumentFloatTest::newDoc(const QString& name, int width, int height)
{
    return window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                                QStringLiteral("white"));
}

QTabBar* DocumentFloatTest::tabBar() const
{
    return window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

QTabWidget* DocumentFloatTest::tabWidget() const
{
    return window_->findChild<QTabWidget*>(QStringLiteral("documentTabs"));
}

QStackedWidget* DocumentFloatTest::documentArea() const
{
    return window_->findChild<QStackedWidget*>(QStringLiteral("documentArea"));
}

DocumentFloatWindow* DocumentFloatTest::floatTopLevel(int index)
{
    PanelFloat::setForceChildOverlayForTest(false);
    if (!window_->floatDocumentForTest(index)) {
        return nullptr;
    }
    DocumentFloatWindow* floating = window_->floatWindowForTest(index);
    if (!floating || !floating->isWindow()) {
        return nullptr;
    }
    floating->activateWindow();
    if (!QTest::qWaitForWindowActive(floating)) {
        return nullptr;
    }
    return floating;
}

QMenu* DocumentFloatTest::headerMenu(DocumentFloatWindow* floating)
{
    QWidget* header = floating->header();
    const QPoint local(8, header->height() / 2);
    QContextMenuEvent event(QContextMenuEvent::Mouse, local, header->mapToGlobal(local));
    QApplication::sendEvent(header, &event);
    return window_->documentMenuForTest();
}

void DocumentFloatTest::closeAll()
{
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
}

void DocumentFloatTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    setUnsavedPromptInteractive(false);
}

void DocumentFloatTest::init()
{
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    PanelFloat::setForceChildOverlayForTest(true);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void DocumentFloatTest::cleanup()
{
    if (window_) {
        if (QMenu* menu = window_->documentMenuForTest()) {
            menu->close();
        }
        window_->setScreenMode(ScreenMode::Standard);
        window_->setPanelsHidden(false);
        window_->setActiveTool(ToolId::Move);
        window_->cancelAllOpens();
        closeAll();
    }
    window_.reset();
    PanelFloat::setForceChildOverlayForTest(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentFloatTest::hostMoveKeepsView()
{
    QVERIFY(newDoc(QStringLiteral("View"), 400, 300));
    QCoreApplication::processEvents();
    ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setZoom(2.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    canvas->panBy(QPointF(-30, -20));
    const double zoom = canvas->zoom();
    const QPointF centre = viewCentre(canvas);
    const int history = window_->viewAt(0)->history_index();

    QVERIFY(window_->floatDocumentForTest(0));
    QVERIFY(window_->floatWindowForTest(0) != nullptr);
    QCoreApplication::processEvents();
    QCOMPARE(window_->imageView(), canvas);
    QCOMPARE(canvas->zoom(), zoom);
    QVERIFY2(QLineF(viewCentre(canvas), centre).length() <= 1.0, "centre kept when floated");
    QCOMPARE(window_->viewAt(0)->history_index(), history);

    QVERIFY(window_->dockDocumentForTest(0, 0));
    QCoreApplication::processEvents();
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QCOMPARE(canvas->zoom(), zoom);
    QVERIFY2(QLineF(viewCentre(canvas), centre).length() <= 1.0, "centre kept when docked");
    QCOMPARE(window_->viewAt(0)->history_index(), history);
}

void DocumentFloatTest::inactiveFloatRepaints()
{
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    window_->setActiveDocumentIndex(1);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    PictureView* floating = window_->viewAt(0);
    ImageView* canvas = window_->canvasAt(0);
    QVERIFY(floating && canvas);
    const quint64 before = pictura::frame_revision(*floating);
    QVERIFY(image_adjust_direct(*floating, QStringLiteral("invert")));
    window_->refresh();
    const quint64 after = pictura::frame_revision(*floating);
    QVERIFY(after != before);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QCOMPARE(canvas->property("picturaFrameRevision").toULongLong(), after);
}

void DocumentFloatTest::tearOutFloats()
{
    QVERIFY(newDoc(QStringLiteral("First"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Second"), 32, 32));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    const QPoint from = bar->mapToGlobal(bar->tabRect(1).center());
    const QPoint to = bar->mapToGlobal(QPoint(bar->tabRect(1).center().x(), bar->height() + 120));
    dragTo(bar, from, to);

    QTRY_VERIFY(window_->floatWindowForTest(1) != nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(1), -1);
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QCOMPARE(bar->count(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentFloatTest::tearOutLastTab()
{
    QVERIFY(newDoc(QStringLiteral("Only"), 32, 32));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    const QPoint from = bar->mapToGlobal(bar->tabRect(0).center());
    const QPoint to = bar->mapToGlobal(QPoint(bar->tabRect(0).center().x(), bar->height() + 120));
    dragTo(bar, from, to);

    QTRY_VERIFY(window_->floatWindowForTest(0) != nullptr);
    QCOMPARE(bar->count(), 0);
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 0);
}

void DocumentFloatTest::sidewaysDragReorders()
{
    QVERIFY(newDoc(QStringLiteral("One"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Two"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Three"), 32, 32));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    const QPoint from = bar->mapToGlobal(bar->tabRect(0).center());
    const QPoint to = bar->mapToGlobal(bar->tabRect(2).center());
    dragTo(bar, from, to);

    QTRY_VERIFY(window_->tabOfDocumentForTest(0) != 0);
    QCOMPARE(bar->count(), 3);
    for (int i = 0; i < 3; ++i) {
        QVERIFY(window_->floatWindowForTest(i) == nullptr);
    }
}

void DocumentFloatTest::dragHeaderDocks()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(2));
    DocumentFloatWindow* floating = window_->floatWindowForTest(2);
    QVERIFY(floating != nullptr);
    window_->setActiveDocumentIndex(0);
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->count(), 2);

    QWidget* header = floating->header();
    const QPoint from = header->mapToGlobal(QPoint(header->width() / 2, header->height() / 2));
    const QPoint to = bar->mapToGlobal(bar->tabRect(1).center());
    dragTo(header, from, to);

    QTRY_COMPARE(window_->tabOfDocumentForTest(2), 1);
    QCOMPARE(bar->count(), 3);
    QVERIFY(window_->floatWindowForTest(2) == nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 2);
}

void DocumentFloatTest::floatHeaderShowsMarker()
{
    QVERIFY(newDoc(QStringLiteral("Marked"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    QVERIFY(floating != nullptr);
    // Let the float's first layout settle the view's zoom, which the label shows.
    QCoreApplication::processEvents();
    window_->refresh();
    QCOMPARE(floating->headerText(), window_->documentWindowLabelForTest(0));

    window_->registry()->refresh();
    QAction* invert = invertAction(*window_);
    QVERIFY(invert && invert->isEnabled());
    invert->trigger();
    QVERIFY(window_->isDocumentDirty(0));
    QCOMPARE(floating->headerText(),
             window_->documentWindowLabelForTest(0) + QStringLiteral(" *"));
}

void DocumentFloatTest::floatHostingFollowsPlatform()
{
    QVERIFY(newDoc(QStringLiteral("Hosted"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* child = window_->floatWindowForTest(0);
    QVERIFY(child != nullptr);
    QVERIFY(!child->isWindow());
    QCOMPARE(child->parentWidget(), static_cast<QWidget*>(window_.get()));
    QVERIFY(window_->dockDocumentForTest(0, 0));

    PanelFloat::setForceChildOverlayForTest(false);
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* topLevel = window_->floatWindowForTest(0);
    QVERIFY(topLevel != nullptr);
    QCOMPARE(topLevel->isWindow(), PanelFloat::overlayUsesTopLevel());
    if (topLevel->isWindow()) {
        QCOMPARE(topLevel->windowType(), Qt::Tool);
        QCOMPARE(topLevel->parentWidget(), static_cast<QWidget*>(window_.get()));
    }
}

void DocumentFloatTest::floatCloseClosesDocument()
{
    QVERIFY(newDoc(QStringLiteral("Closing"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    QVERIFY(floating != nullptr);
    QTest::mouseClick(floating->closeButton(), Qt::LeftButton);

    QTRY_COMPARE(window_->documentCount(), 0);
    QVERIFY(window_->workspaceOpenArmed());
    QTRY_VERIFY(window_->findChildren<DocumentFloatWindow*>().isEmpty());
}

void DocumentFloatTest::headerMenuEntries()
{
    QVERIFY(newDoc(QStringLiteral("Menu"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    QMenu* menu = headerMenu(window_->floatWindowForTest(0));
    QVERIFY(menu != nullptr);
    QCOMPARE(menuEntries(menu),
             QStringList({QStringLiteral("Duplicate…"), QStringLiteral("Image Size…"),
                          QStringLiteral("Canvas Size…"), QStringLiteral("-"),
                          QStringLiteral("File Info…"), QStringLiteral("-"),
                          QStringLiteral("Print…")}));
    menu->close();
}

void DocumentFloatTest::headerMenuTargetsItsWindow()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 48, 40));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    window_->setActiveDocumentIndex(0);
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QMenu* menu = headerMenu(floating);
    QVERIFY(menu != nullptr);
    QAction* imageSize = menuAction(menu, QStringLiteral("Image Size…"));
    QVERIFY(imageSize && imageSize->isEnabled());

    int dialogTarget = -1;
    auto* poll = new QTimer(this);
    poll->setInterval(20);
    connect(poll, &QTimer::timeout, this, [this, poll, &dialogTarget]() {
        for (ImageSizeDialog* dialog : window_->findChildren<ImageSizeDialog*>()) {
            if (dialog->isVisible()) {
                dialogTarget = window_->activeDocumentIndex();
                poll->stop();
                dialog->reject();
                return;
            }
        }
    });
    poll->start();
    menu->close();
    imageSize->trigger();
    poll->stop();
    poll->deleteLater();

    QCOMPARE(dialogTarget, 1);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentFloatTest::undoShortcutInFloat()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Floating"), 16, 16));
    DocumentFloatWindow* floating = floatTopLevel(1);
    QVERIFY2(floating != nullptr, "the top-level float becomes the active window");
    QCOMPARE(window_->activeDocumentIndex(), 1);
    window_->registry()->refresh();
    invertAction(*window_)->trigger();
    PictureView* view = window_->viewAt(1);
    PictureView* other = window_->viewAt(0);
    const int edited = view->history_index();
    const int otherHistory = other->history_index();

    QTest::keyClick(window_->canvasAt(1), Qt::Key_Z, Qt::ControlModifier);
    QCOMPARE(view->history_index(), edited - 1);
    QCOMPARE(other->history_index(), otherHistory);
}

void DocumentFloatTest::menuShortcutInFloat()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Floating"), 16, 16));
    DocumentFloatWindow* floating = floatTopLevel(1);
    QVERIFY2(floating != nullptr, "the top-level float becomes the active window");
    QCOMPARE(QApplication::activeWindow(), static_cast<QWidget*>(floating));
    QVERIFY(!window_->viewAt(0)->has_selection());
    QVERIFY(!window_->viewAt(1)->has_selection());

    QTest::keyClick(window_->canvasAt(1), Qt::Key_A, Qt::ControlModifier);
    QVERIFY(window_->viewAt(1)->has_selection());
    QVERIFY(!window_->viewAt(0)->has_selection());
}

void DocumentFloatTest::toolShortcutInFloat()
{
    QVERIFY(newDoc(QStringLiteral("Floating"), 16, 16));
    window_->setActiveTool(ToolId::Move);
    DocumentFloatWindow* floating = floatTopLevel(0);
    QVERIFY2(floating != nullptr, "the top-level float becomes the active window");
    QTest::keyClick(window_->canvasAt(0), Qt::Key_B);
    QCOMPARE(window_->activeTool(), ToolId::Brush);
}

void DocumentFloatTest::frameKeyInFloat()
{
    QVERIFY(newDoc(QStringLiteral("Floating"), 64, 64));
    window_->setActiveTool(ToolId::Brush);
    auto* tools = window_->findChild<ToolController*>();
    QVERIFY(tools != nullptr);
    const PaintOptions* paint = tools->optionsFor<PaintOptions>(ToolId::Brush);
    QVERIFY(paint != nullptr);
    DocumentFloatWindow* floating = floatTopLevel(0);
    QVERIFY2(floating != nullptr, "the top-level float becomes the active window");
    ImageView* canvas = window_->canvasAt(0);
    const int size = paint->size();
    QTest::keyClick(canvas, Qt::Key_BracketRight);
    QVERIFY(paint->size() > size);

    // Space reaches the frame's own key handling only through the float.
    QTest::keyPress(canvas, Qt::Key_Space);
    QVERIFY(canvas->spacePanForTest());
    QTest::keyRelease(canvas, Qt::Key_Space);
    QVERIFY(!canvas->spacePanForTest());
}

void DocumentFloatTest::fullScreenShowsOnlyActive()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("FloatA"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("FloatB"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    QVERIFY(window_->floatDocumentForTest(2));
    window_->setActiveDocumentIndex(1);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    DocumentFloatWindow* other = window_->floatWindowForTest(2);
    QVERIFY(other != nullptr);

    window_->setScreenMode(ScreenMode::FullWithMenuBar);
    QCoreApplication::processEvents();
    QVERIFY(window_->canvasAt(1)->isVisible());
    QVERIFY(!other->isVisible());
    QVERIFY(!tabWidget()->isVisible());
}

void DocumentFloatTest::fullScreenFloatFillsArea()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    window_->setScreenMode(ScreenMode::Full);
    QStackedWidget* area = documentArea();
    QVERIFY(area != nullptr);
    ImageView* canvas = window_->canvasAt(1);
    QWidget* page = canvas;
    while (page && page->parentWidget() != area) {
        page = page->parentWidget();
    }
    QVERIFY2(page != nullptr, "the float's canvas host is in the document area");
    QCOMPARE(area->currentWidget(), page);
    QTRY_COMPARE(page->size(), area->size());
    QVERIFY(canvas->isVisible());
    QVERIFY(!floating->isVisible());
    QVERIFY(!floating->header()->isVisible());
    QVERIFY(!tabWidget()->isVisible());
}

void DocumentFloatTest::standardModeRestoresLayout()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("FloatA"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("FloatB"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    QVERIFY(window_->floatDocumentForTest(2));
    DocumentFloatWindow* first = window_->floatWindowForTest(1);
    DocumentFloatWindow* second = window_->floatWindowForTest(2);
    QVERIFY(first && second);
    const QRect firstGeometry = first->geometry();
    const QRect secondGeometry = second->geometry();

    window_->setScreenMode(ScreenMode::FullWithMenuBar);
    window_->setScreenMode(ScreenMode::Standard);
    QCoreApplication::processEvents();
    QVERIFY(first->isVisible());
    QVERIFY(second->isVisible());
    QCOMPARE(first->geometry(), firstGeometry);
    QCOMPARE(second->geometry(), secondGeometry);
    QVERIFY(first->isAncestorOf(window_->canvasAt(1)));
    QVERIFY(second->isAncestorOf(window_->canvasAt(2)));
    QVERIFY(tabWidget()->isVisible());
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QCOMPARE(documentArea()->currentWidget(), static_cast<QWidget*>(tabWidget()));
}

void DocumentFloatTest::tabKeepsFloatingDocuments()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    window_->setPanelsHidden(false);
    const QRect geometry = floating->geometry();

    QTest::keyClick(window_->canvasAt(1), Qt::Key_Tab);
    QVERIFY(window_->panelsHidden());
    QVERIFY(floating->isVisible());
    QCOMPARE(floating->geometry(), geometry);
    QCOMPARE(window_->floatWindowForTest(1), floating);
}

void DocumentFloatTest::workspaceSwitchKeepsFloats()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    const QRect geometry = floating->geometry();

    window_->switchWorkspaceForTest(QStringLiteral("Painting"));
    QCoreApplication::processEvents();
    QCOMPARE(window_->floatWindowForTest(1), floating);
    QVERIFY(floating->isVisible());
    QCOMPARE(floating->geometry(), geometry);
    QCOMPARE(tabBar()->count(), 1);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<DocumentFloatTest>(argc, argv);
}

#include "tst_document_float.moc"
