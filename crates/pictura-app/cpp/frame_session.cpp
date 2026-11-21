#include <QtCore/QRect>
#include "frame_includes.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"

#include "api/paint_bridge.h"
#include "import_save_coordinator.h"
#include "gpu_probe.h"
#include "panel_host.h"
#include "panels/actions_panel.h"
#include "update_checker.h"

#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>

#include "api/filters_bridge.h"

namespace pictura {

bool PicturaMainWindow::panelsHidden() const
{
    return panels_ && panels_->panelsHidden();
}

void PicturaMainWindow::setPanelsHidden(bool hidden)
{
    if (panels_) {
        panels_->setPanelsHidden(hidden);
    }
}

PanelColumn* PicturaMainWindow::panelColumn() const
{
    return panels_ ? panels_->panelColumn() : nullptr;
}

PanelColumn* PicturaMainWindow::toolsColumn() const
{
    return panels_ ? panels_->toolsColumn() : nullptr;
}

QList<PanelColumn*> PicturaMainWindow::panelColumns() const
{
    return panels_ ? panels_->panelColumns() : QList<PanelColumn*>{};
}

QList<PanelColumn*> PicturaMainWindow::allPanelColumns() const
{
    return panels_ ? panels_->allPanelColumns() : QList<PanelColumn*>{};
}

int PicturaMainWindow::columnCount() const
{
    return panels_ ? panels_->columnCount() : 0;
}

PanelSide PicturaMainWindow::sideOf(const PanelColumn* column) const
{
    return panels_ ? panels_->sideOf(column) : PanelSide::Right;
}

PanelColumn* PicturaMainWindow::createPanelColumn(PanelSide side, PanelColumn* anchor)
{
    return panels_ ? panels_->createPanelColumn(side, anchor) : nullptr;
}

void PicturaMainWindow::removeColumnIfEmpty(PanelColumn* column)
{
    if (panels_) {
        panels_->removeColumnIfEmpty(column);
    }
}

PanelColumn* PicturaMainWindow::columnForPanel(const QString& objectName) const
{
    return panels_ ? panels_->columnForPanel(objectName) : nullptr;
}

PanelColumn* PicturaMainWindow::columnAtGlobal(const QPoint& globalPos) const
{
    return panels_ ? panels_->columnAtGlobal(globalPos) : nullptr;
}

int PicturaMainWindow::newColumnSideAt(const QPoint& globalPos) const
{
    return panels_ ? panels_->newColumnSideAt(globalPos) : -1;
}

int PicturaMainWindow::newColumnSideAtInsideWorkspace(const QPoint& globalPos) const
{
    return panels_ ? panels_->newColumnSideAtInsideWorkspace(globalPos) : -1;
}

PanelColumn* PicturaMainWindow::columnEdgeAnchorAt(const QPoint& globalPos,
                                                   const PanelColumn* exclude, int* side) const
{
    return panels_ ? panels_->columnEdgeAnchorAt(globalPos, exclude, side) : nullptr;
}

PanelColumn* PicturaMainWindow::resolveColumnMoveTarget(const QPoint& globalPos,
                                                        const PanelColumn* exclude,
                                                        int* side) const
{
    return panels_ ? panels_->resolveColumnMoveTarget(globalPos, exclude, side) : nullptr;
}

bool PicturaMainWindow::movePanelColumn(PanelColumn* column, int side, PanelColumn* anchor)
{
    return panels_ && panels_->movePanelColumn(column, side, anchor);
}

void PicturaMainWindow::beginWidgetColumnResize(PanelColumn* column, int globalX)
{
    if (panels_) {
        panels_->beginWidgetColumnResize(column, globalX);
    }
}

void PicturaMainWindow::updateWidgetColumnResize(int globalX)
{
    if (panels_) {
        panels_->updateWidgetColumnResize(globalX);
    }
}

void PicturaMainWindow::endWidgetColumnResize()
{
    if (panels_) {
        panels_->endWidgetColumnResize();
    }
}

void PicturaMainWindow::layoutWorkspaceFrame()
{
    if (panels_) {
        panels_->layoutWorkspaceFrame();
    }
}

void PicturaMainWindow::updateColumnFrameEdges()
{
    if (panels_) {
        panels_->updateColumnFrameEdges();
    }
}

void PicturaMainWindow::refreshPanels()
{
    if (panels_) {
        panels_->refreshPanels();
    }
}

int PicturaMainWindow::dockRetargetCountForTest() const
{
    return panels_ ? panels_->dockRetargetCount() : 0;
}

LayersPanel* PicturaMainWindow::layersPanelForTest() const
{
    return panels_ ? panels_->layersPanel() : nullptr;
}

void PicturaMainWindow::applyPanelSessionForTest(const SessionState& state)
{
    if (panels_) {
        panels_->applyPanelSession(state);
    }
}

void PicturaMainWindow::applyWorkspaceLayoutForTest(const QJsonArray& columns)
{
    if (panels_) {
        panels_->applyWorkspaceLayout(columns);
    }
}

bool PicturaMainWindow::createWorkspaceForTest(const QString& name)
{
    return panels_ && panels_->createWorkspace(name);
}

bool PicturaMainWindow::deleteWorkspaceForTest(const QString& name)
{
    return panels_ && panels_->deleteWorkspace(name);
}

QString PicturaMainWindow::workspaceNameErrorForTest(const QString& name) const
{
    return panels_ ? panels_->workspaceNameError(name) : QString();
}

void PicturaMainWindow::switchWorkspaceForTest(const QString& name)
{
    if (panels_) {
        panels_->switchWorkspace(name);
    }
}

QString PicturaMainWindow::activeWorkspaceForTest() const
{
    return panels_ ? panels_->activeWorkspace() : QString();
}

QStringList PicturaMainWindow::userWorkspacesForTest() const
{
    return panels_ ? panels_->userWorkspaces() : QStringList{};
}

QJsonArray PicturaMainWindow::workspaceLayoutForTest() const
{
    return panels_ ? panels_->workspaceLayout() : QJsonArray{};
}

void PicturaMainWindow::refreshWorkspaceMenuForTest()
{
    if (panels_) {
        panels_->refreshWorkspaceMenu(panels_->workspaceMenu());
    }
}

QStringList PicturaMainWindow::workspaceMenuLabelsForTest() const
{
    return panels_ ? panels_->workspaceMenuLabels() : QStringList{};
}

int PicturaMainWindow::workspaceMenuSeparatorCountForTest() const
{
    return panels_ ? panels_->workspaceMenuSeparatorCount() : 0;
}

QStringList PicturaMainWindow::deleteWorkspaceChoicesForTest() const
{
    return panels_ ? panels_->deleteWorkspaceChoices() : QStringList{};
}

PanelColumn* PicturaMainWindow::restoreLegacyDefaultForTest()
{
    return panels_ ? panels_->restoreLegacyDefaultForTest() : nullptr;
}

void PicturaMainWindow::applyGpuComputePreference(bool on)
{
    // Re-enabling is the user's explicit reset: clear a cached crash so the
    // next launch retries the candidates.
    if (on && !gpuCompute_) {
        clearGpuBackendCache();
    }
    gpuCompute_ = on;
    for (const DocEntry* entry : documents_->docs()) {
        if (entry->view) {
            entry->view->set_gpu_compute(on);
        }
    }
    saveSession();
    commandRegistry()->refresh();
    refresh();
}

void PicturaMainWindow::saveSession()
{
    // A session restore rebuilds the columns from the store; re-saving mid
    // rebuild would persist a partial layout.
    if (!panels_ || panels_->isRestoringSession()) {
        return;
    }
    SessionState state = pictura::loadSession();
    auto prefs = preferencesBaseline_;
    state.layout = saveState();
    state.layoutRevision = kLayoutRevision;
    state.windowGeometry = saveGeometry();
    state.windowMaximized = isMaximized();
    prefs.brightnessLevel = brightnessLevel_;
    prefs.themeFamily = themeFamily_ == 1 ? 1 : 0;
    prefs.uiScalePercent = uiScalePercent_;
    prefs.gpuCompute = gpuCompute_;
    prefs.memoryUsagePercent = memory_usage_percent();
    prefs.scratchVolumes = scratch_volumes().split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    prefs.colorPolicy = colorPolicy_;
    prefs.cmykPolicy = cmykPolicy_;
    prefs.workingCmyk = workingCmyk_;
    prefs.renderIntent = renderIntent_;
    prefs.blackPointCompensation = blackPointCompensation_;
    prefs.useDither = useDither_;
    prefs.askMismatchOpen = askMismatchOpen_;
    prefs.askMissingOpen = askMissingOpen_;
    prefs.pickerChoice = pickerChoice_;
    prefs.textGammaEnabled = textGammaEnabled_;
    prefs.textGamma = textGamma_;
    state.toolsColumns = toolbox_ ? toolbox_->columns() : 1;
    prefs.useShiftKeyForToolSwitch = useShiftKeyForToolSwitch_;
    prefs.openDocumentsAsTabs = openDocumentsAsTabs_;
    prefs.enableFloatingDocumentDocking = floatingDocumentDocking_;
    prefs.zoomResizesWindows = zoomResizesWindows_;
    prefs.typePlaceholderText = tools_ ? tools_->typePlaceholderText() : true;
    prefs.typeShowAsianOptions = typeShowAsianOptions_;
    prefs.typeMiddleEastern = typeMiddleEastern_;
    prefs.fontPreviewSize = fontPreviewSize_;
    prefs.typeDefaults = typeDefaultsJson_;
    prefs.typeMissingGlyphProtection = typeMissingGlyphProtection_;
    prefs.shortcuts.clear();
    const QHash<QString, QList<QKeySequence>> overrides = commandRegistry()->shortcutOverrides();
    const QSet<QString> shadowed =
        panels_->actionsPanel() ? panels_->actionsPanel()->shadowedCommandIds()
                                : QSet<QString>();
    for (auto it = overrides.cbegin(); it != overrides.cend(); ++it) {
        if (shadowed.contains(it.key())) {
            continue;
        }
        QStringList bindings;
        for (const QKeySequence& binding : it.value()) {
            bindings << binding.toString(QKeySequence::PortableText);
        }
        prefs.shortcuts.insert(it.key(), bindings);
    }
    prefs.toolShortcuts.clear();
    const QMap<ToolId, QChar> toolOverrides = toolShortcutOverrides();
    for (auto it = toolOverrides.cbegin(); it != toolOverrides.cend(); ++it) {
        prefs.toolShortcuts.insert(toolIdName(it.key()), it.value().isNull() ? QString()
                                                                             : QString(it.value()));
    }
    state.rulersVisible = rulersVisible_;
    prefs.rulerUnit = int(rulerUnit_);
    prefs.traditionalPoints = traditionalPoints_;
    state.guidesVisible = guidesVisible_;
    state.guidesLocked = guidesLocked_;
    state.smartGuidesVisible = smartGuidesVisible_;
    state.selectionEdges = selectionEdges_;
    state.showTargetPath = showTargetPath_;
    state.showLayerEdges = showLayerEdges_;
    state.showGrid = showGrid_;
    state.showCount = showCount_;
    state.showSlices = showSlices_;
    state.showNotes = showNotes_;
    state.showPixelGrid = showPixelGrid_;
    state.showColorSamplers = showColorSamplers_;
    state.extras = extras_;
    state.extrasMembersSet = true;
    state.extrasMembers = extrasMembers_;
    state.lockSlices = lockSlices_;
    state.gridColor = gridColor_;
    state.gridStyle = gridStyle_;
    state.gridUnit = gridUnit_;
    state.gridEvery = gridEvery_;
    state.gridSubdivisions = gridSubdivisions_;
    state.freePan = freePan_;
    state.snapCanvasToWorkspace = snapCanvasToWorkspace_;
    state.quickMaskSelected = quickMaskUi_.selected;
    state.quickMaskColor = quickMaskUi_.color.name();
    state.quickMaskOpacity = quickMaskUi_.opacity;
    prefs.guideColor = guideColor_.name();
    prefs.guideDashed = guideDashed_;
    state.proofDevice = proofDevice_;
    state.proofIntent = proofIntent_;
    state.proofBpc = proofBpc_;
    state.proofSimulatePaper = proofSimulatePaper_;
    state.proofSimulateBlack = proofSimulateBlack_;
    state.proofColors = proofColors_;
    if (tools_ && tools_->optionsFor<CropOptions>(ToolId::Crop)) {
        const CropOptions* crop = tools_->optionsFor<CropOptions>(ToolId::Crop);
        state.cropClassicMode = crop->classicMode();
        state.cropDeletePixels = crop->deletePixels();
        state.cropRatio = crop->ratio();
        state.cropGridOverlay = crop->gridOverlay();
    }
    if (tools_) {
        state.toolOptions = toolOptionsToJson(*tools_);
    }
    panels_->saveState(state);
    importSave_->saveState(state, prefs);
    state.schemaVersion = 12;
    if (auto* checker = findChild<UpdateChecker*>(QStringLiteral("updateChecker"))) {
        prefs.checkForUpdatesOnLaunch = checker->settings().automaticChecksEnabled;
    }
    if (savePreferences(preferencesBaseline_, prefs)) {
        preferencesBaseline_ = std::move(prefs);
    }
    // A refused write means another instance wrote the store after this one
    // launched; leave its workspace snapshot alone too.
    if (pictura::saveSession(state)) {
        panels_->snapshotActiveWorkspace();
    }
}

void PicturaMainWindow::restoreSessionWindowGeometry(const SessionState& state)
{
    // A stored geometry (from the last close) overrides the default size; a
    // stale one that lands fully off-screen (a removed monitor) falls back to
    // the default so the window is never unreachable.
    if (!state.windowGeometry.isEmpty() && restoreGeometry(state.windowGeometry)) {
        bool onScreen = false;
        for (const QScreen* screen : QGuiApplication::screens()) {
            if (screen->availableGeometry().intersects(frameGeometry())) {
                onScreen = true;
                break;
            }
        }
        if (!onScreen) {
            resize(1100, 700);
        }
    }
    if (state.windowMaximized) {
        setWindowState(windowState() | Qt::WindowMaximized);
    }
}

void PicturaMainWindow::onCanvasZoomChanged()
{
    updateStatus();
    if (const DocWindow* window = windowOfCanvas(qobject_cast<ImageView*>(sender()))) {
        updateTabTitle(indexOfDoc(window->doc));
    }
}

QString PicturaMainWindow::documentTabTextForTest(int index) const
{
    if (index < 0 || index >= documents_->docs().size() || documents_->docs().at(index)->windows.isEmpty()) {
        return QString();
    }
    const int tab = tabOf(documents_->docs().at(index)->windows.first());
    return tab >= 0 ? documents_->tabs()->tabText(tab) : QString();
}

bool PicturaMainWindow::restoreStoredLayout(const QByteArray& layout, int revision)
{
    if (layout.isEmpty() || revision != kLayoutRevision) {
        return false;
    }
    return restoreState(layout);
}

} // namespace pictura
