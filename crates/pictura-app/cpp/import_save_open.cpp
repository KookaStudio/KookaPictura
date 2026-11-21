// The document-open pipeline: every entry point starts the read/decode/first
// composite/history capture on a worker and returns a token. The worker queues
// its result onto the GUI thread, where the view applies it and emits
// `openFinished`; this file turns that into a filled tab, a removed tab, or a
// cancelled one. The synchronous wrappers pump a local event loop for the
// self-test, the control server, and tests.

#include "import_save_coordinator.h"

#include "api/document_api.h"

#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/type_api.h"

#include "command_ids.h"
#include "dialogs.h"
#include "document_registry.h"
#include "loading_card.h"
#include "memory_refusal.h"
#include "pictura_raw_dialog.h"
#include "profile_warning_dialog.h"
#include "type_fonts.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEventLoop>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonObject>
#include <QtCore/QTimer>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMessageBox>

#include <utility>

namespace pictura {

quint64 ImportSaveCoordinator::nextOpenToken()
{
    return nextOpenToken_++;
}

int ImportSaveCoordinator::indexOfView(PictureView* view) const
{
    return documents_.indexOfView(view);
}

void ImportSaveCoordinator::beginOpenCursor()
{
    if (openCount_++ == 0) {
        QApplication::setOverrideCursor(Qt::BusyCursor);
    }
}

void ImportSaveCoordinator::endOpenCursor()
{
    if (openCount_ <= 0) {
        return;
    }
    if (--openCount_ == 0) {
        QApplication::restoreOverrideCursor();
    }
}

void ImportSaveCoordinator::wireOpenSignals(PictureView* view)
{
    if (!view || openWired_.contains(view)) {
        return;
    }
    openWired_.insert(view);
    connect(view, &PictureView::openDetected, this, [this, view] { onOpenDetected(view); });
    connect(view, &PictureView::openFinished, this,
            [this, view](const QString& status, const QStringList& warnings,
                         quint32 code, bool memoryRefused) {
                onOpenFinished(view, status, warnings, code, memoryRefused);
            });
    connect(view, &PictureView::openProgress, this,
            [this, view](int percent, const QString& phase) {
                onOpenProgress(view, percent, phase);
            });
}

bool ImportSaveCoordinator::confirmProfileForOpen(PictureView* view, const QString& path)
{
    if (!view || path.isEmpty()) {
        return true;
    }
    if (suppressProfileWarnings_) {
        pictura::set_open_policy_override(*view, 0);
        return true;
    }
    const QStringList probe = pictura::probe_file_profile(path);
    if (probe.isEmpty()) {
        return true;
    }
    const QString kind = probe.value(0);
    const bool mismatch = kind == QStringLiteral("mismatch");
    const bool missing = kind == QStringLiteral("missing");
    if (kind == QStringLiteral("malformed")) {
        return true;
    }
    if ((!mismatch && !missing)
        || (mismatch && !pictura::ask_mismatch_open(*view))
        || (missing && !pictura::ask_missing_open(*view))) {
        return true;
    }
    ProfileWarningDialog dialog(mismatch ? ProfileWarningDialog::Mismatch
                                         : ProfileWarningDialog::Missing,
                                probe.value(1), dialogParent_);
    if (runDialog(dialog, dialogParent_) != QDialog::Accepted) {
        return false;
    }
    int policy = 0;
    switch (dialog.choice()) {
    case ProfileWarningDialog::ConvertToWorking:
    case ProfileWarningDialog::AssignThenConvert:
        policy = 1;
        break;
    case ProfileWarningDialog::DiscardEmbedded:
    case ProfileWarningDialog::LeaveUntagged:
        policy = 2;
        break;
    default:
        policy = 0;
        break;
    }
    pictura::set_open_policy_override(*view, policy);
    return true;
}

void ImportSaveCoordinator::restorePreviousTab(PictureView* view)
{
    if (!view) {
        return;
    }
    const int index = indexOfView(view);
    if (index < 0) {
        return;
    }
    documents_.setTabTrackingSuppressed(true);
    documents_.setActiveIndex(index);
    documents_.setTabTrackingSuppressed(false);
}

void ImportSaveCoordinator::cancelOpen(int index)
{
    if (index < 0 || index >= documents_.count()) {
        return;
    }
    const DocEntry entry = *documents_.docAt(index);
    if (!entry.loading) {
        return;
    }
    rawOpens_.remove(entry.view);
    if (entry.view) {
        open_cancel(*entry.view);
    }
    documents_.setTabTrackingSuppressed(true);
    documents_.removeDocument(index);
    documents_.setTabTrackingSuppressed(false);
    restorePreviousTab(entry.previousActiveView);
    finishOpen(entry.token, OpenOutcome::Cancelled);
}

void ImportSaveCoordinator::cancelOpenForView(PictureView* view)
{
    const int index = indexOfView(view);
    if (index >= 0) {
        cancelOpen(index);
    }
}

void ImportSaveCoordinator::finishLoadedDocument(int index)
{
    DocEntry* entry = documents_.docAt(index);
    if (!entry) {
        return;
    }
    const QString recentPath = entry->recentPath;
    const quint64 token = entry->token;
    PictureView* view = entry->view;
    documents_.finishLoadingDocument(index);
    trackRecovery(view);
    if (!recentPath.isEmpty()) {
        rememberRecent(recentPath);
    }
    QString notice = document_api::openNotice(*view);
    const QString rawNotice = rawSaveNotices_.take(token);
    if (!rawNotice.isEmpty()) {
        notice = notice.isEmpty() ? rawNotice : notice + QLatin1Char(' ') + rawNotice;
    }
    if (!notice.isEmpty() && hooks_.showStatus) {
        hooks_.showStatus(notice, 0);
    }
    alertMissingFonts(view);
    finishOpen(token, OpenOutcome::Opened);
}

void ImportSaveCoordinator::waitForOpensForTest()
{
    while (openCount_ > 0) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
    }
}

void ImportSaveCoordinator::finishOpen(quint64 token, OpenOutcome outcome, const QString& error,
                                       quint32 code, bool memoryRefused)
{
    if (token == 0 || finishedOpens_.contains(token)) {
        return;
    }
    finishedOpens_.insert(token);
    const QString recordPath = recordOpenPaths_.take(token);
    if (outcome == OpenOutcome::Opened && !recordPath.isEmpty()) {
        recordCommand(command_ids::FileOpen,
                      QJsonObject{{QStringLiteral("path"), recordPath}});
    }
    reportFailureTokens_.remove(token);
    openFailureMessages_.remove(token);
    if (cursorTokens_.remove(token)) {
        endOpenCursor();
    }
    if (outcome == OpenOutcome::Cancelled) {
        code = quint32(OperationStatus::Cancelled);
        memoryRefused = false;
    } else if (outcome == OpenOutcome::Failed && code == 0) {
        code = quint32(OperationStatus::Host);
    }
    emit documentOpenFinished(token, int(outcome), error, code, memoryRefused);
}

OpenOutcome ImportSaveCoordinator::waitForOpen(quint64 token, bool* memoryRefused)
{
    if (memoryRefused) {
        *memoryRefused = false;
    }
    if (token == 0) {
        return OpenOutcome::Failed;
    }
    OpenOutcome outcome = OpenOutcome::Failed;
    QEventLoop loop;
    const QMetaObject::Connection connection =
        connect(this, &ImportSaveCoordinator::documentOpenFinished, &loop,
                [&](quint64 finished, int result, const QString&, quint32, bool refused) {
                    if (finished == token) {
                        outcome = static_cast<OpenOutcome>(result);
                        if (memoryRefused) {
                            *memoryRefused = refused;
                        }
                        loop.quit();
                    }
                });
    loop.exec();
    disconnect(connection);
    return outcome;
}

void ImportSaveCoordinator::onOpenDetected(PictureView* view)
{
    const auto probe = pdfProbes_.value(view);
    if (probe.token == 0 || indexOfView(view) >= 0) {
        return;
    }
    documents_.addLoadingDocument(view, probe.path, QFileInfo(probe.path).fileName(),
                                  probe.token, probe.path, OpenKind::Pdf);
}

void ImportSaveCoordinator::onOpenProgress(PictureView* view, int percent,
                                           const QString& phase)
{
    const int index = indexOfView(view);
    if (index < 0 || !documents_.isLoading(index)) {
        return;
    }
    const DocWindow* window = documents_.docAt(index)->windows.value(0);
    if (!window || !window->card) {
        return;
    }
    window->card->setProgress(percent, phase);
}

void ImportSaveCoordinator::reportOpenFailure(int index, const QString& status)
{
    if (index < 0 || index >= documents_.count()) {
        return;
    }
    const DocEntry& entry = *documents_.docAt(index);
    const QString message =
        status.startsWith(QStringLiteral("error:")) ? status.mid(6) : status;
    switch (entry.kind) {
    case OpenKind::Pdf:
    case OpenKind::PdfPage:
    case OpenKind::Eps:
        reportImportError(openSourcePaths_.value(entry.token, entry.path),
                          message.isEmpty() ? tr("the document could not be opened.")
                                            : message);
        break;
    case OpenKind::SmartObject:
        if (hooks_.showStatus) {
            hooks_.showStatus(tr("Open As Smart Object: %1 is not a supported image")
                                  .arg(documents_.documentName(index)),
                              0);
        }
        break;
    case OpenKind::Psd:
    case OpenKind::Raster:
        if (status == QStringLiteral("error:memory")) {
            if (hooks_.showStatus) {
                hooks_.showStatus(
                    memoryRefusalMessage(
                        tr("open %1").arg(documents_.documentName(index))),
                    6000);
            }
        } else if (openFailureMessages_.contains(entry.token)) {
            const QString source = entry.path.isEmpty() ? entry.recentPath : entry.path;
            reportImportError(source, openFailureMessages_.value(entry.token));
        } else if (status.startsWith(QStringLiteral("error:")) && hooks_.showStatus) {
            hooks_.showStatus(message, 6000);
        } else if (reportFailureTokens_.contains(entry.token) && hooks_.showStatus) {
            hooks_.showStatus(
                tr("Could not open %1").arg(documents_.documentName(index)), 4000);
        }
        break;
    }
}

void ImportSaveCoordinator::alertMissingFonts(PictureView* view)
{
    const QStringList missing =
        view ? missingFonts(type_api::documentFonts(*view)) : QStringList();
    if (missing.isEmpty()) {
        return;
    }
    const QString remedy = type_api::layersEditable(*view)
        ? tr("Type > Replace All Missing Fonts sets them in the default font.")
        : tr("Type can only be re-set in an 8-bit RGB document, so convert it to 8-bit RGB "
             "before replacing them.");
    auto* box = new QMessageBox(
        QMessageBox::Warning, tr("Missing Fonts"),
        tr("This document's type layers use fonts that are not installed. They keep their "
           "current pixels until edited. %1\n\n%2")
            .arg(remedy, missing.join(QLatin1Char('\n'))),
        QMessageBox::Ok, dialogParent_);
    box->setObjectName(QStringLiteral("missingFontsAlert"));
    box->setAttribute(Qt::WA_DeleteOnClose);
    box->open();
}

void ImportSaveCoordinator::handlePdfProbeFinished(PictureView* view, const PdfProbe& probe,
                                                   const QString& status,
                                                   const QStringList& warnings)
{
    int index = indexOfView(view);
    if (status == QStringLiteral("ok")) {
        if (index < 0) {
            documents_.addLoadingDocument(view, probe.path, QFileInfo(probe.path).fileName(),
                                          probe.token, probe.path, OpenKind::Pdf);
            index = indexOfView(view);
        }
        openWarnings_.insert(probe.token, warnings);
        if (index >= 0) {
            finishLoadedDocument(index);
        }
        return;
    }
    // A probe that found no editing data or needs a password has no tab.
    PictureView* previous =
        index >= 0 ? documents_.docAt(index)->previousActiveView.data() : nullptr;
    if (index >= 0) {
        documents_.setTabTrackingSuppressed(true);
        documents_.removeDocument(index);
        documents_.setTabTrackingSuppressed(false);
        restorePreviousTab(previous);
    } else {
        open_cancel(*view);
        openWired_.remove(view);
        delete view;
    }
    if (status == QStringLiteral("none")) {
        importPdfPathWithToken(probe.path, probe.interactive, probe.password, probe.token);
    } else if (status == QStringLiteral("password")
               || status == QStringLiteral("wrongpassword")) {
        if (!probe.interactive) {
            finishOpen(probe.token, OpenOutcome::Failed, tr("the PDF is protected"));
            return;
        }
        QString password = probe.password;
        if (!askPdfPassword(probe.path, status == QStringLiteral("wrongpassword"), password)) {
            finishOpen(probe.token, OpenOutcome::Cancelled);
            return;
        }
        startPdfProbe(probe.path, probe.interactive, password, probe.token);
    } else {
        const QString message =
            status.startsWith(QStringLiteral("error:")) ? status.mid(6) : status;
        reportImportError(probe.path, message);
        finishOpen(probe.token, OpenOutcome::Failed, message);
    }
}

void ImportSaveCoordinator::onOpenFinished(PictureView* view, const QString& status,
                                           const QStringList& warnings, quint32 code,
                                           bool memoryRefused)
{
    // Deleting the view inside its own signal emission is unsafe. A failure or
    // a probe that falls back removes it on the next event-loop turn; a success
    // only fills the tab, so it can run now.
    const QPointer<PictureView> guard(view);
    if (pdfProbes_.contains(view)) {
        const PdfProbe probe = pdfProbes_.take(view);
        if (status == QStringLiteral("ok")) {
            handlePdfProbeFinished(view, probe, status, warnings);
            return;
        }
        QTimer::singleShot(0, this, [this, guard, probe, status, warnings] {
            if (!guard) {
                finishOpen(probe.token, OpenOutcome::Failed, status);
                return;
            }
            handlePdfProbeFinished(guard.data(), probe, status, warnings);
        });
        return;
    }

    const int index = indexOfView(view);
    if (index < 0 || !documents_.isLoading(index)) {
        return;
    }
    const DocEntry entry = *documents_.docAt(index);
    if (rawOpens_.contains(view)) {
        if (status == QStringLiteral("raw")) {
            const bool interactive = rawOpens_.value(view);
            QTimer::singleShot(0, this, [this, guard, interactive] {
                if (guard) {
                    developRawOpen(guard.data(), interactive);
                }
            });
            return;
        }
        rawOpens_.remove(view);
        if (status.startsWith(QStringLiteral("error:")) && status != QStringLiteral("error:memory")) {
            openFailureMessages_.insert(entry.token, status.mid(6));
        }
    }
    if (status == QStringLiteral("ok")) {
        openWarnings_.insert(entry.token, warnings);
        finishLoadedDocument(index);
        return;
    }
    QString feedback = status;
    if (entry.kind == OpenKind::Psd && code != 0 && !memoryRefused) {
        const QString name = documents_.documentName(index);
        feedback = code == quint32(OperationStatus::Panic)
            ? tr("Could not open %1. The operation stopped unexpectedly.").arg(name)
            : tr("Could not open %1: %2").arg(name, status.mid(6));
    }
    QTimer::singleShot(0, this, [this, guard, entry, status, feedback, code, memoryRefused] {
        if (!guard) {
            finishOpen(entry.token, OpenOutcome::Failed, feedback, code, memoryRefused);
            return;
        }
        const int current = indexOfView(guard.data());
        if (current < 0 || !documents_.isLoading(current)) {
            return;
        }
        if (entry.kind != OpenKind::Psd || reportFailureTokens_.contains(entry.token)) {
            if (feedback != status && hooks_.showStatus) {
                hooks_.showStatus(feedback, 6000);
            } else {
                reportOpenFailure(current, status);
            }
        }
        documents_.setTabTrackingSuppressed(true);
        documents_.removeDocument(current);
        documents_.setTabTrackingSuppressed(false);
        restorePreviousTab(entry.previousActiveView);
        finishOpen(entry.token, OpenOutcome::Failed, feedback, code, memoryRefused);
    });
}

void ImportSaveCoordinator::cancelAllOpens()
{
    for (int i = documents_.count() - 1; i >= 0; --i) {
        if (documents_.isLoading(i)) {
            cancelOpen(i);
        }
    }
    const QHash<PictureView*, PdfProbe> probes = pdfProbes_;
    pdfProbes_.clear();
    for (auto it = probes.cbegin(); it != probes.cend(); ++it) {
        PictureView* view = it.key();
        if (indexOfView(view) >= 0) {
            continue;
        }
        open_cancel(*view);
        openWired_.remove(view);
        delete view;
        finishOpen(it.value().token, OpenOutcome::Cancelled);
    }
}

quint64 ImportSaveCoordinator::startOpenPath(const QString& path, bool reportFailure)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(dialogParent_);
    if (hooks_.applyColorSettings) {
        hooks_.applyColorSettings(view);
    }
    if (!confirmProfileForOpen(view, path)) {
        delete view;
        return 0;
    }
    beginOpenCursor();
    cursorTokens_.insert(token);
    if (reportFailure) {
        reportFailureTokens_.insert(token);
    }
    documents_.addLoadingDocument(view, path, QFileInfo(path).fileName(), token, path,
                                  OpenKind::Psd);
    open_psd_start(*view, path, std::exchange(nextNativeOpenPanicForTest_, false));
    return token;
}

quint64 ImportSaveCoordinator::startOpenImagePath(const QString& path)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(dialogParent_);
    if (hooks_.applyColorSettings) {
        hooks_.applyColorSettings(view);
    }
    if (!confirmProfileForOpen(view, path)) {
        delete view;
        return 0;
    }
    beginOpenCursor();
    cursorTokens_.insert(token);
    documents_.addLoadingDocument(view, path, QFileInfo(path).fileName(), token, path,
                                  OpenKind::Raster);
    open_image_start(*view, path);
    return token;
}

quint64 ImportSaveCoordinator::startOpenRawPath(const QString& path, bool interactive)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(dialogParent_);
    if (hooks_.applyColorSettings) {
        hooks_.applyColorSettings(view);
    }
    beginOpenCursor();
    cursorTokens_.insert(token);
    rawOpens_.insert(view, interactive);
    // No document path: the result is a new document, so Save never writes
    // over the raw file. The raw file is still the recent entry.
    documents_.addLoadingDocument(view, QString(), QFileInfo(path).fileName(), token, path,
                                  OpenKind::Raster);
    open_raw_start(*view, path, rawSettingsStore_);
    return token;
}

void ImportSaveCoordinator::developRawOpen(PictureView* view, bool interactive)
{
    const int index = indexOfView(view);
    if (index < 0 || !documents_.isLoading(index)) {
        rawOpens_.remove(view);
        return;
    }
    QString settings = raw_seed_settings(*view);
    if (interactive) {
        const QString name = documents_.docAt(index)->displayName;
        const quint64 token = documents_.docAt(index)->token;
        PicturaRawDialog dialog(view,
                                AcrAsShot{raw_as_shot_temperature(*view), raw_as_shot_tint(*view)},
                                settings, dialogParent_);
        dialog.setDialogRunner([this](QDialog& nested) { return runImportDialog(nested); });
        // The open's busy cursor must not cover the dialog.
        QApplication::setOverrideCursor(Qt::ArrowCursor);
        const bool accepted = runImportDialog(dialog);
        QApplication::restoreOverrideCursor();
        if (accepted || dialog.saveOnlyRequested()) {
            saveRawSettings(view, token, name, dialog.settingsText(), !accepted);
        }
        if (!accepted) {
            cancelOpen(indexOfView(view));
            return;
        }
        settings = dialog.settingsText();
    }
    open_raw_develop_start(*view, settings);
}

void ImportSaveCoordinator::saveRawSettings(PictureView* view, quint64 token, const QString& name,
                                            const QString& settings, bool saveOnly)
{
    const QString status = raw_save_settings(*view, settings, rawSettingsStore_);
    QString message;
    if (status.startsWith(QStringLiteral("error:"))) {
        message = tr("Camera Raw settings for %1 were not saved: %2").arg(name, status.mid(6));
    } else if (status.startsWith(QStringLiteral("fallback:"))) {
        message = tr("The sidecar for %1 could not be written, so its Camera Raw settings were "
                     "saved in the Camera Raw database.")
                      .arg(name);
    } else if (saveOnly) {
        message = tr("Camera Raw settings saved for %1.").arg(name);
    }
    if (message.isEmpty()) {
        return;
    }
    if (!saveOnly) {
        rawSaveNotices_.insert(token, message);
    } else if (hooks_.showStatus) {
        hooks_.showStatus(message, 0);
    }
}

quint64 ImportSaveCoordinator::startOpenAsSmartObjectPath(const QString& path)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(dialogParent_);
    beginOpenCursor();
    cursorTokens_.insert(token);
    documents_.addLoadingDocument(view, QString(), QFileInfo(path).fileName(), token, QString(),
                                  OpenKind::SmartObject);
    open_smart_object_start(*view, path);
    return token;
}

bool ImportSaveCoordinator::isNativeDocumentPath(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
}

bool ImportSaveCoordinator::saveRunsOnWorker(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
}

quint64 ImportSaveCoordinator::startOpenDocumentAtPath(const QString& path, bool interactive,
                                                       bool reportFailure)
{
    const QString kind(vector_sniff(path));
    quint64 token = 0;
    if (kind == QStringLiteral("pdf") || isPdfPath(path)) {
        token = startOpenPdfPath(path, interactive);
    } else if (kind == QStringLiteral("eps")) {
        token = startImportEpsPath(path, interactive);
    } else if (raw_sniff_path(path)) {
        token = startOpenRawPath(path, interactive);
    } else if (isNativeDocumentPath(path)) {
        token = startOpenPath(path, reportFailure);
    } else {
        token = startOpenImagePath(path);
    }
    if (reportFailure && token != 0) {
        reportFailureTokens_.insert(token);
    }
    return token;
}

quint64 ImportSaveCoordinator::startUserOpen(const QString& path, bool reportFailure)
{
    const quint64 token = startOpenDocumentAtPath(path, true, reportFailure);
    if (token != 0 && !finishedOpens_.contains(token)) {
        recordOpenPaths_.insert(token, QFileInfo(path).absoluteFilePath());
    }
    return token;
}

bool ImportSaveCoordinator::openPath(const QString& path)
{
    return waitForOpen(startOpenPath(path)) == OpenOutcome::Opened;
}

bool ImportSaveCoordinator::openImagePath(const QString& path)
{
    return waitForOpen(startOpenImagePath(path)) == OpenOutcome::Opened;
}

bool ImportSaveCoordinator::openAsSmartObjectPath(const QString& path)
{
    return waitForOpen(startOpenAsSmartObjectPath(path)) == OpenOutcome::Opened;
}

bool ImportSaveCoordinator::openDocumentAtPath(const QString& path, bool interactive)
{
    return openDocumentAtPathOutcome(path, interactive) == OpenOutcome::Opened;
}

OpenOutcome ImportSaveCoordinator::openDocumentAtPathOutcome(const QString& path,
                                                             bool interactive, bool* memoryRefused)
{
    const bool reportFailure = interactive && isNativeDocumentPath(path);
    return waitForOpen(startOpenDocumentAtPath(path, interactive, reportFailure), memoryRefused);
}

} // namespace pictura
