// The document tab's and the floating window header's context menus, tearing
// a tab out of the strip into a floating window, and docking a floating window
// by drag-and-drop.

#include "frame_includes.h"

#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtGui/QDrag>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtWidgets/QApplication>

namespace pictura {

namespace {

// A dock drag's payload: the window's address, compared with the registry and
// never dereferenced until a lookup vouches for it.
std::unique_ptr<QMimeData> windowDockMime(const DocWindow* window)
{
    auto mime = std::make_unique<QMimeData>();
    mime->setData(kDocumentWindowMimeType,
                  QByteArray::number(quint64(reinterpret_cast<quintptr>(window))));
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
        if (floatWindow(window, geometry)) {
            tearFloat_ = window->floating;
            tearFloat_->beginDrag(globalPos);
            // Where the window system moves top-level windows (Wayland), it
            // takes the rest of the drag, or the float stays where it landed.
            if (tearFloat_->startSystemMove()) {
                tearFloat_ = nullptr;
            }
        }
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

void PicturaMainWindow::startWindowDockDrag(DocWindow* window)
{
    if (!windows_.contains(window) || !window->floating) {
        return;
    }
    if (windowDockDragForTest_) {
        windowDockDragForTest_(windowDockMime(window));
        return;
    }
    const QPixmap preview = window->floating->header()->grab();
    // A drop that docks the window deletes its float, so the drag runs from
    // its own event-loop turn, not under the header's mouse handler.
    QTimer::singleShot(0, this, [this, window, preview]() {
        if (!windows_.contains(window) || window->kind != DocWindow::Host::Float) {
            return;
        }
        auto* drag = new QDrag(this);
        drag->setMimeData(windowDockMime(window).release());
        drag->setPixmap(preview);
        drag->exec(Qt::MoveAction);
    });
}

bool PicturaMainWindow::windowDropFilter(QObject* watched, QEvent* event)
{
    const QEvent::Type type = event->type();
    if (type != QEvent::DragEnter && type != QEvent::DragMove && type != QEvent::Drop) {
        return false;
    }
    auto* drop = static_cast<QDropEvent*>(event);
    if (!drop->mimeData() || !drop->mimeData()->hasFormat(kDocumentWindowMimeType)) {
        return false;
    }
    DocWindow* window = windowOfDockMime(drop->mimeData());
    const bool dockable = floatingDocumentDocking_ && window
                          && window->kind == DocWindow::Host::Float && tabs_->isVisible();
    const QPoint globalPos =
        static_cast<QWidget*>(watched)->mapToGlobal(drop->position().toPoint());
    // The tab widget takes the drag in on entry, so its moves keep arriving
    // until the pointer reaches the strip's band.
    const bool overStrip = type == QEvent::DragEnter || tabStripGlobalRect().contains(globalPos);
    if (!dockable || !overStrip) {
        drop->ignore();
        return true;
    }
    if (type == QEvent::Drop) {
        QTabBar* bar = tabs_->tabBar();
        if (!dockWindow(window, bar->tabAt(bar->mapFromGlobal(globalPos)))) {
            drop->ignore();
            return true;
        }
    }
    drop->setDropAction(Qt::MoveAction);
    drop->accept();
    return true;
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
