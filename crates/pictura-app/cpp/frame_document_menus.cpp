// The document tab's and the floating window header's context menus, and
// tearing a tab out of the strip into a floating window.

#include "frame_includes.h"

#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>

namespace pictura {

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
