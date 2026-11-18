#include "frame_includes.h"

#include <QtWidgets/QProgressBar>
#include "file_info_dialog.h"
#include "new_document_dialog.h"
#include "panels/panel_group.h"
#include "document_registry.h"
#include "panel_host.h"
#include "panels/actions_engine_bridge.h"
#include "panels/actions_panel.h"
#include "panels/color_panel.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/engine_bridge.h"

#include "api/document_api.h"
#include "api/selection_api.h"
#include "api/type_api.h"
#include "busy_job.h"
#include "file_drop_router.h"
#include "fonts.h"
#include "frame_canvas.h"
#include "import_save_coordinator.h"
#include "menu_registrar.h"
#include "frame_update.h"
#include "gpu_probe.h"
#include "memory_refusal.h"
#include "api/paint_bridge.h"
#include "panels/numeric_field.h"
#include "pictura_debug_timing.h"
#include "screen_mode_message.h"
#include "text_input.h"
#include "value_input_enter.h"

#include <QtCore/QHash>
#include <QtCore/QSize>
#include <QtCore/QTemporaryDir>
#include <QtGui/QDesktopServices>
#include <QtGui/QFontMetrics>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFrame>
#include <QtGui/QResizeEvent>
#include <QtGui/QShowEvent>
#include <QtWidgets/QTabBar>

namespace pictura {

bool launchCreatesScratchDocument(bool selfTest, bool codecLoaded)
{
    return selfTest && !codecLoaded;
}

PicturaMainWindow::PicturaMainWindow(QWidget* parent, bool enableUpdateCheck)
    : QMainWindow(parent)
{
    installValueInputEnter();
    updateChecksEnabled_ = enableUpdateCheck;
    {
        DocumentRegistry::Hooks hooks;
        hooks.shellWidget = [this]() -> QWidget* { return this; };
        hooks.openDocumentsAsTabs = [this]() { return openDocumentsAsTabs_; };
        hooks.floatingDocumentDocking = [this]() { return floatingDocumentDocking_; };
        hooks.fullScreen = [this]() { return screenMode_ != ScreenMode::Standard; };
        hooks.requestRefresh = [this]() { refresh(); };
        hooks.requestPanelRefresh = [this]() {
            if (panels_) {
                panels_->refreshPanels();
            }
        };
        hooks.stopPanelRefreshTimer = [this]() {
            if (panels_) {
                panels_->stopPanelRefreshTimer();
            }
        };
        hooks.activateShellWindow = [this]() { activateWindow(); };
        hooks.raisePanelOverlays = [this]() {
            for (PanelFloat* panel : findChildren<PanelFloat*>(Qt::FindDirectChildrenOnly)) {
                if (!panel->isWindow()) {
                    panel->raise();
                }
            }
        };
        hooks.retargetPanels = [this]() {
            if (!panels_) {
                return;
            }
            // The last close leaves no active view; the dock still holds the
            // freed one, so it must be unbound even though null == null.
            if (!activeView() || panels_->dockNeedsRebind()) {
                panels_->retargetDock();
            }
        };
        hooks.closeWindowRequested = [this](DocWindow* window) { closeWindow(window, true); };
        hooks.setupCanvas = [this](DocWindow& window) { setupCanvas(window); };
        hooks.wireCanvas = [this](DocWindow& window) { wireCanvas(window); };
        hooks.wireDocument = [this](DocEntry& doc) { wireDocument(doc); };
        hooks.applyViewSettings = [this](PictureView* view) {
            if (view->gpu_compute() != gpuCompute_) {
                view->set_gpu_compute(gpuCompute_);
            }
            applyColorSettings(view);
            gpuAvailable_ = view->gpu_available();
        };
        hooks.wireOpenSignals = [this](PictureView* view) {
            if (importSave_) {
                importSave_->wireOpenSignals(view);
            }
        };
        hooks.rememberRecent = [this](const QString& path) {
            if (importSave_) {
                importSave_->rememberRecent(path);
            }
        };
        hooks.forgetToolView = [this](PictureView* view) {
            if (tools_) {
                tools_->forgetView(view);
            }
        };
        hooks.forgetCanvas = [this](ImageView* canvas) {
            if (tools_ && tools_->canvas() == canvas) {
                tools_->bindCanvas(nullptr);
            }
        };
        hooks.openDelayForTest = [this]() -> quint64 {
            return importSave_ ? quint64(importSave_->openDelayForTest()) : 0;
        };
        hooks.loadingPageColor = [this]() { return canvasColorForIndex(canvasColorIndex_); };
        hooks.scrollAllWindows = [this]() {
            const HandGroup* hand =
                tools_ ? tools_->optionsFor<HandGroup>(ToolId::Hand) : nullptr;
            return hand && hand->get().scrollAll;
        };
        hooks.toolCanvas = [this]() { return tools_ ? tools_->canvas() : nullptr; };
        documents_ = std::make_unique<DocumentRegistry>(std::move(hooks), this, this);
    }
    connect(documents_.get(), &DocumentRegistry::aboutToRemoveDocument, this,
            &PicturaMainWindow::onAboutToRemoveDocument);
    connect(documents_.get(), &DocumentRegistry::cancelOpenRequested, this,
            [this](PictureView* view) {
                if (importSave_) {
                    importSave_->cancelOpenForView(view);
                }
            });
    connect(documents_.get(), &DocumentRegistry::closeDocumentRequested, this,
            [this](int index) {
                if (importSave_) {
                    importSave_->closeDocument(index, true);
                }
            });
    MenuRegistrar::Runtime menuRuntime;
    menuRuntime.requestRefresh = [this]() { refresh(); };
    menuRuntime.showStatus = [this](const QString& message, int ms) {
        if (statusBar()) {
            statusBar()->showMessage(message, ms);
        }
    };
    menuRuntime.optionsBar = [this]() { return optionsBar_; };
    menuRuntime.dialogParent = [this]() -> QWidget* { return this; };
    menuRuntime.updateWindowTitle = [this]() { updateWindowTitle(); };
    menuRuntime.openUrl = [this](const QUrl& url) {
        if (urlOpener_) {
            urlOpener_(url);
        } else {
            QDesktopServices::openUrl(url);
        }
    };
    menuRuntime.installMenuCornerButtons = [this]() { installMenuCornerButtons(); };
    menuRuntime.closeWindow = [this](DocWindow* window, bool interactive) {
        return closeWindow(window, interactive);
    };
    menuRuntime.openSaveSelectionDialog = [this]() { openSaveSelectionDialog(); };
    menuRuntime.openLoadSelectionDialog = [this]() { openLoadSelectionDialog(); };
    menuRuntime.openDuplicateChannelDialog = [this]() { openDuplicateChannelDialog(); };
    menuRuntime.uiScalePercent = [this]() { return uiScalePercent_; };
    menuRuntime.foregroundColor = [this]() { return foregroundColor(); };
    menu_ = std::make_unique<MenuRegistrar>(
        [this](CommandRegistry& registry) { registerFrameCommands(registry); },
        std::move(menuRuntime), this);

    auto* colorState = new ColorState(this);
    auto* actionsEngine = new ActionsEngineBridge(new ActionsEngine(this), this);
    PanelHost::Hooks panelHooks;
    panelHooks.saveSession = [this]() { saveSession(); };
    panelHooks.interfaceOptionsRequested = [this]() {
        showPreferences(PreferencesDialog::kInterface);
    };
    panelHooks.activeView = [this]() { return activeView(); };
    panelHooks.activeCanvas = [this]() { return canvasAt(activeDocumentIndex()); };
    panelHooks.showStatus = [this](const QString& message, int ms) {
        if (statusBar()) {
            statusBar()->showMessage(message, ms);
        }
    };
    panelHooks.cancelDocumentDialogs = [this](PictureView* view) {
        if (menu_) {
            menu_->cancelDocumentDialogs(view);
        }
    };
    panelHooks.saveSelectionOptions = [this]() { openSaveSelectionDialog(); };
    panelHooks.alphaVisibilityChanged = [this]() { refresh(); };
    panelHooks.convertImageMode = [this](const QString& mode) {
        if (menu_) {
            menu_->convertImageMode(mode);
        }
    };
    panelHooks.convertImageDepth = [this](int depth) {
        if (menu_) {
            menu_->convertImageDepth(depth);
        }
    };
    panelHooks.editSmartFilter = [this](const QString& path, int index) {
        if (menu_) {
            menu_->editSmartFilter(path, index);
        }
    };
    panelHooks.attachWorkspaceMenu = [this](QMenu* menu) {
        if (optionsBar_) {
            optionsBar_->setWorkspaceMenu(menu);
        }
    };
    panelHooks.setActiveWorkspace = [this](const QString& name) {
        if (optionsBar_) {
            optionsBar_->setActiveWorkspace(name);
        }
    };
    panels_ = std::make_unique<PanelHost>(this, documents_.get(), menu_->commands(), colorState,
                                          actionsEngine, std::move(panelHooks));
    connect(panels_.get(), &PanelHost::layerEditTargetChanged, this, [this](bool mask) {
        ColorState* state = panels_->colorState();
        if ((activeView() && quick_mask_active(*activeView())) || quickMaskUi_.colorsActive) {
            syncQuickMask();
            return;
        }
        if (state->maskActive() == mask) {
            return;
        }
        state->setMaskActive(mask);
        if (tools_) {
            tools_->setMaskActive(mask);
        }
        const QColor foreground = state->foreground();
        const QColor background = state->background();
        state->setForeground(otherForeground_);
        state->setBackground(otherBackground_);
        otherForeground_ = foreground;
        otherBackground_ = background;
    });
    connect(panels_.get(), &PanelHost::layerSelectionChanged, this, [this]() {
        syncQuickMask();
        if (menu_) {
            menu_->updateAlignControls();
        }
        if (tools_) {
            tools_->syncTransformControls();
        }
    });

    // Double-clicking the empty workspace (no documents open) opens the Open
    // dialog. The blank pane is the tab widget's internal stacked widget.
    installWorkspaceOpenGesture();

    ImportSaveCoordinator::Hooks saveHooks;
    saveHooks.applyColorSettings = [this](PictureView* view) { applyColorSettings(view); };
    saveHooks.gpuCompute = [this]() { return gpuCompute_; };
    saveHooks.requestRefresh = [this]() { refresh(); };
    saveHooks.updateWindowTitle = [this]() { updateWindowTitle(); };
    saveHooks.showStatus = [this](const QString& message, int ms) {
        if (statusBar()) {
            statusBar()->showMessage(message, ms);
        }
    };
    saveHooks.recordCommand = [this](const QString& id, const QJsonObject& values) {
        if (menu_) {
            menu_->commands()->recordCommand(id, CommandParams{values});
        }
    };
    saveHooks.reserveRecording = [actionsEngine](const QString& id, const QJsonObject& values) {
        return actionsEngine->reserveCommand(id, values);
    };
    saveHooks.finishRecording = [actionsEngine](quint64 token, bool saved) {
        actionsEngine->finishReservedCommand(token, saved);
    };
    saveHooks.updateSaveProgress = [this] { updateStatus(); };
    saveHooks.saveSession = [this]() { saveSession(); };
    saveHooks.foregroundColor = [this]() { return foregroundColor(); };
    saveHooks.backgroundColor = [this]() { return backgroundColor(); };
    importSave_ = std::make_unique<ImportSaveCoordinator>(*documents_, this,
                                                         std::move(saveHooks), this);
    menu_->setCollaborators(documents_.get(), panels_.get(), importSave_.get());
    connect(importSave_.get(), &ImportSaveCoordinator::documentOpenFinished, this,
            &PicturaMainWindow::documentOpenFinished);

    const SessionState state = pictura::loadSession();
    importSave_->loadState(state);
    gpuCompute_ = state.gpuCompute;
    set_memory_usage_percent(state.memoryUsagePercent);
    set_scratch_volumes(state.scratchVolumes.join(QLatin1Char('\n')));
    start_memory_eviction();
    colorPolicy_ = state.colorPolicy;
    cmykPolicy_ = state.cmykPolicy;
    workingCmyk_ = state.workingCmyk;
    renderIntent_ = state.renderIntent;
    blackPointCompensation_ = state.blackPointCompensation;
    useDither_ = state.useDither;
    askMismatchOpen_ = state.askMismatchOpen;
    askMissingOpen_ = state.askMissingOpen;
    pickerChoice_ = state.pickerChoice;
    ColorPickerDialog::setPickerPreference(pickerChoice_);
    applyTextGamma(state.textGammaEnabled, state.textGamma);
    typeShowAsianOptions_ = state.typeShowAsianOptions; typeMiddleEastern_ = state.typeMiddleEastern;
    typeMissingGlyphProtection_ = state.typeMissingGlyphProtection;
    pictura::type_api::setMissingGlyphProtection(typeMissingGlyphProtection_);
    useShiftKeyForToolSwitch_ = state.useShiftKeyForToolSwitch;
    openDocumentsAsTabs_ = state.openDocumentsAsTabs;
    floatingDocumentDocking_ = state.enableFloatingDocumentDocking;
    zoomResizesWindows_ = state.zoomResizesWindows;
    applyShortcutOverrides(state);
    loadGuideSession(state);
    proofDevice_ = state.proofDevice;
    proofIntent_ = state.proofIntent;
    proofBpc_ = state.proofBpc;
    proofSimulatePaper_ = state.proofSimulatePaper;
    proofSimulateBlack_ = state.proofSimulateBlack;
    proofColors_ = state.proofColors;
    // The family drives the theme chrome built below, so resolve it before the menu bar (and its theme toggle icon) is built.
    themeFamily_ = state.themeFamily == 1 ? 1 : 0;
    // The application font and every chrome metric built below read the scale,
    // so resolve it before the menu bar, panels, tools, and status bar exist.
    uiScalePercent_ = Theme::normalizedUiScalePercent(state.uiScalePercent);
    Theme::setUiScalePercent(uiScalePercent_);
    applyBundledUiFont();
    // The window predates the application font, so it keeps the default face
    // until told otherwise; the menu bar is created below and inherits it.
    setFont(QApplication::font());
    // Apply the restored theme before the menus, panels, and tools exist, so
    // they are built under it instead of re-polished by a later application.
    // Its style sheet reads the scale and the application font set above.
    brightnessLevel_ = Theme::clampLevel(state.brightnessLevel);
    Theme::apply(themeFamily_ == 1 ? Theme::ThemeFamily::Light : Theme::ThemeFamily::Dark,
                 brightnessLevel_);
    // Probe the adapter once so the toggle can be offered without a document.
    // The marker makes a parent crash during init a one-launch fallback; the
    // cache records a clean failure so the next launch advances.
    {
        PictureView probe;
        const bool marked = beginDeviceInit();
        gpuAvailable_ = probe.gpu_available();
        if (marked) {
            endDeviceInit(gpuAvailable_);
        }
    }
    gpuCandidatesAvailable_ = gpuAvailable_ || gpuChainHasResetPath();
    // No candidate means the preference cannot be honored: read it as off so the
    // Preferences checkbox and the session agree with the CPU backend. A
    // safe-mode reset path keeps the stored value so off-and-on can retry.
    if (!gpuCandidatesAvailable_) {
        gpuCompute_ = false;
    }
    menu_->registerHandlers();
    if (updateChecksEnabled_ && !qEnvironmentVariableIsSet("PICTURA_DISABLE_UPDATE_CHECK")) {
        ensureUpdateChecker(*this, state.checkForUpdatesOnLaunch);
    }
    menu_->buildMenus(menuBar());
    const QList<PanelGroup*> stagedGroups = panels_->buildPanels(state);
    // The Layers row menu's Edit / Replace / Export Contents actions run through
    // the frame, so this is wired here where the panel exists (registerHandlers
    // runs before the panels are built).
    menu_->wireSmartObjectRowMenu();
    wireMaskOverlay();
    buildTools(state.toolsColumns, state.useShiftKeyForToolSwitch, state.toolOptions);
    menu_->setTools(tools_);
    if (tools_) {
        tools_->setTypePlaceholderText(state.typePlaceholderText);
        tools_->applyCropOptions(state.cropClassicMode, state.cropDeletePixels, state.cropRatio,
                                 state.cropGridOverlay);
    }
    buildStatusBar();
    // Route OS file drops by target: a document canvas places into the active
    // document, every other target opens a new tab. Enabling drops is what makes
    // each widget a drop target; per-canvas install happens in addDocument.
    fileDropRouter_ = new FileDropRouter(this);
    for (QWidget* target :
         {static_cast<QWidget*>(documents_->tabs()),
          static_cast<QWidget*>(documents_->tabBar()),
          static_cast<QWidget*>(menuBar()), static_cast<QWidget*>(optionsBar_),
          static_cast<QWidget*>(this)}) {
        target->setAcceptDrops(true);
        target->installEventFilter(fileDropRouter_);
    }
    applyBrightness(state.brightnessLevel);
    panels_->applyPanelSession(state);
    panels_->applyFreshSessionDefault(state, stagedGroups);
    const bool sessionHadLayout = !state.panelColumns.isEmpty() || !state.panelGroups.isEmpty();
    panels_->initWorkspaces(sessionHadLayout);
    if (!state.layout.isEmpty()) {
        restoreStoredLayout(state.layout, state.layoutRevision);
    }

    auto* hidePanels = new QShortcut(QKeySequence(Qt::Key_Tab), this);
    connect(hidePanels, &QShortcut::activated, this, [this]() { togglePanelsHidden(); });
    auto* hidePanelsBack = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_Tab), this);
    connect(hidePanelsBack, &QShortcut::activated, this, [this]() { togglePanelsHidden(); });
    // A focused field owns Tab / Shift+Tab: it commits and moves focus to the
    // next field instead of hiding the panels (#1028). Each connection's context
    // is its shortcut, so teardown disconnects it with the shortcut.
    for (QShortcut* shortcut : {hidePanels, hidePanelsBack}) {
        shortcut->setEnabled(!isTextInput(QApplication::focusWidget()));
        connect(qApp, &QApplication::focusChanged, shortcut,
                [shortcut](QWidget*, QWidget* now) { shortcut->setEnabled(!isTextInput(now)); });
    }
    menu_->wireDocumentMenus();
    menu_->wireWindowArrange();
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
    panels_->reapplyColumnStretch();
    panels_->layoutWorkspaceFrame();
}

void PicturaMainWindow::resizeEvent(QResizeEvent* event)
{
    QMainWindow::resizeEvent(event);
    if (panels_) {
        panels_->handleShellResize();
    }
}

void PicturaMainWindow::showEvent(QShowEvent* event)
{
    QMainWindow::showEvent(event);
    if (panels_) {
        panels_->layoutWorkspaceFrame();
    }
    scheduleUpdateCheck(*this);
    if (panels_) {
        // Let the compositor apply the restored/maximized frame geometry first;
        // an in-window overlay then converts its stored global rect against it.
        QTimer::singleShot(0, this, [this]() { panels_->applyPendingFloatRestores(); });
    }
}

PicturaMainWindow::~PicturaMainWindow()
{
    // Detach the focus-loss Space guard before the child widgets are torn down;
    // a child's clearFocus() emits focusChanged while documents_->tabs() is already gone.
    QObject::disconnect(spacePanFocusConn_);
    QObject::disconnect(deleteShortcutsFocusConn_);
}

QStringList PicturaMainWindow::topLevelMenuTitles() const
{
    return menu_ ? menu_->commands()->topLevelTitles() : QStringList{};
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
    CropCommitSession* session = tools_ ? tools_->cropCommitSession() : nullptr;
    if (!session || !session->commitCrop()) {
        return false;
    }
    refresh();
    return true;
}

bool PicturaMainWindow::registerPanel(QWidget* panel, Qt::DockWidgetArea area)
{
    return panels_ && panels_->registerPanel(panel, area);
}

QSet<QString> PicturaMainWindow::panelObjectNames() const
{
    return panels_ ? panels_->panelObjectNames() : QSet<QString>{};
}

ImageView* PicturaMainWindow::imageView() const
{
    return documents_->imageView();
}

PictureView* PicturaMainWindow::activeView() const
{
    return documents_->activeView();
}

PictureView* PicturaMainWindow::viewForCanvas(const ImageView* canvas) const
{
    DocWindow* window = documents_->windowOfCanvas(canvas);
    return window && window->doc ? window->doc->view : nullptr;
}

void PicturaMainWindow::activateView(PictureView* view)
{
    const int index = documents_->indexOfView(view);
    if (index >= 0) {
        setActiveDocumentIndex(index);
    }
}

int PicturaMainWindow::documentCount() const
{
    return documents_ ? documents_->count() : 0;
}

bool PicturaMainWindow::workspaceOpenArmed() const
{
    return !documents_ || documents_->count() == 0;
}

int PicturaMainWindow::activeDocumentIndex() const
{
    return documents_->activeIndex();
}

void PicturaMainWindow::setActiveDocumentIndex(int index)
{
    documents_->setActiveIndex(index);
}

PictureView* PicturaMainWindow::viewAt(int index) const
{
    return documents_->viewAt(index);
}

ImageView* PicturaMainWindow::canvasAt(int index) const
{
    return documents_->canvasAt(index);
}

QString PicturaMainWindow::documentPath(int index) const
{
    return documents_->documentPath(index);
}

QString PicturaMainWindow::documentName(int index) const
{
    return documents_->documentName(index);
}

bool PicturaMainWindow::isDocumentDirty(int index) const
{
    return documents_->isDocumentDirty(index);
}

QString PicturaMainWindow::activeFilePath() const
{
    return documents_->activeFilePath();
}

QString PicturaMainWindow::activeDocumentName() const
{
    return documents_->activeDocumentName();
}

bool PicturaMainWindow::isActiveDirty() const
{
    return documents_->isActiveDirty();
}

int PicturaMainWindow::addDocument(PictureView* view, const QString& path, bool asTab)
{
    return documents_->addDocument(view, path, asTab);
}

int PicturaMainWindow::indexOfDoc(const DocEntry* doc) const
{
    return documents_ ? documents_->indexOfDoc(doc) : -1;
}

DocWindow* PicturaMainWindow::windowAtTab(int tab) const
{
    return documents_ ? documents_->windowAtTab(tab) : nullptr;
}

int PicturaMainWindow::tabOf(const DocWindow* window) const
{
    return documents_ ? documents_->tabOf(window) : -1;
}

DocWindow* PicturaMainWindow::windowOfCanvas(const ImageView* canvas) const
{
    return documents_ ? documents_->windowOfCanvas(canvas) : nullptr;
}

void PicturaMainWindow::presentWindows()
{
    if (documents_) {
        documents_->presentWindows();
    }
}

void PicturaMainWindow::syncScreenModeWindows()
{
    if (documents_) {
        documents_->syncScreenModeWindows();
    }
}

void PicturaMainWindow::forEachCanvas(const DocEntry& doc,
                                      const std::function<void(ImageView*)>& fn) const
{
    if (documents_) {
        documents_->forEachCanvas(doc, fn);
    }
}

QList<ImageView*> PicturaMainWindow::activeDocumentCanvases() const
{
    return documents_ ? documents_->activeDocumentCanvases() : QList<ImageView*>{};
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
        reportMemoryRefusal(this, tr("create the document"));
        return false;
    }
    addDocument(view, QString());
    return true;
}

bool PicturaMainWindow::closeWindow(DocWindow* window, bool interactive)
{
    if (!window || !documents_ || !documents_->windows().contains(window)) {
        return false;
    }
    DocEntry* doc = window->doc;
    const int index = documents_->indexOfDoc(doc);
    if (doc->windows.size() < 2) {
        return closeDocument(index, interactive);
    }
    documents_->detachWindows(QList<DocWindow*>{window});
    doc->windows.removeOne(window);
    documents_->destroyWindowWidgets(window);
    delete window;
    documents_->updateTabTitle(index);
    refresh();
    panels_->stopPanelRefreshTimer();
    panels_->refreshPanels();
    return true;
}

void PicturaMainWindow::onAboutToRemoveDocument(int index)
{
    DocEntry* entry = documents_ ? documents_->docAt(index) : nullptr;
    if (!entry) {
        return;
    }
    if (menu_) {
        menu_->cancelDocumentDialogs(entry->view);
    }
    if (entry->view) {
        entry->view->cancel_transform();
        if (tools_) {
            tools_->forgetView(entry->view);
        }
    }
}

void PicturaMainWindow::showNewDocumentDialog()
{
    const QString suggested =
        QStringLiteral("Untitled-%1").arg(documents_->untitledCounter() + 1);
    NewDocumentSpec spec;
    if (!NewDocumentDialog::get(this, suggested,
                                tools_ ? tools_->background() : QColor(Qt::white), &spec)) {
        return;
    }
    auto* view = new PictureView(this);
    if (!create_document(*view, spec.width, spec.height, spec.mode, spec.depth,
                         spec.fill.rgb() & 0xffffffu, spec.transparent, spec.ppi, spec.perCm)) {
        delete view;
        if (!reportMemoryRefusal(this, tr("create the document"))) {
            statusBar()->showMessage(tr("Could not create the document."));
        }
        return;
    }
    const int index = addDocument(view, QString());
    // A name the user typed replaces the generated Untitled-n.
    if (index >= 0 && spec.name.trimmed() != documentName(index) && !spec.name.trimmed().isEmpty()) {
        documents_->setDocumentName(index, spec.name.trimmed());
        updateTabTitle(index);
    }
}

void PicturaMainWindow::showFileInfo()
{
    PictureView* view = activeView();
    if (!view) {
        return;
    }
    QStringList rows = view->exif_rows();
    rows += view->iptc_rows();
    rows += view->xmp_rows();
    FileInfoDialog dialog(rows, view->file_info_edit_fields(), view->xmp_packet(), this);
    dialog.onApply = [view](const QStringList& edits) {
        return view->apply_metadata_edits(edits);
    };
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
    runDialog(dialog, this);
}

void PicturaMainWindow::showOpenDialog()
{
    const QStringList paths = getOpenFileNames(this, tr("Open"),
                                               documentDirectory(activeFilePath()),
                                               openFileFilters());
    openUserPaths(paths);
}

void PicturaMainWindow::setScreenMode(ScreenMode mode)
{
    const ScreenMode previous = screenMode_;
    screenMode_ = mode;
    auto setCanvasColor = [this](const QColor& color) {
        for (const DocWindow* window : documents_->windows()) {
            if (window->canvas) {
                window->canvas->setCanvasColor(color);
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
        panels_->setPanelColumnsVisible(!panels_->panelsHidden());
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
        panels_->setPanelColumnsVisible(!panels_->panelsHidden());
        setCanvasColor(QColor(128, 128, 128));
        break;
    case ScreenMode::Full:
        if (previous != ScreenMode::Full) {
            fullScreenChromeShown_ = false;
        }
        setWindowState(Qt::WindowFullScreen);
        menuBar()->setVisible(fullScreenChromeShown_);
        statusBar()->setVisible(false);
        panels_->setPanelColumnsVisible(fullScreenChromeShown_);
        setCanvasColor(Qt::black);
        break;
    }
    syncScreenModeWindows();
    if (menu_) {
        menu_->commands()->refresh();
    }
    if (toolbox_) {
        toolbox_->setActiveScreenMode(static_cast<int>(mode));
    }
    panels_->layoutWorkspaceFrame();
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

void PicturaMainWindow::togglePanelsHidden()
{
    if (screenMode_ != ScreenMode::Full) {
        // Standard and Full Screen With Menu Bar share the persisted Tab state;
        // CS6 toggles the same chrome in both.
        setPanelsHidden(!panels_->panelsHidden());
        return;
    }
    // Full Screen Mode hides the panels and the menu bar; Tab reveals them
    // transiently, and the next Tab hides them again.
    fullScreenChromeShown_ = !fullScreenChromeShown_;
    menuBar()->setVisible(fullScreenChromeShown_);
    panels_->setPanelColumnsVisible(fullScreenChromeShown_);
    panels_->layoutWorkspaceFrame();
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
    if (QTabWidget* tabs = documents_->tabs()) {
        // A full-screen float's page may hold the document area instead.
        tabs->setVisible(documents_->documentArea()->currentWidget() == tabs);
        if (tabs->tabBar()) {
            tabs->tabBar()->setVisible(documents_->count() > 0);
        }
    }

    syncQuickMask();
    presentWindows();
    if (tools_) {
        tools_->bindCanvas(canvas);
    }
    if (toolbox_) {
        toolbox_->refreshToolAvailability();
    }
    if (optionsBar_) {
        optionsBar_->refreshDocumentFacts();
    }
    syncBrushTipPreview();
    if (DocWindow* active = documents_->activeWindow()) {
        forEachCanvas(*active->doc,
                      [this, view](ImageView* each) { syncMaskOverlay(view, each); });
    }
    syncSiblingGuides();

    panels_->schedulePanelRefresh();
    updateStatus();
    // A session can end without a transformChanged (a document switch).
    syncTransformLock();
    if (menu_) {
        pictura::ScopedTimer registryTimer("cxx_registry_refresh");
        menu_->commands()->refresh();
    }
    panels_->setRegistryRefreshPending(false);
    if (menu_) {
        menu_->updateAlignControls();
    }
    updateTabTitle(index);
    updateWindowTitle();
    refreshSelectionOverlay();
}

void PicturaMainWindow::refreshSelectionOverlay()
{
    PictureView* view = activeView();
    DocWindow* active = documents_->activeWindow();
    if (!view || !active) {
        return;
    }
    if (tools_ && tools_->transformSelectionTarget()) {
        // The live border transform owns the ants; it supplies the preview.
        forEachCanvas(*active->doc,
                      [](ImageView* canvas) { canvas->clearSelectionContour(); });
        return;
    }
    if (view->has_selection()) {
        const selection_api::SelectionContour contour = selection_api::contour(*view);
        forEachCanvas(*active->doc,
                      [&contour](ImageView* canvas) { canvas->setSelectionContour(contour); });
    } else {
        forEachCanvas(*active->doc,
                      [](ImageView* canvas) { canvas->clearSelectionContour(); });
    }
}

void PicturaMainWindow::syncBrushTipPreview()
{
    const PaintOptions* paint =
        tools_ ? tools_->optionsFor<PaintOptions>(ToolId::Brush) : nullptr;
    const bool paintTool = tools_
        && (tools_->activeTool() == ToolId::Brush || tools_->activeTool() == ToolId::Pencil);
    const bool penTip = paint && paint->penTip();
    DocWindow* active = documents_->activeWindow();
    for (const DocWindow* window : documents_->windows()) {
        BrushTipPreview* preview = window->tip;
        if (!preview) {
            continue;
        }
        ImageView* canvas = window->canvas;
        const bool show = window == active && paintTool && penTip && canvas
                          && canvas->hasDocument();
        preview->setVisible(show);
        if (show) {
            preview->raise();
        }
    }
}

bool PicturaMainWindow::isLoadingDocument(int index) const
{
    return documents_->isLoading(index);
}

void PicturaMainWindow::updateTabTitle(int index)
{
    if (documents_) {
        documents_->updateTabTitle(index);
    }
}

void PicturaMainWindow::updateWindowTitle()
{
    const int index = activeDocumentIndex();
    const QString name = index >= 0 ? documentName(index) : QStringLiteral("Untitled");
    const QString proof =
        proofColors_ ? tr(" [Proof: %1]").arg(proofSetupName()) : QString();
    setWindowTitle(QStringLiteral("%1%2 — Kooka Pictura").arg(name, proof));
}

void PicturaMainWindow::closeEvent(QCloseEvent* event)
{
    cancelAllOpens();
    if (!saveAllDocumentsForRestart()) {
        event->ignore();
        return;
    }
    for (NumericField* field : findChildren<NumericField*>()) field->commitPendingEdit();
    saveSession();
    QMainWindow::closeEvent(event);
}
void PicturaMainWindow::updateStatus()
{
    // One owner for footer visibility: an empty workspace shows none of it, and
    // a temporary message takes the readouts' row so the banner never overprints
    // them. Qt hides the normal widgets itself, but a later refresh re-shows
    // them, so the gate has to live here.
    const bool hasDoc = documents_ && documents_->count() > 0;
    const bool messageShown = statusBar() && !statusBar()->currentMessage().isEmpty();
    const bool showReadouts = hasDoc && !messageShown;
    if (backgroundSaveProgress_) {
        const DocEntry* doc = documents_ ? documents_->docAt(activeDocumentIndex()) : nullptr;
        backgroundSaveProgress_->setVisible(doc && doc->saveProgress >= 0 && !messageShown);
        if (doc && doc->saveProgress >= 0) {
            backgroundSaveProgress_->setValue(doc->saveProgress);
        }
    }
    ImageView* canvas = imageView();
    if (zoomField_) {
        zoomField_->setVisible(showReadouts);
        if (canvas) {
            // Integer percent like the tab title: the readouts agree and the
            // value always fits the width the box reserves for its range.
            zoomField_->setValue(qRound(canvas->zoom() * 100.0));
        }
    }
    if (sizeLabel_) {
        sizeLabel_->setVisible(showReadouts);
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
        backendLabel_->setVisible(showReadouts);
        PictureView* view = activeView();
        backendLabel_->setText(view ? activeBackendLabel(view->gpu_compute())
                                     : QStringLiteral("—"));
    }
    if (actionLabel_) {
        QString action;
        PictureView* view = activeView();
        if (hasDoc && view && tools_ && tools_->transformSessionActive()) {
            const QString mode = transform_session_mode(*view);
            if (mode == QStringLiteral("scale")) {
                action = QStringLiteral("Scale in progress");
            } else if (mode == QStringLiteral("rotate")) {
                action = QStringLiteral("Rotate in progress");
            } else if (mode == QStringLiteral("skew")) {
                action = QStringLiteral("Skew in progress");
            } else if (mode == QStringLiteral("distort")) {
                action = QStringLiteral("Distort in progress");
            } else if (mode == QStringLiteral("perspective")) {
                action = QStringLiteral("Perspective in progress");
            } else {
                action = QStringLiteral("Free Transform in progress");
            }
        }
        actionLabel_->setText(action);
        actionLabel_->setVisible(showReadouts && !action.isEmpty());
        if (actionSeparator_) {
            actionSeparator_->setVisible(showReadouts && !action.isEmpty());
        }
    }
    const bool showHints = showReadouts && hintsVisible_;
    if (hintBar_) {
        hintBar_->setVisible(showHints);
    }
    if (hintSeparator_) {
        hintSeparator_->setVisible(showHints);
    }
    // The two static rules are normal widgets like the readouts, so a message
    // hides them too; the action and hint rules follow their own owners.
    if (QStatusBar* bar = statusBar()) {
        for (QFrame* separator : bar->findChildren<QFrame*>(QStringLiteral("statusSeparator"))) {
            if (separator != actionSeparator_ && separator != hintSeparator_) {
                separator->setVisible(showReadouts);
            }
        }
    }
    if (statusOptionsButton_) {
        statusOptionsButton_->setVisible(showReadouts);
    }
}
CommandRegistry* PicturaMainWindow::registry() const
{
    return menu_ ? menu_->commands() : nullptr;
}

CommandRegistry* PicturaMainWindow::commandRegistry() const
{
    return registry();
}

bool PicturaMainWindow::beginFreeTransform(const QString& path)
{
    return menu_ && menu_->beginFreeTransform(path);
}

bool PicturaMainWindow::beginTransformMode(const QString& path, const QString& mode)
{
    return menu_ && menu_->beginTransformMode(path, mode);
}

void PicturaMainWindow::selectLayerPath(const QString& path)
{
    if (menu_) {
        menu_->selectLayerPath(path);
    }
}

void PicturaMainWindow::importSystemClipboard()
{
    if (menu_) {
        menu_->importSystemClipboard();
    }
}

bool PicturaMainWindow::replaceSmartObjectContents(const QString& layerPath)
{
    return menu_ && menu_->replaceSmartObjectContents(layerPath);
}

bool PicturaMainWindow::exportSmartObjectContents(const QString& layerPath)
{
    return menu_ && menu_->exportSmartObjectContents(layerPath);
}

bool PicturaMainWindow::copyLayerFromDocument(const void* source, const QString& path)
{
    PictureView* target = activeView();
    PictureView* from = nullptr;
    for (const DocEntry* entry : documents_->docs()) {
        if (entry->view == source) {
            from = entry->view;
        }
    }
    // The bridge holds the source shared and the target exclusive, so the two
    // must be different documents.
    if (!target || !from || from == target || path.isEmpty()) {
        return false;
    }
    const QString selection = layerAnchor();
    const QString created = copy_layer_from_document(*target, *from, path, selection);
    if (created.isEmpty()) {
        return false;
    }
    selectLayerPath(created);
    return true;
}

QMenu* PicturaMainWindow::documentMenuForTest() const
{
    return menu_ ? menu_->documentMenu() : nullptr;
}

QStringList PicturaMainWindow::selectedLayerPaths() const
{
    PictureView* view = activeView();
    return view ? selected_layer_paths(*view) : QStringList{};
}
QString PicturaMainWindow::layerAnchor() const
{
    PictureView* view = activeView();
    return view ? layer_selection_anchor(*view) : QString();
}
} // namespace pictura
#include "moc_frame.cpp"
