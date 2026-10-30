// Floating document windows (#317): moving a window between the tab strip and
// a float keeps its view, tear-out and dock-back, Ctrl+drag and double-click
// docking, the Wayland window drag, the dock zone and its dock line, the header
// and its menu, keys in a top-level float, and how the screen modes and panel
// controls treat floats. The suite shares one frame; cleanup() returns it to
// no documents, the Standard screen mode, Essentials, and docking on. Geometry
// checks force the in-frame hosting; the checks named "top-level" or "under
// Wayland" run top-level floats. Offscreen runs no platform drag: the dock
// drag hook receives the drag data, and synthetic drag events stand in for
// the compositor's.

#include <QtTest/QtTest>

#include <QtCore/QDataStream>
#include <QtCore/QLineF>
#include <QtCore/QMimeData>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QWindow>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>

#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "document_float_window.h"
#include "frame.h"
#include "image_size_dialog.h"
#include "image_view.h"
#include "panels/panel_column.h"
#include "preferences_dialog.h"
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
               Qt::MouseButtons held, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QMouseEvent event(type, QPointF(target->mapFromGlobal(global)), QPointF(global), button,
                      held, modifiers);
    QApplication::sendEvent(target, &event);
}

// Send one drag event of `type` (enter, move, or drop) with `mime` to `target`
// at `pos`; whether it was accepted.
bool sendDrag(QWidget* target, QEvent::Type type, const QPoint& pos, const QMimeData* mime)
{
    if (type == QEvent::DragEnter) {
        QDragEnterEvent enter(pos, Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(target, &enter);
        return enter.isAccepted();
    }
    if (type == QEvent::DragMove) {
        QDragMoveEvent move(pos, Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(target, &move);
        return move.isAccepted();
    }
    QDropEvent drop(QPointF(pos), Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(target, &drop);
    return drop.isAccepted();
}

// The keys QtWayland reads to attach a top-level to a drag, as QDockWidget
// writes them.
const QString kCarriedWindowMime = QStringLiteral("application/x-qt-mainwindowdrag-window");
const QString kCarriedPositionMime = QStringLiteral("application/x-qt-mainwindowdrag-position");

QWindow* carriedWindow(const QMimeData* mime)
{
    if (!mime->hasFormat(kCarriedWindowMime)) {
        return nullptr;
    }
    QDataStream stream(mime->data(kCarriedWindowMime));
    qintptr address = 0;
    stream >> address;
    return reinterpret_cast<QWindow*>(address);
}

QPoint carriedGrabOffset(const QMimeData* mime)
{
    QDataStream stream(mime->data(kCarriedPositionMime));
    QPoint offset;
    stream >> offset;
    return offset;
}

// Drag `mime` over `target` at `pos` and drop it there; whether the drop was
// accepted.
bool dropOn(QWidget* target, const QPoint& pos, const QMimeData* mime)
{
    QDragEnterEvent enter(pos, Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(target, &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(target, &move);
    QDropEvent drop(QPointF(pos), Qt::MoveAction, mime, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(target, &drop);
    return drop.isAccepted();
}

const QString kDockingPref = QStringLiteral("enableFloatingDocumentDocking");

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
    void cleanupTestCase();

    void hostMoveKeepsView();
    void inactiveFloatRepaints();
    void tearOutFloats();
    void tearOutLastTab();
    void sidewaysDragReorders();
    void dragHeaderDocks();
    void tearOutUnderWayland();
    void headerDragCarriesWindow();
    void tearOutDragDocksOnStrip();
    void dockIndicatorBetweenTabs();
    void dockIndicatorWithoutTabs();
    void dockDropWithoutTabs();
    void dockIndicatorHidesOnLeave();
    void handDragShowsIndicator();
    void dockZoneInactiveTiledOrOff();
    void ctrlDragDocksAtDrop();
    void ctrlDragRefusedWithoutDocking();
    void ctrlDragElsewhereKeepsFloat();
    void fileDropOnStripOpensFile();
    void headerDoubleClickDocks();
    void imageSizeKeepsWindowCentre();
    void inactiveCanvasShowsCursor();
    void floatHeaderShowsMarker();
    void floatHostingFollowsPlatform();
    void floatTopLevelUnderWayland();
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
    // Ctrl+drag `floating`'s header past the drag threshold and release; the
    // drag data the frame would have started a drag with, or null.
    std::unique_ptr<QMimeData> ctrlDragHeader(DocumentFloatWindow* floating);
    // Drag `floating`'s header 40 px down with `modifiers` held and release;
    // the drag data the frame would have started a drag with, or null.
    std::unique_ptr<QMimeData> dragHeader(DocumentFloatWindow* floating,
                                          Qt::KeyboardModifiers modifiers);
    // Drag tab `tab` 120 px below the strip and release; the drag data the
    // tear-out would have started a drag with, or null.
    std::unique_ptr<QMimeData> tearOutDrag(int tab);
    // The frame's dock line, or null before it first showed.
    QWidget* dockLine() const;
    bool dockLineShown() const;
    // Turn Enable Floating Document Window Docking on or off as the
    // Preferences dialog does.
    bool setDocking(bool on);
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

std::unique_ptr<QMimeData> DocumentFloatTest::ctrlDragHeader(DocumentFloatWindow* floating)
{
    return dragHeader(floating, Qt::ControlModifier);
}

std::unique_ptr<QMimeData> DocumentFloatTest::dragHeader(DocumentFloatWindow* floating,
                                                         Qt::KeyboardModifiers modifiers)
{
    std::unique_ptr<QMimeData> carried;
    window_->setWindowDockDragForTest(
        [&carried](std::unique_ptr<QMimeData> mime) { carried = std::move(mime); });
    QWidget* header = floating->header();
    const QPoint from = header->mapToGlobal(QPoint(header->width() / 2, header->height() / 2));
    const QPoint to = from + QPoint(0, 40);
    sendMouse(header, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton, modifiers);
    sendMouse(header, QEvent::MouseMove, to, Qt::NoButton, Qt::LeftButton, modifiers);
    sendMouse(header, QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton, modifiers);
    window_->setWindowDockDragForTest(nullptr);
    return carried;
}

std::unique_ptr<QMimeData> DocumentFloatTest::tearOutDrag(int tab)
{
    std::unique_ptr<QMimeData> carried;
    window_->setWindowDockDragForTest(
        [&carried](std::unique_ptr<QMimeData> mime) { carried = std::move(mime); });
    QTabBar* bar = tabBar();
    const QPoint from = bar->mapToGlobal(bar->tabRect(tab).center());
    const QPoint to =
        bar->mapToGlobal(QPoint(bar->tabRect(tab).center().x(), bar->height() + 120));
    dragTo(bar, from, to);
    window_->setWindowDockDragForTest(nullptr);
    return carried;
}

QWidget* DocumentFloatTest::dockLine() const
{
    return window_->findChild<QWidget*>(QStringLiteral("documentDockIndicator"));
}

bool DocumentFloatTest::dockLineShown() const
{
    const QWidget* line = dockLine();
    return line && line->isVisible();
}

bool DocumentFloatTest::setDocking(bool on)
{
    window_->registry()->dispatch(QString::fromLatin1(command_ids::EditPreferencesInterface));
    QCoreApplication::processEvents();
    PreferencesDialog* prefs = window_->preferencesDialog();
    if (!prefs || !prefs->setCheckboxForTest(kDockingPref, on)) {
        return false;
    }
    prefs->close();
    return window_->floatingDocumentDocking() == on;
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
    PanelFloat::setForceChildOverlayForTest(true);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
}

void DocumentFloatTest::init()
{
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    PanelFloat::setForceChildOverlayForTest(true);
    QVERIFY(window_ != nullptr);
    QCOMPARE(window_->documentCount(), 0);
    // A screen mode or a top-level float of the last function may have left
    // the frame resized or inactive.
    window_->resize(1100, 700);
    QTRY_COMPARE(window_->size(), QSize(1100, 700));
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void DocumentFloatTest::cleanup()
{
    if (window_) {
        if (QMenu* menu = window_->documentMenuForTest()) {
            menu->close();
        }
        if (!window_->floatingDocumentDocking()) {
            setDocking(true);
        }
        window_->setWindowDockDragForTest(nullptr);
        window_->setScreenMode(ScreenMode::Standard);
        window_->setPanelsHidden(false);
        if (window_->activeWorkspaceForTest() != QStringLiteral("Essentials")) {
            window_->switchWorkspaceForTest(QStringLiteral("Essentials"));
        }
        window_->setActiveTool(ToolId::Move);
        window_->cancelAllOpens();
        closeAll();
        // A synthetic drag that ended without a drop leaves Qt's drag manager
        // targeting a widget of this frame; a leave clears it, or the next
        // test's drag events reach a freed float.
        QDragLeaveEvent leave;
        QCoreApplication::sendEvent(window_.get(), &leave);
        // A drag move that ended without a leave or a drop keeps the dock line.
        if (QWidget* line = dockLine()) {
            line->hide();
        }
        QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    }
    PanelFloat::setForceChildOverlayForTest(false);
    DocumentFloatWindow::setWindowDragForTest(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentFloatTest::cleanupTestCase()
{
    window_.reset();
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
    // Invert patches a region rather than rebuilding the frame, so the check
    // is what the inactive float actually paints.
    const QPoint centre = canvas->imageToWidget(QPointF(16.0, 16.0)).toPoint();
    const QColor before = canvas->grab().toImage().pixelColor(centre);
    QVERIFY(image_adjust_direct(*floating, QStringLiteral("invert")));
    window_->refresh();
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QTRY_VERIFY(canvas->grab().toImage().pixelColor(centre) != before);
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

// With the window drag forced, the tear-out floats a top-level and hands the
// rest of the press to a drag that carries it, held by its header's centre.
void DocumentFloatTest::tearOutUnderWayland()
{
    QVERIFY(newDoc(QStringLiteral("First"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Second"), 32, 32));
    PanelFloat::setForceChildOverlayForTest(false);
    DocumentFloatWindow::setWindowDragForTest(true);
    std::unique_ptr<QMimeData> mime = tearOutDrag(1);

    QTRY_VERIFY(window_->floatWindowForTest(1) != nullptr);
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating->isWindow());
    QVERIFY(floating->isVisible());
    QCOMPARE(window_->tabOfDocumentForTest(0), 0);
    QCOMPARE(tabBar()->count(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QVERIFY2(mime != nullptr, "the tear-out starts a drag");
    QVERIFY(mime->hasFormat(QString::fromLatin1(kDocumentWindowMimeType)));
    QVERIFY(floating->windowHandle() != nullptr);
    QCOMPARE(carriedWindow(mime.get()), floating->windowHandle());
    QCOMPARE(carriedGrabOffset(mime.get()),
             QPoint(floating->width() / 2, DocumentFloatWindow::kHeaderHeight / 2));
}

void DocumentFloatTest::headerDragCarriesWindow()
{
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    PanelFloat::setForceChildOverlayForTest(false);
    DocumentFloatWindow::setWindowDragForTest(true);
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    QVERIFY(floating != nullptr && floating->isWindow());
    QVERIFY(floating->windowHandle() != nullptr);
    const QRect geometry = floating->geometry();
    QWidget* header = floating->header();
    const QPoint press = header->mapTo(floating, QPoint(header->width() / 2, header->height() / 2));

    std::unique_ptr<QMimeData> mime = dragHeader(floating, Qt::NoModifier);
    QVERIFY2(mime != nullptr, "a plain header drag starts a drag");
    QVERIFY(mime->hasFormat(QString::fromLatin1(kDocumentWindowMimeType)));
    QCOMPARE(carriedWindow(mime.get()), floating->windowHandle());
    QCOMPARE(carriedGrabOffset(mime.get()), press);
    QCOMPARE(floating->geometry(), geometry);
    QCOMPARE(window_->floatWindowForTest(0), floating);
}

void DocumentFloatTest::tearOutDragDocksOnStrip()
{
    QVERIFY(newDoc(QStringLiteral("First"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Second"), 32, 32));
    PanelFloat::setForceChildOverlayForTest(false);
    DocumentFloatWindow::setWindowDragForTest(true);
    std::unique_ptr<QMimeData> mime = tearOutDrag(1);
    QVERIFY(mime != nullptr);
    QTRY_VERIFY(window_->floatWindowForTest(1) != nullptr);

    QTabBar* bar = tabBar();
    QCOMPARE(bar->count(), 1);
    QVERIFY2(dropOn(bar, bar->tabRect(0).center(), mime.get()), "the strip takes the drop");
    QTRY_COMPARE(window_->tabOfDocumentForTest(1), 0);
    QCOMPARE(bar->count(), 2);
    QVERIFY(window_->floatWindowForTest(1) == nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentFloatTest::dockIndicatorBetweenTabs()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(2));
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(window_->floatWindowForTest(2));
    QVERIFY(mime != nullptr);
    QTabBar* bar = tabBar();
    QCOMPARE(bar->count(), 2);

    const QPoint at = bar->tabRect(1).center();
    QVERIFY(sendDrag(bar, QEvent::DragEnter, at, mime.get()));
    QVERIFY(sendDrag(bar, QEvent::DragMove, at, mime.get()));
    QVERIFY2(dockLineShown(), "the dock line shows over the strip");
    QWidget* line = dockLine();
    QCOMPARE(line->width(), 2);
    QVERIFY(line->height() >= bar->height());
    const QPoint lineAt = line->mapToGlobal(QPoint(0, 0));
    QCOMPARE(lineAt.x(), bar->mapToGlobal(bar->tabRect(1).topLeft()).x() - 1);
    QCOMPARE(lineAt.y(), tabWidget()->mapToGlobal(QPoint(0, 0)).y());
}

void DocumentFloatTest::dockIndicatorWithoutTabs()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    QVERIFY(window_->floatDocumentForTest(1));
    QCOMPARE(tabBar()->count(), 0);
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(window_->floatWindowForTest(1));
    QVERIFY(mime != nullptr);

    QTabWidget* tabs = tabWidget();
    QStackedWidget* area = documentArea();
    const int band = qMax(tabBar()->sizeHint().height(), DocumentFloatWindow::kHeaderHeight);
    const QPoint at(tabs->width() / 2, band / 2);
    QVERIFY(sendDrag(tabs, QEvent::DragEnter, at, mime.get()));
    QVERIFY(sendDrag(tabs, QEvent::DragMove, at, mime.get()));
    QVERIFY2(dockLineShown(), "the dock line shows over the empty strip's band");
    QWidget* line = dockLine();
    QCOMPARE(line->height(), 2);
    QCOMPARE(line->width(), area->width());
    const QRect bandRect(area->mapToGlobal(QPoint(0, 0)), QSize(area->width(), band));
    QVERIFY(bandRect.contains(QRect(line->mapToGlobal(QPoint(0, 0)), line->size())));
}

void DocumentFloatTest::dockDropWithoutTabs()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    QVERIFY(window_->floatDocumentForTest(1));
    QCOMPARE(tabBar()->count(), 0);
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(window_->floatWindowForTest(1));
    QVERIFY(mime != nullptr);

    QTabWidget* tabs = tabWidget();
    const QPoint at(tabs->width() / 2, DocumentFloatWindow::kHeaderHeight / 2);
    QVERIFY2(dropOn(tabs, at, mime.get()), "the empty strip's band takes the drop");
    QVERIFY(!dockLineShown());
    QTRY_COMPARE(window_->tabOfDocumentForTest(1), 0);
    QCOMPARE(tabBar()->count(), 1);
    QVERIFY(window_->floatWindowForTest(1) == nullptr);
    QVERIFY(window_->floatWindowForTest(0) != nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentFloatTest::dockIndicatorHidesOnLeave()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(floating);
    QVERIFY(mime != nullptr);

    QTabBar* bar = tabBar();
    const QPoint at = bar->tabRect(0).center();
    QVERIFY(sendDrag(bar, QEvent::DragEnter, at, mime.get()));
    QVERIFY(sendDrag(bar, QEvent::DragMove, at, mime.get()));
    QVERIFY(dockLineShown());
    QDragLeaveEvent leave;
    QApplication::sendEvent(bar, &leave);
    QVERIFY2(!dockLineShown(), "the dock line hides when the drag leaves");
    QCOMPARE(window_->floatWindowForTest(1), floating);
}

// In-frame hosting with the window drag off: the header drag moves the float
// by hand, as on X11.
void DocumentFloatTest::handDragShowsIndicator()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(2));
    DocumentFloatWindow* floating = window_->floatWindowForTest(2);
    QVERIFY(floating != nullptr);
    window_->setActiveDocumentIndex(0);
    QTabBar* bar = tabBar();

    QWidget* header = floating->header();
    const QPoint from = header->mapToGlobal(QPoint(header->width() / 2, header->height() / 2));
    const QPoint to = bar->mapToGlobal(bar->tabRect(1).center());
    sendMouse(header, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
    for (int i = 1; i <= 10; ++i) {
        sendMouse(header, QEvent::MouseMove, from + (to - from) * i / 10, Qt::NoButton,
                  Qt::LeftButton);
    }
    QVERIFY2(dockLineShown(), "the dock line shows while the hand drag is over the strip");
    QCOMPARE(dockLine()->width(), 2);
    sendMouse(header, QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
    QVERIFY2(!dockLineShown(), "the dock line hides on the release");
    QTRY_COMPARE(window_->tabOfDocumentForTest(2), 1);
}

void DocumentFloatTest::dockZoneInactiveTiledOrOff()
{
    QVERIFY(setDocking(false));
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(2));
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(window_->floatWindowForTest(2));
    QVERIFY(mime != nullptr);
    QTabBar* bar = tabBar();
    const QPoint at = bar->tabRect(0).center();
    QVERIFY2(!sendDrag(bar, QEvent::DragEnter, at, mime.get()), "no dock zone with docking off");
    QVERIFY(!sendDrag(bar, QEvent::DragMove, at, mime.get()));
    QVERIFY(!dockLineShown());

    QVERIFY(setDocking(true));
    QVERIFY(window_->registry()->dispatch(QStringLiteral("window.arrange.tile")));
    QCoreApplication::processEvents();
    QVERIFY(window_->tilePaneForTest(0) != nullptr);
    if (!window_->floatWindowForTest(2)) {
        QVERIFY(window_->floatDocumentForTest(2));
    }
    QVERIFY(window_->floatWindowForTest(2) != nullptr);
    QTabWidget* tabs = tabWidget();
    const QPoint band(tabs->width() / 2, DocumentFloatWindow::kHeaderHeight / 2);
    QVERIFY2(!sendDrag(tabs, QEvent::DragEnter, band, mime.get()), "no dock zone while tiled");
    QVERIFY(!sendDrag(tabs, QEvent::DragMove, band, mime.get()));
    QVERIFY(!sendDrag(tabs, QEvent::Drop, band, mime.get()));
    QVERIFY(!dockLineShown());
    QCoreApplication::processEvents();
    QVERIFY(window_->floatWindowForTest(2) != nullptr);
}

void DocumentFloatTest::ctrlDragDocksAtDrop()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(2));
    DocumentFloatWindow* floating = window_->floatWindowForTest(2);
    QVERIFY(floating != nullptr);
    window_->setActiveDocumentIndex(0);
    const QRect geometry = floating->geometry();

    std::unique_ptr<QMimeData> mime = ctrlDragHeader(floating);
    QVERIFY2(mime != nullptr, "a Ctrl+drag on the header starts a drag");
    QVERIFY(mime->hasFormat(QString::fromLatin1(kDocumentWindowMimeType)));
    QVERIFY2(!mime->hasFormat(kCarriedWindowMime), "a Ctrl+drag carries no window");
    QCOMPARE(floating->geometry(), geometry);
    QCOMPARE(window_->floatWindowForTest(2), floating);

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(dropOn(bar, bar->tabRect(1).center(), mime.get()), "the strip takes the drop");
    QTRY_COMPARE(window_->tabOfDocumentForTest(2), 1);
    QCOMPARE(bar->count(), 3);
    QVERIFY(window_->floatWindowForTest(2) == nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 2);
}

void DocumentFloatTest::ctrlDragRefusedWithoutDocking()
{
    QVERIFY(setDocking(false));
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(floating);
    QVERIFY(mime != nullptr);

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(!dropOn(bar, bar->tabRect(0).center(), mime.get()), "the strip refuses the drop");
    QCoreApplication::processEvents();
    QCOMPARE(window_->floatWindowForTest(1), floating);
    QCOMPARE(window_->tabOfDocumentForTest(1), -1);
    QCOMPARE(bar->count(), 1);
}

void DocumentFloatTest::ctrlDragElsewhereKeepsFloat()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    const QRect geometry = floating->geometry();
    std::unique_ptr<QMimeData> mime = ctrlDragHeader(floating);
    QVERIFY(mime != nullptr);

    ImageView* canvas = window_->canvasAt(0);
    QVERIFY(canvas != nullptr);
    QVERIFY2(!dropOn(canvas, canvas->rect().center(), mime.get()), "a canvas refuses the drop");
    QCoreApplication::processEvents();
    QCOMPARE(window_->floatWindowForTest(1), floating);
    QCOMPARE(floating->geometry(), geometry);
    QCOMPARE(tabBar()->count(), 1);
}

void DocumentFloatTest::fileDropOnStripOpensFile()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("dropped.png"));
    QImage image(8, 8, QImage::Format_ARGB32);
    image.fill(Qt::red);
    QVERIFY(image.save(png));
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    const QRect geometry = floating->geometry();

    QMimeData mime;
    mime.setUrls({QUrl::fromLocalFile(png)});
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(dropOn(bar, bar->tabRect(0).center(), &mime), "the strip takes a file drop");
    window_->waitForOpensForTest();
    QTRY_COMPARE(window_->documentCount(), 3);
    QTRY_VERIFY(!window_->isLoadingDocument(2));
    QCOMPARE(window_->documentName(2), QStringLiteral("dropped.png"));
    QCOMPARE(window_->floatWindowForTest(1), floating);
    QCOMPARE(floating->geometry(), geometry);
}

void DocumentFloatTest::headerDoubleClickDocks()
{
    QVERIFY(setDocking(false));
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(1));
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);
    window_->setActiveDocumentIndex(0);
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QWidget* header = floating->header();
    const QPoint at = header->mapToGlobal(QPoint(header->width() / 2, header->height() / 2));
    sendMouse(header, QEvent::MouseButtonDblClick, at, Qt::LeftButton, Qt::LeftButton);

    QTRY_VERIFY(window_->floatWindowForTest(1) == nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(1), 1);
    QCOMPARE(tabBar()->count(), 2);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

// Image Size resamples every window of the document around the point at its
// centre, so a floating window keeps a shrunken image centred.
void DocumentFloatTest::imageSizeKeepsWindowCentre()
{
    QVERIFY(newDoc(QStringLiteral("Resized"), 400, 300));
    PanelFloat::setForceChildOverlayForTest(true);
    QVERIFY(window_->floatDocumentForTest(0));
    QCoreApplication::processEvents();
    ImageView* canvas = window_->canvasAt(0);
    QVERIFY(canvas != nullptr);
    QCOMPARE(viewCentre(canvas), QPointF(200.0, 150.0));

    CommandParams params;
    params.values.insert(QStringLiteral("width"), 100);
    params.values.insert(QStringLiteral("height"), 75);
    params.values.insert(QStringLiteral("resample"), QStringLiteral("bicubic"));
    QVERIFY(window_->registry()->executeRecorded(
        commandIdForPath({QStringLiteral("Image"), QStringLiteral("Image Size…")}), params));
    QCOMPARE(canvas->documentSize(), QSize(100, 75));
    const QPointF centre = viewCentre(canvas);
    QVERIFY2(qAbs(centre.x() - 50.0) <= 1.0 && qAbs(centre.y() - 37.5) <= 1.0,
             qPrintable(QStringLiteral("centre %1,%2").arg(centre.x()).arg(centre.y())));
}

// The brush ring stands in for the pointer only on the active canvas; a float
// left behind shows a cursor again.
void DocumentFloatTest::inactiveCanvasShowsCursor()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 64, 64));
    QVERIFY(newDoc(QStringLiteral("Floating"), 64, 64));
    PanelFloat::setForceChildOverlayForTest(true);
    QVERIFY(window_->floatDocumentForTest(1));
    window_->setActiveTool(ToolId::Brush);
    window_->setActiveDocumentIndex(1);
    QCoreApplication::processEvents();
    ImageView* floating = window_->canvasAt(1);
    QCOMPARE(floating->cursor().shape(), Qt::BlankCursor);

    window_->setActiveDocumentIndex(0);
    QCoreApplication::processEvents();
    QVERIFY(floating->cursor().shape() != Qt::BlankCursor);
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
    QVERIFY(topLevel->isWindow());
    QCOMPARE(topLevel->windowType(), Qt::Tool);
    QVERIFY(topLevel->windowFlags().testFlag(Qt::FramelessWindowHint));
    QCOMPARE(topLevel->parentWidget(), static_cast<QWidget*>(window_.get()));
}

// With the window drag forced the float is hosted as on Wayland, and a header
// drag carries it to a drop on the strip.
void DocumentFloatTest::floatTopLevelUnderWayland()
{
    QVERIFY(newDoc(QStringLiteral("A"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("B"), 32, 32));
    QVERIFY(newDoc(QStringLiteral("C"), 32, 32));
    PanelFloat::setForceChildOverlayForTest(false);
    DocumentFloatWindow::setWindowDragForTest(true);
    QVERIFY(DocumentFloatWindow::movesByWindowDrag());
    QVERIFY(window_->floatDocumentForTest(2));
    DocumentFloatWindow* floating = window_->floatWindowForTest(2);
    QVERIFY(floating != nullptr);
    QVERIFY(floating->isWindow());
    QCOMPARE(floating->windowType(), Qt::Tool);
    QVERIFY(floating->windowHandle() != nullptr);
    QCOMPARE(floating->windowHandle()->transientParent(), window_->windowHandle());
    window_->setActiveDocumentIndex(0);

    std::unique_ptr<QMimeData> mime = dragHeader(floating, Qt::NoModifier);
    QVERIFY2(mime != nullptr, "the header drag carries the window");
    QCOMPARE(carriedWindow(mime.get()), floating->windowHandle());
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(dropOn(bar, bar->tabRect(1).center(), mime.get()), "the strip takes the drop");
    QTRY_COMPARE(window_->tabOfDocumentForTest(2), 1);
    QVERIFY(window_->floatWindowForTest(2) == nullptr);
    QCOMPARE(window_->activeDocumentIndex(), 2);
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
    // Offscreen, a parented Qt::Tool window reports active through its parent
    // while QApplication keeps the frame active, so the shortcut map never sees
    // the float as the active window; the reviewer checklist covers it.
    if (QApplication::activeWindow() != floating) {
        QSKIP("the offscreen platform does not activate a Qt::Tool float");
    }
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
