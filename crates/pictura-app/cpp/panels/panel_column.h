#pragma once

#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QIcon>
#include <QtWidgets/QWidget>

#include <functional>

class QBoxLayout;
class QGraphicsOpacityEffect;
class QHideEvent;
class QLabel;
class QMenu;
class QResizeEvent;
class QScrollArea;
class QShowEvent;
class QSplitter;
class QTabBar;
class QToolButton;

namespace pictura {

class PanelColumn;
class PanelGroup;
class PicturaMainWindow;

// A panel column's side is a property of its order in the central splitter
// relative to the document tabs, never of its on-screen geometry (M43).
enum class PanelSide { Left, Right };

// A torn-off group in a frameless floating overlay. It hosts a real
// `PanelGroup` (same tabs, menu, minimize, iconic row) whose own tab-bar drag
// path routes back through the owning column, so it can be dragged back and
// re-docked. The hosting is platform-adaptive: where the platform lets a client
// position its own top-levels it is a frameless `Qt::Tool` parented to (transient
// for) the main window, with no title bar, no decorations, no taskbar entry, and
// it may move anywhere on the screen; otherwise (Wayland) it is an in-window
// child of the frame that follows the cursor and is clamped to the frame rect.
// The column deletes the overlay once its group is empty or re-docked.
class PanelFloat : public QWidget {
    Q_OBJECT

public:
    explicit PanelFloat(QWidget* parent = nullptr);
    // True when the overlay hosts as a positionable top-level window; false on
    // platforms (Wayland) that ignore a client's `move()` of a top-level, where
    // the overlay must be an in-window child instead. The test override forces
    // child mode so the branch can be exercised on a top-level platform.
    static bool overlayUsesTopLevel();
    static void setForceChildOverlayForTest(bool force);
    PanelGroup* group() const { return group_; }
    void setGroup(PanelGroup* group);
    // A whole torn-off `PanelColumn` is hosted through the same overlay: the
    // content setter takes either a group or a column, so the windowing
    // behaviour, size grip, and minimum width are shared by construction.
    void setContent(QWidget* content);
    QWidget* content() const { return content_; }
    QWidget* sizeGripForTest() const { return sizeGrip_; }
    // When false the grip is removed and the overlay takes the minimum its
    // content needs; the floating tools column uses this.
    void setResizable(bool on);
    bool resizableForTest() const { return resizable_; }
    // Keep the overlay sized to its hosted group. Collapsed-to-icons snaps
    // to the icon row's width and height; expanded drops the icon floor and
    // grows to the hint. A whole-column overlay snaps when its column is iconic.
    void syncToContent();
    // M47: dim the overlay while it hovers a valid drop target. A child widget
    // ignores `setWindowOpacity`, so one shared `QGraphicsOpacityEffect` carries
    // it and only its opacity changes.
    void setDragDimmed(bool dimmed);
    qreal dragOpacityForTest() const;
    // Phase 3: the shared `#2a7fff` insertion indicator, drawn over this float's
    // hosted group tab bar while a dragged panel/group hovers the overlay.
    void showTabIndicator(PanelGroup* group, int index);
    void hideTabIndicator();
    bool tabIndicatorVisibleForTest() const { return indicator_ && indicator_->isVisible(); }
    static constexpr int kFloatIconMinHeight = 36;
    static constexpr int kFloatMinWidth = 300;
    static constexpr int kFloatMinHeight = 48;
    std::function<void()> onClose;

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    PanelGroup* group_ = nullptr;
    QWidget* content_ = nullptr;
    QGraphicsOpacityEffect* opacityEffect_ = nullptr;
    QWidget* sizeGrip_ = nullptr;
    bool resizable_ = true;
    QWidget* indicator_ = nullptr;
};

// The frameless overlay that hosts the whole `PanelGroup` a compact-strip icon
// was clicked in (M45 C1), so docked, flyout, and floating present the same
// widget with no parity differences. It is persistent (not a `Qt::Popup`) and
// reports its own hide so the column can put the group back exactly once.
class PanelFlyout : public QWidget {
public:
    explicit PanelFlyout(QWidget* parent = nullptr);
    std::function<void()> onHidden;
    // Which canvas-facing edge carries the width grip: true for the left edge.
    void setResizeEdge(bool left);
    QWidget* resizeGripForTest() const { return resizeGrip_; }

protected:
    void hideEvent(QHideEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    void updateResizeGrip();

    QWidget* resizeGrip_ = nullptr;
    bool resizeLeft_ = true;
};

// The right-hand panel host. A vertical QSplitter of PanelGroups inside a
// QScrollArea: group heights drag with the splitter handles, and a short window
// scrolls the column instead of forcing the main window to grow. There are no
// hard minimum widths or heights here. A thin header carries the
// `panelColumnToggle` that switches the whole column to the iconic icon strip.
class PanelColumn : public QWidget {
    Q_OBJECT

public:
    explicit PanelColumn(QWidget* parent = nullptr);
    ~PanelColumn() override;

    void addGroup(PanelGroup* group);
    // M45 W4: adopt a group from another column without clobbering its panel
    // visibility (used to rehome a hidden group before its empty dynamic column
    // is removed, so the panel widget survives for a later show).
    void adoptGroup(PanelGroup* group);
    PanelGroup* groupForPanel(const QString& objectName) const;
    // The live float hosting a group that contains `objectName`, or null. A
    // floated panel has left `groups_`, so close/show/Window-menu lookups must
    // consult this before concluding the panel is unknown.
    PanelFloat* floatForPanel(const QString& objectName) const;
    QList<PanelGroup*> groups() const { return groups_; }

    // M43 multi-column host. `side` is derived from the splitter order relative
    // to the document tabs.
    PanelSide side() const;

    bool showPanel(const QString& objectName, bool visible);
    bool isPanelVisible(const QString& objectName) const;
    void closeGroup(PanelGroup* group);

    // M47: move this column's live floating overlays to another column (drag
    // wiring and `floats_` ownership) so an emptied source column can be removed.
    void rehomeFloatsTo(PanelColumn* target);

    // True while the whole column is torn off into a frameless `PanelFloat`
    // overlay. A floated column is not a splitter pane, so the empty-column
    // cleanup must never delete it.
    bool isColumnFloating() const { return columnFloat_ != nullptr; }
    PanelFloat* columnFloat() const { return columnFloat_; }
    QRect columnFloatGeometry() const;
    // Drop a live whole-column tear-off overlay without moving the column (used
    // when a session rebuild re-parents the column into the splitter).
    void cancelColumnFloat();
    // Rebuild a restored overlay at a stored geometry.
    PanelFloat* restoreGroupFloat(PanelGroup* group, const QRect& geometry);
    PanelFloat* restoreColumnFloat(const QRect& geometry);
    // Re-clamp live overlays after the frame geometry changes.
    void reclampFloats();
    // Re-dock this column's group overlays into the column (empty-layout reset).
    void redockGroupFloats();
    // Hide the flyout from the frame's hide-all / rebuild paths.
    void closeFlyout() { closeIconFlyout(); }
    // Re-place a live child-mode flyout after the frame geometry changes.
    void reclampFlyout();

    // M43 session v6: detach a group so it can be adopted by another column
    // (used to rebuild a stored multi-column layout at startup). Returns the
    // live group, unwired from this column, or nullptr when unknown.
    PanelGroup* takeGroup(const QString& groupObjectName);
    // Pointer overload: safe when group names collide after a tear-off.
    PanelGroup* takeGroup(PanelGroup* group);

    // D1: a tools column hosts one plain content widget instead of groups. It
    // has no tab bar, no `PanelGroup`, and no rail mode; it is atomic (the drop
    // resolver never combines it with a widget panel/group). Its header toggle
    // reflows the content between one and two columns through the supplied
    // callbacks rather than switching rail mode.
    void setToolsContent(QWidget* content, std::function<int()> columnsState,
                         std::function<void()> onToggleRequested);
    bool isToolsColumn() const { return toolsContent_ != nullptr; }
    QWidget* toolsContentForTest() const { return toolsContent_; }
    // Re-fit the tools column width to its content (after a 1<->2 column flip).
    void refreshToolsWidth();

    // A widget column's invisible grip on its workspace-facing edge. Placed in
    // whichever side is adjacent to the workspace; hidden for the tools column
    // and for a floating (non-splitter) column.
    void updateResizeGrip();
    // Suppress the per-side 1 px border margin where the central band's frame
    // line already provides the edge, so the two never double up.
    void setBorderEdgeSuppressed(bool left, bool right);

    // `normal` (splitter of groups) <-> `iconic` (narrow icon strip).
    void setRailMode(bool iconic);
    bool railMode() const { return railMode_; }
    QToolButton* panelColumnToggle() const { return columnToggle_; }

    // v5 persisted state. `setAutoShowHidden(on)` also widens the iconic strip
    // to include the hidden panels' icons (the small M41 effect for the flag).
    void setAutoCollapseIconic(bool on);
    void setAutoShowHidden(bool on);
    bool autoCollapseIconic() const { return autoCollapseIconic_; }
    bool autoShowHidden() const { return autoShowHidden_; }
    void setPreferredWidth(int width);
    // A migrated v6 per-column entry carries no width; use this until one is set.
    static constexpr int kDefaultNormalWidth = 300;
    // The normal-mode width range. A restored or persisted width is clamped to it
    // so a stale store can never expand a column across the workspace.
    static constexpr int kMinNormalWidth = 300;
    static constexpr int kMaxNormalWidth = 400;
    // Width to persist for this column: its own width in normal mode, or the
    // remembered normal width while iconic, clamped to the range above. During an
    // iconic->normal flip the widening may still be pending, so the remembered
    // width is written instead of the transient icon-strip width (0 when neither
    // is known yet).
    int persistedWidth() const
    {
        // The tools column is not governed by the widget-column width range; it
        // records its live content width so a 1/2 column flip round-trips.
        if (toolsContent_) {
            return qMax(0, width());
        }
        const int width = railMode_ || (widthFlipPending_ && normalWidthBeforeIconic_ > 0)
            ? normalWidthBeforeIconic_
            : this->width();
        return width <= 0 ? 0 : qBound(kMinNormalWidth, width, kMaxNormalWidth);
    }
    // Apply a width loaded from the session. An iconic column records it as the
    // normal width to restore on the next toggle instead of resizing the strip.
    void setRestoredWidth(int width);

    // M45 W8: drop a column's contribution to the shared minimum floor after it
    // is added or removed, recomputing from the columns currently alive so a
    // since-destroyed wide column cannot pin every column's minimum high.
    static void refreshSharedFloor(PicturaMainWindow* frame);

    // Per-group order/visibility/minimized/collapsed as a compact JSON array;
    // round-trips exactly and skips groups whose stored name is gone.
    QJsonArray savePanelState() const;
    void restorePanelState(const QJsonArray& state);

    // Test hooks.
    QString groupOfForTest(const QString& objectName) const;
    bool scrollableForTest() const;
    int minimumWidthForTest() const;
    bool railModeForTest() const { return railMode_; }
    QToolButton* panelColumnToggleForTest() const { return columnToggle_; }
    bool iconStripVisibleForTest() const;
    int dividerCountForTest() const;
    bool iconLabelsShownForTest() const { return iconLabelsShown_; }
    // M44 chrome checks.
    int groupDividerWidthForTest() const;
    QString stripLabelTextForTest(const QString& objectName) const;
    bool stripLabelVisibleForTest(const QString& objectName) const;
    void setIconStripWidthForTest(int width);
    bool openIconFlyoutForTest(const QString& objectName);
    bool iconFlyoutVisibleForTest() const;
    QStringList tabMenuActionsForTest() const;
    QStringList tabMenuActionsForTest(const QString& groupObjectName) const;
    bool triggerTabMenuForTest(const QString& text);
    bool autoCollapseIconicForTest() const { return autoCollapseIconic_; }
    bool autoShowHiddenForTest() const { return autoShowHidden_; }

    // Phase C drag/drop. These hooks drive the same begin/update/commit path
    // the tab-bar mouse handler uses; they never synthesize OS drags.
    bool beginTabDragForTest(const QString& objectName);
    void dragToForTest(const QPoint& globalPos);
    bool dropForTest(const QPoint& globalPos);
    void cancelDragForTest();
    // M48: dock the named panel to the requested side of its own source column
    // through the real drag path; true when a new column lands there.
    bool selfAnchorDockForTest(const QString& objectName, bool toRight);
    bool dropIndicatorVisibleForTest() const;
    QRect dropIndicatorGeometryForTest() const;
    QRect dropIndicatorGlobalGeometryForTest() const;
    // The column-move edge mark is a frame-level widget so a floating overlay
    // (which follows the cursor) cannot cover it.
    bool edgeIndicatorOnFrameForTest() const;
    // Phase 4: the blue region outline shown around the target group for a
    // group-on-group tabify.
    bool outlineIndicatorVisibleForTest() const;
    int scrollViewportHeightForTest() const;
    int dropIndexForTest() const;
    int horizontalScrollPolicyForTest() const;
    int horizontalScrollRangeForTest() const;
    int minimumWidthFloorForTest() const;
    int contentMinimumWidthForTest() const;
    int viewportWidthForTest() const;
    // M45 T3: the frame resolves a floating-Tools drop through this column's
    // grammar and shows the same `#2a7fff` new-column edge indicator.
    void showEdgeDropIndicator(PanelSide side);
    void hideEdgeDropIndicator();
    int floatCountForTest() const;
    PanelFloat* floatForTest(int index) const;
    QStringList floatPanelNamesForTest(int index) const;
    bool floatTabIndicatorVisibleForTest(int index) const;
    bool tearOffForTest(const QString& groupName);
    bool tearOffPanelForTest(const QString& objectName);
    bool redockForTest(int floatIndex, int boundaryIndex);
    bool closeFloatForTest(int index);
    QToolButton* floatCloseButtonForTest(int index) const;
    bool floatIsToolWindowForTest(int index) const;
    QRect floatGeometryForTest(int index) const;
    QRect floatHostRectForTest() const;
    bool floatClampedForTest(int index, const QPoint& globalTopLeft);
    QPoint boundaryPointForTest(int boundary) const;
    bool ensureGroupVisibleForTest(const QString& panelName);
    // M47: drag the whole widget column from its top header.
    QWidget* columnHeaderForTest() const;
    bool beginColumnHeaderDragForTest(const QPoint& globalPos);
    void dragColumnHeaderToForTest(const QPoint& globalPos);
    bool dropColumnHeaderForTest(const QPoint& globalPos);
    // The whole-column tear-off overlay while it is floating, or null.
    PanelFloat* columnFloatForTest() const { return columnFloat_; }
    // M47: the header's left-click menu (collapse/auto toggles/interface options).
    QStringList columnHeaderMenuTextsForTest() const;
    bool triggerColumnHeaderMenuForTest(const QString& text);
    // M43 Phase B: begin a whole-group drag and a compact-strip entry point.
    bool beginGroupDragForTest(const QString& panelName);
    QPoint stripEntryPointForTest(const QString& panelName, int where) const;

    // Phase B iconic-strip drag. These drive the same begin/update/commit path
    // the strip button's mouse handler uses.
    QStringList stripOrderForTest() const;
    bool beginStripDragForTest(const QString& objectName);
    QPoint stripInsertionPointForTest(int index) const;
    int stripDropIndexForTest() const;
    bool dropStripOnGroupForTest(const QString& objectName, const QString& targetPanel);

    // Phase C flyout refinement.
    QString flyoutSideForTest() const;
    QString activeIconNameForTest() const { return activeIconName_; }
    QRect iconFlyoutGeometryForTest() const;
    QWidget* iconFlyoutForTest() const { return flyout_; }
    QWidget* iconFlyoutResizeGripForTest() const;
    QString flyoutHeaderTitleForTest() const;
    bool flyoutHeaderCloseForTest() const;
    bool triggerFlyoutCloseForTest();
    // M44 C1/S2: the flyout shares the docked widget's scoped style/container.
    bool popupStyleParityForTest(const QString& objectName) const;
    // M45 C1: the whole PanelGroup currently hosted in the flyout, or null when
    // the flyout is closed. The caller compares it to the docked group.
    PanelGroup* iconFlyoutGroupForTest() const { return flyoutGroup_; }

    // M44 Phase C: whole-widget-column docking and compact drop.
    bool dragActiveForTest() const { return dragActive_; }
    bool dragIsPanelForTest() const { return dragIsPanel_; }
    // Phase 6: opacity of the widget currently dimmed for this drag (the docked
    // source group or the float), 1.0 when nothing is dimmed.
    qreal dragDimOpacityForTest() const;
    bool dragSourceGroupAliveForTest() const;
    QWidget* dragHandleForTest(const QString& groupName) const;
    QStringList compactStripGroupOrderForTest() const;
    int compactStripGroupIndexForTest(const QString& panelName) const;
    QPoint compactStripBoundaryPointForTest(int boundary) const;
    bool beginGroupGripDragForTest(const QString& groupName);

    // Phase D per-widget header action menu.
    QToolButton* widgetMenuButtonForTest(const QString& groupObjectName) const;
    QStringList widgetMenuTextsForTest(const QString& panelName) const;
    bool widgetMenuHasCloseForTest(const QString& panelName) const;
    bool triggerWidgetMenuForTest(const QString& panelName, const QString& text);
signals:
    void interfaceOptionsRequested();
    void stateChanged();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;
    void showEvent(QShowEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    // The frozen M43 drop-target grammar. `onTabBar`/`onStrip`/`outside` are
    // kept alongside `kind` so the M41/M42 hooks keep their meaning.
    enum class DropKind {
        None,
        Reorder,
        IntoGroup,
        AboveGroup,
        BelowGroup,
        OnStrip,
        NewColumnLeft,
        NewColumnRight,
        Outside,
    };

    struct DropTarget {
        bool valid = false;
        bool outside = false;
        bool onTabBar = false;
        bool onStrip = false;
        PanelGroup* group = nullptr;
        int tabIndex = -1;
        int boundary = -1;
        int stripIndex = -1;
        DropKind kind = DropKind::None;
        // M44 W5: a whole-column drop beside another column creates the new
        // column adjacent to this anchor instead of at the workspace end.
        PanelColumn* anchorColumn = nullptr;
        // M45 W1/W2: the column that owns this target (the target group's
        // column for a cross-column result). The indicator is rendered through
        // the owner so the line is drawn where the commit will place it.
        PanelColumn* owner = nullptr;
        // Phase 3: the floating overlay hosting `group`, when the point resolved
        // over an overlay. Null for docked targets; the tabify commit and the
        // indicator both key off it.
        PanelFloat* floatTarget = nullptr;
        // Phase 4: a whole-group drag resolved onto another group's tab bar
        // tabifies into it; the indicator draws the blue region outline instead
        // of the thin insertion line.
        bool groupTabify = false;
    };

    // One icon in the iconic strip, in strip order. `button` is the drag source
    // and the anchor for the strip insertion maths.
    struct StripEntry {
        PanelGroup* group = nullptr;
        QString name;
        QToolButton* button = nullptr;
    };

    // M44 C3: the compact strip renders one container per group (a drag-handle
    // grip above its icons) with a dark divider between containers. The divider
    // and container geometry drive the group-relative proximity rule.
    struct StripDivider {
        PanelGroup* after = nullptr;
        QWidget* widget = nullptr;
    };

    void updateColumnToggle();
    void updateMinimumWidth();
    void buildIconStrip();
    void clearIconStrip();
    void updateIconStripLabels();
    void ensureFlyout();
    void openIconFlyout(const QString& objectName, const QPoint& globalPos);
    // Phase 7: a compact icon lives either in a docked group (`groups_`) or in a
    // live float, so the flyout resolves both before it opens.
    PanelGroup* resolveFlyoutGroup(const QString& objectName) const;
    void closeIconFlyout();
    // Remove a group from the flyout without restoring it (it is being dragged
    // out into its own overlay).
    void detachFlyoutForDrag(PanelGroup* group);
    void restoreFlyoutGroup();
    void placeFlyout(const QRect& buttonGlobalRect, const QSize& size);
    QString flyoutSide() const;
    QToolButton* flyoutButtonFor(const QString& objectName) const;
    void setActiveIcon(const QString& objectName);
    QToolButton* stripButtonFor(const QString& objectName) const;
    void showTabMenu(PanelGroup* group, const QPoint& globalPos);
    QMenu* buildTabMenu(PanelGroup* group);
    QStringList tabMenuTextsFor(const PanelGroup* group) const;
    // M47: left-click menu on the column's top header.
    void showColumnHeaderMenu(const QPoint& globalPos);
    QMenu* buildColumnHeaderMenu();
    QToolButton* makeIconButton(QWidget* parent, const QString& objectName,
                                const QString& title, const QIcon& icon);

    void wireGroup(PanelGroup* group);
    void insertGroupAt(PanelGroup* group, int index);
    bool removeGroup(PanelGroup* group);
    void cleanupEmptyGroup(PanelGroup* group);
    // M45 W4: remove this dynamic column when no group/panel is left, through
    // the frame's one cleanup entry point.
    void maybeRemoveSelf();
    // The main window that owns this column, whether it is docked or hosted in
    // a frameless floating overlay (whose parent is the frame). `window()` is the
    // float once the column is floated, so callers that need the frame use this.
    PicturaMainWindow* owningFrame() const;
    DropTarget resolveDrop(const QPoint& globalPos) const;
    DropTarget resolveLocalDrop(const QPoint& globalPos, bool dragIsPanel = false,
                                PanelGroup* dragGroup = nullptr) const;
    bool resolveIconicDrop(const QPoint& globalPos, DropTarget& target) const;
    int boundaryIndexForGlobalY(const QPoint& globalPos) const;
    int stripInsertionIndexAt(const QPoint& globalPos) const;
    void showIndicatorFor(const DropTarget& target);
    void clearIndicator();
    // The new-column/edge mark, drawn on the frame (above the splitter) so a
    // following floating overlay cannot hide it.
    void showColumnEdgeIndicator(bool left);
    // The mark for a bare workspace edge with no column on that side: drawn at
    // the central area's left/right edge, not at an arbitrary column.
    void showWorkspaceEdgeIndicator(bool left);
    QList<PanelGroup*> visibleGroups() const;
    bool applyPanelDrop(PanelGroup* source, const QString& name, const DropTarget& target);
    bool applyStripDrop(PanelGroup* source, const QString& name, int stripIndex);
    bool applyGroupDrop(PanelGroup* group, const DropTarget& target);
    // Phase 3/4: move every tab of `source` into `dest` at `at`, then tear the
    // emptied source down through the one cleanup path. One merge mechanism for
    // a float target now and group-on-group later.
    bool mergeGroupInto(PanelGroup* source, PanelGroup* dest, PanelColumn* owner, int at);
    bool applyNewColumnDrop(PanelSide side, PanelColumn* anchor);
    void beginPanelDrag(PanelGroup* group, const QString& objectName, const QPoint& globalPos);
    void beginGroupDrag(PanelGroup* group, const QPoint& globalPos);
    void updateDrag(const QPoint& globalPos);
    // Phase 6: dim the dragged thing for the whole drag, not only while a valid
    // target is under the pointer. `setDragDimTarget` restores the previous
    // target first; a docked group gets a transient `QGraphicsOpacityEffect`,
    // a float reuses its own.
    void updateDragDim();
    void setDragDimTarget(QWidget* target);
    bool commitDrop();
    void cancelDrag();
    // M47: the header-drag column move reuses the edge-drop indicator.
    void updateColumnDrag(const QPoint& globalPos);
    bool finishColumnDrag(const QPoint& globalPos);
    PanelFloat* createFloat(PanelGroup* group, const QPoint& globalPos);
    void destroyFloat(PanelFloat* floatWindow);
    void closeFloat(PanelFloat* floatWindow);
    void moveFloat(PanelFloat* floatWindow, const QPoint& globalTopLeft);
    QRect floatBounds(QWidget* host) const;
    PanelFloat* floatForGroup(PanelGroup* group) const;
    // Recompute the shared minimum-width floor across the frame's columns when
    // panel content changed, so a closed wide panel lets the floor shrink.
    void refreshFloorAfterContentChange();
    // Phase 3: this column's floating overlay hosting a `PanelGroup` whose rect
    // contains `globalPos`, or null. A whole-column float (no group) is skipped.
    PanelFloat* groupFloatAtGlobal(const QPoint& globalPos) const;
    // The whole-column tear-off overlay lifecycle. `floatColumn` hosts this
    // column in an floating overlay at `globalTopLeft`; `redockColumnFloat`
    // puts it back in the splitter through the frame's move path and removes the
    // overlay; `destroyColumnFloat` tears the overlay down without moving.
    PanelFloat* floatColumn(const QPoint& globalTopLeft);
    bool redockColumnFloat(int side, PanelColumn* anchor);
    void destroyColumnFloat();
    PanelGroup* findGroupByName(const QString& objectName) const;
    QWidget* makeGroupGrip(PanelGroup* group);
    QWidget* stripGroupBoxFor(PanelGroup* group) const;

    QWidget* header_ = nullptr;
    QToolButton* columnToggle_ = nullptr;
    // The 4 px invisible width-drag grip on a widget column's workspace edge.
    QWidget* resizeGrip_ = nullptr;
    // D1/D2: set while this column hosts a single plain tools content widget.
    QWidget* toolsContent_ = nullptr;
    std::function<int()> toolsColumnsState_;
    std::function<void()> toolsToggleAction_;
    QScrollArea* scroll_ = nullptr;
    QSplitter* splitter_ = nullptr;
    QWidget* iconStrip_ = nullptr;
    QBoxLayout* iconStripLayout_ = nullptr;
    QList<QLabel*> stripLabels_;
    QList<StripEntry> stripEntries_;
    // M44 C3: group containers and inter-group dividers in strip order.
    QList<PanelGroup*> stripGroupOrder_;
    QHash<PanelGroup*, QWidget*> stripGroupBoxes_;
    QList<StripDivider> stripDividers_;
    QString activeIconName_;
    QList<PanelGroup*> groups_;
    QHash<QString, bool> panelVisible_;

    bool railMode_ = false;
    bool iconLabelsShown_ = false;
    // Last side observed while docked in the central splitter. A floating column
    // has no splitter parent, so `side()` falls back to this instead of a
    // hardcoded right, keeping its compact flyout on the correct side.
    mutable PanelSide lastSide_ = PanelSide::Right;
    int pendingWidth_ = 0;
    int normalWidthBeforeIconic_ = 0;
    // Set while an iconic->normal flip waits for the widening to be laid out, so
    // `persistedWidth` does not write the icon-strip width in the meantime.
    bool widthFlipPending_ = false;

    PanelFlyout* flyout_ = nullptr;
    QBoxLayout* flyoutLayout_ = nullptr;
    PanelGroup* flyoutGroup_ = nullptr;
    QString flyoutName_;
    int flyoutGroupIndex_ = -1;
    // Phase 7: the live float that hosted the group when its compact icon was
    // clicked, so the flyout returns it there instead of docking it.
    QPointer<PanelFloat> flyoutFloat_;
    QRect flyoutAnchorRect_;
    // Phase 7: a collapsed group is expanded for the flyout and re-collapsed on
    // close, so a compact icon opens the same content a docked strip icon does.
    bool flyoutWasCollapsed_ = false;
    bool restoringFlyout_ = false;

    bool autoCollapseIconic_ = false;
    bool autoShowHidden_ = false;
    PanelGroup* menuGroup_ = nullptr;

    QWidget* indicator_ = nullptr;
    QWidget* stripIndicator_ = nullptr;
    QWidget* outlineIndicator_ = nullptr;
    // The new-column/edge mark for a drag: parented to the frame, not the
    // viewport, so an in-window floating overlay (which is raised above the
    // splitter and follows the cursor) cannot hide it.
    QWidget* edgeIndicator_ = nullptr;
    QSet<PanelGroup*> wired_;
    QList<PanelFloat*> floats_;

    bool dragActive_ = false;
    bool dragIsPanel_ = false;
    PanelGroup* dragGroup_ = nullptr;
    // M44 W4: the panel's original group is kept alive for the whole drag so the
    // tab bar that owns the implicit mouse grab survives until release; it is
    // emptied by `createFloat` and cleaned up on commit/cancel.
    PanelGroup* dragSourceGroup_ = nullptr;
    QString dragPanel_;
    QPoint dragGrabOffset_;
    int dragOriginalIndex_ = -1;
    // The dragged panel's tab index in its source group, so a cancelled single
    // panel drag lands back where it was instead of in a new group.
    int dragOriginalPanelIndex_ = -1;
    PanelFloat* dragFloat_ = nullptr;
    // Phase 6: the widget currently dimmed for this drag (a docked group or a
    // float), or null. Cleared on commit and cancel.
    QWidget* dimTarget_ = nullptr;
    DropTarget dropTarget_;
    // M45 W1/W2: the column currently owning the rendered drop indicator.
    PanelColumn* indicatorOwner_ = nullptr;
    // Phase 3: the float whose tab-bar indicator this column currently shows.
    PanelFloat* floatIndicator_ = nullptr;

    bool stripPressPending_ = false;
    bool stripDragging_ = false;
    QPoint stripPressGlobal_;
    QToolButton* stripDragButton_ = nullptr;
    // M44 C3: the group whose grip press/held state is active.
    PanelGroup* stripGripGroup_ = nullptr;

    // M47: drag the whole column from its top header.
    bool columnPressPending_ = false;
    bool columnDragging_ = false;
    QPoint columnPressGlobal_;
    // Cursor offset inside the column at drag start, so a torn-off overlay keeps
    // the header under the pointer instead of jumping to its top-left corner.
    QPoint columnGrabOffset_;
    PanelColumn* columnDropAnchor_ = nullptr;
    int columnDropSide_ = -1;
    PanelFloat* columnFloat_ = nullptr;
};

} // namespace pictura
