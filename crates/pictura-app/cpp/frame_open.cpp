// PicturaMainWindow open entry points: thin forwarders to the
// ImportSaveCoordinator. The pipeline and its tokens live in
// import_save_open.cpp.

#include "frame_includes.h"

#include "import_save_coordinator.h"

namespace pictura {

int PicturaMainWindow::indexOfView(PictureView* view) const
{
    return documents_ ? documents_->indexOfView(view) : -1;
}

void PicturaMainWindow::setOpenDelayForTest(int ms)
{
    if (importSave_) {
        importSave_->setOpenDelayForTest(ms);
    }
}

bool PicturaMainWindow::confirmProfileForOpen(PictureView* view, const QString& path)
{
    return importSave_ && importSave_->confirmProfileForOpen(view, path);
}

void PicturaMainWindow::waitForOpensForTest()
{
    if (importSave_) {
        importSave_->waitForOpensForTest();
    }
}

void PicturaMainWindow::cancelAllOpens()
{
    if (importSave_) {
        importSave_->cancelAllOpens();
    }
}

quint64 PicturaMainWindow::startOpenPath(const QString& path)
{
    return importSave_ ? importSave_->startOpenPath(path) : 0;
}

quint64 PicturaMainWindow::startOpenImagePath(const QString& path)
{
    return importSave_ ? importSave_->startOpenImagePath(path) : 0;
}

quint64 PicturaMainWindow::startOpenAsSmartObjectPath(const QString& path)
{
    return importSave_ ? importSave_->startOpenAsSmartObjectPath(path) : 0;
}

bool PicturaMainWindow::isNativeDocumentPath(const QString& path)
{
    return ImportSaveCoordinator::isNativeDocumentPath(path);
}

bool PicturaMainWindow::saveRunsOnWorker(const QString& path)
{
    return ImportSaveCoordinator::saveRunsOnWorker(path);
}

quint64 PicturaMainWindow::startOpenDocumentAtPath(const QString& path, bool interactive,
                                                   bool reportFailure)
{
    return importSave_ ? importSave_->startOpenDocumentAtPath(path, interactive, reportFailure)
                       : 0;
}

bool PicturaMainWindow::openPath(const QString& path)
{
    return importSave_ && importSave_->openPath(path);
}

bool PicturaMainWindow::openImagePath(const QString& path)
{
    return importSave_ && importSave_->openImagePath(path);
}

bool PicturaMainWindow::openAsSmartObjectPath(const QString& path)
{
    return importSave_ && importSave_->openAsSmartObjectPath(path);
}

bool PicturaMainWindow::openDocumentAtPath(const QString& path, bool interactive)
{
    return importSave_ && importSave_->openDocumentAtPath(path, interactive);
}

PicturaMainWindow::OpenOutcome PicturaMainWindow::openDocumentAtPathOutcome(
    const QString& path, bool interactive)
{
    return importSave_ ? importSave_->openDocumentAtPathOutcome(path, interactive)
                       : OpenOutcome::Failed;
}

} // namespace pictura
