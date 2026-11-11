#include "document_command.h"

#include <QtWidgets/QApplication>

#include "busy_job.h"

#include "api/document_bridge.h"

namespace pictura {

bool runDocumentCommand(QWidget* parent, PictureView& view, const QString& title,
                        const std::function<void()>& command)
{
    return runCancellableJob(
        parent, title, QApplication::translate("DocumentCommand", "Working…"),
        [&view, &command] {
            document_job_arm(view, true);
            command();
            document_job_arm(view, false);
            return true;
        },
        [&view] { return document_job_running(view); },
        [&view] { return document_job_progress(view); },
        [&view] { document_job_cancel(view); },
        // A small document's command has already applied; a job lands here.
        [&view] { return document_job_finish(view); });
}

} // namespace pictura
