#include "filter_commit.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QEventLoop>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QVBoxLayout>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"

namespace pictura {

namespace {

// A commit faster than this shows only the wait cursor, so a quick filter
// does not flash a window.
const qint64 kBusyWindowDelayMs = 250;

} // namespace

QString filterHistoryLabel(const QString& name)
{
    return QStringLiteral("Filter: %1").arg(name);
}

bool runFilterCommit(QWidget* parent, PictureView* view, const QString& name,
                     const std::function<bool()>& start)
{
    if (!view || !start()) {
        return false;
    }
    if (filter_commit_running(*view)) {
        QApplication::setOverrideCursor(Qt::WaitCursor);
        QDialog busy(parent, Qt::Dialog | Qt::CustomizeWindowHint | Qt::WindowTitleHint);
        busy.setObjectName(QStringLiteral("filterBusy"));
        busy.setWindowTitle(name);
        auto* layout = new QVBoxLayout(&busy);
        layout->addWidget(new QLabel(QApplication::translate("FilterCommit", "Applying %1…").arg(name), &busy));
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
            if (!filter_commit_running(*view)) {
                loop.quit();
            } else if (!busy.isVisible() && elapsed.elapsed() >= kBusyWindowDelayMs) {
                busy.show();
            }
        });
        poll.start();
        // Paints, timers and the busy bar keep running; clicks and keys wait
        // until the commit has landed.
        loop.exec(QEventLoop::ExcludeUserInputEvents);
        busy.hide();
        QApplication::restoreOverrideCursor();
    }
    return filter_commit_finish(*view);
}

} // namespace pictura
