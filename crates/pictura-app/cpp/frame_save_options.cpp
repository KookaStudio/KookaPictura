// PicturaMainWindow Save As and per-format option entry points: thin
// forwarders to the ImportSaveCoordinator.

#include "frame_includes.h"

#include "import_save_coordinator.h"

namespace pictura {

bool PicturaMainWindow::saveActiveAsWithRememberedOptions(const QString& path)
{
    return importSave_ && importSave_->saveActiveAtPath(path);
}

bool PicturaMainWindow::saveActiveAsWithOptions(const QString& path, SaveFormatKind hint)
{
    return importSave_ && importSave_->saveActiveAsWithOptions(path, hint);
}

bool PicturaMainWindow::promptSaveFormatOptions(const QString& path, SaveFormatKind hint)
{
    return importSave_ && importSave_->promptSaveFormatOptions(path, hint);
}

void PicturaMainWindow::setSaveFormatDialogForTest(
    std::function<bool(SaveFormatKind, SaveFormatOptions&)> hook)
{
    if (importSave_) {
        importSave_->setSaveFormatDialogForTest(std::move(hook));
    }
}

void PicturaMainWindow::setGifDialogForTest(
    std::function<bool(PictureView*, SaveFormatOptions&)> hook)
{
    if (importSave_) {
        importSave_->setGifDialogForTest(std::move(hook));
    }
}

void PicturaMainWindow::setWarningDriverForTest(
    std::function<bool(const QString&, const QString&)> driver)
{
    if (importSave_) {
        importSave_->setWarningDriverForTest(std::move(driver));
    }
}

void PicturaMainWindow::setProfileDialogForTest(std::function<bool(bool, bool&)> hook)
{
    if (importSave_) {
        importSave_->setProfileDialogForTest(std::move(hook));
    }
}

void PicturaMainWindow::setFeatureWarningForTest(std::function<bool()> hook)
{
    if (importSave_) {
        importSave_->setFeatureWarningForTest(std::move(hook));
    }
}

void PicturaMainWindow::setGifDialogDriverForTest(
    std::function<bool(IndexedColorDialog&)> driver)
{
    if (importSave_) {
        importSave_->setGifDialogDriverForTest(std::move(driver));
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
