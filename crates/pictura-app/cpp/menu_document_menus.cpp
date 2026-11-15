// The document tab's and the floating window header's context menus and their
// actions. The registry owns the windows and emits the context-menu request.

#include "menu_registrar.h"

#include "document_registry.h"

#include "commands.h"

#include <QtCore/QFileInfo>
#include <QtCore/QUrl>
#include <QtGui/QAction>
#include <QtGui/QDesktopServices>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTabBar>

#include <utility>

namespace pictura {

QMenu* MenuRegistrar::documentMenu() const
{
    return documentMenu_;
}

void MenuRegistrar::wireDocumentMenus()
{
    if (!documents_) {
        return;
    }
    documents_->wireDocumentMenus();
    connect(documents_, &DocumentRegistry::tabContextMenuRequested, this,
            &MenuRegistrar::showTabMenu);
    connect(documents_, &DocumentRegistry::headerContextMenuRequested, this,
            &MenuRegistrar::showHeaderMenu);
}

void MenuRegistrar::showTabMenu(DocWindow* clicked, const QPoint& barPos)
{
    if (!clicked || !documents_->windows().contains(clicked)) {
        return;
    }
    if (documentMenu_) {
        documentMenu_->close();
    }
    auto* menu = new QMenu(dialogParent());
    menu->setObjectName(QStringLiteral("documentTabMenu"));
    menu->setAttribute(Qt::WA_DeleteOnClose);
    const auto add = [this, menu, clicked](const QString& text, bool enabled,
                                           std::function<void(DocWindow*)> run) {
        QAction* action = menu->addAction(text);
        action->setEnabled(enabled);
        connect(action, &QAction::triggered, this, [this, clicked, run = std::move(run)]() {
            if (documents_->windows().contains(clicked)) {
                run(clicked);
            }
        });
    };
    add(tr("Consolidate All to Here"), true,
        [this](DocWindow* window) { consolidateAllToTabs(window); });
    add(tr("Close"), true, [this](DocWindow* window) {
        if (runtime_.closeWindow) {
            runtime_.closeWindow(window, true);
        }
    });
    add(tr("Close All"), true, [this](DocWindow*) {
        commands_->dispatch(QString::fromLatin1(command_ids::FileCloseAll));
    });
    add(tr("Close Others"), documents_->windows().size() > 1,
        [this](DocWindow* window) { closeOtherWindows(window); });
    add(tr("Move to New Window"), !clicked->doc->loading,
        [this](DocWindow* window) { documents_->floatWindow(window); });
    menu->addSeparator();
    add(tr("New Document…"), true,
        [this](DocWindow*) { commands_->dispatch(QString::fromLatin1(command_ids::FileNew)); });
    add(tr("Open Document…"), true,
        [this](DocWindow*) { commands_->dispatch(QString::fromLatin1(command_ids::FileOpen)); });
    menu->addSeparator();
    add(tr("Reveal in File Manager"), !clicked->doc->path.isEmpty(),
        [this](DocWindow* window) { revealInFileManager(window); });
    documentMenu_ = menu;
    menu->popup(documents_->tabBar()->mapToGlobal(barPos));
}

void MenuRegistrar::showHeaderMenu(DocWindow* window, const QPoint& globalPos)
{
    if (documentMenu_) {
        documentMenu_->close();
    }
    documents_->activateDocWindow(window);
    commands_->refresh();
    struct Entry {
        QString text;
        QString id;
    };
    const QList<Entry> entries = {
        {tr("Duplicate…"), QString::fromLatin1(command_ids::ImageDuplicate)},
        {tr("Image Size…"), QString::fromLatin1(command_ids::ImageImageSize)},
        {tr("Canvas Size…"), QString::fromLatin1(command_ids::ImageCanvasSize)},
        {QString(), QString()},
        {tr("File Info…"), QString::fromLatin1(command_ids::FileInfo)},
        {QString(), QString()},
        {tr("Print…"), QString::fromLatin1(command_ids::FilePrint)},
    };
    auto* menu = new QMenu(dialogParent());
    menu->setObjectName(QStringLiteral("documentHeaderMenu"));
    menu->setAttribute(Qt::WA_DeleteOnClose);
    for (const Entry& entry : entries) {
        if (entry.id.isEmpty()) {
            menu->addSeparator();
            continue;
        }
        QAction* source = commands_->action(entry.id);
        QAction* action = menu->addAction(entry.text);
        action->setEnabled(source && source->isEnabled());
        connect(action, &QAction::triggered, this, [this, window, id = entry.id]() {
            if (!documents_->windows().contains(window)) {
                return;
            }
            documents_->activateDocWindow(window);
            commands_->refresh();
            if (QAction* command = commands_->action(id); command && command->isEnabled()) {
                commands_->dispatch(id);
            }
        });
    }
    documentMenu_ = menu;
    menu->popup(globalPos);
}

void MenuRegistrar::consolidateAllToTabs(DocWindow* target)
{
    DocWindow* active = target ? target : documents_->activeWindow();
    const QList<DocWindow*> windows = documents_->windows();
    for (DocWindow* window : windows) {
        if (window->kind != DocWindow::Host::Tab) {
            documents_->dockWindow(window, -1);
        }
    }
    documents_->activateDocWindow(active);
}

void MenuRegistrar::closeOtherWindows(DocWindow* keep)
{
    const QList<DocWindow*> windows = documents_->windows();
    for (DocWindow* window : windows) {
        if (window == keep || !documents_->windows().contains(window)) {
            continue;
        }
        if (runtime_.closeWindow && !runtime_.closeWindow(window, true)) {
            break;
        }
    }
    documents_->activateDocWindow(keep);
}

void MenuRegistrar::revealInFileManager(const DocWindow* window)
{
    const QString path = window->doc->path;
    if (path.isEmpty()) {
        return;
    }
    const QUrl folder = QUrl::fromLocalFile(QFileInfo(path).absolutePath());
    if (runtime_.openUrl) {
        runtime_.openUrl(folder);
    } else {
        QDesktopServices::openUrl(folder);
    }
}

} // namespace pictura
