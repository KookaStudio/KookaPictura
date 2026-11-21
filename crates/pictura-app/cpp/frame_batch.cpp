// File > Automate > Batch (`BatchHost`) and the user-open entry point that
// records Open action steps.

#include "frame_includes.h"

#include "batch_dialog.h"
#include "batch_runner.h"
#include "command_ids.h"
#include "dialogs.h"
#include "import_save_coordinator.h"
#include "panel_host.h"
#include "panels/actions_panel.h"
#include "session.h"

namespace pictura {

void PicturaMainWindow::openUserPaths(const QStringList& paths)
{
    // Start every chosen path; a failure on one must not skip the rest.
    for (const QString& path : paths) {
        if (importSave_) {
            importSave_->startUserOpen(path, true);
        }
    }
}

ActionsPanel* PicturaMainWindow::actionsPanel() const
{
    return panels_ ? panels_->actionsPanel() : nullptr;
}

BatchHost& PicturaMainWindow::batch()
{
    if (!batch_) {
        batch_ = new BatchHost(*this);
    }
    return *batch_;
}

void PicturaMainWindow::setSuppressProfileWarnings(bool suppress)
{
    if (importSave_) {
        importSave_->setSuppressProfileWarnings(suppress);
    }
}

BatchHost::BatchHost(PicturaMainWindow& frame)
    : QObject(&frame)
    , frame_(frame)
{
}

bool BatchHost::running() const
{
    return runner_ && runner_->isRunning();
}

std::optional<QString> BatchHost::saveAsRedirect(const QString& format)
{
    if (!runner_ || !runner_->redirectsSaveAs()) {
        return std::nullopt;
    }
    return runner_->saveAsRedirect(format);
}

void BatchHost::report(const QString& text)
{
    if (messageHandler_) {
        messageHandler_(QObject::tr("Batch"), text);
    } else {
        QMessageBox::warning(&frame_, QObject::tr("Batch"), text);
    }
}

void BatchHost::showDialog()
{
    ActionsPanel* panel = frame_.actionsPanel();
    if (!panel || running()) {
        return;
    }
    BatchDialog dialog(&frame_);
    dialog.optionsWidget()->setLibrary(panel->actionLibraryNames());
    BatchOptions initial = BatchOptions::fromJson(pictura::loadSession().batchOptions);
    const QPair<QString, QString> selected = panel->selectedActionNames();
    if (!selected.first.isEmpty()) {
        initial.set = selected.first;
        initial.action = selected.second;
    }
    dialog.optionsWidget()->setOptions(initial);
    dialog.setValidator(
        [panel](const BatchOptions& options) { return panel->validateBatch(options.toJson()); });
    dialog.setOpenProbe([panel](const BatchOptions& options) {
        return panel->actionReachesCommand(options.set, options.action,
                                           QString::fromLatin1(command_ids::FileOpen));
    });
    if (runDialog(dialog, &frame_) != QDialog::Accepted) {
        return;
    }
    const BatchOptions chosen = dialog.options();
    SessionState state = pictura::loadSession();
    state.batchOptions = chosen.toJson();
    pictura::saveSession(state);
    start(chosen);
}

bool BatchHost::start(const BatchOptions& options)
{
    ActionsPanel* panel = frame_.actionsPanel();
    if (!panel || panel->isPlaying() || panel->isRecording() || running()) {
        return false;
    }
    const QString error = panel->validateBatch(options.toJson());
    if (!error.isEmpty()) {
        report(error);
        return false;
    }
    if (runner_) {
        runner_->deleteLater();
    }
    runner_ = new BatchRunner(frame_, options, this);
    if (messageHandler_) {
        runner_->setMessageHandler(messageHandler_);
    }
    connect(runner_, &BatchRunner::finished, this, [this] {
        if (CommandRegistry* commands = frame_.registry()) {
            commands->refresh();
        }
    });
    if (!runner_->start()) {
        report(runner_->startError());
        return false;
    }
    return true;
}

} // namespace pictura
