// PicturaMainWindow generic PDF/EPS import and File > Open As entry points:
// thin forwarders to the ImportSaveCoordinator.

#include "frame_includes.h"

#include "import_save_coordinator.h"

namespace pictura {

quint64 PicturaMainWindow::startImportEpsPath(const QString& path, bool interactive)
{
    return importSave_ ? importSave_->startImportEpsPath(path, interactive) : 0;
}

void PicturaMainWindow::showOpenAsDialog()
{
    if (importSave_) {
        importSave_->showOpenAsDialog();
    }
}

quint64 PicturaMainWindow::startOpenAsPath(const QString& path, int formatIndex)
{
    return importSave_ ? importSave_->startOpenAsPath(path, formatIndex) : 0;
}

bool PicturaMainWindow::openAsPath(const QString& path, int formatIndex)
{
    return importSave_ && importSave_->openAsPath(path, formatIndex);
}

void PicturaMainWindow::setImportDialogDriverForTest(std::function<bool(QDialog&)> driver)
{
    if (importSave_) {
        importSave_->setImportDialogDriverForTest(std::move(driver));
    }
}

} // namespace pictura
