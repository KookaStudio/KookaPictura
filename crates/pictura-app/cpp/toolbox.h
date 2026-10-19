#pragma once

#include <QtCore/QList>
#include <QtCore/QMap>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include "tools.h"

class QAction;
class QEvent;
class QGridLayout;
class QHBoxLayout;
class QMenu;
class QMouseEvent;
class QPaintEvent;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class ColorState;
class ToolController;

// CS6-style foreground/background swatches: two overlapping squares with the
// active one outlined, plus a corner reset to the default black/white pair.
class ForegroundBackgroundWidget : public QWidget {
    Q_OBJECT

public:
    explicit ForegroundBackgroundWidget(ColorState* state, QWidget* parent = nullptr);

    void resetColors();
    // Exchange foreground and background (CS6 double-headed-arrow swap).
    void swapForegroundBackground();

    // The column width scales the swatches; the widget never dictates a wider
    // dock. `side` is clamped to a square that still reads as two swatches.
    void setSide(int side);

    // Test hooks.
    QRect swapRectForTest() const { return swapRect(); }
    int swapCountForTest() const { return swapCount_; }
    QColor foregroundForTest() const;
    QColor backgroundForTest() const;

signals:
    void clicked();
    void reset();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;

private:
    QRect foregroundRect() const;
    QRect backgroundRect() const;
    QRect resetRect() const;
    QRect swapRect() const;
    int swatchSize() const;
    int resetSize() const;

    ColorState* state_ = nullptr;
    int swapCount_ = 0;
};

// The Tools content: a CS6 list of flyout slots (one button per group) that
// reflows between one and two columns, the foreground/background swatches, and
// the screen-mode toggle. It is a plain content widget hosted by the tools
// `PanelColumn` (no dock, no title bar); its column header owns the width
// toggle, which drives `setColumns` through the column.
class Toolbox : public QWidget {
    Q_OBJECT

public:
    explicit Toolbox(ToolController* controller, ColorState* colors, QWidget* parent = nullptr);
    ~Toolbox() override;

    // The group slot buttons in catalogue order (self-test accessor).
    QList<QToolButton*> slotButtons() const { return slotButtons_; }
    // The group id of each slot in `slotButtons()` order.
    QList<int> slotGroupsForTest() const { return slotGroups_; }

    // Base slot/icon metrics (96 DPI) scaled by a logical DPI. Exposed so the
    // Qt Test can assert the no-double-scale rule without a second screen.
    static QSize slotSizeForDpi(qreal logicalDpi);
    static QSize iconSizeForDpi(qreal logicalDpi);

    // The screen-mode button's InstantPopup menu, sharing the registry's
    // `ViewScreenMode*` actions (checks/handlers come from the one source).
    void setScreenModeActions(const QList<QAction*>& actions);
    void setActiveScreenMode(int mode);

    // Current column count, 1 or 2.
    int columns() const { return columns_; }
    void setColumns(int columns);

    // Gated by `Use Shift Key For Tool Switch`: with it on a plain letter
    // activates the slot's current member and `Shift`+letter cycles; with it off
    // the letter alone cycles. Returns true when a group handled the key.
    void setShiftKeyForToolSwitch(bool on) { shiftKeyForToolSwitch_ = on; }
    bool shiftKeyForToolSwitch() const { return shiftKeyForToolSwitch_; }
    bool handleToolKey(const QChar& key, bool shift);
    // Re-read every tool's letter into the slot tooltips and flyout rows after
    // the shortcut editor changed them.
    void refreshToolShortcuts();

    // Self-test hooks.
    void openSlotFlyoutForTest(int group);
    void openFlyoutForTest(int group) { openSlotFlyoutForTest(group); }
    QMenu* slotMenuForTest(int group) const;
    QList<QAction*> slotMenuActionsForTest(int group) const;
    // The Zoom tool's preset menu, also shown on a canvas right-click with the
    // Zoom tool active.
    QMenu* zoomMenu() const { return zoomMenu_; }
    bool hasFlyoutTriangleForTest(int group) const;
    int contentWidthForTest() const;
    int foregroundBackgroundWidthForTest() const;
    // Exchanges the foreground/background swatches (the frame's `X` key).
    void swapForegroundBackground();
    // Resets the swatches to the active target's defaults (the frame's `D` key):
    // black/white on layer pixels, white/black on a mask.
    void resetForegroundBackground();
    ForegroundBackgroundWidget* foregroundBackgroundForTest() const { return fgbg_; }

signals:
    void paintMaskToggled(bool checked);
    void columnsChanged(int columns);

private:
    ToolId groupCurrentTool(int group) const;
    void refreshSlot(int group);
    void selectMember(int group, ToolId id);
    // `key` limits the cycle to the members that carry it (the P slot's Pen and
    // Freeform Pen, not its anchor tools); null cycles every member.
    void cycleGroup(int group, QChar key = QChar());
    void showSlotMenu(int group);
    // The Zoom tool's right-click menu (Fit on Screen, 100%, 200%, Print Size,
    // Zoom In, Zoom Out), built instead of a flyout since the group is single.
    QMenu* buildZoomMenu();
    void closeOpenSlotMenu();
    QToolButton* slotButtonAt(const QPoint& globalPos) const;
    // Re-derive slot/icon geometry from the current screen's logical DPI.
    void applyMetrics();
    // Orient/size the Quick Mask + Screen Mode footer for the current columns.
    void layoutFooter();
    void reflow();
    void updateContentMetrics();
    int contentWidth(int columns) const;

    bool eventFilter(QObject* watched, QEvent* event) override;
    bool event(QEvent* event) override;

    ToolController* controller_ = nullptr;
    ColorState* colors_ = nullptr;
    QMap<int, ToolId> currentByGroup_;
    QList<QToolButton*> slotButtons_;
    QList<int> slotGroups_;
    QMap<int, QToolButton*> slotButtonByGroup_;
    QMap<int, QMenu*> slotMenuByGroup_;
    QMenu* zoomMenu_ = nullptr;
    QGridLayout* grid_ = nullptr;
    QWidget* gridWidget_ = nullptr;
    QVBoxLayout* bodyLayout_ = nullptr;
    QHBoxLayout* footerRow_ = nullptr;
    ForegroundBackgroundWidget* fgbg_ = nullptr;
    QToolButton* screenMode_ = nullptr;
    QList<QAction*> screenModeActions_;
    QToolButton* paintMask_ = nullptr;
    QMenu* openSlotMenu_ = nullptr;
    QToolButton* openSlotButton_ = nullptr;
    QSize slotSize_ { 36, 28 };
    QSize iconSize_ { 24, 20 };
    int columns_ = 1;
    bool shiftKeyForToolSwitch_ = true;
};

} // namespace pictura
