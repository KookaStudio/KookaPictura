#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QIcon>
#include <QtWidgets/QWidget>

#include <functional>

class QBoxLayout;
class QMenu;
class QTabBar;
class QTabWidget;
class QToolButton;

namespace pictura {

// One CS6 panel group: a top-tab QTabWidget whose tab text is the panel title.
// There is deliberately no group-title label; the tabs are the group chrome.
// A group can roll up to its tab bar (`Minimize`) or shrink to a row of panel
// icons (`Collapse to Icons`); the iconic row reuses the column's flyout.
class PanelGroup : public QWidget {
    Q_OBJECT

public:
    explicit PanelGroup(QWidget* parent = nullptr);

    void addPanel(QWidget* panel, const QString& title, const QIcon& icon);
    QStringList titles() const;
    QStringList visibleTitles() const;
    // Reorder the tabs to match `order` (names not present are ignored).
    void setPanelOrder(const QStringList& order);
    bool isPanelVisible(const QString& objectName) const;
    QList<QWidget*> panels() const;
    QList<QWidget*> visiblePanels() const;
    bool containsPanel(const QString& objectName) const;
    bool setPanelVisible(const QString& objectName, bool visible);
    QString currentPanelName() const;
    // M45 C1: make the named panel the active tab (the compact flyout opens the
    // whole group with the clicked panel current). False when unknown.
    bool setCurrentPanel(const QString& objectName);
    QString titleForPanel(const QString& objectName) const;
    QIcon iconForPanel(const QString& objectName) const;

    // Phase C drag/drop primitives. `takePanel` removes a tab and hands back
    // the widget plus its tab text/icon/index so a drop can re-parent it
    // elsewhere; `insertPanel` puts it back at an index. A panel is never in
    // two tab stacks: take first, then insert.
    QWidget* takePanel(const QString& objectName, QString* title = nullptr,
                       QIcon* icon = nullptr, int* index = nullptr);
    void insertPanel(QWidget* panel, const QString& title, const QIcon& icon, int index);
    QTabBar* tabBar() const;
    int indexOfPanel(const QString& objectName) const;
    // The width the per-widget corner `▾` button reserves in the header, so the
    // column minimum can include it (M43 corner-button fix).
    int headerCornerWidthForTest() const;

    // Tab-bar geometry used by the column's drop resolver. All coordinates are
    // global except `tabInsertionX`, which is in tab-bar space.
    QRect tabBarGlobalRect() const;
    int tabInsertionIndexAt(const QPoint& globalPos) const;
    int tabInsertionX(int index) const;
    QPoint tabInsertionGlobalPointForTest(int index) const;

    // Minimize keeps the tab bar and hides the content; collapse-to-icons
    // replaces the tabs with the panel icon row. These are distinct states.
    bool isMinimized() const { return minimized_; }
    void setMinimized(bool minimized);
    bool isCollapsedToIcons() const { return collapsedToIcons_; }
    void setCollapsedToIcons(bool collapsed);

    // M44: make the first visible panel current (a stored layout used to leave
    // the last visible panel active after `setPanelVisible` walked the tabs).
    void setCurrentToFirstVisible();

    // A panel may refine its own menu on aboutToShow (enablement, check state,
    // dynamic rows, and installed lists).
    using PanelMenuHook = std::function<void(QMenu*)>;
    void setPanelMenuHook(const QString& panelName, PanelMenuHook hook);

    // M47 D10: a group hosted in a floating overlay shows a rightmost close
    // control; docked and flyout hosts hide it.
    void setFloating(bool on);
    QToolButton* floatCloseButton() const { return floatCloseButton_; }
    // A group hosted in a compact flyout shows a chevrons-right close control
    // immediately left of the per-widget menu button.
    void setFlyout(bool on);
    QToolButton* flyoutCloseButtonForTest() const { return flyoutCloseButton_; }

    // Test hooks.
    int tabPositionForTest() const;    int titleCountForTest() const;
    QStringList titleTextsForTest() const;
    QIcon titleIconForTest(const QString& title) const;
    bool tabHasIconForTest(const QString& title) const;
    QIcon iconicIconForTest(const QString& objectName) const;
    int headerCornerRightMarginForTest() const;
    bool groupLabelForTest() const;
    bool isMinimizedForTest() const { return minimized_; }
    void setMinimizedForTest(bool minimized) { setMinimized(minimized); }
    bool isCollapsedToIconsForTest() const { return collapsedToIcons_; }
    void setCollapsedToIconsForTest(bool collapsed) { setCollapsedToIcons(collapsed); }
    bool contentHiddenForTest() const;
    bool tabBarVisibleForTest() const;
    // M47: the tab bar squeezes/elides instead of showing scroll arrows.
    bool tabUsesScrollButtonsForTest() const;
    // M47: reserved draggable corner grip and the floating group's top bar.
    QWidget* headerGripForTest() const { return headerGrip_; }
    QWidget* headerCornerForTest() const { return headerCorner_; }
    QWidget* headerBandForTest() const { return headerBand_; }
    QWidget* floatHeaderForTest() const { return floatHeader_; }
    QToolButton* floatToggleForTest() const { return floatToggle_; }
    bool floatHeaderVisibleForTest() const;
    // M44 default-active check: the current tab index and the first visible one.
    int currentTabIndexForTest() const;
    int firstVisibleTabIndexForTest() const;

    // Phase D: the per-widget header action button and its per-panel menu.
    QToolButton* headerMenuButtonForTest() const { return headerButton_; }
    bool headerMenuAtRightForTest() const;
    QStringList panelMenuTextsForTest() const;
    bool panelMenuEnabledForTest(const QString& text) const;
    QString panelMenuToolTipForTest(const QString& text) const;
    bool triggerPanelMenuForTest(const QString& text);
    static bool panelHasMenu(const QString& panelName);
    static QStringList menuTextsForPanel(const QString& panelName);

signals:
    void panelActivated(const QString& objectName, const QPoint& globalPos);
    void tabContextMenuRequested(const QPoint& globalPos);
    // Phase C: a press on a tab that moves past the drag threshold. A press on
    // the empty part of the tab bar is a whole-group drag (empty objectName).
    void tabDragStarted(const QString& objectName, const QPoint& globalPos);
    void groupDragStarted(const QPoint& globalPos);
    void dragMoved(const QPoint& globalPos);
    void dragFinished(const QPoint& globalPos);
    void dragCanceled();
    // M47: a floating group resizes itself to the collapsed icon row.
    void collapsedToIconsChanged(bool collapsed);
    // M47: the per-widget menu's Close / Close Group entries. The column that
    // wires the group performs the actual close through its existing paths.
    void closePanelRequested(const QString& objectName);
    void closeGroupRequested();
    // Emitted by the flyout-only close control.
    void flyoutCloseRequested();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void rebuildIconRow();
    void applyMinimize();
    void updateHeaderMenu();
    void runPanelMenuAction(const QString& actionId);
    QToolButton* makeIconButton(const QIcon& icon, const QString& title,
                                const QString& objectName);
    // M47: mirror `PanelColumn::updateColumnToggle` for the float header.
    void updateFloatToggle();
    // M47: keep the full-width header band sized to the tab bar.
    void updateHeaderBand();

    QTabWidget* tabs_ = nullptr;
    QWidget* iconRow_ = nullptr;
    QBoxLayout* iconRowLayout_ = nullptr;
    QToolButton* headerButton_ = nullptr;
    QWidget* headerCorner_ = nullptr;
    // M47: lowered band behind the tab bar and corner, full header width.
    QWidget* headerBand_ = nullptr;
    // M47: always-present blank drag grip in the corner, right of the tabs.
    QWidget* headerGrip_ = nullptr;
    // M47: the floating group's top bar with its collapse toggle and close.
    QWidget* floatHeader_ = nullptr;
    QToolButton* floatToggle_ = nullptr;
    QToolButton* floatCloseButton_ = nullptr;
    // M45: the flyout-only close control in the header corner.
    QToolButton* flyoutCloseButton_ = nullptr;
    QMenu* headerMenu_ = nullptr;
    QHash<QString, PanelMenuHook> panelMenuHooks_;
    // M48: panel icons are kept off the normal tabs but retained here for the
    // iconic/icon strip and the tab-drag payloads, keyed by panel objectName.
    QHash<QString, QIcon> panelIcons_;
    bool collapsedToIcons_ = false;
    bool minimized_ = false;
    int savedMaxHeight_ = QWIDGETSIZE_MAX;
    int savedGroupMaxHeight_ = QWIDGETSIZE_MAX;
    int savedMinHeight_ = 0;
    int savedTabsMinHeight_ = 0;

    bool pressPending_ = false;
    bool dragging_ = false;
    QPoint pressGlobal_;
    QString pressedPanel_;
    // M47: corner-grip and float-header drags each keep their own state so a
    // grip drag cannot clobber a tab-bar drag.
    bool gripPressPending_ = false;
    bool gripDragging_ = false;
    QPoint gripPressGlobal_;
    bool floatPressPending_ = false;
    bool floatDragging_ = false;
    QPoint floatPressGlobal_;
    // A collapsed-row icon drag: its own state so it cannot clobber the grips.
    bool iconPressPending_ = false;
    bool iconDragging_ = false;
    QPoint iconPressGlobal_;
    QString iconDragName_;
};

} // namespace pictura
