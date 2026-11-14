// File > Open Recent: the registrar keeps the submenu and its actions; the
// list itself lives in the ImportSaveCoordinator.

#include "menu_registrar.h"

#include "import_save_coordinator.h"

#include "commands.h"
#include <QtGui/QAction>

#include <QtCore/QFileInfo>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>

namespace pictura {

QMenu* MenuRegistrar::recentMenu() const
{
    if (!menuBar_) {
        return nullptr;
    }
    for (QMenu* menu : menuBar_->findChildren<QMenu*>()) {
        if (menu->title() == QStringLiteral("Open Recent")) {
            return menu;
        }
    }
    return nullptr;
}

void MenuRegistrar::refreshRecentMenu(QMenu* menu)
{
    if (!menu) {
        return;
    }
    for (QAction* action : recentActions_) {
        menu->removeAction(action);
        delete action;
    }
    recentActions_.clear();

    // A file that has since moved or been deleted is dropped, not offered.
    QStringList present;
    for (const QString& path : importSave_ ? importSave_->recentFiles() : QStringList{}) {
        if (QFileInfo::exists(path)) {
            present.append(path);
        }
    }
    if (importSave_ && present != importSave_->recentFiles()) {
        importSave_->setRecentFiles(present);
    }
    // The registry's "No Recent Files" row stands in for an empty list.
    for (QAction* action : menu->actions()) {
        action->setVisible(present.isEmpty());
    }
    if (present.isEmpty()) {
        return;
    }

    for (const QString& path : present) {
        // The name alone is ambiguous across folders, so the full path is the
        // tooltip.
        auto* entry = new QAction(QFileInfo(path).fileName(), menu);
        entry->setToolTip(path);
        connect(entry, &QAction::triggered, this, [this, path]() {
            if (importSave_) {
                importSave_->openRecent(path);
            }
        });
        recentActions_.append(entry);
    }
    auto* separator = new QAction(menu);
    separator->setSeparator(true);
    recentActions_.append(separator);
    auto* clear = new QAction(tr("Clear Recent File List"), menu);
    connect(clear, &QAction::triggered, this, [this]() {
        if (importSave_) {
            importSave_->setRecentFiles({});
        }
    });
    recentActions_.append(clear);
    menu->setToolTipsVisible(true);
    menu->addActions(recentActions_);
}

} // namespace pictura
