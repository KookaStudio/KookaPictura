// The document-open entry points and the loading indication shown around a
// blocking read/decode (issue #667). Split from frame.cpp, which sits at the
// file-size hard cap.

#include "frame_includes.h"

#include "api/document_api.h"

#include <QtCore/QEventLoop>
#include <QtCore/QFileInfo>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStatusBar>

namespace pictura {

namespace {
// The banner floats a small margin in from the window's lower-left corner and
// wraps rather than run past the window.
constexpr int kBannerMargin = 12;
constexpr int kBannerMaxWidth = 480;
} // namespace

// Show a boxed "Opening <file>…" banner with a wait cursor and paint it before
// the blocking load starts, so a large open reads as work in progress rather
// than a hang.
void PicturaMainWindow::beginOpenIndicator(const QString& path)
{
    if (!openBanner_) {
        openBanner_ = new QLabel(this);
        openBanner_->setObjectName(QStringLiteral("documentOpenBanner"));
        openBanner_->setAttribute(Qt::WA_TransparentForMouseEvents);
        openBanner_->setWordWrap(true);
        openBanner_->setMaximumWidth(kBannerMaxWidth);
    }
    openBanner_->setText(tr("Opening %1…").arg(QFileInfo(path).fileName()));
    openBanner_->adjustSize();
    positionOpenBanner();
    openBanner_->show();
    openBanner_->raise();
    QApplication::setOverrideCursor(Qt::WaitCursor);
    openBanner_->repaint();
    // User input is excluded so pumped events cannot start a second open.
    QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    emit documentOpenStarted(path);
}

void PicturaMainWindow::endOpenIndicator()
{
    if (openBanner_) {
        openBanner_->hide();
    }
    QApplication::restoreOverrideCursor();
}

// Keep the banner pinned above the status bar at the window's lower-left.
void PicturaMainWindow::positionOpenBanner()
{
    if (!openBanner_) {
        return;
    }
    int bottom = height();
    if (statusBar() && statusBar()->isVisible()) {
        bottom = statusBar()->geometry().top();
    }
    openBanner_->move(kBannerMargin,
                      qMax(kBannerMargin, bottom - openBanner_->height() - kBannerMargin));
}

QString PicturaMainWindow::openBannerTextForTest() const
{
    return openBanner_ ? openBanner_->text() : QString();
}

bool PicturaMainWindow::openBannerVisibleForTest() const
{
    return openBanner_ && openBanner_->isVisibleTo(this);
}

bool PicturaMainWindow::openPath(const QString& path)
{
    auto* view = new PictureView(this);
    view->set_color_policy(colorPolicy_);
    beginOpenIndicator(path);
    const bool loaded = document_api::open(*view, path);
    endOpenIndicator();
    if (!loaded) {
        delete view;
        return false;
    }
    addDocument(view, path);
    const QString notice = document_api::openNotice(*view);
    if (!notice.isEmpty()) {
        statusBar()->showMessage(notice);
    }
    return true;
}

bool PicturaMainWindow::openImagePath(const QString& path)
{
    auto* view = new PictureView(this);
    beginOpenIndicator(path);
    const bool loaded = document_api::openImage(*view, path);
    endOpenIndicator();
    if (!loaded) {
        delete view;
        return false;
    }
    // Imported pixels become an untitled PSD document; keeping the image path
    // would let Ctrl+S write PSD bytes over the source image. The file's base
    // name is kept only as the tab's display name.
    const int index = addDocument(view, QString());
    if (index >= 0) {
        docs_[index].displayName = QFileInfo(path).fileName();
        updateTabTitle(index);
    }
    // The source image is still what Open Recent should reopen.
    rememberRecent(path);
    return true;
}

bool PicturaMainWindow::isNativeDocumentPath(const QString& path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    return suffix == QStringLiteral("psd") || suffix == QStringLiteral("psb");
}

bool PicturaMainWindow::openDocumentAtPath(const QString& path)
{
    if (isPdfPath(path)) {
        return openPdfPath(path);
    }
    return isNativeDocumentPath(path) ? openPath(path) : openImagePath(path);
}

bool PicturaMainWindow::openAsSmartObjectPath(const QString& path)
{
    auto* view = new PictureView(this);
    beginOpenIndicator(path);
    const bool loaded = document_api::openAsSmartObject(*view, path);
    endOpenIndicator();
    if (!loaded) {
        delete view;
        statusBar()->showMessage(tr("Open As Smart Object: %1 is not a supported image")
                                     .arg(QFileInfo(path).fileName()));
        return false;
    }
    const int index = addDocument(view, QString());
    if (index >= 0) {
        docs_[index].displayName = QFileInfo(path).fileName();
        updateTabTitle(index);
    }
    return true;
}

} // namespace pictura
