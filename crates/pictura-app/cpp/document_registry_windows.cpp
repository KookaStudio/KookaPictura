// Document window hosts: the tab strip, floating windows and tile panes, the
// active window, and window presentation.

#include "document_registry.h"

#include "api/document_bridge.h"
#include "canvas_scrollbars.h"
#include "document_float_window.h"
#include "document_tile_host.h"
#include "image_view.h"

#include <QtCore/QSizeF>
#include <QtCore/QTimer>
#include <QtGui/QKeySequence>
#include <QtWidgets/QApplication>
#include <QtGui/QShortcut>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QWidget>

namespace pictura {

namespace {

constexpr double kFloatAreaFraction = 0.6;

DocWindow* documentWindowAt(const QList<DocEntry*>& docs, int index, int ordinal)
{
    if (index < 0 || index >= docs.size() || ordinal < 0
        || ordinal >= docs.at(index)->windows.size()) {
        return nullptr;
    }
    return docs.at(index)->windows.at(ordinal);
}

} // namespace

void DocumentRegistry::wireDocumentCycling()
{
    QWidget* shell = hooks_.shellWidget ? hooks_.shellWidget() : nullptr;
    auto* next = new QShortcut(QKeySequence(Qt::CTRL | Qt::Key_Tab), shell);
    connect(next, &QShortcut::activated, this, [this]() { cycleDocumentWindow(1); });
    auto* previous = new QShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_Tab), shell);
    connect(previous, &QShortcut::activated, this, [this]() { cycleDocumentWindow(-1); });
}

void DocumentRegistry::cycleDocumentWindow(int step)
{
    const int count = int(windows_.size());
    const int current = int(windows_.indexOf(activeWindow_));
    if (count < 2 || current < 0) {
        return;
    }
    activateDocWindow(windows_.at(((current + step) % count + count) % count));
}

int DocumentRegistry::addTabWindow(DocWindow* window)
{
    window->kind = DocWindow::Host::Tab;
    windows_.insert(tabs_->count(), window);
    return tabs_->addTab(window->page, documentDisplayName(*window->doc));
}

void DocumentRegistry::hostReadyWindow(DocWindow* window, bool activate)
{
    if (tileHost_) {
        tileWindow(window, activate);
        return;
    }
    if ((!hooks_.openDocumentsAsTabs || !hooks_.openDocumentsAsTabs())
        && window->kind == DocWindow::Host::Tab) {
        floatWindow(window, QRect(), activate);
    }
}

bool DocumentRegistry::stripOwnsActivation() const
{
    return !activeWindow_ || !windows_.contains(activeWindow_)
           || activeWindow_->kind == DocWindow::Host::Tab;
}

void DocumentRegistry::activateDocWindow(DocWindow* window)
{
    if (!window || !windows_.contains(window)) {
        return;
    }
    if (window->kind == DocWindow::Host::Tab) {
        const int tab = tabOf(window);
        if (tab < 0) {
            return;
        }
        if (qobject_cast<DocumentFloatWindow*>(QApplication::activeWindow())
            && hooks_.activateShellWindow) {
            hooks_.activateShellWindow();
        }
        if (tabs_->currentIndex() != tab) {
            const bool stripFollows = stripOwnsActivation();
            tabs_->setCurrentIndex(tab);
            if (stripFollows) {
                return;
            }
        }
    } else if (window->kind == DocWindow::Host::Tile) {
        if (qobject_cast<DocumentFloatWindow*>(QApplication::activeWindow())
            && hooks_.activateShellWindow) {
            hooks_.activateShellWindow();
        }
    } else if (DocumentFloatWindow* floating = window->floating) {
        if (floating->isVisible()) {
            raiseDocumentFloat(floating);
            if (floating->isWindow() && !floating->isActiveWindow()) {
                floating->activateWindow();
            }
        }
    }
    if (activeWindow_ != window) {
        activeWindow_ = window;
        onActiveWindowChanged();
    }
}

void DocumentRegistry::onTabCurrentChanged(int tab)
{
    if (movingHost_ || !stripOwnsActivation()) {
        return;
    }
    DocWindow* window = windowAtTab(tab);
    if (!window && !windows_.isEmpty()) {
        window = windows_.first();
    }
    activeWindow_ = window;
    onActiveWindowChanged();
}

void DocumentRegistry::onTabBarClicked(int tab)
{
    DocWindow* window = windowAtTab(tab);
    if (window && !stripOwnsActivation()) {
        activateDocWindow(window);
    }
}

void DocumentRegistry::requestCloseWindow(DocWindow* window)
{
    if (!window || !windows_.contains(window)) {
        return;
    }
    if (window->doc->windows.size() < 2) {
        emit closeDocumentRequested(indexOfDoc(window->doc));
    } else if (hooks_.closeWindowRequested) {
        hooks_.closeWindowRequested(window);
    }
}

void DocumentRegistry::onActiveWindowChanged()
{
    if (activeWindow_) {
        activeWindow_->doc->lastActive = activeWindow_;
    }
    if (hooks_.fullScreen && hooks_.fullScreen()) {
        syncScreenModeWindows();
    }
    if (!suppressTabTracking_) {
        for (DocEntry* doc : docs_) {
            if (doc->loading && (!activeWindow_ || activeWindow_->doc != doc)) {
                doc->userMovedAway = true;
            }
        }
    }
    PictureView* active = activeView();
    for (const DocEntry* doc : docs_) {
        PictureView* view = doc->view;
        if (view && view != active && view->transform_session_active()) {
            view->cancel_transform();
            forEachCanvas(*doc, [](ImageView* canvas) { canvas->clearTransformPreview(); });
        }
    }
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (hooks_.stopPanelRefreshTimer) {
        hooks_.stopPanelRefreshTimer();
    }
    if (hooks_.requestPanelRefresh) {
        hooks_.requestPanelRefresh();
    }
    emit activeDocumentChanged();
}

bool DocumentRegistry::addDocumentWindow(DocEntry* doc)
{
    if (!doc || !docs_.contains(doc) || doc->loading || !doc->view) {
        return false;
    }
    auto* window = new DocWindow;
    window->doc = doc;
    doc->windows.append(window);
    if (hooks_.setupCanvas) {
        hooks_.setupCanvas(*window);
    }
    window->page = window->host;
    if (hooks_.wireCanvas) {
        hooks_.wireCanvas(*window);
    }
    addTabWindow(window);
    installCloseButton(window);
    activateDocWindow(window);
    hostReadyWindow(window, true);
    updateTabTitle(indexOfDoc(doc));
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (hooks_.stopPanelRefreshTimer) {
        hooks_.stopPanelRefreshTimer();
    }
    if (hooks_.requestPanelRefresh) {
        hooks_.requestPanelRefresh();
    }
    return true;
}

QRect DocumentRegistry::documentAreaGlobalRect() const
{
    return documentArea_ ? QRect(documentArea_->mapToGlobal(QPoint(0, 0)), documentArea_->size())
                         : QRect();
}

QRect DocumentRegistry::clampFloatGeometry(const QRect& geometry) const
{
    const QRect area = documentAreaGlobalRect();
    if (area.isEmpty()) {
        return geometry;
    }
    QRect clamped(geometry.topLeft(), geometry.size().boundedTo(area.size()));
    const int maxX = qMax(area.left(), area.right() - clamped.width() + 1);
    const int maxY = qMax(area.top(), area.bottom() - DocumentFloatWindow::kHeaderHeight + 1);
    clamped.moveTopLeft(QPoint(qBound(area.left(), clamped.left(), maxX),
                               qBound(area.top(), clamped.top(), maxY)));
    return clamped;
}

QRect DocumentRegistry::defaultFloatGeometry() const
{
    const QRect area = documentAreaGlobalRect();
    QRect geometry(QPoint(), (QSizeF(area.size()) * kFloatAreaFraction).toSize());
    geometry.moveCenter(area.center());
    return geometry;
}

void DocumentRegistry::raiseDocumentFloat(DocumentFloatWindow* floating)
{
    floating->raise();
    if (floating->isWindow()) {
        return;
    }
    if (hooks_.raisePanelOverlays) {
        hooks_.raisePanelOverlays();
    }
}

bool DocumentRegistry::floatWindow(DocWindow* window, const QRect& globalGeometry, bool activate)
{
    if (!window || !windows_.contains(window) || window->doc->loading
        || window->kind == DocWindow::Host::Float || !window->page) {
        return false;
    }
    const bool wasActive = activeWindow_ == window;
    auto* floating = new DocumentFloatWindow(hooks_.shellWidget());
    movingHost_ = true;
    const int tab = tabOf(window);
    if (tab >= 0) {
        tabs_->removeTab(tab);
    }
    movingHost_ = false;
    untileWindow(window);
    windows_.removeOne(window);
    windows_.append(window);
    window->kind = DocWindow::Host::Float;
    window->floating = floating;
    floating->setPage(window->page);
    floating->onActivated = [this, window, floating]() {
        if (windows_.contains(window) && window->floating == floating) {
            activateDocWindow(window);
        }
    };
    floating->onCloseRequested = [this, window, floating]() {
        QTimer::singleShot(0, this, [this, window, guard = QPointer<DocumentFloatWindow>(floating)] {
            if (guard && windows_.contains(window) && window->floating == guard) {
                requestCloseWindow(window);
            }
        });
    };
    floating->onContextMenu = [this, window, floating](const QPoint& globalPos) {
        if (windows_.contains(window) && window->floating == floating) {
            emit headerContextMenuRequested(window, globalPos);
        }
    };
    floating->onDragMoved = [this, window, floating](const QPoint& globalPos) {
        if (windows_.contains(window) && window->floating == floating) {
            // The dock indicator is drawn by the drag path.
            updateDockIndicator(globalPos);
        }
    };
    floating->onDragReleased = [this, window](const QPoint& globalPos) {
        onFloatDragReleased(window, globalPos);
    };
    floating->onDockDragRequested = [this, window, floating]() {
        if (windows_.contains(window) && window->floating == floating) {
            startWindowDockDrag(window);
        }
    };
    floating->onWindowDragRequested = [this, window, floating](const QPoint& grabOffset) {
        if (windows_.contains(window) && window->floating == floating) {
            startWindowDockDrag(window, true, grabOffset);
        }
    };
    floating->onDockRequested = [this, window, floating]() {
        if (windows_.contains(window) && window->floating == floating) {
            dockWindow(window, -1);
        }
    };
    floating->setGlobalGeometry(
        clampFloatGeometry(globalGeometry.isEmpty() ? defaultFloatGeometry() : globalGeometry));
    floating->show();
    updateTabTitle(indexOfDoc(window->doc));
    if (activate || wasActive) {
        activateDocWindow(window);
    } else {
        raiseDocumentFloat(floating);
        if (!stripOwnsActivation() && activeWindow_->floating) {
            raiseDocumentFloat(activeWindow_->floating);
        }
    }
    if (hooks_.fullScreen && hooks_.fullScreen()) {
        syncScreenModeWindows();
    }
    if (wasActive && hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    emit windowsChanged();
    return true;
}

bool DocumentRegistry::dockWindow(DocWindow* window, int index)
{
    if (!window || !windows_.contains(window) || window->kind == DocWindow::Host::Tab
        || window->doc->loading || !window->page) {
        return false;
    }
    DocumentFloatWindow* floating = window->floating;
    const int count = tabs_->count();
    const int at = index < 0 || index > count ? count : index;
    windows_.removeOne(window);
    windows_.insert(at, window);
    untileWindow(window);
    window->kind = DocWindow::Host::Tab;
    window->floating = nullptr;
    if (window->page->parentWidget() == documentArea_) {
        documentArea_->removeWidget(window->page);
    }
    movingHost_ = true;
    tabs_->insertTab(at, window->page, documentDisplayName(*window->doc));
    movingHost_ = false;
    installCloseButton(window);
    if (floating) {
        floating->hide();
        floating->deleteLater();
    }
    activateDocWindow(window);
    updateTabTitle(indexOfDoc(window->doc));
    if (hooks_.fullScreen && hooks_.fullScreen()) {
        syncScreenModeWindows();
    }
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    emit windowsChanged();
    return true;
}

bool DocumentRegistry::tileWindow(DocWindow* window, bool activate)
{
    if (!window || !windows_.contains(window) || window->doc->loading
        || window->kind == DocWindow::Host::Tile || !window->page) {
        return false;
    }
    const bool wasActive = activeWindow_ == window;
    if (!tileHost_) {
        tileHost_ = new DocumentTileHost(documentArea_);
        documentArea_->addWidget(tileHost_);
    }
    movingHost_ = true;
    const int tab = tabOf(window);
    if (tab >= 0) {
        tabs_->removeTab(tab);
    }
    movingHost_ = false;
    if (DocumentFloatWindow* floating = window->floating) {
        if (window->page->parentWidget() == documentArea_) {
            documentArea_->removeWidget(window->page);
        }
        floating->hide();
        floating->deleteLater();
        window->floating = nullptr;
    }
    windows_.removeOne(window);
    windows_.append(window);
    window->kind = DocWindow::Host::Tile;
    DocumentTilePane* pane = tileHost_->addPane(window->page);
    window->tile = pane;
    pane->onActivated = [this, window, pane]() {
        if (windows_.contains(window) && window->tile == pane) {
            activateDocWindow(window);
        }
    };
    pane->onCloseRequested = [this, window, pane]() {
        QTimer::singleShot(0, this, [this, window, guard = QPointer<DocumentTilePane>(pane)] {
            if (guard && windows_.contains(window) && window->tile == guard) {
                requestCloseWindow(window);
            }
        });
    };
    pane->onContextMenu = [this, window, pane](const QPoint& globalPos) {
        if (windows_.contains(window) && window->tile == pane) {
            emit headerContextMenuRequested(window, globalPos);
        }
    };
    if (!hooks_.fullScreen || !hooks_.fullScreen()) {
        documentArea_->setCurrentWidget(tileHost_);
    }
    updateTabTitle(indexOfDoc(window->doc));
    if (activate || wasActive) {
        activateDocWindow(window);
    }
    if (hooks_.fullScreen && hooks_.fullScreen()) {
        syncScreenModeWindows();
    }
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    emit windowsChanged();
    return true;
}

void DocumentRegistry::untileWindow(DocWindow* window)
{
    DocumentTilePane* pane = window->tile;
    if (!pane || !tileHost_) {
        return;
    }
    window->tile = nullptr;
    tileHost_->removePane(pane);
    if (tileHost_->paneCount() > 0) {
        return;
    }
    DocumentTileHost* host = tileHost_;
    tileHost_ = nullptr;
    if (documentArea_->currentWidget() == host) {
        documentArea_->setCurrentWidget(tabs_);
    }
    documentArea_->removeWidget(host);
    host->hide();
    host->deleteLater();
    emit windowsChanged();
}

QRect DocumentRegistry::dockZoneGlobalRect() const
{
    if ((hooks_.floatingDocumentDocking && !hooks_.floatingDocumentDocking()) || tileHost_
        || !tabs_->isVisible()) {
        return QRect();
    }
    QTabBar* bar = tabs_->tabBar();
    const int barHeight =
        bar->isVisible() && bar->count() > 0 ? bar->height() : bar->sizeHint().height();
    return QRect(tabs_->mapToGlobal(QPoint(0, 0)),
                 QSize(tabs_->width(), qMax(barHeight, DocumentFloatWindow::kHeaderHeight)));
}

int DocumentRegistry::dockIndexAt(const QPoint& globalPos) const
{
    QTabBar* bar = tabs_->tabBar();
    return bar->isVisible() ? bar->tabAt(bar->mapFromGlobal(globalPos)) : -1;
}

void DocumentRegistry::onFloatDragReleased(DocWindow* window, const QPoint& globalPos)
{
    hideDockIndicator();
    if (!windows_.contains(window) || window->kind != DocWindow::Host::Float
        || !dockZoneGlobalRect().contains(globalPos)) {
        return;
    }
    dockWindow(window, dockIndexAt(globalPos));
}

void DocumentRegistry::detachWindows(const QList<DocWindow*>& windows)
{
    for (DocWindow* window : windows) {
        windows_.removeOne(window);
        if (tearWindow_ == window) {
            tearWindow_ = nullptr;
        }
        if (pendingDockWindow_ == window) {
            pendingDockWindow_ = nullptr;
        }
        if (window->doc->lastActive == window) {
            window->doc->lastActive = nullptr;
        }
    }
    for (DocWindow* window : windows) {
        const int tab = tabOf(window);
        if (tab >= 0) {
            tabs_->removeTab(tab);
        }
        untileWindow(window);
        if (activeWindow_ == window) {
            activeWindow_ = nullptr;
        }
    }
    if (!activeWindow_ && !windows_.isEmpty()) {
        DocWindow* current = windowAtTab(tabs_->currentIndex());
        activeWindow_ = current ? current : windows_.first();
    }
    if (hooks_.fullScreen && hooks_.fullScreen()) {
        syncScreenModeWindows();
    }
    emit windowsChanged();
}

void DocumentRegistry::destroyWindowWidgets(DocWindow* window)
{
    if (window->canvas) {
        window->canvas->clearTransformPreview();
        if (hooks_.forgetCanvas) {
            hooks_.forgetCanvas(window->canvas);
        }
    }
    if (window->page && window->page != window->host) {
        delete window->page;
    } else if (window->host) {
        delete window->host;
    } else if (window->canvas) {
        delete window->canvas;
    }
    if (window->floating) {
        window->floating->hide();
        window->floating->deleteLater();
        window->floating = nullptr;
    }
}

void DocumentRegistry::presentWindows()
{
    for (DocWindow* window : windows_) {
        PictureView* view = window->doc->view;
        ImageView* canvas = window->canvas;
        if (!view || !canvas) {
            continue;
        }
        if (!view->has_document()) {
            if (activeWindow_ && window->doc == activeWindow_->doc) {
                canvas->setImage(view->image());
            }
            continue;
        }
        const quint64 frame = quint64(pictura::frame_revision(*view));
        if (canvas->property(kFrameRevisionKey) == QVariant(frame)) {
            continue;
        }
        canvas->setProperty(kFrameRevisionKey, frame);
        canvas->replaceDocument(QSize(view->document_width(), view->document_height()));
        if (window != activeWindow_) {
            updateTabTitle(indexOfDoc(window->doc));
        }
    }
}

void DocumentRegistry::syncScreenModeWindows()
{
    if (!documentArea_) {
        return;
    }
    const bool full = hooks_.fullScreen && hooks_.fullScreen();
    DocWindow* shown = full && !stripOwnsActivation() && activeWindow_->floating
                           ? activeWindow_
                           : nullptr;
    for (DocWindow* window : windows_) {
        if (DocumentTilePane* pane = window->tile) {
            pane->setVisible(!full || window == activeWindow_);
            pane->setHeaderVisible(!full);
        }
        DocumentFloatWindow* floating = window->floating;
        if (!floating || !window->page) {
            continue;
        }
        const bool borrowed = window->page->parentWidget() == documentArea_;
        if (window == shown) {
            if (!borrowed) {
                documentArea_->addWidget(window->page);
            }
            documentArea_->setCurrentWidget(window->page);
        } else if (borrowed) {
            documentArea_->removeWidget(window->page);
            floating->setPage(window->page);
        }
        floating->setVisible(!full);
    }
    if (!shown) {
        documentArea_->setCurrentWidget(tileHost_ ? static_cast<QWidget*>(tileHost_) : tabs_);
    }
}

void DocumentRegistry::forEachCanvas(const DocEntry& doc,
                                     const std::function<void(ImageView*)>& fn) const
{
    for (const DocWindow* window : doc.windows) {
        if (window->canvas) {
            fn(window->canvas);
        }
    }
}

QList<ImageView*> DocumentRegistry::activeDocumentCanvases() const
{
    QList<ImageView*> canvases;
    if (!activeWindow_ || !activeWindow_->canvas) {
        return canvases;
    }
    canvases.append(activeWindow_->canvas);
    forEachCanvas(*activeWindow_->doc, [&canvases](ImageView* canvas) {
        if (!canvases.contains(canvas)) {
            canvases.append(canvas);
        }
    });
    return canvases;
}

DocumentFloatWindow* DocumentRegistry::floatWindowForTest(int index) const
{
    if (index < 0 || index >= docs_.size()) {
        return nullptr;
    }
    for (const DocWindow* window : docs_.at(index)->windows) {
        if (window->floating) {
            return window->floating;
        }
    }
    return nullptr;
}

DocumentTilePane* DocumentRegistry::tilePaneForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return nullptr;
    }
    return docs_.at(index)->windows.first()->tile;
}

bool DocumentRegistry::floatDocumentForTest(int index)
{
    return index >= 0 && index < docs_.size() && !docs_.at(index)->windows.isEmpty()
           && floatWindow(docs_.at(index)->windows.first());
}

bool DocumentRegistry::dockDocumentForTest(int index, int tab)
{
    return index >= 0 && index < docs_.size() && !docs_.at(index)->windows.isEmpty()
           && dockWindow(docs_.at(index)->windows.first(), tab);
}

int DocumentRegistry::tabOfDocumentForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return -1;
    }
    return tabOf(docs_.at(index)->windows.first());
}

int DocumentRegistry::windowCountForTest(int index) const
{
    return index >= 0 && index < docs_.size() ? int(docs_.at(index)->windows.size()) : 0;
}

ImageView* DocumentRegistry::windowCanvasForTest(int index, int ordinal) const
{
    const DocWindow* window = documentWindowAt(docs_, index, ordinal);
    return window ? window->canvas : nullptr;
}

QString DocumentRegistry::windowLabelForTest(int index, int ordinal) const
{
    return fullLabel(documentWindowAt(docs_, index, ordinal));
}

int DocumentRegistry::windowTabForTest(int index, int ordinal) const
{
    return tabOf(documentWindowAt(docs_, index, ordinal));
}

DocumentFloatWindow* DocumentRegistry::windowFloatForTest(int index, int ordinal) const
{
    const DocWindow* window = documentWindowAt(docs_, index, ordinal);
    return window ? window->floating : nullptr;
}

void DocumentRegistry::activateWindowForTest(int index, int ordinal)
{
    activateDocWindow(documentWindowAt(docs_, index, ordinal));
}

QString DocumentRegistry::documentWindowLabelForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return QString();
    }
    return fullLabel(docs_.at(index)->windows.first());
}

} // namespace pictura
