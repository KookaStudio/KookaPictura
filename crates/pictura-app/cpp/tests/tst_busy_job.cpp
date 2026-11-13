// The cancellable job window (#1278): the wait cursor at once, the progress
// window only after 250 ms, Esc cancelling like the button, and a busy bar for
// a job that cannot count its work.

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>
#include <QtGui/QCursor>
#include <QtWidgets/QApplication>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QPushButton>

#include "busy_job.h"

namespace {

QProgressDialog* jobWindow()
{
    for (QWidget* w : QApplication::topLevelWidgets()) {
        if (w->objectName() == QLatin1String("jobProgress")) {
            return qobject_cast<QProgressDialog*>(w);
        }
    }
    return nullptr;
}

bool jobWindowVisible()
{
    const QProgressDialog* d = jobWindow();
    return d && d->isVisible();
}

bool waitCursorSet()
{
    const QCursor* c = QApplication::overrideCursor();
    return c && c->shape() == Qt::WaitCursor;
}

} // namespace

class BusyJobTest : public QObject {
    Q_OBJECT

private slots:
    void quickJobShowsOnlyTheWaitCursor();
    void slowJobShowsTheProgressWindow();
    void escCancelsTheJob();
    void aNegativeProgressShowsABusyBar();
};

// [lop_quick_job]
void BusyJobTest::quickJobShowsOnlyTheWaitCursor()
{
    QElapsedTimer clock;
    QObject probeContext;
    bool started = false;
    bool sawWait = false;
    bool sawWindow = false;
    const bool finished = pictura::runCancellableJob(
        nullptr, QStringLiteral("Quick"), QStringLiteral("Working…"), [] { return true; },
        [&] {
            if (!started) {
                started = true;
                // The dialog exists by the first poll, so timing the job from
                // here keeps dialog setup out of the 60 ms window.
                clock.start();
                QTimer::singleShot(0, &probeContext,
                                   [&] { sawWait = sawWait || waitCursorSet(); });
            }
            sawWait = sawWait || waitCursorSet();
            sawWindow = sawWindow || jobWindowVisible();
            return clock.elapsed() < 60;
        },
        [] { return 500; }, [] {}, [] { return true; });
    QVERIFY(finished);
    QVERIFY2(sawWait, "the wait cursor is set while the job runs");
    QVERIFY2(!sawWindow, "a job under 250 ms shows no window");
    QVERIFY2(QApplication::overrideCursor() == nullptr, "the cursor is restored");
}

// [lop_window_delay]
void BusyJobTest::slowJobShowsTheProgressWindow()
{
    QElapsedTimer clock;
    bool shownEarly = false;
    bool shownLate = false;
    bool hasCancel = false;
    int ticks = 0;
    QTimer tick;
    tick.setInterval(10);
    connect(&tick, &QTimer::timeout, this, [&] { ++ticks; });
    tick.start();
    pictura::runCancellableJob(
        nullptr, QStringLiteral("Slow"), QStringLiteral("Working…"),
        [&] {
            clock.start();
            return true;
        },
        [&] {
            const qint64 t = clock.elapsed();
            if (t < 200) {
                shownEarly = shownEarly || jobWindowVisible();
            } else if (t > 450 && jobWindowVisible()) {
                shownLate = true;
                hasCancel = hasCancel || jobWindow()->findChild<QPushButton*>() != nullptr;
            }
            return t < 700;
        },
        [&] { return static_cast<int>(clock.elapsed()); }, [] {}, [] { return true; });
    tick.stop();
    QVERIFY2(!shownEarly, "the window waits 250 ms");
    QVERIFY2(shownLate, "the window appears for a slow job");
    QVERIFY2(hasCancel, "the window has a Cancel button");
    QVERIFY2(ticks > 30, qPrintable(QStringLiteral("timers kept running: %1 ticks").arg(ticks)));
}

// [lop_esc_cancels]
void BusyJobTest::escCancelsTheJob()
{
    QElapsedTimer clock;
    bool cancelled = false;
    bool pressed = false;
    const bool finished = pictura::runCancellableJob(
        nullptr, QStringLiteral("Cancel"), QStringLiteral("Working…"),
        [&] {
            clock.start();
            return true;
        },
        [&] {
            if (!pressed && jobWindowVisible()) {
                pressed = true;
                QTimer::singleShot(0, [] {
                    if (QProgressDialog* d = jobWindow()) {
                        QTest::keyClick(d, Qt::Key_Escape);
                    }
                });
            }
            return !cancelled && clock.elapsed() < 10000;
        },
        [] { return 0; }, [&] { cancelled = true; }, [&] { return !cancelled; });
    QVERIFY2(pressed, "the window appeared");
    QVERIFY2(cancelled, "Esc ran the cancel callback");
    QVERIFY(!finished);
    QVERIFY2(clock.elapsed() < 10000, "the job ended on the cancel");
}

// [lop_busy_bar]
void BusyJobTest::aNegativeProgressShowsABusyBar()
{
    QElapsedTimer clock;
    bool busy = false;
    bool counted = false;
    pictura::runCancellableJob(
        nullptr, QStringLiteral("Busy"), QStringLiteral("Working…"),
        [&] {
            clock.start();
            return true;
        },
        [&] {
            if (const QProgressDialog* d = jobWindow(); d && d->isVisible()) {
                busy = busy || d->maximum() == 0;
                counted = counted || d->maximum() == 1000;
            }
            return clock.elapsed() < 600;
        },
        [] { return -1; }, [] {}, [] { return true; });
    QVERIFY2(busy, "a job that cannot count shows a busy bar");
    QVERIFY2(!counted, "the bar never shows a percentage");
}

QTEST_MAIN(BusyJobTest)
#include "tst_busy_job.moc"
