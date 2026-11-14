// The open documents, their windows, the active window, and the loading
// entries. The frame keeps chrome, canvas/view wiring and dialogs behind the
// DocumentHooks seam.

#include "document_registry.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "canvas_scrollbars.h"
#include "document_float_window.h"
#include "document_tile_host.h"
#include "fonts.h"
#include "image_view.h"
#include "loading_card.h"
#include "tab_spinner.h"

#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtGui/QFontMetrics>
#include <QtGui/QKeySequence>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtGui/QShortcut>
#include <QtWidgets/QStackedLayout>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <utility>

namespace pictura {

DocumentRegistry::DocumentRegistry(Hooks hooks, QWidget* shell, QObject* parent)
    : QObject(parent)
    , hooks_(std::move(hooks))
{
    tabs_ = new QTabWidget(shell);
    tabs_->setObjectName(QStringLiteral("documentTabs"));
    tabs_->tabBar()->setObjectName(QStringLiteral("documentTabBar"));
    tabs_->setTabsClosable(false);
    tabs_->setMovable(true);
    tabs_->tabBar()->setElideMode(Qt::ElideRight);
    tabs_->tabBar()->installEventFilter(this);
    connect(tabs_->tabBar(), &QTabBar::tabMoved, this, [this](int from, int to) {
        if (from >= 0 && from < tabs_->count() && to >= 0 && to < tabs_->count()
            && tabs_->count() <= windows_.size()) {
            windows_.move(from, to);
        }
    });
    tabs_->setDocumentMode(true);
    buildDocumentTabs();

    documentArea_ = new QStackedWidget(shell);
    documentArea_->setObjectName(QStringLiteral("documentArea"));
    documentArea_->addWidget(tabs_);

    // The tab/header menus are MenuRegistrar's presentation; this registry only
    // reports which window the user asked about.
    tabs_->tabBar()->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(tabs_->tabBar(), &QWidget::customContextMenuRequested, this,
            [this](const QPoint& pos) {
                if (DocWindow* window = windowAtTab(tabs_->tabBar()->tabAt(pos))) {
                    emit tabContextMenuRequested(window, pos);
                }
            });
    connect(tabs_, &QTabWidget::currentChanged, this,
            [this](int tab) { onTabCurrentChanged(tab); });
    connect(tabs_->tabBar(), &QTabBar::tabBarClicked, this,
            [this](int tab) { onTabBarClicked(tab); });
    wireDocumentCycling();
}

DocumentRegistry::~DocumentRegistry()
{
    // Stop the tab signals while the child widgets are torn down.
    if (tabs_) {
        QObject::disconnect(tabs_, nullptr, this, nullptr);
        QObject::disconnect(tabs_->tabBar(), nullptr, this, nullptr);
    }
    activeWindow_ = nullptr;
    windows_.clear();
    for (DocEntry* doc : docs_) {
        qDeleteAll(doc->windows);
    }
    qDeleteAll(docs_);
    docs_.clear();
}

QTabBar* DocumentRegistry::tabBar() const
{
    return tabs_ ? tabs_->tabBar() : nullptr;
}

void DocumentRegistry::buildDocumentTabs()
{
    tabs_->tabBar()->setDrawBase(false);
    applyTabBarFont(tabs_->tabBar());
}

int DocumentRegistry::activeIndex() const
{
    return activeWindow_ ? indexOfDoc(activeWindow_->doc) : -1;
}

void DocumentRegistry::setActiveIndex(int index)
{
    if (index < 0 || index >= docs_.size()) {
        return;
    }
    const DocEntry* doc = docs_.at(index);
    if (activeWindow_ && activeWindow_->doc == doc) {
        activateDocWindow(activeWindow_);
    } else if (doc->lastActive) {
        activateDocWindow(doc->lastActive);
    } else if (!doc->windows.isEmpty()) {
        activateDocWindow(doc->windows.first());
    }
}

DocEntry* DocumentRegistry::docAt(int index) const
{
    return index >= 0 && index < docs_.size() ? docs_.at(index) : nullptr;
}

void DocumentRegistry::setTabTrackingSuppressed(bool suppressed)
{
    suppressTabTracking_ = suppressed;
}

PictureView* DocumentRegistry::viewAt(int index) const
{
    return docAt(index) ? docAt(index)->view : nullptr;
}

PictureView* DocumentRegistry::activeView() const
{
    return viewAt(activeIndex());
}

ImageView* DocumentRegistry::canvasAt(int index) const
{
    const DocEntry* doc = docAt(index);
    if (!doc) {
        return nullptr;
    }
    if (activeWindow_ && activeWindow_->doc == doc) {
        return activeWindow_->canvas;
    }
    if (doc->lastActive) {
        return doc->lastActive->canvas;
    }
    return doc->windows.isEmpty() ? nullptr : doc->windows.first()->canvas;
}

ImageView* DocumentRegistry::imageView() const
{
    return canvasAt(activeIndex());
}

QString DocumentRegistry::documentPath(int index) const
{
    return docAt(index) ? docAt(index)->path : QString();
}

QString DocumentRegistry::documentName(int index) const
{
    return docAt(index) ? documentDisplayName(*docAt(index)) : QStringLiteral("Untitled");
}

bool DocumentRegistry::isDocumentDirty(int index) const
{
    PictureView* view = viewAt(index);
    return view && document_api::isDirty(*view);
}

QString DocumentRegistry::activeFilePath() const
{
    return documentPath(activeIndex());
}

QString DocumentRegistry::activeDocumentName() const
{
    return documentName(activeIndex());
}

bool DocumentRegistry::isActiveDirty() const
{
    return isDocumentDirty(activeIndex());
}

bool DocumentRegistry::isLoading(int index) const
{
    return docAt(index) && docAt(index)->loading;
}

void DocumentRegistry::setDocumentName(int index, const QString& name)
{
    if (DocEntry* doc = docAt(index)) {
        doc->displayName = name;
        updateTabTitle(index);
    }
}

void DocumentRegistry::setDocumentPath(int index, const QString& path)
{
    if (DocEntry* doc = docAt(index)) {
        doc->path = path;
    }
}

int DocumentRegistry::indexOfView(const PictureView* view) const
{
    if (!view) {
        return -1;
    }
    for (int i = 0; i < docs_.size(); ++i) {
        if (docs_.at(i)->view == view) {
            return i;
        }
    }
    return -1;
}

int DocumentRegistry::indexOfDoc(const DocEntry* doc) const
{
    return doc ? int(docs_.indexOf(const_cast<DocEntry*>(doc))) : -1;
}

DocWindow* DocumentRegistry::windowAtTab(int tab) const
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

int DocumentRegistry::tabOf(const DocWindow* window) const
{
    if (!tabs_ || !window || window->kind != DocWindow::Host::Tab || !window->page) {
        return -1;
    }
    return tabs_->indexOf(window->page);
}

DocWindow* DocumentRegistry::windowOfCanvas(const ImageView* canvas) const
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

int DocumentRegistry::addDocument(PictureView* view, const QString& path, bool asTab)
{
    if (!view) {
        return -1;
    }
    hooks_.applyViewSettings(view);
    auto* entry = new DocEntry;
    entry->view = view;
    if (path.isEmpty()) {
        entry->untitledNumber = ++untitledCounter_;
    } else {
        entry->path = path;
    }
    auto* window = new DocWindow;
    window->doc = entry;
    entry->windows.append(window);
    if (hooks_.setupCanvas) {
        hooks_.setupCanvas(*window);
    }
    window->page = window->host;
    if (hooks_.wireDocument) {
        hooks_.wireDocument(*entry);
    }
    if (hooks_.wireCanvas) {
        hooks_.wireCanvas(*window);
    }

    docs_.append(entry);
    const int index = docs_.size() - 1;
    emit documentsChanged();
    addTabWindow(window);
    installCloseButton(window);
    activateDocWindow(window);
    if (!asTab || tileHost_) {
        hostReadyWindow(window, true);
    }
    if (!path.isEmpty() && hooks_.rememberRecent) {
        hooks_.rememberRecent(path);
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
    return index;
}

int DocumentRegistry::addLoadingDocument(PictureView* view, const QString& path,
                                         const QString& displayName, quint64 token,
                                         const QString& recentPath, OpenKind kind)
{
    if (!view) {
        return -1;
    }
    hooks_.applyViewSettings(view);
    if (hooks_.wireOpenSignals) {
        hooks_.wireOpenSignals(view);
    }
    if (hooks_.openDelayForTest) {
        open_set_delay(*view, hooks_.openDelayForTest());
    }

    auto* entry = new DocEntry;
    entry->view = view;
    entry->path = path;
    entry->displayName = displayName;
    entry->recentPath = recentPath;
    entry->token = token;
    entry->kind = kind;
    entry->loading = true;
    entry->previousActiveView = activeView();
    auto* window = new DocWindow;
    window->doc = entry;
    entry->windows.append(window);
    window->page = new QWidget(hooks_.shellWidget());
    window->page->setObjectName(QStringLiteral("documentLoadingPage"));
    window->page->setAutoFillBackground(true);
    QPalette palette = window->page->palette();
    palette.setColor(QPalette::Window,
                     hooks_.loadingPageColor ? hooks_.loadingPageColor() : QColor());
    window->page->setPalette(palette);
    auto* stack = new QStackedLayout(window->page);
    auto* cardPage = new QWidget(window->page);
    auto* cardLayout = new QVBoxLayout(cardPage);
    cardLayout->setContentsMargins(0, 0, 0, 0);
    cardLayout->setSpacing(0);
    cardLayout->addStretch();
    const QString name = displayName.isEmpty() ? QFileInfo(path).fileName() : displayName;
    window->card = new LoadingCard(name, cardPage);
    cardLayout->addWidget(window->card, 0, Qt::AlignHCenter);
    cardLayout->addStretch();
    stack->addWidget(cardPage);
    connect(window->card, &LoadingCard::cancelRequested, this,
            [this, guard = QPointer<PictureView>(view)] {
                QTimer::singleShot(0, this, [this, guard] {
                    if (guard) {
                        emit cancelOpenRequested(guard.data());
                    }
                });
            });

    docs_.append(entry);
    const int index = docs_.size() - 1;
    emit documentsChanged();
    const int tab = addTabWindow(window);
    if (tabs_->count() == 1 && !tileHost_) {
        activateDocWindow(window);
    }
    auto* spinner = new TabSpinner(tabs_->tabBar());
    spinner->setOpeningName(displayName.isEmpty() ? QFileInfo(path).fileName() : displayName);
    tabs_->tabBar()->setTabButton(tab, QTabBar::RightSide, spinner);
    connect(spinner, &TabSpinner::cancelRequested, this,
            [this, guard = QPointer<PictureView>(view)] {
                QTimer::singleShot(0, this, [this, guard] {
                    if (guard) {
                        emit cancelOpenRequested(guard.data());
                    }
                });
            });
    updateTabTitle(index);
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (hooks_.stopPanelRefreshTimer) {
        hooks_.stopPanelRefreshTimer();
    }
    if (hooks_.requestPanelRefresh) {
        hooks_.requestPanelRefresh();
    }
    return index;
}

void DocumentRegistry::installCloseButton(DocWindow* window)
{
    const int tab = tabOf(window);
    if (tab < 0) {
        return;
    }
    auto* close = new TabCloseButton(tabs_->tabBar());
    tabs_->tabBar()->setTabButton(tab, QTabBar::RightSide, close);
    connect(close, &TabCloseButton::closeRequested, this,
            [this, window, guard = QPointer<PictureView>(window->doc->view)] {
                QTimer::singleShot(0, this, [this, window, guard] {
                    if (guard && windows_.contains(window) && window->doc->view == guard) {
                        requestCloseWindow(window);
                    }
                });
            });
}

void DocumentRegistry::finishLoadingDocument(int index)
{
    DocEntry* doc = docAt(index);
    if (!doc || !doc->loading) {
        return;
    }
    DocEntry& entry = *doc;
    entry.loading = false;
    DocWindow* window = entry.windows.first();
    if (window->card) {
        window->card->hide();
        window->card->deleteLater();
        window->card = nullptr;
    }
    if (hooks_.setupCanvas) {
        hooks_.setupCanvas(*window);
    }
    if (window->page && window->page != window->host) {
        if (auto* stack = qobject_cast<QStackedLayout*>(window->page->layout())) {
            stack->addWidget(window->host);
            stack->setCurrentWidget(window->host);
        } else {
            window->host->setParent(window->page);
            window->host->show();
        }
    }
    if (hooks_.wireDocument) {
        hooks_.wireDocument(entry);
    }
    if (hooks_.wireCanvas) {
        hooks_.wireCanvas(*window);
    }
    installCloseButton(window);
    updateTabTitle(index);
    if (!entry.userMovedAway) {
        suppressTabTracking_ = true;
        activateDocWindow(window);
        suppressTabTracking_ = false;
    }
    hostReadyWindow(window, false);
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (hooks_.stopPanelRefreshTimer) {
        hooks_.stopPanelRefreshTimer();
    }
    if (hooks_.requestPanelRefresh) {
        hooks_.requestPanelRefresh();
    }
}

void DocumentRegistry::removeDocument(int index)
{
    DocEntry* entry = docAt(index);
    if (!entry) {
        return;
    }
    // Owners connected to this signal (frame dialogs, the coordinator's open
    // maps/recovery/sessions, the panel host) clean up before destruction.
    emit aboutToRemoveDocument(index);

    entry = docs_.takeAt(index);
    // Replace Color and smart-object sessions are frame/coordinator concerns;
    // they clean up in the signal above if they still hold this view.
    if (entry->view && hooks_.forgetToolView) {
        hooks_.forgetToolView(entry->view);
    }
    const QList<DocWindow*> windows = entry->windows;
    detachWindows(windows);
    if (hooks_.retargetPanels) {
        hooks_.retargetPanels();
    }
    if (entry->view) {
        entry->view->cancel_transform();
    }
    for (DocWindow* window : windows) {
        destroyWindowWidgets(window);
    }
    qDeleteAll(entry->windows);
    delete entry->view;
    delete entry;
    emit documentsChanged();
    if (hooks_.requestRefresh) {
        hooks_.requestRefresh();
    }
    if (hooks_.stopPanelRefreshTimer) {
        hooks_.stopPanelRefreshTimer();
    }
    if (hooks_.requestPanelRefresh) {
        hooks_.requestPanelRefresh();
    }
}

void DocumentRegistry::updateTabTitle(int index)
{
    const DocEntry* doc = docAt(index);
    if (!tabs_ || !doc) {
        return;
    }
    const QFontMetrics metrics(tabs_->tabBar()->font());
    const bool dirty = !doc->loading && isDocumentDirty(index);
    for (const DocWindow* window : doc->windows) {
        const QString label = fullLabel(window);
        if (window->floating) {
            window->floating->setLabel(label, dirty);
        }
        if (window->tile) {
            window->tile->setLabel(label, dirty);
        }
        const int tab = tabOf(window);
        if (tab >= 0) {
            tabs_->setTabText(tab, documentTabText(label, dirty, metrics));
            tabs_->setTabToolTip(tab, label);
        }
    }
    emit documentStateChanged(index);
}

void DocumentRegistry::updateAllTabTitles()
{
    for (int i = 0; i < docs_.size(); ++i) {
        updateTabTitle(i);
    }
}

} // namespace pictura
#include "moc_document_registry.cpp"
