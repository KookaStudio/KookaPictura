// The document window registry: which window shows which document, where it
// is hosted, and which one is active.

#include "frame_includes.h"

#include <QtWidgets/QTabWidget>

namespace pictura {

int PicturaMainWindow::indexOfDoc(const DocEntry* doc) const
{
    return doc ? int(docs_.indexOf(const_cast<DocEntry*>(doc))) : -1;
}

DocWindow* PicturaMainWindow::windowAtTab(int tab) const
{
    QWidget* page = tabs_ && tab >= 0 ? tabs_->widget(tab) : nullptr;
    if (!page) {
        return nullptr;
    }
    for (DocWindow* window : windows_) {
        if (window->kind == DocWindow::Host::Tab && window->page == page) {
            return window;
        }
    }
    return nullptr;
}

int PicturaMainWindow::tabOf(const DocWindow* window) const
{
    if (!tabs_ || !window || window->kind != DocWindow::Host::Tab || !window->page) {
        return -1;
    }
    return tabs_->indexOf(window->page);
}

DocWindow* PicturaMainWindow::windowOfCanvas(const ImageView* canvas) const
{
    if (!canvas) {
        return nullptr;
    }
    for (DocWindow* window : windows_) {
        if (window->canvas == canvas) {
            return window;
        }
    }
    return nullptr;
}

int PicturaMainWindow::addTabWindow(DocWindow* window)
{
    window->kind = DocWindow::Host::Tab;
    // The tab windows lead `windows_`, so the new last tab goes after them.
    windows_.insert(tabs_->count(), window);
    return tabs_->addTab(window->page, documentDisplayName(*window->doc));
}

void PicturaMainWindow::activateDocWindow(DocWindow* window)
{
    const int tab = tabOf(window);
    if (tab >= 0) {
        tabs_->setCurrentIndex(tab);
    }
}

void PicturaMainWindow::onActiveWindowChanged()
{
    // A loading entry whose window is not active must not steal the activation
    // when it finishes. The frame's own switches are suppressed.
    if (!suppressTabTracking_) {
        for (DocEntry* doc : docs_) {
            if (doc->loading && (!activeWindow_ || activeWindow_->doc != doc)) {
                doc->userMovedAway = true;
            }
        }
    }
    PictureView* active = activeView();
    for (int i = 0; i < docs_.size(); ++i) {
        PictureView* view = viewAt(i);
        if (view && view != active && view->transform_session_active()) {
            view->cancel_transform();
            if (ImageView* canvas = canvasAt(i)) {
                canvas->clearTransformPreview();
            }
        }
    }
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
}

} // namespace pictura
