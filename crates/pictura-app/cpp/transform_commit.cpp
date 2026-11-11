#include "transform_commit.h"

#include <QtWidgets/QApplication>

#include "busy_job.h"

#include "api/document_bridge.h"

namespace pictura {

bool runTransformCommit(QWidget* parent, PictureView& view)
{
    return runCancellableJob(
        parent, QApplication::translate("TransformCommit", "Free Transform"),
        QApplication::translate("TransformCommit", "Transforming…"),
        [&view] { return view.commit_transform(); },
        [&view] { return transform_commit_running(view); },
        [&view] { return transform_commit_progress(view); },
        [&view] { transform_commit_cancel(view); },
        // A small commit has already landed; a job lands here.
        [&view] { return !transform_pending(view) || transform_commit_finish(view); });
}

} // namespace pictura
