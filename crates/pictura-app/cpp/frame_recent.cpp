// File > Open Recent: the persisted recent-files list and its submenu. Ported
// from photorust's MainWindow::refreshRecentMenu. The submenu is repopulated
// each time it opens, so a file opened this session appears at once.

#include "frame_includes.h"

#include <QtWidgets/QMenu>

namespace pictura {

namespace {

// docs/02-ui-ux/preferences.md: "Recent File List Contains", default 20.
constexpr int kRecentLimit = 20;

} // namespace

void PicturaMainWindow::rememberRecent(const QString& path)
{
    if (path.isEmpty()) {
        return;
    }
    QStringList files = recent_;
    files.removeAll(path);
    files.prepend(path);
    setRecentFiles(files);
}

void PicturaMainWindow::setRecentFiles(const QStringList& files)
{
    recent_ = files.mid(0, kRecentLimit);
    // Keep the file dialogs' Places sidebar in step with the recent list.
    setFileDialogRecentPaths(recent_);
    saveSession();
}

QMenu* PicturaMainWindow::recentMenu() const
{
    for (QMenu* menu : menuBar()->findChildren<QMenu*>()) {
        if (menu->title() == QStringLiteral("Open Recent")) {
            return menu;
        }
    }
    return nullptr;
}

void PicturaMainWindow::refreshRecentMenu(QMenu* menu)
{
    for (QAction* action : recentActions_) {
        menu->removeAction(action);
        delete action;
    }
    recentActions_.clear();

    // A file that has since moved or been deleted is dropped, not offered.
    QStringList present;
    for (const QString& path : recent_) {
        if (QFileInfo::exists(path)) {
            present.append(path);
        }
    }
    if (present != recent_) {
        setRecentFiles(present);
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
        connect(entry, &QAction::triggered, this, [this, path]() { openRecent(path); });
        recentActions_.append(entry);
    }
    auto* separator = new QAction(menu);
    separator->setSeparator(true);
    recentActions_.append(separator);
    auto* clear = new QAction(tr("Clear Recent File List"), menu);
    connect(clear, &QAction::triggered, this, [this]() { setRecentFiles({}); });
    recentActions_.append(clear);
    menu->setToolTipsVisible(true);
    menu->addActions(recentActions_);
}

void PicturaMainWindow::openRecent(const QString& path)
{
    // The same routing as File > Open; the completion shows the failure message
    // for a file that could not be read.
    startOpenDocumentAtPath(path, true, true);
}

} // namespace pictura
