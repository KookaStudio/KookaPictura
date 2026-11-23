#include "export_batch.h"

#include "api/view_bridge.h"
#include "busy_job.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QFileInfo>
#include <QtCore/QPointer>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtWidgets/QMessageBox>

namespace pictura {

namespace {

QVector<ExportTarget> parseTargets(const ExportBatchSpec& spec)
{
    QVector<ExportTarget> targets;
    const QStringList lines = spec.targetsText.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    targets.reserve(lines.size());
    for (const QString& line : lines) {
        const int tab = line.indexOf(QLatin1Char('\t'));
        if (tab <= 0) {
            continue;
        }
        targets.append({line.left(tab),
                        spec.destination + QLatin1Char('/') + line.mid(tab + 1) + spec.suffix});
    }
    return targets;
}

QString failureMessage(const ExportBatchSpec& spec, const ExportTarget& target)
{
    return spec.failureText ? spec.failureText(target) : QString();
}

void warn(QWidget* parent, const QString& title, const QString& message)
{
    if (parent && !message.isEmpty()) {
        QMessageBox::warning(parent, title, message);
    }
}

void inform(QWidget* parent, const QString& title, const QString& message)
{
    if (parent && !message.isEmpty()) {
        QMessageBox::information(parent, title, message);
    }
}

} // namespace

int ExportBatchRun::progressPermille() const
{
    return total > 0 ? written * 1000 / total : 1000;
}

bool exportBatch(const ExportBatchSpec& spec)
{
    ExportBatchRun run;
    return exportBatch(spec, run);
}

bool exportBatch(const ExportBatchSpec& spec, ExportBatchRun& run)
{
    run.total = 0;
    run.written = 0;
    run.cancelled = false;
    run.failed = false;

    const QFileInfo destination(spec.destination);
    if (!destination.isDir() || !destination.isWritable()) {
        warn(spec.parent, spec.title, spec.destinationFailureText);
        return false;
    }
    const QVector<ExportTarget> targets = parseTargets(spec);
    run.total = int(targets.size());
    if (targets.isEmpty()) {
        if (spec.targetsText.isEmpty()) {
            inform(spec.parent, spec.title, spec.emptyInfoText);
        }
        return false;
    }
    for (const ExportTarget& target : targets) {
        if (QFileInfo(target.path).isDir()) {
            warn(spec.parent, spec.title, failureMessage(spec, target));
            return false;
        }
    }

    const QPointer<PictureView> viewGuard(spec.view);
    const QPointer<QWidget> parentGuard(spec.parent);
    int failing = -1;

    return runCancellableJob(
        spec.parent, spec.title, QCoreApplication::translate("PicturaUi", "Writing files…"),
        [] { return true; },
        [&] {
            if (run.cancelled || run.failed || run.written >= run.total) {
                return false;
            }
            const ExportTarget& target = targets.at(run.written);
            if (!viewGuard) {
                run.failed = true;
                failing = run.written;
                return false;
            }
            if (!spec.write || !spec.write(*viewGuard.data(), target)) {
                run.failed = true;
                failing = run.written;
                return false;
            }
            ++run.written;
            return !run.cancelled && !run.failed && run.written < run.total;
        },
        [&run] { return run.progressPermille(); }, [&run] { run.cancel(); },
        [&] {
            if (run.failed && failing >= 0) {
                warn(parentGuard.data(), spec.title, failureMessage(spec, targets.at(failing)));
            }
            return !run.cancelled && !run.failed;
        });
}

} // namespace pictura
