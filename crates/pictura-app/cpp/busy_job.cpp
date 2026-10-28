#include "busy_job.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QEventLoop>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// A job faster than this shows only the wait cursor, so a quick save or filter
// does not flash a window.
constexpr qint64 kBusyWindowDelayMs = 250;

} // namespace

bool runBusyJob(QWidget* parent, const QString& title, const QString& label,
                const std::function<bool()>& start, const std::function<bool()>& running,
                const std::function<bool()>& finish)
{
    if (!start()) {
        return false;
    }
    if (running()) {
        QApplication::setOverrideCursor(Qt::WaitCursor);
        QDialog busy(parent, Qt::Dialog | Qt::CustomizeWindowHint | Qt::WindowTitleHint);
        busy.setObjectName(QStringLiteral("jobBusy"));
        busy.setWindowTitle(title);
        auto* layout = new QVBoxLayout(&busy);
        layout->addWidget(new QLabel(label, &busy));
        auto* bar = new QProgressBar(&busy);
        bar->setRange(0, 0);
        bar->setTextVisible(false);
        bar->setMinimumWidth(260);
        layout->addWidget(bar);

        QEventLoop loop;
        QTimer poll;
        poll.setInterval(16);
        QElapsedTimer elapsed;
        elapsed.start();
        QObject::connect(&poll, &QTimer::timeout, &loop, [&] {
            if (!running()) {
                loop.quit();
            } else if (!busy.isVisible() && elapsed.elapsed() >= kBusyWindowDelayMs) {
                busy.show();
            }
        });
        poll.start();
        // Paints, timers and the busy bar keep running; clicks and keys wait
        // until the job has landed.
        loop.exec(QEventLoop::ExcludeUserInputEvents);
        busy.hide();
        QApplication::restoreOverrideCursor();
    }
    return finish();
}

} // namespace pictura
