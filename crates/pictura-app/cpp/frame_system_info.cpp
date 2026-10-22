#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
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
