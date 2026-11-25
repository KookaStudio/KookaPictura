#include "frame.h"

#include "api/paint_bridge.h"
#include "api/view_bridge.h"

#include "command_ids.h"
#include "commands.h"
#include "debug_toolset_panel.h"
#include "system_info.h"
#include "system_info_dialog.h"

#include <QtGui/QClipboard>
#include <QtGui/QDesktopServices>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QMessageBox>

namespace pictura {

void PicturaMainWindow::showSystemInfo()
{
    PictureView probe;
    const SystemInfo info = collectSystemInfo(probe.gpu_adapter_info());
    if (!systemInfoDialog_) {
        systemInfoDialog_ = new SystemInfoDialog(info, this);
    } else {
        systemInfoDialog_->setInfo(info);
    }
    systemInfoDialog_->show();
    systemInfoDialog_->raise();
    systemInfoDialog_->activateWindow();
}

void PicturaMainWindow::setDebugToolsetEnabled(bool enabled)
{
    if (!debugToolset_) {
        if (!enabled) {
            return;
        }
        debugToolset_ = new DebugToolsetPanel(this);
        connect(debugToolset_, &DebugToolsetPanel::deactivated, this,
                [this] { syncDebugToolsetCheck(); });
        // The snapshot is empty unless the process started with PICTURA_METRICS, so
        // skip building a view at all in the common case.
        debugToolset_->setMetricsProvider([]() -> QString {
            if (qEnvironmentVariableIntValue("PICTURA_METRICS") == 0) {
                return {};
            }
            PictureView probe;
            return probe.metrics_snapshot(false);
        });
        debugToolset_->setMemoryProvider([]() { return memory_status_json(); });
    }
    debugToolset_->setActive(enabled);
    syncDebugToolsetCheck();
}

void PicturaMainWindow::syncDebugToolsetCheck()
{
    if (QAction* action = registry() ? registry()->action(command_ids::HelpDeveloperDebugToolset)
                                     : nullptr) {
        action->setChecked(debugToolsetEnabled());
    }
}

bool PicturaMainWindow::debugToolsetEnabled() const
{
    return debugToolset_ && debugToolset_->isActive();
}

void PicturaMainWindow::submitIssue()
{
    PictureView probe;
    const SystemInfo info = collectSystemInfo(probe.gpu_adapter_info());
    const QString report = formatSystemInfo(info);
    QGuiApplication::clipboard()->setText(report);
    if (QGuiApplication::platformName() == QStringLiteral("offscreen")) {
        return;
    }
    if (!QDesktopServices::openUrl(submitIssueUrl(report))) {
        QMessageBox::warning(
            this, tr("Submit Issue"),
            tr("Could not open the issue page. The system info has been copied to the "
               "clipboard."));
    }
}

} // namespace pictura
