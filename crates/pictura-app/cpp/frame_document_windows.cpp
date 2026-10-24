// The document window registry: which window shows which document, where it
// is hosted, and which one is active; opening another window of a document,
// moving windows between the tab strip, floating windows, and tile panes,
// closing them, presenting them, and the screen modes.

#include "frame_includes.h"

#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"

#include <QtWidgets/QApplication>
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

void PicturaMainWindow::hostReadyWindow(DocWindow* window, bool activate)
{
    // Tile is a layout the user chose: while it lasts, a new window joins it
    // whatever Open Documents As Tabs says.
    if (tileHost_) {
        tileWindow(window, activate);
        return;
    }
    if (!openDocumentsAsTabs_ && window->kind == DocWindow::Host::Tab) {
        floatWindow(window, QRect(), activate);
    }
}

bool PicturaMainWindow::stripOwnsActivation() const
{
    return !activeWindow_ || !windows_.contains(activeWindow_)
           || activeWindow_->kind == DocWindow::Host::Tab;
}

void PicturaMainWindow::activateDocWindow(DocWindow* window)
{
    if (!window || !windows_.contains(window)) {
        return;
    }
    if (window->kind == DocWindow::Host::Tab) {
        const int tab = tabOf(window);
        if (tab < 0) {
            return;
        }
        // A tab chosen while a top-level float has the keyboard brings the
        // frame forward, so the frame's keys and shortcuts follow it.
        if (qobject_cast<DocumentFloatWindow*>(QApplication::activeWindow())) {
            activateWindow();
        }
        if (tabs_->currentIndex() != tab) {
            const bool stripFollows = stripOwnsActivation();
            tabs_->setCurrentIndex(tab);
            if (stripFollows) {
                return;
            }
        }
    } else if (window->kind == DocWindow::Host::Tile) {
        if (qobject_cast<DocumentFloatWindow*>(QApplication::activeWindow())) {
            activateWindow();
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

void PicturaMainWindow::onTabCurrentChanged(int tab)
{
    // A host move settles the activation itself, and while another host holds
    // the active window the strip's own changes (an insert, a removal) leave
    // it there.
    if (movingHost_ || !stripOwnsActivation()) {
        return;
    }
    DocWindow* window = windowAtTab(tab);
    // The last tab went away: a window of another host takes over.
    if (!window && !windows_.isEmpty()) {
        window = windows_.first();
    }
    activeWindow_ = window;
    onActiveWindowChanged();
}

void PicturaMainWindow::onTabBarClicked(int tab)
{
    // A click on a tab, even the current one, takes the activation back from
    // another host; while the strip owns it, currentChanged follows the click.
    DocWindow* window = windowAtTab(tab);
    if (window && !stripOwnsActivation()) {
        activateDocWindow(window);
    }
}

void PicturaMainWindow::wireDocumentCycling()
{
    // Window-context frame shortcuts, not catalogue commands: CS6 has no menu
    // item for them, and unlike application shortcuts they stay quiet while a
    // dialog is the active window. The shortcut map matches before the tab
    // widget's own Ctrl+Tab handling sees the key.
    auto* next = new QShortcut(QKeySequence(Qt::CTRL | Qt::Key_Tab), this);
    connect(next, &QShortcut::activated, this, [this]() { cycleDocumentWindow(1); });
    // X11's Shift+Ctrl+Backtab falls back to this sequence in the shortcut map.
    auto* previous = new QShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_Tab), this);
    connect(previous, &QShortcut::activated, this, [this]() { cycleDocumentWindow(-1); });
}

void PicturaMainWindow::cycleDocumentWindow(int step)
{
    const int count = int(windows_.size());
    const int current = int(windows_.indexOf(activeWindow_));
    if (count < 2 || current < 0) {
        return;
    }
    activateDocWindow(windows_.at(((current + step) % count + count) % count));
}

bool PicturaMainWindow::addDocumentWindow(DocEntry* doc)
{
    if (!doc || !docs_.contains(doc) || doc->loading || !doc->view) {
        return false;
    }
    auto* window = new DocWindow;
    window->doc = doc;
    // The new window takes the next ordinal, so it is labelled `:N`.
    doc->windows.append(window);
    setupCanvas(*window);
    window->page = window->host;
    wireCanvas(*window);
    addTabWindow(window);
    installCloseButton(window);
    activateDocWindow(window);
    hostReadyWindow(window, true);
    updateTabTitle(indexOfDoc(doc));
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
    return true;
}

void PicturaMainWindow::onActiveWindowChanged()
{
    if (activeWindow_) {
        activeWindow_->doc->lastActive = activeWindow_;
    }
    if (screenMode_ != ScreenMode::Standard) {
        syncScreenModeWindows();
    }
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
    for (const DocEntry* doc : docs_) {
        PictureView* view = doc->view;
        if (view && view != active && view->transform_session_active()) {
            view->cancel_transform();
            forEachCanvas(*doc, [](ImageView* canvas) { canvas->clearTransformPreview(); });
        }
    }
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
}

namespace {

// A float placed by a command takes this share of the document area.
constexpr double kFloatAreaFraction = 0.6;

} // namespace

QRect PicturaMainWindow::documentAreaGlobalRect() const
{
    return documentArea_ ? QRect(documentArea_->mapToGlobal(QPoint(0, 0)), documentArea_->size())
                         : QRect();
}

QRect PicturaMainWindow::clampFloatGeometry(const QRect& geometry) const
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

QRect PicturaMainWindow::defaultFloatGeometry() const
{
    const QRect area = documentAreaGlobalRect();
    QRect geometry(QPoint(), (QSizeF(area.size()) * kFloatAreaFraction).toSize());
    geometry.moveCenter(area.center());
    return geometry;
}

void PicturaMainWindow::raiseDocumentFloat(DocumentFloatWindow* floating)
{
    floating->raise();
    if (floating->isWindow()) {
        return;
    }
    // A child-hosted float stays below the floating panel overlays.
    for (PanelFloat* panel : findChildren<PanelFloat*>(Qt::FindDirectChildrenOnly)) {
        if (!panel->isWindow()) {
            panel->raise();
        }
    }
}

bool PicturaMainWindow::floatWindow(DocWindow* window, const QRect& globalGeometry,
                                    bool activate)
{
    if (!window || !windows_.contains(window) || window->doc->loading
        || window->kind == DocWindow::Host::Float || !window->page) {
        return false;
    }
    const bool wasActive = activeWindow_ == window;
    auto* floating = new DocumentFloatWindow(this);
    movingHost_ = true;
    const int tab = tabOf(window);
    if (tab >= 0) {
        tabs_->removeTab(tab);
    }
    movingHost_ = false;
    untileWindow(window);
    // Floats follow the tabs in the cycle order, in the order they floated.
    windows_.removeOne(window);
    windows_.append(window);
    window->kind = DocWindow::Host::Float;
    window->floating = floating;
    // The page moves with its canvas host, so the view keeps its zoom and
    // scroll position.
    floating->setPage(window->page);
    floating->onActivated = [this, window, floating]() {
        if (windows_.contains(window) && window->floating == floating) {
            activateDocWindow(window);
        }
    };
    floating->onCloseRequested = [this, window, floating]() {
        QTimer::singleShot(0, this, [this, window, guard = QPointer<DocumentFloatWindow>(floating)] {
            if (guard && windows_.contains(window) && window->floating == guard) {
                closeWindow(window, true);
            }
        });
    };
    floating->onContextMenu = [this, window, floating](const QPoint& globalPos) {
        if (windows_.contains(window) && window->floating == floating) {
            showHeaderMenu(window, globalPos);
        }
    };
    floating->onDragMoved = [this, window, floating](const QPoint& globalPos) {
        if (windows_.contains(window) && window->floating == floating) {
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
    // A double-click is an explicit dock, like Consolidate, so it ignores the
    // docking preference.
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
        // A float that opens behind the active one stays under it.
        raiseDocumentFloat(floating);
        if (!stripOwnsActivation() && activeWindow_->floating) {
            raiseDocumentFloat(activeWindow_->floating);
        }
    }
    if (screenMode_ != ScreenMode::Standard) {
        syncScreenModeWindows();
    }
    if (wasActive) {
        refresh();
    }
    return true;
}

bool PicturaMainWindow::dockWindow(DocWindow* window, int index)
{
    if (!window || !windows_.contains(window) || window->kind == DocWindow::Host::Tab
        || window->doc->loading || !window->page) {
        return false;
    }
    DocumentFloatWindow* floating = window->floating;
    const int count = tabs_->count();
    const int at = index < 0 || index > count ? count : index;
    // The tab windows lead `windows_` in their visual order.
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
        // The float may be handling the header release that docked it.
        floating->hide();
        floating->deleteLater();
    }
    activateDocWindow(window);
    updateTabTitle(indexOfDoc(window->doc));
    if (screenMode_ != ScreenMode::Standard) {
        syncScreenModeWindows();
    }
    refresh();
    return true;
}

bool PicturaMainWindow::tileWindow(DocWindow* window, bool activate)
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
        // A full-screen mode may have lent the float's page to the area.
        if (window->page->parentWidget() == documentArea_) {
            documentArea_->removeWidget(window->page);
        }
        floating->hide();
        floating->deleteLater();
        window->floating = nullptr;
    }
    // Panes follow the tabs in the cycle order, in the order they were tiled,
    // which is also their order in the grid.
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
                closeWindow(window, true);
            }
        });
    };
    pane->onContextMenu = [this, window, pane](const QPoint& globalPos) {
        if (windows_.contains(window) && window->tile == pane) {
            showHeaderMenu(window, globalPos);
        }
    };
    if (screenMode_ == ScreenMode::Standard) {
        documentArea_->setCurrentWidget(tileHost_);
    }
    updateTabTitle(indexOfDoc(window->doc));
    if (activate || wasActive) {
        activateDocWindow(window);
    }
    if (screenMode_ != ScreenMode::Standard) {
        syncScreenModeWindows();
    }
    refresh();
    return true;
}

void PicturaMainWindow::untileWindow(DocWindow* window)
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
}

QRect PicturaMainWindow::dockZoneGlobalRect() const
{
    if (!floatingDocumentDocking_ || tileHost_ || !tabs_->isVisible()) {
        return QRect();
    }
    // Without tabs the bar takes no room, so the zone is the band a bar would
    // take at the top of the document area, which the tab widget fills.
    QTabBar* bar = tabs_->tabBar();
    const int barHeight =
        bar->isVisible() && bar->count() > 0 ? bar->height() : bar->sizeHint().height();
    return QRect(tabs_->mapToGlobal(QPoint(0, 0)),
                 QSize(tabs_->width(), qMax(barHeight, DocumentFloatWindow::kHeaderHeight)));
}

int PicturaMainWindow::dockIndexAt(const QPoint& globalPos) const
{
    QTabBar* bar = tabs_->tabBar();
    return bar->isVisible() ? bar->tabAt(bar->mapFromGlobal(globalPos)) : -1;
}

void PicturaMainWindow::onFloatDragReleased(DocWindow* window, const QPoint& globalPos)
{
    hideDockIndicator();
    if (!windows_.contains(window) || window->kind != DocWindow::Host::Float
        || !dockZoneGlobalRect().contains(globalPos)) {
        return;
    }
    dockWindow(window, dockIndexAt(globalPos));
}

bool PicturaMainWindow::closeWindow(DocWindow* window, bool interactive)
{
    if (!window || !windows_.contains(window)) {
        return false;
    }
    DocEntry* doc = window->doc;
    const int index = indexOfDoc(doc);
    if (doc->windows.size() < 2) {
        return closeDocument(index, interactive);
    }
    // Another window keeps the document, its history, and its modified state.
    detachWindows(QList<DocWindow*>{window});
    doc->windows.removeOne(window);
    destroyWindowWidgets(window);
    delete window;
    updateTabTitle(index);
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
    return true;
}

void PicturaMainWindow::detachWindows(const QList<DocWindow*>& windows)
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
        // Removing the active tab hands the activation to the strip's new
        // current tab through currentChanged.
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
    if (screenMode_ != ScreenMode::Standard) {
        syncScreenModeWindows();
    }
}

void PicturaMainWindow::destroyWindowWidgets(DocWindow* window)
{
    if (window->canvas) {
        window->canvas->clearTransformPreview();
        // A closed active float or pane hands the activation over without a
        // refresh, so the tools may still hold this canvas.
        if (tools_ && tools_->canvas() == window->canvas) {
            tools_->bindCanvas(nullptr);
        }
    }
    // A ready window's page is its canvas host; a loading window's page is the
    // placeholder. Deleting the page deletes the host (and the canvas) it owns.
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

void PicturaMainWindow::presentWindows()
{
    // A document canvas holds no image: it crops the view pyramid, so a
    // rebuilt frame only invalidates its crops. A `changed` that rebuilt no
    // frame (a bounded undo, a rename, a slice) leaves it alone. Every window
    // is presented, so a float whose document changes while another window is
    // active (a background filter commit) does not go stale.
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

void PicturaMainWindow::syncScreenModeWindows()
{
    if (!documentArea_) {
        return;
    }
    const bool full = screenMode_ != ScreenMode::Standard;
    DocWindow* shown = full && !stripOwnsActivation() && activeWindow_->floating
                           ? activeWindow_
                           : nullptr;
    for (DocWindow* window : windows_) {
        if (DocumentTilePane* pane = window->tile) {
            // The grid gives the one pane left visible the whole host.
            pane->setVisible(!full || window == activeWindow_);
            pane->setHeaderVisible(!full);
        }
        DocumentFloatWindow* floating = window->floating;
        if (!floating || !window->page) {
            continue;
        }
        // An active float's page fills the document area in place of the tab
        // widget; its header stays behind with the hidden float.
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

DocumentFloatWindow* PicturaMainWindow::floatWindowForTest(int index) const
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

DocumentTilePane* PicturaMainWindow::tilePaneForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return nullptr;
    }
    return docs_.at(index)->windows.first()->tile;
}

bool PicturaMainWindow::floatDocumentForTest(int index)
{
    return index >= 0 && index < docs_.size() && !docs_.at(index)->windows.isEmpty()
           && floatWindow(docs_.at(index)->windows.first());
}

bool PicturaMainWindow::dockDocumentForTest(int index, int tab)
{
    return index >= 0 && index < docs_.size() && !docs_.at(index)->windows.isEmpty()
           && dockWindow(docs_.at(index)->windows.first(), tab);
}

int PicturaMainWindow::tabOfDocumentForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return -1;
    }
    return tabOf(docs_.at(index)->windows.first());
}

namespace {

// Window `ordinal` of document `index`, or null.
DocWindow* documentWindowAt(const QList<DocEntry*>& docs, int index, int ordinal)
{
    if (index < 0 || index >= docs.size() || ordinal < 0
        || ordinal >= docs.at(index)->windows.size()) {
        return nullptr;
    }
    return docs.at(index)->windows.at(ordinal);
}

} // namespace

int PicturaMainWindow::windowCountForTest(int index) const
{
    return index >= 0 && index < docs_.size() ? int(docs_.at(index)->windows.size()) : 0;
}

ImageView* PicturaMainWindow::windowCanvasForTest(int index, int ordinal) const
{
    const DocWindow* window = documentWindowAt(docs_, index, ordinal);
    return window ? window->canvas : nullptr;
}

QString PicturaMainWindow::windowLabelForTest(int index, int ordinal) const
{
    return fullLabel(documentWindowAt(docs_, index, ordinal));
}

int PicturaMainWindow::windowTabForTest(int index, int ordinal) const
{
    return tabOf(documentWindowAt(docs_, index, ordinal));
}

DocumentFloatWindow* PicturaMainWindow::windowFloatForTest(int index, int ordinal) const
{
    const DocWindow* window = documentWindowAt(docs_, index, ordinal);
    return window ? window->floating : nullptr;
}

void PicturaMainWindow::activateWindowForTest(int index, int ordinal)
{
    activateDocWindow(documentWindowAt(docs_, index, ordinal));
}

QString PicturaMainWindow::documentWindowLabelForTest(int index) const
{
    if (index < 0 || index >= docs_.size() || docs_.at(index)->windows.isEmpty()) {
        return QString();
    }
    return fullLabel(docs_.at(index)->windows.first());
}

} // namespace pictura
