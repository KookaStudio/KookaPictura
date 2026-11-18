#pragma once

#include "batch_options.h"

#include <QtCore/QDate>
#include <QtCore/QFile>
#include <QtCore/QHash>
#include <QtCore/QObject>
#include <QtCore/QPair>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <functional>
#include <optional>

class QProgressDialog;

namespace pictura {

class PicturaMainWindow;
class PictureView;

// Runs one validated Batch: per input open (or activate), play the action with
// dialogs suppressed, apply the destination, close. Signal-driven: it waits on
// `documentOpenFinished` and `ActionsPanel::playbackFinished` instead of
// nesting event loops, so Cancel lands between steps.
class BatchRunner : public QObject {
    Q_OBJECT

public:
    using MessageHandler = std::function<void(const QString& title, const QString& text)>;

    BatchRunner(PicturaMainWindow& frame, const BatchOptions& options, QObject* parent = nullptr);
    ~BatchRunner() override;

    // Resolve the inputs and begin. False (with the reason in startError())
    // when nothing can run.
    bool start();
    void cancel();
    bool isRunning() const { return running_; }
    QString startError() const { return startError_; }

    int processedCount() const { return processed_; }
    int errorCount() const { return errors_; }
    bool wasCancelled() const { return cancelled_; }
    QString stopError() const { return stopError_; }
    QStringList inputs() const;

    QProgressDialog* progressDialog() const;
    // Whether this run redirects replayed Save As steps (the override).
    bool redirectsSaveAs() const;
    // The batch path for a Save As step in `format`; empty refuses the save.
    QString saveAsRedirect(const QString& format);
    void setMessageHandler(MessageHandler handler) { messageHandler_ = std::move(handler); }

signals:
    // Before each input: zero-based index, total, and the input's name.
    void progressChanged(int index, int total, const QString& name);
    void finished();

private:
    struct Input {
        QString path;
        QPointer<PictureView> view;
        QString name;
    };
    enum class Phase { Idle, Opening, Playing };

    void next();
    void advance();
    void onOpenFinished(quint64 token, int outcome, const QString& error);
    void handleOpen(int outcome, const QString& error);
    void play();
    void onPlaybackFinished(bool ok, const QString& error);
    QString applyDestination();
    void failInput(const QString& message);
    void discardCurrent();
    void finish();
    int indexOfView(PictureView* view) const;
    QString inputLabel() const;
    void report(const QString& text);

    PicturaMainWindow& frame_;
    BatchOptions options_;
    QList<Input> inputs_;
    int index_ = 0;
    Phase phase_ = Phase::Idle;
    quint64 openToken_ = 0;
    QHash<quint64, QPair<int, QString>> earlyOpens_;
    QPointer<PictureView> current_;
    bool batchOpened_ = false;
    bool redirectedSave_ = false;
    bool running_ = false;
    bool cancelled_ = false;
    bool logging_ = false;
    int processed_ = 0;
    int errors_ = 0;
    QString stopError_;
    QString startError_;
    QDate date_;
    QFile log_;
    QPointer<QProgressDialog> progress_;
    MessageHandler messageHandler_;
    QList<QMetaObject::Connection> connections_;
};

// The frame's Batch entry points: the dialog, starting a run, and the run in
// progress, which a replayed Save As step consults for the override.
class BatchHost : public QObject {
public:
    explicit BatchHost(PicturaMainWindow& frame);

    void showDialog();
    // Validate and start a batch; false when one cannot start.
    bool start(const BatchOptions& options);
    BatchRunner* runner() const { return runner_; }
    bool running() const;
    // Where a replayed Save As step writes during a run that overrides it.
    std::optional<QString> saveAsRedirect(const QString& format);
    void setMessageHandler(BatchRunner::MessageHandler handler)
    {
        messageHandler_ = std::move(handler);
    }

private:
    void report(const QString& text);

    PicturaMainWindow& frame_;
    QPointer<BatchRunner> runner_;
    BatchRunner::MessageHandler messageHandler_;
};

} // namespace pictura
