#pragma once

#include <QtCore/QString>

#include <functional>

class QWidget;

namespace pictura {

class PictureView;

struct ExportTarget {
    QString key;
    QString path;
};

struct ExportBatchSpec {
    QWidget* parent = nullptr;
    PictureView* view = nullptr;
    QString title;
    QString destination;
    QString suffix;
    QString targetsText;
    QString destinationFailureText;
    QString emptyInfoText;
    std::function<bool(PictureView&, const ExportTarget&)> write;
    std::function<QString(const ExportTarget&)> failureText;
};

struct ExportBatchRun {
    int total = 0;
    int written = 0;
    bool cancelled = false;
    bool failed = false;

    void cancel() { cancelled = true; }
    int progressPermille() const;
};

// Validate the destination, split `targetsText` into ordered targets, preflight
// each computed path, then write one target per step with completed-item
// progress, cancellation before the next write, view/parent lifetime, and
// stop-on-failure reporting.
bool exportBatch(const ExportBatchSpec& spec);
bool exportBatch(const ExportBatchSpec& spec, ExportBatchRun& run);

} // namespace pictura
