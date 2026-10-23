// The document-open entry points and the loading-tab lifecycle (#729).
//
// Every entry point starts the read/decode/first composite/history capture on
// a worker and returns a token. The worker queues its result onto the GUI
// thread, where the view applies it and emits `openFinished`; this file turns
// that into a filled tab, a removed tab, or a cancelled one. The synchronous
// wrappers at the bottom pump a local event loop for the self-test, the
// control server, and tests.

#include "frame_includes.h"

#include "loading_card.h"
#include "tab_spinner.h"

#include "api/document_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/open_jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEventLoop>
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStackedLayout>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

quint64 PicturaMainWindow::nextOpenToken()
{
    return nextOpenToken_++;
}

int PicturaMainWindow::indexOfView(PictureView* view) const
{
    for (int i = 0; i < docs_.size(); ++i) {
        if (docs_.at(i).view == view) {
            return i;
        }
    }
    return -1;
}

void PicturaMainWindow::beginOpenCursor()
{
    if (openCount_++ == 0) {
        QApplication::setOverrideCursor(Qt::BusyCursor);
    }
}

void PicturaMainWindow::endOpenCursor()
{
    if (openCount_ <= 0) {
        return;
    }
    if (--openCount_ == 0) {
        QApplication::restoreOverrideCursor();
    }
}

void PicturaMainWindow::wireOpenSignals(PictureView* view)
{
    if (!view || openWired_.contains(view)) {
        return;
    }
    openWired_.insert(view);
    connect(view, &PictureView::openDetected, this, [this, view] { onOpenDetected(view); });
    connect(view, &PictureView::openFinished, this,
            [this, view](const QString& status, const QStringList& warnings) {
                onOpenFinished(view, status, warnings);
            });
    connect(view, &PictureView::openProgress, this,
            [this, view](int percent, const QString& phase) {
                onOpenProgress(view, percent, phase);
            });
}

void PicturaMainWindow::beginLoadingDocument(PictureView* view, const QString& path,
                                             const QString& displayName, quint64 token,
                                             const QString& recentPath, OpenKind kind)
{
    if (!view) {
        return;
    }
    if (view->gpu_compute() != gpuCompute_) {
        view->set_gpu_compute(gpuCompute_);
    }
    if (view->color_policy() != colorPolicy_) {
        view->set_color_policy(colorPolicy_);
    }
    open_set_delay(*view, quint64(openDelayForTest_));
    wireOpenSignals(view);

    DocEntry entry;
    entry.view = view;
    entry.path = path;
    entry.displayName = displayName;
    entry.recentPath = recentPath;
    entry.token = token;
    entry.kind = kind;
    entry.loading = true;
    entry.previousActiveView = activeView();
    entry.page = new QWidget(this);
    entry.page->setObjectName(QStringLiteral("documentLoadingPage"));
    entry.page->setAutoFillBackground(true);
    QPalette palette = entry.page->palette();
    palette.setColor(QPalette::Window, canvasColorForIndex(canvasColorIndex_));
    entry.page->setPalette(palette);
    auto* stack = new QStackedLayout(entry.page);
    auto* cardPage = new QWidget(entry.page);
    auto* cardLayout = new QVBoxLayout(cardPage);
    cardLayout->setContentsMargins(0, 0, 0, 0);
    cardLayout->setSpacing(0);
    cardLayout->addStretch();
    const QString name = displayName.isEmpty() ? QFileInfo(path).fileName() : displayName;
    entry.loadingCard = new LoadingCard(name, cardPage);
    cardLayout->addWidget(entry.loadingCard, 0, Qt::AlignHCenter);
    cardLayout->addStretch();
    stack->addWidget(cardPage);
    connect(entry.loadingCard, &LoadingCard::cancelRequested, this,
            [this, guard = QPointer<PictureView>(view)] {
                QTimer::singleShot(0, this, [this, guard] {
                    if (guard) {
                        cancelOpenForView(guard.data());
                    }
                });
            });

    docs_.append(entry);
    const int index = docs_.size() - 1;
    tabs_->addTab(entry.page, documentName(index));
    if (tabs_->count() == 1) {
        tabs_->setCurrentIndex(index);
    }
    auto* spinner = new TabSpinner(tabs_->tabBar());
    spinner->setOpeningName(displayName.isEmpty() ? QFileInfo(path).fileName() : displayName);
    tabs_->tabBar()->setTabButton(index, QTabBar::RightSide, spinner);
    // The spinner is deleted with the tab; cancel on the next event-loop turn
    // so its own mouse handler has returned.
    connect(spinner, &TabSpinner::cancelRequested, this, [this, guard = QPointer<PictureView>(view)] {
        QTimer::singleShot(0, this, [this, guard] {
            if (guard) {
                cancelOpenForView(guard.data());
            }
        });
    });
    updateTabTitle(index);
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
}

void PicturaMainWindow::installCloseButton(int index, PictureView* view)
{
    auto* close = new TabCloseButton(tabs_->tabBar());
    tabs_->tabBar()->setTabButton(index, QTabBar::RightSide, close);
    connect(close, &TabCloseButton::closeRequested, this,
            [this, guard = QPointer<PictureView>(view)] {
                QTimer::singleShot(0, this, [this, guard] {
                    if (guard) {
                        closeDocument(indexOfView(guard.data()), true);
                    }
                });
            });
}

void PicturaMainWindow::finishLoadingDocument(int index)
{
    if (index < 0 || index >= docs_.size() || !docs_.at(index).loading) {
        return;
    }
    DocEntry& entry = docs_[index];
    entry.loading = false;
    if (entry.loadingCard) {
        entry.loadingCard->hide();
        entry.loadingCard->deleteLater();
        entry.loadingCard = nullptr;
    }
    setupCanvas(entry);
    if (entry.page && entry.page != entry.canvasHost) {
        if (auto* stack = qobject_cast<QStackedLayout*>(entry.page->layout())) {
            stack->addWidget(entry.canvasHost);
            stack->setCurrentWidget(entry.canvasHost);
        } else {
            entry.canvasHost->setParent(entry.page);
            entry.canvasHost->show();
        }
    }
    wireCanvas(entry);
    installCloseButton(index, entry.view);
    if (!entry.recentPath.isEmpty()) {
        rememberRecent(entry.recentPath);
    }
    const QString notice = document_api::openNotice(*entry.view);
    if (!notice.isEmpty()) {
        statusBar()->showMessage(notice);
    }
    updateTabTitle(index);
    if (!entry.userMovedAway) {
        suppressTabTracking_ = true;
        tabs_->setCurrentIndex(index);
        suppressTabTracking_ = false;
    }
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
    alertMissingFonts(entry.view);
    finishOpen(entry.token, OpenOutcome::Opened);
}

void PicturaMainWindow::restorePreviousTab(PictureView* view)
{
    if (!view) {
        return;
    }
    const int index = indexOfView(view);
    if (index < 0) {
        return;
    }
    suppressTabTracking_ = true;
    tabs_->setCurrentIndex(index);
    suppressTabTracking_ = false;
}

void PicturaMainWindow::cancelOpen(int index)
{
    if (index < 0 || index >= docs_.size()) {
        return;
    }
    const DocEntry entry = docs_.at(index);
    if (!entry.loading) {
        return;
    }
    if (entry.view) {
        open_cancel(*entry.view);
    }
    suppressTabTracking_ = true;
    removeDocument(index);
    suppressTabTracking_ = false;
    restorePreviousTab(entry.previousActiveView);
    finishOpen(entry.token, OpenOutcome::Cancelled);
}

void PicturaMainWindow::cancelOpenForView(PictureView* view)
{
    const int index = indexOfView(view);
    if (index >= 0) {
        cancelOpen(index);
    }
}

void PicturaMainWindow::waitForOpensForTest()
{
    while (openCount_ > 0) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
    }
}

void PicturaMainWindow::finishOpen(quint64 token, OpenOutcome outcome, const QString& error)
{
    if (token == 0 || finishedOpens_.contains(token)) {
        return;
    }
    finishedOpens_.insert(token);
    reportFailureTokens_.remove(token);
    openFailureMessages_.remove(token);
    if (cursorTokens_.remove(token)) {
        endOpenCursor();
    }
    emit documentOpenFinished(token, int(outcome), error);
}

PicturaMainWindow::OpenOutcome PicturaMainWindow::waitForOpen(quint64 token)
{
    if (token == 0) {
        return OpenOutcome::Failed;
    }
    OpenOutcome outcome = OpenOutcome::Failed;
    QEventLoop loop;
    const QMetaObject::Connection connection =
        connect(this, &PicturaMainWindow::documentOpenFinished, &loop,
                [&](quint64 finished, int result, const QString&) {
                    if (finished == token) {
                        outcome = static_cast<OpenOutcome>(result);
                        loop.quit();
                    }
                });
    loop.exec();
    disconnect(connection);
    return outcome;
}

void PicturaMainWindow::onOpenDetected(PictureView* view)
{
    const auto probe = pdfProbes_.value(view);
    if (probe.token == 0 || indexOfView(view) >= 0) {
        return;
    }
    beginLoadingDocument(view, probe.path, QFileInfo(probe.path).fileName(), probe.token,
                         probe.path, OpenKind::Pdf);
}

void PicturaMainWindow::onOpenProgress(PictureView* view, int percent, const QString& phase)
{
    const int index = indexOfView(view);
    if (index < 0 || !docs_.at(index).loading || !docs_.at(index).loadingCard) {
        return;
    }
    docs_.at(index).loadingCard->setProgress(percent, phase);
}

void PicturaMainWindow::reportOpenFailure(int index, const QString& status)
{
    if (index < 0 || index >= docs_.size()) {
        return;
    }
    const DocEntry& entry = docs_.at(index);
    const QString message =
        status.startsWith(QStringLiteral("error:")) ? status.mid(6) : status;
    switch (entry.kind) {
    case OpenKind::Pdf:
    case OpenKind::PdfPage:
    case OpenKind::Eps:
        reportImportError(openSourcePaths_.value(entry.token, entry.path),
                          message.isEmpty() ? tr("the document could not be opened.") : message);
        break;
    case OpenKind::SmartObject:
        statusBar()->showMessage(tr("Open As Smart Object: %1 is not a supported image")
                                     .arg(documentName(index)));
        break;
    case OpenKind::Psd:
    case OpenKind::Raster:
        if (openFailureMessages_.contains(entry.token)) {
            const QString source = entry.path.isEmpty() ? entry.recentPath : entry.path;
            reportImportError(source, openFailureMessages_.value(entry.token));
        } else if (reportFailureTokens_.contains(entry.token)) {
            statusBar()->showMessage(tr("Could not open %1").arg(documentName(index)), 4000);
        }
        break;
    }
}

void PicturaMainWindow::handlePdfProbeFinished(PictureView* view, const PdfProbe& probe,
                                               const QString& status,
                                               const QStringList& warnings)
{
    int index = indexOfView(view);
    if (status == QStringLiteral("ok")) {
        if (index < 0) {
            beginLoadingDocument(view, probe.path, QFileInfo(probe.path).fileName(),
                                 probe.token, probe.path, OpenKind::Pdf);
            index = indexOfView(view);
        }
        openWarnings_.insert(probe.token, warnings);
        if (index >= 0) {
            finishLoadingDocument(index);
        }
        return;
    }
    // A probe that found no editing data or needs a password has no tab.
    PictureView* previous =
        index >= 0 ? docs_.at(index).previousActiveView.data() : nullptr;
    if (index >= 0) {
        suppressTabTracking_ = true;
        removeDocument(index);
        suppressTabTracking_ = false;
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

void PicturaMainWindow::onOpenFinished(PictureView* view, const QString& status,
                                       const QStringList& warnings)
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
    if (index < 0 || !docs_.at(index).loading) {
        return;
    }
    const DocEntry entry = docs_.at(index);
    if (status == QStringLiteral("ok")) {
        openWarnings_.insert(entry.token, warnings);
        finishLoadingDocument(index);
        return;
    }
    QTimer::singleShot(0, this, [this, guard, entry, status] {
        if (!guard) {
            finishOpen(entry.token, OpenOutcome::Failed, status);
            return;
        }
        const int current = indexOfView(guard.data());
        if (current < 0 || !docs_.at(current).loading) {
            return;
        }
        reportOpenFailure(current, status);
        suppressTabTracking_ = true;
        removeDocument(current);
        suppressTabTracking_ = false;
        restorePreviousTab(entry.previousActiveView);
        finishOpen(entry.token, OpenOutcome::Failed, status);
    });
}

void PicturaMainWindow::cancelAllOpens()
{
    for (int i = docs_.size() - 1; i >= 0; --i) {
        if (docs_.at(i).loading) {
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

quint64 PicturaMainWindow::startOpenPath(const QString& path)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(this);
    beginOpenCursor();
    cursorTokens_.insert(token);
    beginLoadingDocument(view, path, QFileInfo(path).fileName(), token, path, OpenKind::Psd);
    open_psd_start(*view, path);
    return token;
}

quint64 PicturaMainWindow::startOpenImagePath(const QString& path)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(this);
    beginOpenCursor();
    cursorTokens_.insert(token);
    beginLoadingDocument(view, path, QFileInfo(path).fileName(), token, path,
                         OpenKind::Raster);
    open_image_start(*view, path);
    return token;
}

quint64 PicturaMainWindow::startOpenAsSmartObjectPath(const QString& path)
{
    const quint64 token = nextOpenToken();
    auto* view = new PictureView(this);
    beginOpenCursor();
    cursorTokens_.insert(token);
    beginLoadingDocument(view, QString(), QFileInfo(path).fileName(), token, QString(),
                         OpenKind::SmartObject);
    open_smart_object_start(*view, path);
    return token;
}

bool PicturaMainWindow::isNativeDocumentPath(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
}

quint64 PicturaMainWindow::startOpenDocumentAtPath(const QString& path, bool interactive,
                                                   bool reportFailure)
{
    const QString kind(vector_sniff(path));
    quint64 token = 0;
    if (kind == QStringLiteral("pdf") || isPdfPath(path)) {
        token = startOpenPdfPath(path, interactive);
    } else if (kind == QStringLiteral("eps")) {
        token = startImportEpsPath(path, interactive);
    } else if (isNativeDocumentPath(path)) {
        token = startOpenPath(path);
    } else {
        token = startOpenImagePath(path);
    }
    if (reportFailure && token != 0) {
        reportFailureTokens_.insert(token);
    }
    return token;
}

bool PicturaMainWindow::openPath(const QString& path)
{
    return waitForOpen(startOpenPath(path)) == OpenOutcome::Opened;
}

bool PicturaMainWindow::openImagePath(const QString& path)
{
    return waitForOpen(startOpenImagePath(path)) == OpenOutcome::Opened;
}

bool PicturaMainWindow::openAsSmartObjectPath(const QString& path)
{
    return waitForOpen(startOpenAsSmartObjectPath(path)) == OpenOutcome::Opened;
}

bool PicturaMainWindow::openDocumentAtPath(const QString& path, bool interactive)
{
    return openDocumentAtPathOutcome(path, interactive) == OpenOutcome::Opened;
}

PicturaMainWindow::OpenOutcome PicturaMainWindow::openDocumentAtPathOutcome(const QString& path,
                                                                            bool interactive)
{
    return waitForOpen(startOpenDocumentAtPath(path, interactive));
}

} // namespace pictura
