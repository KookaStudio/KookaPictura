// PicturaMainWindow Adobe PDF entry points: thin forwarders to the
// ImportSaveCoordinator.

#include "frame_includes.h"

#include "import_save_coordinator.h"

namespace pictura {

bool PicturaMainWindow::isPdfPath(const QString& path)
{
    return ImportSaveCoordinator::isPdfPath(path);
}

bool PicturaMainWindow::savePdfWithDialog(const QString& path)
{
    return importSave_ && importSave_->savePdfWithDialog(path);
}

quint64 PicturaMainWindow::startOpenPdfPath(const QString& path, bool interactive)
{
    return importSave_ ? importSave_->startOpenPdfPath(path, interactive) : 0;
}

void PicturaMainWindow::setPdfDialogDriverForTest(std::function<bool(QDialog&)> driver)
{
    if (importSave_) {
        importSave_->setPdfDialogDriverForTest(std::move(driver));
    }
}

void PicturaMainWindow::setPdfViewerForTest(std::function<void(const QString&)> viewer)
{
    if (importSave_) {
        importSave_->setPdfViewerForTest(std::move(viewer));
    }
}

void PicturaMainWindow::showPdfPresets()
{
    if (importSave_) {
        importSave_->showPdfPresets();
    }
}

void PicturaMainWindow::wirePdfCommands()
{
    commandRegistry()->setHandler(command_ids::EditPdfPresets, [this]() { showPdfPresets(); });
}

} // namespace pictura
