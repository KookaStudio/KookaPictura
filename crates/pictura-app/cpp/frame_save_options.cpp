// PicturaMainWindow Save As and per-format option entry points: thin
// forwarders to the ImportSaveCoordinator.

#include "frame_includes.h"

#include "import_save_coordinator.h"

namespace pictura {

bool PicturaMainWindow::saveActiveAsWithOptions(const QString& path)
{
    return importSave_ && importSave_->saveActiveAsWithOptions(path);
}

bool PicturaMainWindow::promptSaveFormatOptions(const QString& path)
{
    return importSave_ && importSave_->promptSaveFormatOptions(path);
}

void PicturaMainWindow::setSaveFormatDialogForTest(
    std::function<bool(SaveFormatKind, SaveFormatOptions&)> hook)
{
    if (importSave_) {
        importSave_->setSaveFormatDialogForTest(std::move(hook));
    }
}

bool PicturaMainWindow::runSaveFormatDialog(SaveFormatKind kind, SaveFormatOptions& options)
{
    return importSave_ && importSave_->runSaveFormatDialog(kind, options);
}

void PicturaMainWindow::rememberSaveFormatOptions(SaveFormatKind kind,
                                                  const SaveFormatOptions& options)
{
    if (importSave_) {
        importSave_->rememberSaveFormatOptions(kind, options);
    }
}

void PicturaMainWindow::applySaveRasterOptions(PictureView& view, SaveFormatKind kind,
                                               const SaveFormatOptions& options)
{
    if (importSave_) {
        importSave_->applySaveRasterOptions(view, kind, options);
    }
}

} // namespace pictura
