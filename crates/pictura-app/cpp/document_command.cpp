#include "document_command.h"

#include <QtCore/QPointer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QStatusBar>

#include "document_window.h"

#include "busy_job.h"

#include "api/document_bridge.h"

namespace pictura {

namespace {

bool finishCommand(QWidget* parent, PictureView& view, const QString& title, quint64 attempt)
{
    const auto result = document_job_finish_for(view, attempt);
    if (result.code == quint32(OperationStatus::Panic) && parent) {
        if (auto* window = qobject_cast<QMainWindow*>(parent->window())) {
            window->statusBar()->showMessage(
                QApplication::translate("DocumentCommand", "Could not complete %1. The operation stopped unexpectedly.")
                    .arg(title),
                6000);
        }
    }
    return result.applied;
}

} // namespace

bool runDocumentCommand(QWidget* parent, PictureView& view, const QString& title,
                        const std::function<void()>& command)
{
    const QPointer<PictureView> target(&view);
    quint64 attempt = 0;
    const auto current = [&] { return target && document_job_attempt(*target) == attempt; };
    return runCancellableJob(
        parent, title, QApplication::translate("DocumentCommand", "Working…"),
        [&] {
            document_job_arm(*target, true);
            attempt = document_job_attempt(*target);
            command();
            if (current()) {
                document_job_arm(*target, false);
            }
            return bool(target);
        },
        [&] { return current() && document_job_running(*target); },
        [&] { return current() ? document_job_progress(*target) : 0; },
        [&] {
            if (current()) {
                document_job_cancel(*target);
            }
        },
        [&] { return target && finishCommand(parent, *target, title, attempt); });
}

bool runSourceCommand(QWidget* parent, PictureView& view, const QString& title,
                      const std::function<bool(PictureView&)>& start)
{
    const QPointer<PictureView> target(&view);
    quint64 attempt = 0;
    const auto current = [&] { return target && document_job_attempt(*target) == attempt; };
    return runCancellableJob(
        parent, title, QApplication::translate("DocumentCommand", "Working…"),
        [&] {
            if (!target || !start(*target)) {
                return false;
            }
            attempt = document_job_attempt(*target);
            return true;
        },
        [&] { return current() && document_job_running(*target); },
        [&] { return current() ? document_job_progress(*target) : 0; },
        [&] {
            if (current()) {
                document_job_cancel(*target);
            }
        },
        [&] { return target && finishCommand(parent, *target, title, attempt); });
}

} // namespace pictura
