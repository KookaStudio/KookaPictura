#include "panel_host.h"

#include "api/filters_bridge.h"
#include "commands.h"
#include "document_registry.h"
#include "panels/adjustments_panel.h"
#include "panels/actions_engine_bridge.h"
#include "panels/actions_panel.h"
#include "panels/brush_panel.h"
#include "panels/channels_panel.h"
#include "panels/character_panel.h"
#include "panels/character_styles_panel.h"
#include "panels/clone_source_panel.h"
#include "panels/color_panel.h"
#include "panels/glyphs_panel.h"
#include "panels/histogram_panel.h"
#include "panels/history_panel.h"
#include "panels/info_panel.h"
#include "panels/layer_comps_panel.h"
#include "panels/layers_panel.h"
#include "panels/navigator_panel.h"
#include "panels/notes_panel.h"
#include "panels/panel_group.h"
#include "panels/paragraph_panel.h"
#include "panels/paragraph_styles_panel.h"
#include "panels/paths_panel.h"
#include "panels/placeholder_panel.h"
#include "panels/properties_panel.h"
#include "panels/swatches_panel.h"
#include "pictura_debug_timing.h"
#include "session.h"
#include "type_fonts.h"
#include "workspace_store.h"

#include <QtCore/QRect>
#include <QtGui/QResizeEvent>
#include <QtWidgets/QDockWidget>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QStyle>
#include <QtCore/QTimer>

namespace pictura {
namespace {

// The document pane keeps a minimum width even with no document open, so the
// widget columns can never absorb the whole workspace (and the splitter keeps a
// grabbable handle on each side of it). ponytail: chosen, not a sourced CS6
// metric.
constexpr int kWorkspaceMinWidth = 160;
constexpr int kWorkspaceFrameWidth = 3;

} // namespace

QJsonArray workspacePresets(const QString& name);

PanelHost::PanelHost(QWidget* shell, DocumentRegistry* documents, CommandRegistry* commands,
                     ColorState* colorState, ActionsEngineBridge* actionsEngine, Hooks hooks,
                     QObject* parent)
    : QObject(parent)
    , shell_(shell)
    , documents_(documents)
    , commands_(commands)
    , colorState_(colorState)
    , actionsEngine_(actionsEngine)
    , hooks_(std::move(hooks))
{
    if (!colorState_) {
        colorState_ = new ColorState(this);
    }

    panelColumn_ = new PanelColumn(shell_);
    panelColumn_->setHost(this);
    // The splitter itself is the central widget, so drop-target geometry keeps
    // its existing meaning. It reserves the 3 px band on each side; 0-width
    // handles leave no resize seam between panes.
    centerSplitter_ = new QSplitter(Qt::Horizontal, shell_);
    centerSplitter_->setObjectName(QStringLiteral("centerSplitter"));
    centerSplitter_->setHandleWidth(0);
    centerSplitter_->setContentsMargins(kWorkspaceFrameWidth, 0, kWorkspaceFrameWidth, 0);
    if (documents_ && documents_->documentArea()) {
        centerSplitter_->addWidget(documents_->documentArea());
    }
    centerSplitter_->addWidget(panelColumn_);
    if (documents_ && documents_->documentArea()) {
        documents_->documentArea()->setMinimumWidth(kWorkspaceMinWidth);
    }
    centerSplitter_->setStretchFactor(0, 1);
    centerSplitter_->setStretchFactor(1, 0);
    centerSplitter_->setChildrenCollapsible(false);
    if (auto* window = qobject_cast<QMainWindow*>(shell_)) {
        window->setCentralWidget(centerSplitter_);
        window->setDockOptions(QMainWindow::AnimatedDocks | QMainWindow::AllowTabbedDocks);
    }
    buildFrameLines();
    wirePanelColumn(panelColumn_);

    // Expensive panel work (layer thumbnails, histogram) is coalesced off the
    // paint-dab/commit path so the canvas can repaint first.
    panelRefreshTimer_ = new QTimer(this);
    panelRefreshTimer_->setSingleShot(true);
    panelRefreshTimer_->setInterval(120);
    connect(panelRefreshTimer_, &QTimer::timeout, this, &PanelHost::refreshPanels);

    // A splitter drag has no other save trigger; coalesce the column-width
    // changes into one write instead of saving on every pixel of the drag.
    sessionSaveTimer_ = new QTimer(this);
    sessionSaveTimer_->setSingleShot(true);
    sessionSaveTimer_->setInterval(400);
    connect(sessionSaveTimer_, &QTimer::timeout, this, [this]() { requestSessionSave(); });
    connect(centerSplitter_, &QSplitter::splitterMoved, this,
            [this](int pos, int index) { onCenterSplitterMoved(pos, index); });

    if (documents_) {
        connect(documents_, &DocumentRegistry::aboutToRemoveDocument, this, [this](int index) {
            DocEntry* entry = documents_->docAt(index);
            if (entry && entry->view && entry->view == dockView_) {
                dockView_ = nullptr;
                dockCanvas_ = nullptr;
                dockGeneration_ = 0;
            }
        });
    }
}

PanelHost::~PanelHost() = default;

void PanelHost::buildFrameLines()
{
    // The two 1 px inner frame lines span the whole central band (options bar
    // included); the outer 2 px is the window `${surface}` showing through the
    // reserved margin. They are raised above the chrome in layoutWorkspaceFrame.
    workspaceFrameLeft_ = new QWidget(shell_);
    workspaceFrameRight_ = new QWidget(shell_);
    for (QWidget* line : {workspaceFrameLeft_, workspaceFrameRight_}) {
        line->setObjectName(QStringLiteral("workspaceFrameLine"));
        line->setAttribute(Qt::WA_StyledBackground, true);
        line->setAttribute(Qt::WA_TransparentForMouseEvents, true);
    }
}

QWidget* PanelHost::workspaceWidget() const
{
    return centerSplitter_;
}

QList<QWidget*> PanelHost::panelWidgets() const
{
    return {layersPanel_,   historyPanel_,       navigatorPanel_,     colorPanel_,
            swatchesPanel_, infoPanel_,          histogramPanel_,     gradientsPanel_,
            patternsPanel_, notesPanel_,         propertiesPanel_,    adjustmentsPanel_,
            channelsPanel_, pathsPanel_,         actionsPanel_,       stylesPanel_,
            brushPanel_,    cloneSourcePanel_,   characterPanel_,     paragraphPanel_,
            glyphsPanel_,   paragraphStylesPanel_, characterStylesPanel_};
}

bool PanelHost::registerPanel(QWidget* panel, Qt::DockWidgetArea area)
{
    if (!panel || panelNames_.contains(panel->objectName())) {
        return false;
    }
    panelNames_.insert(panel->objectName());
    if (auto* dock = qobject_cast<QDockWidget*>(panel)) {
        if (auto* window = qobject_cast<QMainWindow*>(shell_)) {
            window->addDockWidget(area, dock);
        }
    }
    return true;
}

PanelColumn* PanelHost::createToolsColumn()
{
    if (toolsColumn_) {
        return toolsColumn_;
    }
    toolsColumn_ = new PanelColumn(shell_);
    toolsColumn_->setHost(this);
    toolsColumn_->setObjectName(QStringLiteral("toolsColumn"));
    wirePanelColumn(toolsColumn_);
    centerSplitter_->insertWidget(0, toolsColumn_);
    connect(toolsColumn_, &PanelColumn::closeRequested, this, [this]() {
        if (!toolsColumn_) {
            return;
        }
        // Closing a floating toolbar must also tear its overlay down, or an
        // empty grey window is left behind. Re-dock it first so the Window
        // menu restores it as a splitter pane.
        if (toolsColumn_->isColumnFloating()) {
            const PanelSide side = toolsColumn_->side();
            toolsColumn_->cancelColumnFloat();
            if (centerSplitter_->indexOf(toolsColumn_) < 0) {
                centerSplitter_->insertWidget(
                    side == PanelSide::Left ? 0 : centerSplitter_->count(), toolsColumn_);
                reapplyColumnStretch();
            }
        }
        toolsColumn_->hide();
        reapplyColumnStretch();
        centerSplitter_->update();
        if (commands_) {
            commands_->refresh();
        }
        requestSessionSave();
    });
    return toolsColumn_;
}

PanelColumn* PanelHost::adoptToolsColumn(PanelColumn* column)
{
    if (!column) {
        return toolsColumn_;
    }
    toolsColumn_ = column;
    toolsColumn_->setHost(this);
    wirePanelColumn(toolsColumn_);
    if (centerSplitter_->indexOf(toolsColumn_) < 0) {
        centerSplitter_->insertWidget(0, toolsColumn_);
    }
    return toolsColumn_;
}

void PanelHost::requestSessionSave()
{
    if (buildingPanels_ || restoringPanelSession_) {
        return;
    }
    emit layoutChanged();
    if (hooks_.saveSession) {
        hooks_.saveSession();
    }
}

void PanelHost::wirePanelColumn(PanelColumn* column)
{
    if (!column) {
        return;
    }
    connect(column, &PanelColumn::stateChanged, this, [this]() { requestSessionSave(); });
    connect(column, &PanelColumn::interfaceOptionsRequested, this, [this]() {
        if (hooks_.interfaceOptionsRequested) {
            hooks_.interfaceOptionsRequested();
        }
    });
}

void PanelHost::reapplyColumnStretch()
{
    if (!centerSplitter_ || !documents_ || !documents_->documentArea()) {
        return;
    }
    for (int i = 0; i < centerSplitter_->count(); ++i) {
        centerSplitter_->setStretchFactor(
            i, centerSplitter_->widget(i) == documents_->documentArea() ? 1 : 0);
    }
    // The visible width drags come from the widget columns' own 4 px grips on
    // their workspace-facing edge; the splitter's handles are kept inert so a
    // column edge exposes exactly one resize affordance.
    for (int i = 0; i + 1 < centerSplitter_->count(); ++i) {
        if (QSplitterHandle* handle = centerSplitter_->handle(i)) {
            handle->setCursor(Qt::ArrowCursor);
            handle->setAttribute(Qt::WA_TransparentForMouseEvents, true);
        }
    }
    for (int i = 0; i < centerSplitter_->count(); ++i) {
        if (auto* column = qobject_cast<PanelColumn*>(centerSplitter_->widget(i))) {
            column->updateResizeGrip();
        }
    }
}

void PanelHost::onCenterSplitterMoved(int, int)
{
    // A divider between two widget columns resizes only the outer column;
    // restore the workspace-side column and let the workspace absorb the change.
    if (!adjustingColumns_ && resizeAnchorColumn_ && resizeAnchorWidth_ > 0
        && centerSplitter_ && documents_ && documents_->documentArea()) {
        const int ai = centerSplitter_->indexOf(resizeAnchorColumn_);
        const int ti = centerSplitter_->indexOf(documents_->documentArea());
        if (ai >= 0 && ti >= 0 && ai != ti) {
            QList<int> sizes = centerSplitter_->sizes();
            if (ai < sizes.size() && ti < sizes.size()) {
                adjustingColumns_ = true;
                const int delta = sizes.at(ai) - resizeAnchorWidth_;
                sizes[ai] = resizeAnchorWidth_;
                sizes[ti] += delta;
                centerSplitter_->setSizes(sizes);
                adjustingColumns_ = false;
            }
        }
    }
    sessionSaveTimer_->start();
}

void PanelHost::beginWidgetColumnResize(PanelColumn* column, int globalX)
{
    resizeHandleIndex_ = -1;
    resizeAnchorColumn_ = nullptr;
    resizeAnchorWidth_ = -1;
    if (!centerSplitter_ || !documents_ || !documents_->documentArea() || !column) {
        return;
    }
    const int k = centerSplitter_->indexOf(column);
    if (k < 0) {
        return;
    }
    // The workspace-facing edge: a Left column's right side is handle k, a Right
    // column's left side is handle k-1.
    const int handleIndex = sideOf(column) == PanelSide::Left ? k : k - 1;
    if (handleIndex < 0 || handleIndex + 1 >= centerSplitter_->count()) {
        return;
    }
    QSplitterHandle* handle = centerSplitter_->handle(handleIndex);
    if (!handle) {
        return;
    }
    resizeHandleIndex_ = handleIndex;
    resizeStartGlobalX_ = globalX;
    resizeStartSizes_ = centerSplitter_->sizes();
    auto* left = qobject_cast<PanelColumn*>(centerSplitter_->widget(handleIndex));
    auto* right = qobject_cast<PanelColumn*>(centerSplitter_->widget(handleIndex + 1));
    if (left && right) {
        const int areaIndex = centerSplitter_->indexOf(documents_->documentArea());
        PanelColumn* anchor = handleIndex < areaIndex ? right : left;
        resizeAnchorColumn_ = anchor;
        resizeAnchorWidth_ = anchor->width();
    }
}

void PanelHost::updateWidgetColumnResize(int globalX)
{
    if (resizeHandleIndex_ < 0 || !centerSplitter_) {
        return;
    }
    const int handle = resizeHandleIndex_;
    if (handle + 1 >= resizeStartSizes_.size()) {
        return;
    }
    // moveSplitter is protected, so move the handle by trading width between the
    // two panes it separates, from the press-time sizes.
    const int delta = globalX - resizeStartGlobalX_;
    QList<int> sizes = resizeStartSizes_;
    sizes[handle] += delta;
    sizes[handle + 1] -= delta;
    centerSplitter_->setSizes(sizes);
}

void PanelHost::endWidgetColumnResize()
{
    resizeHandleIndex_ = -1;
    resizeStartSizes_.clear();
    resizeAnchorColumn_ = nullptr;
    resizeAnchorWidth_ = -1;
}

void PanelHost::handleShellResize()
{
    layoutWorkspaceFrame();
    for (PanelColumn* column : allPanelColumns()) {
        if (column) {
            column->reclampFloats();
            column->reclampFlyout();
        }
    }
}

void PanelHost::layoutWorkspaceFrame()
{
    if (!workspaceFrameLeft_ || !workspaceFrameRight_) {
        return;
    }
    auto* window = qobject_cast<QMainWindow*>(shell_);
    int top = 0;
    if (window && window->menuBar() && window->menuBar()->isVisible()) {
        top = window->menuBar()->geometry().bottom() + 1;
    }
    int bottom = shell_->height();
    if (window && window->statusBar() && window->statusBar()->isVisible()) {
        bottom = window->statusBar()->geometry().top();
    }
    const int lineHeight = qMax(0, bottom - top);
    workspaceFrameLeft_->setGeometry(kWorkspaceFrameWidth - 1, top, 1, lineHeight);
    workspaceFrameRight_->setGeometry(qMax(0, shell_->width() - kWorkspaceFrameWidth), top, 1,
                                      lineHeight);
    workspaceFrameLeft_->raise();
    workspaceFrameRight_->raise();
    updateColumnFrameEdges();
}

void PanelHost::updateColumnFrameEdges()
{
    if (!centerSplitter_) {
        return;
    }
    const int count = centerSplitter_->count();
    for (int i = 0; i < count; ++i) {
        auto* column = qobject_cast<PanelColumn*>(centerSplitter_->widget(i));
        if (!column) {
            continue;
        }
        const QString edge = i == 0 ? QStringLiteral("left")
                             : i == count - 1 ? QStringLiteral("right")
                                              : QString();
        column->setBorderEdgeSuppressed(edge == QStringLiteral("left"),
                                        edge == QStringLiteral("right"));
        if (column->property("frameEdge").toString() == edge) {
            continue;
        }
        column->setProperty("frameEdge", edge);
        column->style()->unpolish(column);
        column->style()->polish(column);
    }
}

QList<PanelColumn*> PanelHost::panelColumns() const
{
    QList<PanelColumn*> out;
    if (!centerSplitter_) {
        return out;
    }
    for (int i = 0; i < centerSplitter_->count(); ++i) {
        if (auto* column = qobject_cast<PanelColumn*>(centerSplitter_->widget(i))) {
            out << column;
        }
    }
    return out;
}

QList<PanelColumn*> PanelHost::allPanelColumns() const
{
    QList<PanelColumn*> out = panelColumns();
    const QList<PanelFloat*> overlays =
        shell_->findChildren<PanelFloat*>(Qt::FindDirectChildrenOnly);
    for (PanelFloat* overlay : overlays) {
        auto* column = overlay ? qobject_cast<PanelColumn*>(overlay->content()) : nullptr;
        if (column && !out.contains(column)) {
            out << column;
        }
    }
    return out;
}

int PanelHost::columnCount() const
{
    return panelColumns().size();
}

PanelSide PanelHost::sideOf(const PanelColumn* column) const
{
    if (!column || !centerSplitter_ || !documents_ || !documents_->documentArea()) {
        return PanelSide::Right;
    }
    const int areaIndex = centerSplitter_->indexOf(documents_->documentArea());
    const int index = centerSplitter_->indexOf(const_cast<PanelColumn*>(column));
    return index >= 0 && index < areaIndex ? PanelSide::Left : PanelSide::Right;
}

PanelColumn* PanelHost::createPanelColumn(PanelSide side, PanelColumn* anchor)
{
    if (!centerSplitter_) {
        return nullptr;
    }
    auto* column = new PanelColumn(shell_);
    column->setHost(this);
    wirePanelColumn(column);
    int insertAt;
    const int anchorIndex = anchor ? centerSplitter_->indexOf(anchor) : -1;
    if (anchorIndex >= 0) {
        insertAt = side == PanelSide::Left ? anchorIndex : anchorIndex + 1;
    } else {
        insertAt = side == PanelSide::Left ? 0 : centerSplitter_->count();
    }
    centerSplitter_->insertWidget(insertAt, column);
    reapplyColumnStretch();
    column->setVisible(!panelsHidden_);
    refreshSharedFloor();
    return column;
}

void PanelHost::removeColumnIfEmpty(PanelColumn* column)
{
    if (!column) {
        return;
    }
    // D1: the tools column hosts one plain content child and is never empty.
    if (column->isToolsColumn()) {
        return;
    }
    // A whole column torn off into an in-window overlay is not a splitter pane
    // but is still owned by this host; never delete it out from under its float.
    if (column->isColumnFloating()) {
        return;
    }
    for (PanelGroup* group : column->groups()) {
        if (group && !group->visibleTitles().isEmpty()) {
            return;
        }
    }
    // The primary column keeps its identity but hides when empty; a hidden
    // splitter child takes no space and `PanelColumn::showPanel` re-shows it.
    if (column == panelColumn_) {
        column->closeFlyout();
        column->hide();
        refreshSharedFloor();
        requestSessionSave();
        return;
    }
    // A host-parented flyout must not survive its owning column.
    column->closeFlyout();
    column->rehomeFloatsTo(panelColumn_);
    const QList<PanelGroup*> remaining = column->groups();
    for (PanelGroup* group : remaining) {
        if (!group) {
            continue;
        }
        if (PanelGroup* taken = column->takeGroup(group)) {
            panelColumn_->adoptGroup(taken);
        }
    }
    column->hide();
    column->setParent(nullptr);
    refreshSharedFloor();
    column->deleteLater();
    reapplyColumnStretch();
    requestSessionSave();
}

PanelColumn* PanelHost::columnForPanel(const QString& objectName) const
{
    for (PanelColumn* column : allPanelColumns()) {
        if (column->groupForPanel(objectName) || column->floatForPanel(objectName)) {
            return column;
        }
    }
    return nullptr;
}

PanelColumn* PanelHost::columnAtGlobal(const QPoint& globalPos) const
{
    for (PanelColumn* column : allPanelColumns()) {
        if (column->isVisible()
            && QRect(column->mapToGlobal(QPoint(0, 0)), column->size()).contains(globalPos)) {
            return column;
        }
    }
    return nullptr;
}

int PanelHost::newColumnSideAt(const QPoint& globalPos) const
{
    QWidget* central = workspaceWidget();
    if (!central) {
        return -1;
    }
    // ponytail: chosen constant band at the central area's outer edges, not a
    // sourced CS6 metric. A screenshot can retune it.
    constexpr int kNewColumnMargin = 28;
    const int left = central->mapToGlobal(QPoint(0, 0)).x();
    const int right = left + central->width();
    if (globalPos.x() < left + kNewColumnMargin) {
        return 0;
    }
    if (globalPos.x() > right - kNewColumnMargin) {
        return 1;
    }
    return -1;
}

int PanelHost::newColumnSideAtInsideWorkspace(const QPoint& globalPos) const
{
    QWidget* central = workspaceWidget();
    if (!central) {
        return -1;
    }
    const QRect workspace(central->mapToGlobal(QPoint(0, 0)), central->size());
    if (!workspace.contains(globalPos)) {
        return -1;
    }
    return newColumnSideAt(globalPos);
}

PanelColumn* PanelHost::columnEdgeAnchorAt(const QPoint& globalPos, const PanelColumn* exclude,
                                           int* side) const
{
    if (side) {
        *side = -1;
    }
    // ponytail: chosen proximity band beside a column, not a sourced CS6 metric.
    constexpr int kEdgeBand = 26;
    constexpr int kEdgeInside = 0;
    for (PanelColumn* column : panelColumns()) {
        if (!column || column == exclude || !column->isVisible()) {
            continue;
        }
        const QRect r(column->mapToGlobal(QPoint(0, 0)), column->size());
        if (!r.isValid() || r.width() <= kEdgeBand + kEdgeInside) {
            continue;
        }
        if (globalPos.y() < r.top() || globalPos.y() > r.bottom()) {
            continue;
        }
        if (globalPos.x() >= r.left() - kEdgeBand && globalPos.x() <= r.left() + kEdgeInside) {
            if (side) {
                *side = 0;
            }
            return column;
        }
        if (globalPos.x() >= r.right() - kEdgeInside && globalPos.x() <= r.right() + kEdgeBand) {
            if (side) {
                *side = 1;
            }
            return column;
        }
    }
    return nullptr;
}

PanelColumn* PanelHost::resolveColumnMoveTarget(const QPoint& globalPos,
                                                const PanelColumn* exclude, int* side) const
{
    if (side) {
        *side = -1;
    }
    int edgeSide = -1;
    if (PanelColumn* anchor = columnEdgeAnchorAt(globalPos, exclude, &edgeSide)) {
        if (side) {
            *side = edgeSide;
        }
        return anchor;
    }
    for (PanelColumn* column : allPanelColumns()) {
        if (!column || column == exclude || centerSplitter_->indexOf(column) < 0) {
            continue;
        }
        const QRect r(column->mapToGlobal(QPoint(0, 0)), column->size());
        if (!r.isValid() || !r.contains(globalPos)) {
            continue;
        }
        if (side) {
            *side = globalPos.x() >= r.center().x() ? 1 : 0;
        }
        return column;
    }
    const int wsSide = newColumnSideAt(globalPos);
    if (wsSide >= 0) {
        if (side) {
            *side = wsSide;
        }
    }
    return nullptr;
}

bool PanelHost::movePanelColumn(PanelColumn* column, int side, PanelColumn* anchor)
{
    if (!column || !centerSplitter_) {
        return false;
    }
    // `side` is 0 (left) or 1 (right); -1 means the drag resolved no target, so
    // leave the column where it is rather than snapping it to an edge.
    if (side != 0 && side != 1) {
        return false;
    }
    const int from = centerSplitter_->indexOf(column);
    const int count = centerSplitter_->count();
    int target = -1;
    if (anchor && anchor != column) {
        const int anchorIndex = centerSplitter_->indexOf(anchor);
        if (anchorIndex < 0) {
            return false;
        }
        target = side == 0 ? anchorIndex : anchorIndex + 1;
    } else {
        target = side == 0 ? 0 : count;
    }
    if (from >= 0) {
        if (from < target) {
            --target;
        }
        target = qBound(0, target, count - 1);
        if (target == from) {
            return false;
        }
    } else {
        target = qBound(0, target, count);
    }
    column->setParent(nullptr);
    centerSplitter_->insertWidget(target, column);
    column->setVisible(!panelsHidden_);
    reapplyColumnStretch();
    refreshSharedFloor();
    requestSessionSave();
    return true;
}

void PanelHost::clearDynamicColumns()
{
    // Move every drop-created column's groups back into the primary column and
    // delete the column, without the `removeColumnIfEmpty` save side effect.
    const QList<PanelColumn*> columns = panelColumns();
    bool removedAny = false;
    for (PanelColumn* column : columns) {
        if (!column || column == panelColumn_ || column->isToolsColumn()) {
            continue;
        }
        column->closeFlyout();
        const QList<PanelGroup*> groups = column->groups();
        for (PanelGroup* group : groups) {
            if (!group) {
                continue;
            }
            if (PanelGroup* taken = column->takeGroup(group)) {
                panelColumn_->addGroup(taken);
            }
        }
        column->rehomeFloatsTo(panelColumn_);
        column->hide();
        column->setParent(nullptr);
        column->deleteLater();
        removedAny = true;
    }
    reapplyColumnStretch();
    if (removedAny) {
        refreshSharedFloor();
    }
}

void PanelHost::refreshSharedFloor()
{
    PanelColumn::refreshSharedFloor(this);
}

void PanelHost::setPanelsHidden(bool hidden)
{
    panelsHidden_ = hidden;
    if (hidden) {
        // Host-parented flyouts are not panel-column children; hide them with
        // the rest of the panels.
        for (PanelColumn* column : allPanelColumns()) {
            if (column) {
                column->closeFlyout();
            }
        }
    }
    setPanelColumnsVisible(!hidden);
    emit panelVisibilityChanged();
}

void PanelHost::setPanelColumnsVisible(bool visible)
{
    for (PanelColumn* column : panelColumns()) {
        column->setVisible(visible);
    }
}

void PanelHost::togglePanel(const QString& objectName)
{
    PanelColumn* owner = columnForPanel(objectName);
    if (!owner) {
        owner = panelColumn_;
    }
    if (owner) {
        owner->showPanel(objectName, !owner->isPanelVisible(objectName));
    }
}

void PanelHost::schedulePanelRefresh()
{
    if (panelRefreshTimer_) {
        panelRefreshTimer_->start();
    }
}

void PanelHost::stopPanelRefreshTimer()
{
    if (panelRefreshTimer_) {
        panelRefreshTimer_->stop();
    }
}

void PanelHost::retargetDock()
{
    pictura::ScopedTimer dockTimer("cxx_retargetDock (all panels)");
    PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
    ImageView* canvas = hooks_.activeCanvas ? hooks_.activeCanvas() : nullptr;

    if (layersPanel_) {
        pictura::ScopedTimer t("cxx_layersPanel_refresh");
        layersPanel_->setView(view);
    }
    if (historyPanel_) {
        pictura::ScopedTimer t("cxx_historyPanel_refresh");
        historyPanel_->setView(view);
    }
    if (navigatorPanel_) {
        pictura::ScopedTimer t("cxx_navigatorPanel_refresh");
        navigatorPanel_->setCanvas(canvas);
        navigatorPanel_->setView(view);
    }
    if (infoPanel_) {
        pictura::ScopedTimer t("cxx_infoPanel_refresh");
        infoPanel_->setView(view);
    }
    if (propertiesPanel_) {
        pictura::ScopedTimer t("cxx_propertiesPanel_refresh");
        propertiesPanel_->setView(view);
    }
    if (adjustmentsPanel_) {
        pictura::ScopedTimer t("cxx_adjustmentsPanel_refresh");
        adjustmentsPanel_->setView(view);
    }
    if (channelsPanel_) {
        pictura::ScopedTimer t("cxx_channelsPanel_refresh");
        channelsPanel_->setView(view);
    }
    if (characterPanel_) {
        pictura::ScopedTimer t("cxx_characterPanel_refresh");
        characterPanel_->setView(view);
    }
    if (paragraphPanel_) {
        pictura::ScopedTimer t("cxx_paragraphPanel_refresh");
        paragraphPanel_->setView(view);
    }
    if (paragraphStylesPanel_) {
        pictura::ScopedTimer t("cxx_paragraphStylesPanel_refresh");
        paragraphStylesPanel_->setView(view);
    }
    if (characterStylesPanel_) {
        pictura::ScopedTimer t("cxx_characterStylesPanel_refresh");
        characterStylesPanel_->setView(view);
    }
    if (notesPanel_) {
        pictura::ScopedTimer t("cxx_notesPanel_refresh");
        notesPanel_->setView(view);
    }
    // Color Range and Replace Color hold the same raw view; the frame cancels
    // them before the view is left behind.
    if (hooks_.cancelDocumentDialogs) {
        hooks_.cancelDocumentDialogs(view);
    }
    if (pathsPanel_) {
        pictura::ScopedTimer t("cxx_pathsPanel_refresh");
        pathsPanel_->setView(view);
    }
    if (layerCompsPanel_) {
        pictura::ScopedTimer t("cxx_layerCompsPanel_refresh");
        layerCompsPanel_->setView(view);
    }
    if (histogramPanel_) {
        pictura::ScopedTimer t("cxx_histogramPanel_refresh");
        histogramPanel_->setView(view);
    }

    dockView_ = view;
    dockCanvas_ = canvas;
    dockGeneration_ = view ? view->document_generation() : 0;
    panelsRefreshedByChanged_ = false;
    panelsFreshlyBound_ = true;
    ++dockRetargetCount_;
}

bool PanelHost::dockNeedsRebind() const
{
    PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
    ImageView* canvas = hooks_.activeCanvas ? hooks_.activeCanvas() : nullptr;
    return view != dockView_ || canvas != dockCanvas_
           || (view && view->document_generation() != dockGeneration_);
}

void PanelHost::refreshDock(bool includeSelfRefreshing, bool includeLayers)
{
    // Each panel is timed by name so a panel whose refresh scales with the
    // document shows up under PICTURA_PAINT_TIMING (#1151).
    const auto timed = [](const char* label, auto* panel) {
        if (panel) {
            pictura::ScopedTimer t(label);
            panel->refresh();
        }
    };
    if (includeSelfRefreshing) {
        if (includeLayers) {
            timed("cxx_layersPanel_refresh", layersPanel_);
        }
        timed("cxx_historyPanel_refresh", historyPanel_);
    }
    timed("cxx_navigatorPanel_refresh", navigatorPanel_);
    timed("cxx_infoPanel_refresh", infoPanel_);
    timed("cxx_propertiesPanel_refresh", propertiesPanel_);
    timed("cxx_adjustmentsPanel_refresh", adjustmentsPanel_);
    timed("cxx_channelsPanel_refresh", channelsPanel_);
    timed("cxx_characterPanel_refresh", characterPanel_);
    timed("cxx_paragraphPanel_refresh", paragraphPanel_);
    timed("cxx_paragraphStylesPanel_refresh", paragraphStylesPanel_);
    timed("cxx_characterStylesPanel_refresh", characterStylesPanel_);
    timed("cxx_notesPanel_refresh", notesPanel_);
    timed("cxx_pathsPanel_refresh", pathsPanel_);
    timed("cxx_layerCompsPanel_refresh", layerCompsPanel_);
    timed("cxx_histogramPanel_refresh", histogramPanel_);
}

void PanelHost::refreshPanels()
{
    pictura::ScopedTimer t("cxx_refreshPanels (debounced timer)");
    PictureView* view = hooks_.activeView ? hooks_.activeView() : nullptr;
    pictura::MetricsTimer<PictureView> metrics(view, "panels_refresh");
    // The per-script fallback scan opens every installed family; a document
    // without type never needs it (#1151). It runs before the early returns
    // below on purpose: it is once-only, and a held refresh lands soon after.
    if (view && type_api::hasTypeLayers(*view)) {
        registerTypeFallbacks();
    }
    // A filter commit holds the history on its worker; the commit's own
    // refresh restarts this timer once it lands.
    if (view && filter_commit_running(*view)) {
        return;
    }
    if (view && view->is_painting()) {
        return;
    }
    if (dockNeedsRebind()) {
        retargetDock();
    } else if (panelsFreshlyBound_) {
        panelsFreshlyBound_ = false;
    } else {
        const bool includeSelfRefreshing = !panelsRefreshedByChanged_;
        const bool includeLayers = !layersPanel_ || !layersPanel_->consumeVisibilityPatch();
        panelsRefreshedByChanged_ = false;
        refreshingDock_ = true;
        refreshDock(includeSelfRefreshing, includeLayers);
        refreshingDock_ = false;
    }
    if (registryRefreshPending_) {
        registryRefreshPending_ = false;
        if (commands_) {
            pictura::ScopedTimer registryTimer("cxx_registry_refresh");
            commands_->refresh();
        }
    }
}

} // namespace pictura

#include "moc_panel_host.cpp"
