// The document tab's and the floating window header's context menus, tearing
// a tab out of the strip into a floating window, and docking a floating window
// by drag-and-drop, with the dock line that shows where it lands.

#include "frame_includes.h"

#include <QtCore/QDataStream>
#include <QtCore/QIODevice>
#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtGui/QDrag>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QGuiApplication>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtGui/QWindow>
#include <QtWidgets/QApplication>

#include <utility>

namespace pictura {

namespace {

// The dock line's thickness.
constexpr int kDockLineWidth = 2;

template <typename T>
QByteArray streamed(const T& value)
{
    QByteArray data;
    QDataStream stream(&data, QIODevice::WriteOnly);
    stream << value;
    return data;
}

// A dock drag's payload: the window's address, compared with the registry and
// never dereferenced until a lookup vouches for it. A `carried` top-level is
// named under the keys QDockWidget uses on Wayland, encoded as it encodes
// them: QtWayland then attaches that window to the drag (xdg_toplevel_drag_v1)
// held at `grabOffset`, and the compositor moves it with the pointer.
std::unique_ptr<QMimeData> windowDockMime(const DocWindow* window, QWindow* carried,
                                          const QPoint& grabOffset)
{
    auto mime = std::make_unique<QMimeData>();
    mime->setData(kDocumentWindowMimeType,
                  QByteArray::number(quint64(reinterpret_cast<quintptr>(window))));
    if (carried) {
        mime->setData(QStringLiteral("application/x-qt-mainwindowdrag-window"),
                      streamed(reinterpret_cast<qintptr>(carried)));
        mime->setData(QStringLiteral("application/x-qt-mainwindowdrag-position"),
                      streamed(grabOffset));
    }
    return mime;
}

} // namespace

void PicturaMainWindow::wireDocumentMenus()
{
    QTabBar* bar = tabs_->tabBar();
    bar->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(bar, &QWidget::customContextMenuRequested, this,
            [this](const QPoint& pos) { showTabMenu(pos); });
}

QMenu* PicturaMainWindow::documentMenuForTest() const
{
    return documentMenu_;
}

void PicturaMainWindow::showTabMenu(const QPoint& barPos)
{
    QTabBar* bar = tabs_->tabBar();
    DocWindow* clicked = windowAtTab(bar->tabAt(barPos));
    if (!clicked) {
        return;
    }
    if (documentMenu_) {
        documentMenu_->close();
    }
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("documentTabMenu"));
    menu->setAttribute(Qt::WA_DeleteOnClose);
    // Every entry acts on the clicked tab's window, not the active one; the
    // window may be gone by the time an entry is chosen.
    const auto add = [this, menu, clicked](const QString& text, bool enabled,
                                           std::function<void(DocWindow*)> run) {
        QAction* action = menu->addAction(text);
        action->setEnabled(enabled);
        connect(action, &QAction::triggered, this, [this, clicked, run = std::move(run)]() {
            if (windows_.contains(clicked)) {
                run(clicked);
            }
        });
    };
    add(tr("Consolidate All to Here"), true,
        [this](DocWindow* window) { consolidateAllToTabs(window); });
    add(tr("Close"), true, [this](DocWindow* window) { closeWindow(window, true); });
    add(tr("Close All"), true, [this](DocWindow*) {
        registry_->dispatch(QString::fromLatin1(command_ids::FileCloseAll));
    });
    add(tr("Close Others"), windows_.size() > 1,
        [this](DocWindow* window) { closeOtherWindows(window); });
    add(tr("Move to New Window"), !clicked->doc->loading,
        [this](DocWindow* window) { floatWindow(window); });
    menu->addSeparator();
    add(tr("New Document…"), true,
        [this](DocWindow*) { registry_->dispatch(QString::fromLatin1(command_ids::FileNew)); });
    add(tr("Open Document…"), true,
        [this](DocWindow*) { registry_->dispatch(QString::fromLatin1(command_ids::FileOpen)); });
    menu->addSeparator();
    add(tr("Reveal in File Manager"), !clicked->doc->path.isEmpty(),
        [this](DocWindow* window) { revealInFileManager(window); });
    documentMenu_ = menu;
    menu->popup(bar->mapToGlobal(barPos));
}

void PicturaMainWindow::showHeaderMenu(DocWindow* window, const QPoint& globalPos)
{
    if (documentMenu_) {
        documentMenu_->close();
    }
    // The entries are the menu-bar commands, enabled as they are while this
    // window is active, so the right-click activates it first.
    activateDocWindow(window);
    registry_->refresh();
    struct Entry {
        QString text;
        QString id;
    };
    const QList<Entry> entries = {
        {tr("Duplicate…"), QString::fromLatin1(command_ids::ImageDuplicate)},
        {tr("Image Size…"), commandIdForPath({QStringLiteral("Image"), QStringLiteral("Image Size…")})},
        {tr("Canvas Size…"),
         commandIdForPath({QStringLiteral("Image"), QStringLiteral("Canvas Size…")})},
        {QString(), QString()},
        {tr("File Info…"), QString::fromLatin1(command_ids::FileInfo)},
        {QString(), QString()},
        {tr("Print…"), commandIdForPath({QStringLiteral("File"), QStringLiteral("Print…")})},
    };
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("documentHeaderMenu"));
    menu->setAttribute(Qt::WA_DeleteOnClose);
    for (const Entry& entry : entries) {
        if (entry.id.isEmpty()) {
            menu->addSeparator();
            continue;
        }
        QAction* source = registry_->action(entry.id);
        QAction* action = menu->addAction(entry.text);
        action->setEnabled(source && source->isEnabled());
        connect(action, &QAction::triggered, this, [this, window, id = entry.id]() {
            if (!windows_.contains(window)) {
                return;
            }
            activateDocWindow(window);
            registry_->refresh();
            if (QAction* command = registry_->action(id); command && command->isEnabled()) {
                registry_->dispatch(id);
            }
        });
    }
    documentMenu_ = menu;
    menu->popup(globalPos);
}

bool PicturaMainWindow::tearOutFilter(QEvent* event)
{
    if (tearSynthesizing_) {
        return false;
    }
    QTabBar* bar = tabs_->tabBar();
    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        tearFloat_ = nullptr;
        tearWindow_ = nullptr;
        if (mouse->button() == Qt::LeftButton) {
            tearWindow_ = windowAtTab(bar->tabAt(mouse->position().toPoint()));
            tearPressPos_ = mouse->position().toPoint();
        }
        return false;
    }
    case QEvent::MouseMove: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const QPoint globalPos = mouse->globalPosition().toPoint();
        if (tearFloat_) {
            tearFloat_->dragTo(globalPos);
            return true;
        }
        if (!tearWindow_ || !(mouse->buttons() & Qt::LeftButton)
            || !windows_.contains(tearWindow_) || tearWindow_->doc->loading) {
            return false;
        }
        // Only a pull across the strip tears out; a drag along it reorders.
        const int y = mouse->position().toPoint().y();
        const int beyond = y < 0 ? -y : y - (bar->height() - 1);
        if (beyond <= QApplication::startDragDistance() * 3) {
            return false;
        }
        DocWindow* window = tearWindow_;
        tearWindow_ = nullptr;
        // End the strip's own tab move at the press point first, so the tab
        // does not stay half-dragged when it leaves the strip.
        tearSynthesizing_ = true;
        QMouseEvent release(QEvent::MouseButtonRelease, QPointF(tearPressPos_),
                            QPointF(bar->mapToGlobal(tearPressPos_)), Qt::LeftButton,
                            Qt::NoButton, mouse->modifiers());
        QCoreApplication::sendEvent(bar, &release);
        tearSynthesizing_ = false;
        // The float opens with its header under the pointer and takes over
        // the drag.
        QRect geometry = defaultFloatGeometry();
        geometry.moveTopLeft(globalPos
                             - QPoint(geometry.width() / 2, DocumentFloatWindow::kHeaderHeight / 2));
        if (!floatWindow(window, geometry)) {
            return true;
        }
        DocumentFloatWindow* floating = window->floating;
        // Where a client can't move its own top-level (Wayland), the rest of
        // the press becomes the drag that carries the new window, held by its
        // header's centre as placed above. The float is shown, so its
        // top-level exists, and the drag starts once this filter returns.
        if (floating->isWindow() && DocumentFloatWindow::movesByWindowDrag()) {
            startWindowDockDrag(
                window, true,
                QPoint(floating->width() / 2, DocumentFloatWindow::kHeaderHeight / 2));
            return true;
        }
        tearFloat_ = floating;
        tearFloat_->beginDrag(globalPos);
        return true;
    }
    case QEvent::MouseButtonRelease: {
        tearWindow_ = nullptr;
        if (!tearFloat_) {
            return false;
        }
        const QPointer<DocumentFloatWindow> floating = tearFloat_;
        tearFloat_ = nullptr;
        floating->endDrag(static_cast<QMouseEvent*>(event)->globalPosition().toPoint());
        return true;
    }
    default:
        return false;
    }
}

DocWindow* PicturaMainWindow::windowOfDockMime(const QMimeData* mime) const
{
    if (!mime || !mime->hasFormat(kDocumentWindowMimeType)) {
        return nullptr;
    }
    const quint64 address = mime->data(kDocumentWindowMimeType).toULongLong();
    for (DocWindow* window : windows_) {
        if (quint64(reinterpret_cast<quintptr>(window)) == address) {
            return window;
        }
    }
    return nullptr;
}

void PicturaMainWindow::startWindowDockDrag(DocWindow* window, bool carryWindow,
                                            const QPoint& grabOffset)
{
    if (!windows_.contains(window) || !window->floating) {
        return;
    }
    if (windowDockDragForTest_) {
        windowDockDragForTest_(windowDockMime(
            window, carryWindow ? window->floating->windowHandle() : nullptr, grabOffset));
        return;
    }
    // A carried window is its own drag image.
    const QPixmap preview = carryWindow ? QPixmap() : window->floating->header()->grab();
    // The drag runs from its own event-loop turn, not under the header's or
    // the tab strip's mouse handler; by then the button may be up.
    QTimer::singleShot(0, this, [this, window, carryWindow, grabOffset, preview]() {
        if (!windows_.contains(window) || window->kind != DocWindow::Host::Float
            || !window->floating || !(QGuiApplication::mouseButtons() & Qt::LeftButton)) {
            return;
        }
        // ponytail: a compositor without xdg_toplevel_drag_v1 runs a plain
        // drag-and-drop, so the carried window stays put until the drop; the
        // dock zone and the header double-click still dock it.
        QWindow* carried = carryWindow ? window->floating->windowHandle() : nullptr;
        auto* drag = new QDrag(this);
        drag->setMimeData(windowDockMime(window, carried, grabOffset).release());
        if (!preview.isNull()) {
            drag->setPixmap(preview);
        }
        windowDockDragRunning_ = true;
        drag->exec(Qt::MoveAction);
        windowDockDragRunning_ = false;
        hideDockIndicator();
        applyPendingDock();
    });
}

bool PicturaMainWindow::windowDropFilter(QObject* watched, QEvent* event)
{
    const QEvent::Type type = event->type();
    // A leave carries no drag data; the line goes whatever the drag was.
    if (type == QEvent::DragLeave) {
        hideDockIndicator();
        return false;
    }
    if (type != QEvent::DragEnter && type != QEvent::DragMove && type != QEvent::Drop) {
        return false;
    }
    auto* drop = static_cast<QDropEvent*>(event);
    if (!drop->mimeData() || !drop->mimeData()->hasFormat(kDocumentWindowMimeType)) {
        return false;
    }
    DocWindow* window = windowOfDockMime(drop->mimeData());
    const bool dockable = window && window->kind == DocWindow::Host::Float;
    const QPoint globalPos =
        static_cast<QWidget*>(watched)->mapToGlobal(drop->position().toPoint());
    const bool inZone = dockable && updateDockIndicator(globalPos);
    // The tab widget takes a dockable drag in on entry, so its moves keep
    // arriving until the pointer reaches the dock zone.
    const bool entering =
        type == QEvent::DragEnter && dockable && !dockZoneGlobalRect().isEmpty();
    if (type == QEvent::Drop) {
        hideDockIndicator();
        if (inZone) {
            // Docking deletes the float, which a running platform drag may
            // still carry, so the dock waits for the drag to finish.
            pendingDockWindow_ = window;
            pendingDockIndex_ = dockIndexAt(globalPos);
            QTimer::singleShot(0, this, [this]() { applyPendingDock(); });
        }
    }
    if (!inZone && !entering) {
        drop->ignore();
        return true;
    }
    drop->setDropAction(Qt::MoveAction);
    drop->accept();
    return true;
}

bool PicturaMainWindow::updateDockIndicator(const QPoint& globalPos)
{
    const QRect zone = dockZoneGlobalRect();
    if (!zone.contains(globalPos)) {
        hideDockIndicator();
        return false;
    }
    if (!dockIndicator_) {
        // The same accent line as the panel drop indicators; the theme colours
        // it by its object name.
        dockIndicator_ = new QWidget(this);
        dockIndicator_->setObjectName(QStringLiteral("documentDockIndicator"));
        dockIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    QTabBar* bar = tabs_->tabBar();
    QRect line;
    if (bar->isVisible() && bar->count() > 0) {
        const int index = dockIndexAt(globalPos);
        const int x = index >= 0 ? bar->tabRect(index).left()
                                 : bar->tabRect(bar->count() - 1).right() + 1;
        line = QRect(bar->mapToGlobal(QPoint(x, 0)).x() - kDockLineWidth / 2, zone.top(),
                     kDockLineWidth, zone.height());
    } else {
        line = QRect(zone.left(), zone.bottom() - kDockLineWidth + 1, zone.width(),
                     kDockLineWidth);
    }
    dockIndicator_->setGeometry(QRect(mapFromGlobal(line.topLeft()), line.size()));
    dockIndicator_->show();
    dockIndicator_->raise();
    return true;
}

void PicturaMainWindow::hideDockIndicator()
{
    if (dockIndicator_) {
        dockIndicator_->hide();
    }
}

void PicturaMainWindow::applyPendingDock()
{
    if (windowDockDragRunning_ || !pendingDockWindow_) {
        return;
    }
    DocWindow* window = std::exchange(pendingDockWindow_, nullptr);
    if (windows_.contains(window) && window->kind == DocWindow::Host::Float) {
        dockWindow(window, pendingDockIndex_);
    }
}

void PicturaMainWindow::consolidateAllToTabs(DocWindow* target)
{
    DocWindow* active = target ? target : activeWindow_;
    const QList<DocWindow*> windows = windows_;
    for (DocWindow* window : windows) {
        if (window->kind != DocWindow::Host::Tab) {
            dockWindow(window, -1);
        }
    }
    activateDocWindow(active);
}

void PicturaMainWindow::closeOtherWindows(DocWindow* keep)
{
    const QList<DocWindow*> windows = windows_;
    for (DocWindow* window : windows) {
        if (window == keep || !windows_.contains(window)) {
            continue;
        }
        // A cancelled prompt stops the rest, as Close All does.
        if (!closeWindow(window, true)) {
            break;
        }
    }
    activateDocWindow(keep);
}

void PicturaMainWindow::revealInFileManager(const DocWindow* window)
{
    const QString path = window->doc->path;
    if (path.isEmpty()) {
        return;
    }
    const QUrl folder = QUrl::fromLocalFile(QFileInfo(path).absolutePath());
    if (urlOpener_) {
        urlOpener_(folder);
    } else {
        QDesktopServices::openUrl(folder);
    }
}

} // namespace pictura
