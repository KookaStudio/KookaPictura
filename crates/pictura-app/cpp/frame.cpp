#include "frame_includes.h"

#include "api/document_api.h"
#include "api/paint_api.h"
#include "api/selection_api.h"
#include "file_drop_router.h"
#include "frame_canvas.h"
#include "panels/numeric_field.h"
#include "pictura_debug_timing.h"
#include "screen_mode_message.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"

#include <QtCore/QHash>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QTabBar>

namespace pictura {

namespace {

// The canvas property recording the view `frame_revision` its image came from.
constexpr const char* kFrameKey = "picturaFrameRevision";

// The document pane keeps a minimum width even with no document open, so the
// widget columns can never absorb the whole workspace (and the splitter keeps a
// grabbable handle on each side of it). ponytail: chosen, not a sourced CS6
// metric.
constexpr int kWorkspaceMinWidth = 160;

// The tab title's mode label for an `image_mode()` key.
QString modeLabel(const QString& mode)
{
    static const QHash<QString, QString> labels = {
        {QStringLiteral("bitmap"), QStringLiteral("Bitmap")},
        {QStringLiteral("grayscale"), QStringLiteral("Grayscale")},
        {QStringLiteral("duotone"), QStringLiteral("Duotone")},
        {QStringLiteral("indexed"), QStringLiteral("Index")},
        {QStringLiteral("rgb"), QStringLiteral("RGB")},
        {QStringLiteral("cmyk"), QStringLiteral("CMYK")},
        {QStringLiteral("lab"), QStringLiteral("Lab")},
        {QStringLiteral("multichannel"), QStringLiteral("Multichannel")},
    };
    return labels.value(mode, mode.toUpper());
}

// Keycap label for a key event, matching the labels in `toolHintEntries`; empty
// for a key that no hint shows.
QString hintKeyLabel(const QKeyEvent* event)
{
    switch (event->key()) {
    case Qt::Key_Shift:
        return QStringLiteral("Shift");
    case Qt::Key_Alt:
        return QStringLiteral("Alt");
    case Qt::Key_Control:
        return QStringLiteral("Ctrl");
    case Qt::Key_BracketLeft:
        return QStringLiteral("[");
    case Qt::Key_BracketRight:
        return QStringLiteral("]");
    case Qt::Key_Left:
    case Qt::Key_Right:
    case Qt::Key_Up:
    case Qt::Key_Down:
        return QStringLiteral("Arrows");
    default:
        break;
    }
    if (event->key() >= Qt::Key_A && event->key() <= Qt::Key_Z) {
        return QString(QChar(event->key()));
    }
    return QString();
}

} // namespace

bool launchCreatesScratchDocument(bool selfTest, bool codecLoaded)
{
    return selfTest && !codecLoaded;
}

PicturaMainWindow::PicturaMainWindow(QWidget* parent)
    : QMainWindow(parent)
{
    tabs_ = new QTabWidget(this);
    tabs_->setObjectName(QStringLiteral("documentTabs"));
    tabs_->tabBar()->setObjectName(QStringLiteral("documentTabBar"));
    tabs_->setTabsClosable(true);
    tabs_->setMovable(true);
    // docs_ is indexed in lockstep with the tab order (activeDocumentIndex,
    // viewAt, removeDocument), so a dragged tab must move its DocEntry too.
    // QTabBar::tabMoved(from, to) mirrors QList::move(from, to), and Qt keeps
    // the current tab current across the move.
    connect(tabs_->tabBar(), &QTabBar::tabMoved, this, [this](int from, int to) {
        if (from >= 0 && from < docs_.size() && to >= 0 && to < docs_.size()) {
            docs_.move(from, to);
        }
    });
    tabs_->setDocumentMode(true);
    buildDocumentTabs();
    // The document area and the panel columns share the central widget through
    // a splitter; the columns are the only host for the panels. M43: the
    // splitter is an ordered set of left columns, the document tabs, then right
    // columns, with the tabs keeping the stretch.
    panelColumn_ = new PanelColumn(this);
    // The splitter itself is the central widget, so drop-target geometry keeps
    // its existing meaning. It reserves the 3 px band on each side; 0-width
    // handles leave no resize seam between panes.
    centerSplitter_ = new QSplitter(Qt::Horizontal, this);
    centerSplitter_->setObjectName(QStringLiteral("centerSplitter"));
    centerSplitter_->setHandleWidth(0);
    centerSplitter_->setContentsMargins(kWorkspaceFrameWidth, 0, kWorkspaceFrameWidth, 0);
    centerSplitter_->addWidget(tabs_);
    centerSplitter_->addWidget(panelColumn_);
    tabs_->setMinimumWidth(kWorkspaceMinWidth);
    centerSplitter_->setStretchFactor(0, 1);
    centerSplitter_->setStretchFactor(1, 0);
    centerSplitter_->setChildrenCollapsible(false);
    setCentralWidget(centerSplitter_);
    setDockOptions(QMainWindow::AnimatedDocks | QMainWindow::AllowTabbedDocks);

    // Double-clicking the empty workspace (no documents open) opens the Open
    // dialog. The blank pane is the tab widget's internal stacked widget.
    installWorkspaceOpenGesture();

    registry_ = new CommandRegistry(this);
    addDefaultCommands(*registry_);

    const SessionState state = pictura::loadSession();
    recent_ = state.recent;
    setFileDialogRecentPaths(recent_);
    gpuCompute_ = state.gpuCompute;
    colorPolicy_ = state.colorPolicy;
    useShiftKeyForToolSwitch_ = state.useShiftKeyForToolSwitch;
    autoSaveRecovery_ = state.autoSaveRecovery;
    autoSaveMinutes_ = state.autoSaveMinutes;
    loadGuideSession(state);
    // Probe the adapter once so the toggle can be offered without a document.
    {
        PictureView probe;
        gpuAvailable_ = probe.gpu_available();
    }
    registerHandlers();
    buildMenus();
    buildPanels();
    buildTools(state.toolsColumns, state.useShiftKeyForToolSwitch);
    if (tools_) {
        tools_->applyCropOptions(state.cropClassicMode, state.cropDeletePixels, state.cropRatio,
                                 state.cropGridOverlay);
    }
    buildStatusBar();
    // The two 1 px inner frame lines span the whole central band (options bar
    // included); the outer 2 px is the window `${panel}` showing through the
    // reserved margin. They are raised above the chrome in layoutWorkspaceFrame.
    workspaceFrameLeft_ = new QWidget(this);
    workspaceFrameRight_ = new QWidget(this);
    for (QWidget* line : {workspaceFrameLeft_, workspaceFrameRight_}) {
        line->setObjectName(QStringLiteral("workspaceFrameLine"));
        line->setAttribute(Qt::WA_StyledBackground, true);
        line->setAttribute(Qt::WA_TransparentForMouseEvents, true);
    }
    // Route OS file drops by target: a document canvas places into the active
    // document, every other target opens a new tab. Enabling drops is what makes
    // each widget a drop target; per-canvas install happens in addDocument.
    fileDropRouter_ = new FileDropRouter(this);
    for (QWidget* target :
         {static_cast<QWidget*>(tabs_), static_cast<QWidget*>(tabs_->tabBar()),
          static_cast<QWidget*>(menuBar()), static_cast<QWidget*>(optionsBar_),
          static_cast<QWidget*>(this)}) {
        target->setAcceptDrops(true);
        target->installEventFilter(fileDropRouter_);
    }
    applyBrightness(state.brightnessLevel);
    applyPanelSession(state);
    applyFreshSessionDefault(state);
    const bool sessionHadLayout = !state.panelColumns.isEmpty() || !state.panelGroups.isEmpty();
    initWorkspaces(sessionHadLayout);
    // Persist every column change through the same path as the Window toggles,
    // and route the tab menu's `Interface Options…` to the Interface pane.
    wirePanelColumn(panelColumn_);
    if (!state.layout.isEmpty()) {
        restoreStoredLayout(state.layout, state.layoutRevision);
    }

    // Expensive panel work (layer thumbnails, histogram) is coalesced off the
    // paint-dab/commit path so the canvas can repaint first.
    panelRefreshTimer_ = new QTimer(this);
    panelRefreshTimer_->setSingleShot(true);
    panelRefreshTimer_->setInterval(120);
    connect(panelRefreshTimer_, &QTimer::timeout, this, &PicturaMainWindow::refreshPanels);

    // A splitter drag has no other save trigger; coalesce the column-width
    // changes into one write instead of saving on every pixel of the drag.
    sessionSaveTimer_ = new QTimer(this);
    sessionSaveTimer_->setSingleShot(true);
    sessionSaveTimer_->setInterval(400);
    connect(sessionSaveTimer_, &QTimer::timeout, this, &PicturaMainWindow::saveSession);
    connect(centerSplitter_, &QSplitter::splitterMoved, this,
            [this](int pos, int index) { onCenterSplitterMoved(pos, index); });

    connect(tabs_, &QTabWidget::currentChanged, this, [this](int) {
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
    });
    connect(tabs_, &QTabWidget::tabCloseRequested, this,
            [this](int index) { closeDocument(index, true); });

    auto* hidePanels = new QShortcut(QKeySequence(Qt::Key_Tab), this);
    connect(hidePanels, &QShortcut::activated, this, [this]() { togglePanelsHidden(); });
    auto* hidePanelsBack = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_Tab), this);
    connect(hidePanelsBack, &QShortcut::activated, this, [this]() { togglePanelsHidden(); });
    // Space is reserved for transient panning, so the canvas-colour cycle is
    // bound to a double `F` instead; a bare Space reaches the widgets.
    auto* cycleCanvas = new QShortcut(QKeySequence(Qt::Key_F, Qt::Key_F), this);
    connect(cycleCanvas, &QShortcut::activated, this, [this]() { cycleCanvasColor(true); });
    auto* brightnessDown = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F1), this);
    connect(brightnessDown, &QShortcut::activated, this,
            [this]() { setBrightnessLevel(brightnessLevel_ - 1); });
    auto* brightnessUp = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F2), this);
    connect(brightnessUp, &QShortcut::activated, this,
            [this]() { setBrightnessLevel(brightnessLevel_ + 1); });

    resize(1100, 700);
    restoreSessionWindowGeometry(state);
    // A normal launch opens no document; refresh() is the only writer of the
    // tab pane's visibility, so hide the empty ghost canvas at startup.
    refresh();
    setWindowTitle(QStringLiteral("Kooka Pictura"));
    // Make the splitter handles inert from the start: the widget columns' own
    // grips are the only width-drag affordance on a workspace-facing edge.
    reapplyColumnStretch();
    layoutWorkspaceFrame();
}

PicturaMainWindow::~PicturaMainWindow()
{
    shutdownRecovery();
    for (const SmartObjectEditSession& session : editSessions_) {
        delete session.temp;
    }
    editSessions_.clear();
}

QStringList PicturaMainWindow::topLevelMenuTitles() const
{
    return registry_->topLevelTitles();
}

ToolId PicturaMainWindow::activeTool() const
{
    return tools_ ? tools_->activeTool() : ToolId::Move;
}

void PicturaMainWindow::setActiveTool(ToolId id)
{
    if (tools_) {
        tools_->setActiveTool(id);
    }
}

bool PicturaMainWindow::hasPendingCrop() const
{
    return tools_ && tools_->hasPendingCrop();
}

bool PicturaMainWindow::commitCrop()
{
    if (!tools_ || !tools_->commitCrop()) {
        return false;
    }
    refresh();
    return true;
}

bool PicturaMainWindow::registerPanel(QWidget* panel, Qt::DockWidgetArea area)
{
    if (!panel || panelNames_.contains(panel->objectName())) {
        return false;
    }
    panelNames_.insert(panel->objectName());
    if (auto* dock = qobject_cast<QDockWidget*>(panel)) {
        addDockWidget(area, dock);
    }
    return true;
}

ImageView* PicturaMainWindow::imageView() const
{
    return canvasAt(activeDocumentIndex());
}

PictureView* PicturaMainWindow::activeView() const
{
    return viewAt(activeDocumentIndex());
}

int PicturaMainWindow::activeDocumentIndex() const
{
    if (!tabs_ || tabs_->count() == 0 || docs_.isEmpty()) {
        return -1;
    }
    return tabs_->currentIndex();
}

void PicturaMainWindow::setActiveDocumentIndex(int index)
{
    if (tabs_ && index >= 0 && index < tabs_->count()) {
        tabs_->setCurrentIndex(index);
    }
}

PictureView* PicturaMainWindow::viewAt(int index) const
{
    if (index < 0 || index >= docs_.size()) {
        return nullptr;
    }
    return docs_.at(index).view;
}

ImageView* PicturaMainWindow::canvasAt(int index) const
{
    if (index < 0 || index >= docs_.size()) {
        return nullptr;
    }
    return docs_.at(index).canvas;
}

QString PicturaMainWindow::documentPath(int index) const
{
    if (index < 0 || index >= docs_.size()) {
        return QString();
    }
    return docs_.at(index).path;
}

QString PicturaMainWindow::documentName(int index) const
{
    if (index < 0 || index >= docs_.size()) {
        return QStringLiteral("Untitled");
    }
    const DocEntry& entry = docs_.at(index);
    if (!entry.displayName.isEmpty()) {
        return entry.displayName;
    }
    if (!entry.path.isEmpty()) {
        return QFileInfo(entry.path).fileName();
    }
    return QStringLiteral("Untitled-%1").arg(entry.untitledNumber);
}

bool PicturaMainWindow::isDocumentDirty(int index) const
{
    PictureView* view = viewAt(index);
    return view && document_api::isDirty(*view);
}

QString PicturaMainWindow::activeFilePath() const
{
    return documentPath(activeDocumentIndex());
}

QString PicturaMainWindow::activeDocumentName() const
{
    return documentName(activeDocumentIndex());
}

bool PicturaMainWindow::isActiveDirty() const
{
    return isDocumentDirty(activeDocumentIndex());
}

int PicturaMainWindow::addDocument(PictureView* view, const QString& path)
{
    if (!view) {
        return -1;
    }
    // Apply the persisted preference to every newly created/opened tab.
    if (view->gpu_compute() != gpuCompute_) {
        view->set_gpu_compute(gpuCompute_);
    }
    if (view->color_policy() != colorPolicy_) {
        view->set_color_policy(colorPolicy_);
    }
    gpuAvailable_ = view->gpu_available();
    DocEntry entry;
    entry.view = view;
    if (path.isEmpty()) {
        entry.untitledNumber = ++untitledCounter_;
    } else {
        entry.path = path;
    }
    entry.canvas = new ImageView(this);
    entry.canvasHost = new CanvasScrollBars(this);
    entry.canvasHost->setView(entry.canvas);
    if (fileDropRouter_) {
        entry.canvas->setAcceptDrops(true);
        entry.canvas->installEventFilter(fileDropRouter_);
    }
    if (view->has_document()) {
        entry.canvas->setDocument(QSize(view->document_width(), view->document_height()));
        entry.canvas->setProperty(kFrameKey, quint64(pictura::frame_revision(*view)));
    } else {
        entry.canvas->replaceImage(view->image());
    }
    entry.canvas->setCanvasColor(canvasColorForIndex(canvasColorIndex_));
    entry.canvas->setFreePan(freePan_);
    prepareCanvasGuides(entry);

    // Present from the view pyramid: the canvas crops a level instead of
    // scaling the full-resolution image (see frame_canvas.cpp).
    wireCanvasLevelProvider(view, entry.canvas);

    connect(view, &PictureView::changed, this, &PicturaMainWindow::refresh);
    trackRecovery(view);
    connect(view, &PictureView::regionBlitted, this,
            [this, canvas = entry.canvas, view](const QImage& region, int x, int y) {
                pictura::ScopedTimer blitSlotTimer("cxx_regionBlitted_slot");
                if (canvas) {
                    canvas->blitRegion(region, x, y);
                }
                // The cheapest sync only: restart the same 120 ms panel timer the
                // `changed` path uses, and never call image()/replaceImage(), so
                // the region path cannot refresh panels faster than a full
                // recomposite.
                panelRefreshTimer_->start();
                // A region update can change the active layer's visibility
                // without a `changed` emission; keep the tool cursor in sync.
                if (tools_) {
                    tools_->refreshCursor();
                }
                // Mid-stroke, skip the command-registry and title fan-out on
                // every dab (Krita's "unnecessary objects per event"). Releasing
                // the stroke emits `changed`, whose refresh() runs them once.
                if (view && view->is_painting()) {
                    // A present arms its own flush: the next event-loop turn
                    // folds every dab that arrived since into one region, so the
                    // composite and the present track the frame rate, not the
                    // input rate.
                    paint_api::presentDeferred(*view);
                    return;
                }
                updateTabTitle(activeDocumentIndex());
                updateWindowTitle();
                if (registry_) {
                    registry_->refresh();
                }
            });
    connect(entry.canvas, &ImageView::zoomChanged, this,
            &PicturaMainWindow::onCanvasZoomChanged);
    connect(entry.canvas, &ImageView::mouseMoved, this, [this](const QPointF& p) {
        if (infoPanel_) {
            infoPanel_->setCursorPosition(p);
        }
    });
    // With the Zoom tool active a canvas right-click shows the same preset menu
    // as the toolbox slot (the image or the workspace around it).
    connect(entry.canvas, &ImageView::contextMenuRequested, this,
            [this](const QPoint& globalPos) {
                if (activeTool() == ToolId::Zoom && toolbox_ && toolbox_->zoomMenu()) {
                    toolbox_->zoomMenu()->popup(globalPos);
                }
            });

    docs_.append(entry);
    const int index = docs_.size() - 1;
    tabs_->addTab(entry.canvasHost, documentName(index));
    tabs_->setCurrentIndex(index);
    if (!path.isEmpty()) {
        rememberRecent(path);
    }
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
    return index;
}

bool PicturaMainWindow::newDocument(const QString& name, int width, int height,
                                    const QString& mode, int depth, const QString& background)
{
    // `name` is accepted for the frozen API; DocEntry carries no name field, so
    // the tab always shows the generated Untitled-<n>.
    Q_UNUSED(name);
    auto* view = new PictureView(this);
    if (!document_api::newDocument(*view, width, height, mode, depth, background)) {
        delete view;
        return false;
    }
    addDocument(view, QString());
    return true;
}

bool PicturaMainWindow::openPath(const QString& path)
{
    auto* view = new PictureView(this);
    view->set_color_policy(colorPolicy_);
    if (!document_api::open(*view, path)) {
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
    if (!document_api::openImage(*view, path)) {
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
    return isNativeDocumentPath(path) ? openPath(path) : openImagePath(path);
}

bool PicturaMainWindow::openAsSmartObjectPath(const QString& path)
{
    auto* view = new PictureView(this);
    if (!document_api::openAsSmartObject(*view, path)) {
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

bool PicturaMainWindow::editSmartObjectContents(const QString& layerPath)
{
    PictureView* origin = activeView();
    if (!origin || layerPath.isEmpty()
        || !origin->layer_can_edit_smart_object_contents(layerPath)) {
        return false;
    }
    auto* temp = new QTemporaryDir();
    if (!temp->isValid()) {
        delete temp;
        return false;
    }
    const QString filename = QStringLiteral("contents.psd");
    const QString file = temp->filePath(filename);
    if (!origin->export_smart_object_contents(layerPath, file)) {
        delete temp;
        return false;
    }
    auto* editor = new PictureView(this);
    if (!document_api::open(*editor, file)) {
        delete editor;
        delete temp;
        return false;
    }
    addDocument(editor, QString());
    editSessions_.append(SmartObjectEditSession{editor, origin, layerPath, filename, temp});
    return true;
}

bool PicturaMainWindow::saveActive()
{
    const int index = activeDocumentIndex();
    if (index < 0) {
        return false;
    }
    PictureView* view = viewAt(index);
    for (const SmartObjectEditSession& session : editSessions_) {
        if (session.editor != view) {
            continue;
        }
        const QString file = session.temp->filePath(session.filename);
        if (!document_api::save(*view, file)
            || !session.origin->commit_smart_object_edit(session.layerPath, file)) {
            return false;
        }
        refresh();
        updateTabTitle(index);
        updateWindowTitle();
        return true;
    }
    QString path = docs_.at(index).path;
    if (path.isEmpty()
        || savePathNeedsFormatDialog(path, view, isNativeDocumentPath(path))) {
        return saveAsWithDialog();
    }
    return saveActiveAs(path);
}

bool PicturaMainWindow::saveActiveAs(const QString& path)
{
    const int index = activeDocumentIndex();
    PictureView* view = viewAt(index);
    if (!view || path.isEmpty() || !document_api::save(*view, path)) {
        return false;
    }
    docs_[index].path = path;
    dropRecoverySnapshot(view);
    updateTabTitle(index);
    rememberRecent(path);
    updateWindowTitle();
    return true;
}

bool PicturaMainWindow::revertActive()
{
    const int index = activeDocumentIndex();
    PictureView* view = viewAt(index);
    const QString path = documentPath(index);
    if (!view || path.isEmpty()) {
        return false;
    }
    // Reload in place: a raster path re-imports through the Qt edge, a PSD/PSB
    // path through the native reader.
    const bool reloaded = isNativeDocumentPath(path) ? document_api::open(*view, path)
                                                     : document_api::openImage(*view, path);
    if (!reloaded) {
        return false;
    }
    updateTabTitle(index);
    refresh();
    return true;
}

bool PicturaMainWindow::closeDocument(int index, bool interactive)
{
    if (index < 0 || index >= docs_.size()) {
        return false;
    }
    if (interactive && isDocumentDirty(index)) {
        switch (askUnsaved(this, documentName(index))) {
        case UnsavedChoice::Cancel:
            return false;
        case UnsavedChoice::Save:
            setActiveDocumentIndex(index);
            if (!saveActive()) {
                return false;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    removeDocument(index);
    return true;
}

bool PicturaMainWindow::closeActiveDocument(bool interactive)
{
    return closeDocument(activeDocumentIndex(), interactive);
}

void PicturaMainWindow::removeDocument(int index)
{
    if (index < 0 || index >= docs_.size()) {
        return;
    }
    const DocEntry entry = docs_.takeAt(index);
    // Replace Color is non-modal and holds the raw view; cancel it before the
    // view is deleted, or its preview teardown would touch freed memory.
    if (replaceColorDialog_ && replaceColorDialog_->view() == entry.view) {
        replaceColorDialog_->reject();
    }
    for (int i = editSessions_.size() - 1; i >= 0; --i) {
        const SmartObjectEditSession& session = editSessions_.at(i);
        if (session.editor == entry.view || session.origin == entry.view) {
            delete session.temp;
            editSessions_.removeAt(i);
        }
    }
    tabs_->removeTab(index);
    forgetRecovery(entry.view);
    entry.view->cancel_transform();
    entry.canvas->clearTransformPreview();
    if (entry.canvasHost) {
        delete entry.canvasHost;
    } else {
        delete entry.canvas;
    }
    delete entry.view;
    refresh();
    panelRefreshTimer_->stop();
    refreshPanels();
}

void PicturaMainWindow::showNewDocumentDialog()
{
    const QString suggested = QStringLiteral("Untitled-%1").arg(untitledCounter_ + 1);
    NewDocumentSpec spec;
    if (!NewDocumentDialog::get(this, suggested,
                                tools_ ? tools_->background() : QColor(Qt::white), &spec)) {
        return;
    }
    auto* view = new PictureView(this);
    if (!create_document(*view, spec.width, spec.height, spec.mode, spec.depth,
                         spec.fill.rgb() & 0xffffffu, spec.transparent, spec.ppi, spec.perCm)) {
        delete view;
        statusBar()->showMessage(tr("Could not create the document."));
        return;
    }
    const int index = addDocument(view, QString());
    // A name the user typed replaces the generated Untitled-n.
    if (index >= 0 && spec.name.trimmed() != documentName(index) && !spec.name.trimmed().isEmpty()) {
        docs_[index].displayName = spec.name.trimmed();
        updateTabTitle(index);
    }
}

void PicturaMainWindow::showFileInfo()
{
    PictureView* view = activeView();
    if (!view) {
        return;
    }
    FileInfoDialog dialog(view->exif_rows(), view->xmp_rows(), view->iptc_edit_fields(),
                          view->iptc_rows(), view->xmp_packet(), this);
    dialog.onExportTemplate = [this, &dialog, view] {
        const QString filter = QStringLiteral("XMP files (*.xmp)");
        QString dest = getSaveFileName(this, tr("Export Metadata Template"),
                                       documentDirectory(activeFilePath()),
                                       {filter}, nullptr);
        if (dest.isEmpty()) {
            return;
        }
        // ponytail: the static getSaveFileName has no setDefaultSuffix; append it when omitted.
        if (!dest.endsWith(QStringLiteral(".xmp"), Qt::CaseInsensitive)) {
            dest += QStringLiteral(".xmp");
        }
        if (!view->export_metadata_template(dest)) {
            QMessageBox::warning(&dialog, tr("Export Metadata Template"),
                                 tr("Could not write the template."));
        }
    };
    dialog.onApplyTemplate = [this, &dialog, view](int mode) {
        const QString filter = QStringLiteral("XMP files (*.xmp)");
        const QString path =
            getOpenFileName(&dialog, tr("Apply Metadata Template"),
                            documentDirectory(activeFilePath()), {filter});
        if (path.isEmpty()) {
            return;
        }
        if (!view->apply_metadata_template(path, mode)) {
            QMessageBox::warning(&dialog, tr("Apply Metadata Template"),
                                 tr("Could not apply the template."));
        }
    };
    if (runDialog(dialog, this) == QDialog::Accepted) {
        view->apply_metadata_edits(dialog.edits());
    }
}

void PicturaMainWindow::showProfileCommand(bool convert)
{
    PictureView* view = activeView();
    if (!view || !view->has_document() || document_api::documentMode(*view) != QStringLiteral("rgb")) {
        return;
    }
    ProfileDialog dialog(convert, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    const int index = dialog.profileIndex();
    const bool ok = convert ? view->convert_profile(index) : view->assign_profile(index);
    if (ok) {
        refresh();
    }
}

void PicturaMainWindow::showColorSettings()
{
    ColorSettingsDialog dialog(colorPolicy_, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    colorPolicy_ = dialog.policyCode();
    for (const DocEntry& entry : docs_) {
        if (entry.view) {
            entry.view->set_color_policy(colorPolicy_);
        }
    }
    saveSession();
}

void PicturaMainWindow::showOpenDialog()
{
    const QString path =
        getOpenFileName(this, tr("Open"), documentDirectory(activeFilePath()), openFileFilters());
    if (path.isEmpty()) {
        return;
    }
    openDocumentAtPath(path);
}

void PicturaMainWindow::setBrightnessLevel(int level)
{
    applyBrightness(level);
}

void PicturaMainWindow::setScreenMode(ScreenMode mode)
{
    const ScreenMode previous = screenMode_;
    screenMode_ = mode;
    auto setCanvasColor = [this](const QColor& color) {
        for (const DocEntry& entry : docs_) {
            if (entry.canvas) {
                entry.canvas->setCanvasColor(color);
            }
        }
    };
    switch (mode) {
    case ScreenMode::Standard: {
        Qt::WindowStates state = windowState();
        state.setFlag(Qt::WindowFullScreen, false);
        setWindowState(state);
        menuBar()->setVisible(true);
        statusBar()->setVisible(true);
        setPanelColumnsVisible(!panelsHidden_);
        setCanvasColor(canvasColorForIndex(canvasColorIndex_));
        break;
    }
    case ScreenMode::FullWithMenuBar:
        fullScreenChromeShown_ = false;
        setWindowState(Qt::WindowFullScreen);
        menuBar()->setVisible(true);
        statusBar()->setVisible(false);
        // CS6 keeps the Tools panel, Options bar, and panels in this mode; only
        // the document-window chrome (tab, scroll bars, status bar) goes away.
        setPanelColumnsVisible(!panelsHidden_);
        setCanvasColor(QColor(128, 128, 128));
        break;
    case ScreenMode::Full:
        if (previous != ScreenMode::Full) {
            fullScreenChromeShown_ = false;
        }
        setWindowState(Qt::WindowFullScreen);
        menuBar()->setVisible(fullScreenChromeShown_);
        statusBar()->setVisible(false);
        setPanelColumnsVisible(fullScreenChromeShown_);
        setCanvasColor(Qt::black);
        break;
    }
    registry_->refresh();
    if (toolbox_) {
        toolbox_->setActiveScreenMode(static_cast<int>(mode));
    }
    layoutWorkspaceFrame();
}

PicturaMainWindow::ScreenMode PicturaMainWindow::nextScreenMode(bool forward) const
{
    int index = static_cast<int>(screenMode_);
    index += forward ? 1 : -1;
    index = (index % 3 + 3) % 3;
    return static_cast<ScreenMode>(index);
}

void PicturaMainWindow::cycleScreenMode(bool forward)
{
    setScreenMode(nextScreenMode(forward));
}

void PicturaMainWindow::requestScreenMode(ScreenMode mode)
{
    if (mode != ScreenMode::Standard && screenMode_ == ScreenMode::Standard
        && !confirmFullScreenMode(this)) {
        return;
    }
    setScreenMode(mode);
}

void PicturaMainWindow::setPanelColumnsVisible(bool visible)
{
    for (PanelColumn* column : panelColumns()) {
        column->setVisible(visible);
    }
}

void PicturaMainWindow::togglePanelsHidden()
{
    if (screenMode_ != ScreenMode::Full) {
        // Standard and Full Screen With Menu Bar share the persisted Tab state;
        // CS6 toggles the same chrome in both.
        setPanelsHidden(!panelsHidden_);
        return;
    }
    // Full Screen Mode hides the panels and the menu bar; Tab reveals them
    // transiently, and the next Tab hides them again.
    fullScreenChromeShown_ = !fullScreenChromeShown_;
    menuBar()->setVisible(fullScreenChromeShown_);
    setPanelColumnsVisible(fullScreenChromeShown_);
    layoutWorkspaceFrame();
}

void PicturaMainWindow::refresh()
{
    pictura::ScopedTimer refreshTimer("cxx_frame_refresh (total)");
    const int index = activeDocumentIndex();
    PictureView* view = activeView();
    ImageView* canvas = canvasAt(index);

    // Empty workspace: keep the document pane (it holds the splitter stretch and
    // the minimum width, so the widget columns can never absorb the workspace),
    // but hide the empty tab strip. This is the single writer of the pane's
    // visibility so every document-set change routes here.
    if (tabs_) {
        tabs_->setVisible(true);
        if (tabs_->tabBar()) {
            tabs_->tabBar()->setVisible(!docs_.isEmpty());
        }
    }

    // A document canvas holds no image: it crops the view pyramid, so a
    // rebuilt frame only invalidates its crops. A `changed` that rebuilt no
    // frame (a bounded undo, a rename, a slice) leaves it alone.
    const quint64 frame = view ? quint64(pictura::frame_revision(*view)) : 0;
    if (view && canvas && !(view->has_document() && canvas->property(kFrameKey) == QVariant(frame))) {
        canvas->setProperty(kFrameKey, frame);
        if (view->has_document()) {
            canvas->replaceDocument(QSize(view->document_width(), view->document_height()));
        } else {
            canvas->setImage(view->image());
        }
    }
    if (tools_) {
        tools_->bindCanvas(canvas);
    }

    panelRefreshTimer_->start();
    updateStatus();
    if (registry_) {
        pictura::ScopedTimer registryTimer("cxx_registry_refresh");
        registry_->refresh();
    }
    updateAlignControls();
    updateTabTitle(index);
    updateWindowTitle();
    refreshSelectionOverlay();
}

void PicturaMainWindow::refreshSelectionOverlay()
{
    PictureView* view = activeView();
    ImageView* canvas = imageView();
    if (!view || !canvas) {
        return;
    }
    if (view->has_selection()) {
        canvas->setSelectionContour(selection_api::contour(*view));
    } else {
        canvas->clearSelectionContour();
    }
}

void PicturaMainWindow::updateTabTitle(int index)
{
    if (!tabs_ || index < 0 || index >= docs_.size()) {
        return;
    }
    QString title = documentName(index);
    if (ImageView* canvas = canvasAt(index)) {
        title += QStringLiteral(" @ %1%").arg(qRound(canvas->zoom() * 100.0));
    }
    if (PictureView* view = viewAt(index); view && view->has_document()) {
        const QString mode = modeLabel(image_mode(*view));
        const int bits = image_depth_bits(*view);
        if (!mode.isEmpty() && bits > 0) {
            title += QStringLiteral(" (%1/%2)").arg(mode).arg(bits);
        }
    }
    if (isDocumentDirty(index)) {
        title += QStringLiteral(" *");
    }
    tabs_->setTabText(index, title);
    tabs_->setTabToolTip(index, docs_.at(index).path);
}

void PicturaMainWindow::updateWindowTitle()
{
    const int index = activeDocumentIndex();
    const QString name = index >= 0 ? documentName(index) : QStringLiteral("Untitled");
    setWindowTitle(QStringLiteral("%1 — Kooka Pictura").arg(name));
}

void PicturaMainWindow::closeEvent(QCloseEvent* event)
{
    for (int i = 0; i < docs_.size(); ++i) {
        if (!isDocumentDirty(i)) {
            continue;
        }
        switch (askUnsaved(this, documentName(i))) {
        case UnsavedChoice::Cancel:
            event->ignore();
            return;
        case UnsavedChoice::Save:
            setActiveDocumentIndex(i);
            if (!saveActive()) {
                event->ignore();
                return;
            }
            break;
        case UnsavedChoice::Discard:
            break;
        }
    }
    saveSession();
    QMainWindow::closeEvent(event);
}

void PicturaMainWindow::keyPressEvent(QKeyEvent* event)
{
    if (hintBar_) {
        hintBar_->setPressedKey(hintKeyLabel(event));
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Space) {
        ImageView* canvas = imageView();
        if (canvas && (!tools_ || !tools_->transformSessionActive())) {
            canvas->setSpacePan(true);
            event->accept();
            return;
        }
    }
    if (tools_
        && (event->key() == Qt::Key_Shift || event->key() == Qt::Key_Alt
            || event->key() == Qt::Key_Control)) {
        tools_->refreshCursor();
    }
    if (!event->isAutoRepeat()
        && (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter)
        && tools_
        && (tools_->activeTool() == ToolId::Crop
            || tools_->activeTool() == ToolId::PerspectiveCrop)) {
        commitCrop();
        return;
    }
    if (!event->isAutoRepeat()
        && (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter)
        && tools_
        && (tools_->activeTool() == ToolId::PolygonalLasso
            || tools_->activeTool() == ToolId::MagneticLasso
            || tools_->activeTool() == ToolId::Pen)) {
        tools_->commitPolygonLasso();
        return;
    }
    if (dispatchCommandKey(event)) {
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Escape && tools_
        && tools_->cancelPolygonLasso()) {
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Escape
        && screenMode_ != ScreenMode::Standard) {
        setScreenMode(ScreenMode::Standard);
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_F) {
        requestScreenMode(nextScreenMode(!(event->modifiers() & Qt::ShiftModifier)));
        return;
    }
    // EU/Scandinavian fallback for the `[`/`]` brush shortcuts: the US
    // QShortcuts above consume the event first, so this only runs when the
    // layout produced a different key value; the native scan code still maps.
    if (!event->isAutoRepeat() && tools_
        && tools_->applyBrushShortcut(event->key(), event->nativeScanCode(),
                                      event->modifiers().testFlag(Qt::ShiftModifier))) {
        return;
    }
    if (tools_ && tools_->handleNudgeKey(event->key(), event->modifiers())) {
        event->accept();
        return;
    }
    QMainWindow::keyPressEvent(event);
}

void PicturaMainWindow::keyReleaseEvent(QKeyEvent* event)
{
    if (hintBar_) {
        hintBar_->setPressedKey(QString());
    }
    if (event->key() == Qt::Key_Space) {
        if (ImageView* canvas = imageView(); canvas && canvas->spacePanForTest()) {
            canvas->setSpacePan(false);
            canvas->setPanEnabled(tools_ && tools_->activeTool() == ToolId::Hand);
            if (tools_) {
                tools_->refreshCursor();
            }
        }
    }
    if (tools_
        && (event->key() == Qt::Key_Shift || event->key() == Qt::Key_Alt
            || event->key() == Qt::Key_Control)) {
        tools_->refreshCursor();
    }
    QMainWindow::keyReleaseEvent(event);
}

void PicturaMainWindow::updateStatus()
{
    // One owner for footer visibility: an empty workspace shows none of it.
    const bool hasDoc = !docs_.isEmpty();
    ImageView* canvas = imageView();
    if (zoomField_) {
        zoomField_->setVisible(hasDoc);
        if (canvas) {
            // Integer percent like the tab title: the readouts agree and the
            // value always fits the width the box reserves for its range.
            zoomField_->setValue(qRound(canvas->zoom() * 100.0));
        }
    }
    if (sizeLabel_) {
        sizeLabel_->setVisible(hasDoc);
        QString text = QStringLiteral("—");
        PictureView* view = activeView();
        if (view && view->has_document()) {
            const int width = view->document_width();
            const int height = view->document_height();
            if (statusReadout_ == QStringLiteral("dimensions")) {
                text = QStringLiteral("W %1  H %2").arg(width).arg(height);
            } else {
                // ponytail: 72 ppi is the PSD default; wire to document
                // resolution once the codec surfaces it.
                text = QStringLiteral("%1 px x %2 px (72 ppi)").arg(width).arg(height);
            }
        }
        sizeLabel_->setText(text);
    }
    if (backendLabel_) {
        backendLabel_->setVisible(hasDoc);
        PictureView* view = activeView();
        backendLabel_->setText(view ? view->active_backend() : QStringLiteral("—"));
    }
    const bool showHints = hasDoc && hintsVisible_;
    if (hintBar_) {
        hintBar_->setVisible(showHints);
    }
    if (hintSeparator_) {
        hintSeparator_->setVisible(showHints);
    }
    if (statusOptionsButton_) {
        statusOptionsButton_->setVisible(hasDoc);
    }
}

void PicturaMainWindow::applyBrightness(int level)
{
    brightnessLevel_ = Theme::clampLevel(level);
    Theme::apply(brightnessLevel_);
    applyWorkspaceCanvasColor();
}

} // namespace pictura
