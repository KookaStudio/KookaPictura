#include "filter_commit.h"

#include <QtWidgets/QApplication>

#include "busy_job.h"

#include "api/filters_bridge.h"

namespace pictura {

QString filterHistoryLabel(const QString& name)
{
    return QStringLiteral("Filter: %1").arg(name);
}

bool runFilterCommit(QWidget* parent, PictureView* view, const QString& name,
                     const std::function<bool()>& start)
{
    if (!view) {
        return false;
    }
    return runBusyJob(
        parent, name,
        QApplication::translate("FilterCommit", "Applying %1…").arg(name),
        start,
        [view] { return filter_commit_running(*view); },
        [view] { return filter_commit_finish(*view); });
}

} // namespace pictura
