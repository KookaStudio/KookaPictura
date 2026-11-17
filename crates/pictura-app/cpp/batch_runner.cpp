#include "batch_runner.h"

#include "frame.h"
#include "api/view_bridge.h"
#include "panels/actions_panel.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QProgressDialog>

namespace pictura {

BatchRunner::BatchRunner(PicturaMainWindow& frame, const BatchOptions& options, QObject* parent)
    : QObject(parent)
    , frame_(frame)
    , options_(options)
{
}

BatchRunner::~BatchRunner()
{
    if (running_) {
        frame_.setSuppressProfileWarnings(false);
    }
    delete progress_.data();
}

QStringList BatchRunner::inputs() const
{
    QStringList out;
    for (const Input& input : inputs_) {
        out << (input.path.isEmpty() ? input.name : input.path);
    }
    return out;
}

bool BatchRunner::start()
{
    ActionsPanel* panel = frame_.actionsPanel();
    if (!panel) {
        startError_ = tr("The Actions panel is not available.");
        return false;
    }
    if (options_.source == BatchOptions::Source::Folder) {
        for (const QString& path : batchResolveFolder(options_.sourceFolder,
                                                      options_.includeSubfolders)) {
            inputs_ << Input{path, nullptr, QFileInfo(path).fileName()};
        }
    } else {
        for (int i = 0; i < frame_.documentCount(); ++i) {
            inputs_ << Input{frame_.documentPath(i), frame_.viewAt(i), frame_.documentName(i)};
        }
    }
    date_ = QDate::currentDate();
    running_ = true;

    if (options_.errors == BatchOptions::Errors::Log) {
        log_.setFileName(options_.errorLog);
        logging_ = log_.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text);
        if (!logging_) {
            report(tr("Cannot write the error log “%1”. The batch will stop at the first error.")
                       .arg(QDir::toNativeSeparators(options_.errorLog)));
        }
    }
    frame_.setSuppressProfileWarnings(options_.suppressProfile);
    connections_ << connect(&frame_, &PicturaMainWindow::documentOpenFinished, this,
                            &BatchRunner::onOpenFinished);
    connections_ << connect(panel, &ActionsPanel::playbackFinished, this,
                            &BatchRunner::onPlaybackFinished);

    progress_ = new QProgressDialog(tr("Batch"), tr("Cancel"), 0, int(inputs_.size()), &frame_);
    progress_->setObjectName(QStringLiteral("batchProgress"));
    progress_->setWindowTitle(tr("Batch"));
    progress_->setWindowModality(Qt::WindowModal);
    progress_->setAutoClose(false);
    progress_->setAutoReset(false);
    progress_->setMinimumDuration(0);
    connect(progress_, &QProgressDialog::canceled, this, &BatchRunner::cancel);
    progress_->show();

    QTimer::singleShot(0, this, &BatchRunner::next);
    return true;
}

void BatchRunner::cancel()
{
    if (!running_ || cancelled_) {
        return;
    }
    cancelled_ = true;
    if (phase_ == Phase::Playing) {
        // Stops after the current step; onPlaybackFinished discards the input.
        if (ActionsPanel* panel = frame_.actionsPanel()) {
            panel->cancelPlayback();
        }
    }
    // Opening: the open completes, then handleOpen discards it. Idle: next()
    // sees the flag.
}

void BatchRunner::next()
{
    if (!running_) {
        return;
    }
    if (cancelled_ || index_ >= inputs_.size()) {
        finish();
        return;
    }
    const Input& input = inputs_.at(index_);
    if (progress_) {
        progress_->setValue(index_);
        progress_->setLabelText(tr("Processing %1 (%2 of %3)")
                                    .arg(input.name)
                                    .arg(index_ + 1)
                                    .arg(inputs_.size()));
    }
    emit progressChanged(index_, int(inputs_.size()), input.name);
    if (!running_) {
        return;
    }
    if (cancelled_) {
        finish();
        return;
    }
    current_ = nullptr;
    batchOpened_ = false;
    redirectedSave_ = false;
    if (options_.source == BatchOptions::Source::OpenedFiles) {
        const int index = indexOfView(input.view);
        if (index < 0) {
            // Closed by the user before its turn: skipped, not an error.
            advance();
            return;
        }
        frame_.setActiveDocumentIndex(index);
        current_ = input.view;
        play();
        return;
    }
    phase_ = Phase::Opening;
    openToken_ = 0;
    earlyOpens_.clear();
    const quint64 token =
        frame_.startOpenDocumentAtPath(input.path, !options_.suppressOpen, false);
    if (token == 0) {
        phase_ = Phase::Idle;
        failInput(tr("Could not open the file."));
        return;
    }
    openToken_ = token;
    if (earlyOpens_.contains(token)) {
        openToken_ = 0;
        const auto early = earlyOpens_.take(token);
        handleOpen(early.first, early.second);
    }
}

void BatchRunner::advance()
{
    ++index_;
    QTimer::singleShot(0, this, &BatchRunner::next);
}

void BatchRunner::onOpenFinished(quint64 token, int outcome, const QString& error)
{
    if (phase_ != Phase::Opening) {
        return;
    }
    if (openToken_ == 0) {
        earlyOpens_.insert(token, qMakePair(outcome, error));
        return;
    }
    if (token == openToken_) {
        // Not inside the open's own signal: closing that document (on cancel
        // or error) while it is still emitting is unsafe.
        openToken_ = 0;
        QTimer::singleShot(0, this, [this, outcome, error] { handleOpen(outcome, error); });
    }
}

void BatchRunner::handleOpen(int outcome, const QString& error)
{
    phase_ = Phase::Idle;
    openToken_ = 0;
    const bool opened = outcome == int(PicturaMainWindow::OpenOutcome::Opened);
    if (opened) {
        current_ = frame_.activeView();
        batchOpened_ = true;
    }
    if (cancelled_) {
        discardCurrent();
        finish();
        return;
    }
    if (!opened) {
        failInput(error.isEmpty() ? tr("Could not open the file.") : error);
        return;
    }
    play();
}

void BatchRunner::play()
{
    ActionsPanel* panel = frame_.actionsPanel();
    phase_ = Phase::Playing;
    ActionsPanel::BatchPlayback playback;
    playback.overrideOpen = options_.overrideOpen;
    if (!panel || !panel->playActionForBatch(options_.set, options_.action, playback)) {
        phase_ = Phase::Idle;
        failInput(panel ? panel->lastError() : tr("The Actions panel is not available."));
    }
}

void BatchRunner::onPlaybackFinished(bool ok, const QString& error)
{
    if (phase_ != Phase::Playing) {
        return;
    }
    phase_ = Phase::Idle;
    if (cancelled_) {
        discardCurrent();
        finish();
        return;
    }
    if (!ok) {
        failInput(error.isEmpty() ? tr("The action failed.") : error);
        return;
    }
    const QString destinationError = applyDestination();
    if (!destinationError.isEmpty()) {
        failInput(destinationError);
        return;
    }
    ++processed_;
    advance();
}

QString BatchRunner::applyDestination()
{
    if (options_.destination == BatchOptions::Destination::None) {
        return {};
    }
    const int index = frame_.activeDocumentIndex();
    if (index < 0) {
        return tr("No document is open to save.");
    }
    // An overridden Save As already wrote this input's output.
    if (!(options_.overrideSave && redirectedSave_)) {
        const QString current = frame_.documentPath(index);
        QString target;
        if (options_.destination == BatchOptions::Destination::SaveAndClose) {
            if (current.isEmpty()) {
                return tr("The document has no file to save over.");
            }
            target = current;
        } else {
            const QString ext = current.isEmpty() ? QStringLiteral("psd")
                                                  : QFileInfo(current).suffix();
            const Input& input = inputs_.at(index_);
            const QString name = input.path.isEmpty() ? frame_.documentName(index)
                                                      : QFileInfo(input.path).fileName();
            target = QDir(options_.destinationFolder)
                         .filePath(batchRenderName(options_.namingJson(), name, ext, index_,
                                                   date_));
        }
        if (!frame_.saveActiveAsWithRememberedOptions(target)) {
            return tr("Could not save “%1”.").arg(QDir::toNativeSeparators(target));
        }
    }
    frame_.closeDocument(index, false);
    if (current_ && batchOpened_) {
        // The action switched documents; the input it opened is not kept.
        discardCurrent();
    }
    current_ = nullptr;
    return {};
}

void BatchRunner::failInput(const QString& message)
{
    ++errors_;
    if (logging_) {
        log_.write(batchErrorLogLine(inputLabel(), message).toUtf8());
        log_.flush();
        discardCurrent();
        advance();
        return;
    }
    stopError_ = tr("%1: %2").arg(QDir::toNativeSeparators(inputLabel()), message);
    finish();
}

void BatchRunner::discardCurrent()
{
    if (batchOpened_ && current_) {
        const int index = indexOfView(current_);
        if (index >= 0) {
            frame_.closeDocument(index, false);
        }
    }
    current_ = nullptr;
    batchOpened_ = false;
}

void BatchRunner::finish()
{
    if (!running_) {
        return;
    }
    running_ = false;
    phase_ = Phase::Idle;
    for (const QMetaObject::Connection& connection : connections_) {
        disconnect(connection);
    }
    connections_.clear();
    frame_.setSuppressProfileWarnings(false);
    if (progress_) {
        progress_->setValue(progress_->maximum());
        progress_->hide();
    }
    if (log_.isOpen()) {
        log_.close();
    }
    if (!stopError_.isEmpty()) {
        report(stopError_);
    } else if (logging_ && errors_ > 0) {
        const QString path = QDir::toNativeSeparators(options_.errorLog);
        report(errors_ == 1 ? tr("1 error was logged to “%1”.").arg(path)
                            : tr("%1 errors were logged to “%2”.").arg(errors_).arg(path));
    }
    emit finished();
}

int BatchRunner::indexOfView(PictureView* view) const
{
    if (!view) {
        return -1;
    }
    for (int i = 0; i < frame_.documentCount(); ++i) {
        if (frame_.viewAt(i) == view) {
            return i;
        }
    }
    return -1;
}

QString BatchRunner::inputLabel() const
{
    if (index_ < 0 || index_ >= inputs_.size()) {
        return {};
    }
    const Input& input = inputs_.at(index_);
    return input.path.isEmpty() ? input.name : input.path;
}

bool BatchRunner::redirectsSaveAs() const
{
    return running_ && options_.overrideSave
        && options_.destination != BatchOptions::Destination::None;
}

QString BatchRunner::saveAsRedirect(const QString& format)
{
    if (index_ < 0 || index_ >= inputs_.size()) {
        return {};
    }
    QString input = inputs_.at(index_).path;
    if (input.isEmpty()) {
        // An untitled opened document has no folder to save beside.
        if (options_.destination != BatchOptions::Destination::Folder) {
            return {};
        }
        input = inputs_.at(index_).name;
    }
    const QString path = batchOverrideSavePath(options_, input, format, index_, date_);
    if (!path.isEmpty()) {
        redirectedSave_ = true;
    }
    return path;
}

void BatchRunner::report(const QString& text)
{
    if (messageHandler_) {
        messageHandler_(tr("Batch"), text);
        return;
    }
    QMessageBox::information(&frame_, tr("Batch"), text);
}

} // namespace pictura

#include "moc_batch_runner.cpp"
