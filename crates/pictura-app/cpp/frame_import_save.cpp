// Save, Revert, document close, and smart-object edit entry points kept on
// PicturaMainWindow for main.cpp, the control server, the self-test, and the
// existing Qt suites: thin forwarders to the ImportSaveCoordinator.

#include "frame_includes.h"

#include "import_save_coordinator.h"
#include "menu_registrar.h"

namespace pictura {

bool PicturaMainWindow::saveActive(bool interactive)
{
    return importSave_ && importSave_->saveActive(interactive);
}

bool PicturaMainWindow::saveActiveAs(const QString& path)
{
    return importSave_ && importSave_->saveActiveAs(path);
}

bool PicturaMainWindow::saveAsWithDialog()
{
    return importSave_ && importSave_->saveAsWithDialog();
}

bool PicturaMainWindow::saveCopyWithDialog()
{
    return importSave_ && importSave_->saveCopyWithDialog();
}

const QStringList& PicturaMainWindow::recentFiles() const
{
    static const QStringList empty;
    return importSave_ ? importSave_->recentFiles() : empty;
}

void PicturaMainWindow::setRecentFiles(const QStringList& files)
{
    if (importSave_) {
        importSave_->setRecentFiles(files);
    }
}

QMenu* PicturaMainWindow::recentMenu() const
{
    return menu_ ? menu_->recentMenu() : nullptr;
}

bool PicturaMainWindow::revertActive()
{
    return importSave_ && importSave_->revertActive();
}

bool PicturaMainWindow::saveAllDocumentsForRestart()
{
    return importSave_ && importSave_->saveAllDocumentsForRestart();
}

bool PicturaMainWindow::closeDocument(int index, bool interactive)
{
    return importSave_ && importSave_->closeDocument(index, interactive);
}

bool PicturaMainWindow::closeActiveDocument(bool interactive)
{
    return importSave_ && importSave_->closeActiveDocument(interactive);
}

bool PicturaMainWindow::editSmartObjectContents(const QString& layerPath)
{
    return importSave_ && importSave_->editSmartObjectContents(layerPath);
}

} // namespace pictura
