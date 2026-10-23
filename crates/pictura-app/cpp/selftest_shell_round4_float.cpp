#include "selftest_shell_round4_float.h"
#include "selftest_report.h"

#include "frame.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "session.h"
#include "toolbox.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

namespace {

void pumpFloat(int count)
{
    for (int i = 0; i < count; ++i) {
        QCoreApplication::processEvents();
    }
}

// Dock the tools column, float it with no resolved target, and leave the
// overlay live. Returns the overlay (null when the float did not happen).
pictura::PanelFloat* floatToolsColumnForTest(pictura::PicturaMainWindow& frame,
                                             pictura::PanelColumn* tools)
{
    QWidget* tabs = frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
    if (!tools || !tabs) {
        return nullptr;
    }
    const QPoint parked = tabs->mapToGlobal(tabs->rect().center());
    tools->beginColumnHeaderDragForTest(
        tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
    tools->dragColumnHeaderToForTest(parked);
    tools->dropColumnHeaderForTest(parked);
    pumpFloat(6);
    return tools->columnFloatForTest();
}

// The first live group (with at least one tab) and one of its panel names, or
// null/empty. Reveals the group and expands it so an overlay grip/icon check
// has a normal-mode group to work with.
pictura::PanelGroup* liveGroup(pictura::PanelColumn* column)
{
    if (!column) {
        return nullptr;
    }
    for (pictura::PanelGroup* group : column->groups()) {
        if (group && group->titleCountForTest() > 0) {
            return group;
        }
    }
    return nullptr;
}

QString revealedPanel(pictura::PanelColumn* column, pictura::PanelGroup* group)
{
    if (!column || !group) {
        return QString();
    }
    if (group->visiblePanels().isEmpty() && !group->panels().isEmpty()
        && group->panels().first()) {
        column->showPanel(group->panels().first()->objectName(), true);
    }
    if (group->isCollapsedToIcons()) {
        group->setCollapsedToIcons(false);
    }
    const QString panel = group->visiblePanels().isEmpty()
                              ? QString()
                              : group->visiblePanels().first()->objectName();
    if (!panel.isEmpty()) {
        column->ensureGroupVisibleForTest(panel);
    }
    return panel;
}

void closeFloats(pictura::PanelColumn* column)
{
    for (int i = 0; column && i < 24 && column->floatCountForTest() > 0; ++i) {
        if (!column->closeFloatForTest(0)) {
            break;
        }
    }
}

} // namespace

int pictura::runShellRound4FloatCheck(pictura::PicturaMainWindow& frame)
{
    // float_child_overlay (440): on a platform that forbids a client from
    // positioning its own top-level windows (Wayland), or when forced for test,
    // a dragged float is an in-window child of the frame rather than a
    // top-level, and it follows the cursor parent-relative, staying clamped to
    // the frame rect. The auto (top-level) branch is asserted first so the
    // platform decision itself is covered.
    frame.applyPanelSessionForTest(pictura::SessionState{});
    pumpFloat(6);
    const bool autoTopLevel = pictura::PanelFloat::overlayUsesTopLevel();
    pictura::PanelFloat::setForceChildOverlayForTest(true);
    const bool forcedChild = !pictura::PanelFloat::overlayUsesTopLevel();
    bool torn = false;
    bool child = false;
    bool follows = false;
    bool clamped = false;
    pictura::PanelColumn* column = frame.panelColumn();
    if (column) {
        pictura::PanelGroup* group = nullptr;
        for (pictura::PanelGroup* candidate : column->groups()) {
            if (candidate && candidate->titleCountForTest() > 0) {
                group = candidate;
                break;
            }
        }
        if (group && group->visiblePanels().isEmpty() && !group->panels().isEmpty()
            && group->panels().first()) {
            column->showPanel(group->panels().first()->objectName(), true);
            pumpFloat(4);
        }
        const QString panel = group && !group->visiblePanels().isEmpty()
                                  && group->visiblePanels().first()
                              ? group->visiblePanels().first()->objectName()
                              : QString();
        if (!panel.isEmpty()) {
            torn = column->tearOffForTest(panel);
            pumpFloat(8);
            const int index = column->floatCountForTest() - 1;
            pictura::PanelFloat* floatWindow = column->floatForTest(index);
            child = torn && floatWindow && !floatWindow->isWindow()
                    && qobject_cast<QMainWindow*>(floatWindow->parentWidget()) == &frame;
            if (child) {
                const QPoint a(
                    frame.mapToGlobal(QPoint(frame.width() / 2, frame.height() / 4)));
                const QPoint b = a + QPoint(4, 18);
                const bool movedA = column->floatClampedForTest(index, a);
                const QPoint posA = floatWindow->pos();
                const bool movedB = column->floatClampedForTest(index, b);
                const QPoint posB = floatWindow->pos();
                // Parent-relative `pos()` advances exactly with the cursor.
                follows = movedA && movedB && (posB - posA) == (b - a);
                const QRect frameRect(frame.mapToGlobal(QPoint(0, 0)), frame.size());
                clamped = frameRect.contains(column->floatGeometryForTest(index));
            }
            column->closeFloatForTest(index);
            pumpFloat(6);
        }
    }
    pictura::PanelFloat::setForceChildOverlayForTest(false);
    const bool restored = pictura::PanelFloat::overlayUsesTopLevel() == autoTopLevel;
    ST_BEGIN("float_child_overlay");
    ST_PASS("float_child_overlay auto_tl=%d forced=%d torn=%d child=%d follows=%d "
            "clamped=%d restored=%d",
            autoTopLevel ? 1 : 0, forcedChild ? 1 : 0, torn ? 1 : 0, child ? 1 : 0,
            follows ? 1 : 0, clamped ? 1 : 0, restored ? 1 : 0);
    if (!(autoTopLevel && forcedChild && torn && child && follows && clamped && restored)) {
        return pictura::selfTest().fail(440, "child float overlay");
    }
    frame.applyPanelSessionForTest(pictura::SessionState{});
    pumpFloat(6);

    // docked_tools_fixed_width (441): while docked the Tools column is a
    // fixed-width splitter pane: its width range is pinned to the content width,
    // and the splitter handle beside it has 0 width, so there is no draggable
    // seam and no drag can resize the toolbar.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        auto* tools = frame.toolsColumn();
        bool isPane = false;
        bool pinned = false;
        bool handleFlat = false;
        if (splitter && tools) {
            const int idx = splitter->indexOf(tools);
            isPane = idx >= 0;
            pinned = tools->minimumWidth() == tools->maximumWidth()
                     && tools->minimumWidth() > 0;
            QSplitterHandle* handle = nullptr;
            if (idx > 0) {
                handle = splitter->handle(idx - 1);
            } else if (idx == 0) {
                handle = splitter->handle(0);
            }
            handleFlat = handle && handle->width() == 0;
        }
        ST_BEGIN("docked_tools_fixed_width");
        ST_PASS("docked_tools_fixed_width pane=%d pinned=%d handle_flat=%d", isPane ? 1 : 0,
                pinned ? 1 : 0, handleFlat ? 1 : 0);
        if (!(isPane && pinned && handleFlat)) {
            return pictura::selfTest().fail(441, "docked tools fixed width");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // floating_tools_toggle_resize (442): toggling the floating tools overlay
    // between one and two tool columns re-fits the overlay to the new content on
    // both axes — wider and shorter — so a two-column grid is neither clipped
    // nor left at the one-column height.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        auto* tools = frame.toolsColumn();
        auto* toolbox = frame.findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
        bool floated = false;
        bool grew = false;
        bool matches = false;
        bool fitsHeight = false;
        if (tools && toolbox) {
            pictura::PanelFloat* floatWindow = floatToolsColumnForTest(frame, tools);
            floated = floatWindow && floatWindow->isVisible();
            if (floated) {
                const int beforeWidth = floatWindow->width();
                const int beforeHeight = floatWindow->height();
                const int beforeToolboxHeight = toolbox->sizeHint().height();
                QToolButton* toggle = tools->panelColumnToggleForTest();
                if (toggle) {
                    toggle->click();
                    pumpFloat(6);
                }
                const int afterWidth = floatWindow->width();
                const int afterHeight = floatWindow->height();
                grew = toolbox->columns() == 2 && afterWidth > beforeWidth;
                matches = afterWidth == tools->minimumWidth();
                // A short screen caps the overlay height in both modes, so
                // check that the grid itself got shorter and the overlay fits
                // the column's current hint.
                fitsHeight = toolbox->sizeHint().height() < beforeToolboxHeight
                             && afterHeight <= tools->sizeHint().height() + 8;
                if (toggle) {
                    toggle->click();
                    pumpFloat(6);
                }
            }
        }
        ST_BEGIN("floating_tools_toggle_resize");
        ST_PASS("floating_tools_toggle_resize floated=%d grew=%d matches=%d fits_h=%d",
                floated ? 1 : 0, grew ? 1 : 0, matches ? 1 : 0, fitsHeight ? 1 : 0);
        if (!(floated && grew && matches && fitsHeight)) {
            return pictura::selfTest().fail(442, "floating tools toggle resize");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_column_drag_indicator (443): dragging an already-floating column by
    // the same header path as a docked one resolves the target and draws the
    // edge indicator. In child (Wayland) mode the overlay follows the cursor
    // above the splitter and would cover a viewport-drawn line, so the mark is a
    // frame-level widget raised over the overlay; release re-docks the column.
    // A bare workspace edge marks the central area's own edge: dragging to the
    // leftmost side draws the line at the left, not on a right-hand column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        auto* tools = frame.toolsColumn();
        auto* primary = frame.panelColumn();
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        pictura::PanelFloat::setForceChildOverlayForTest(true);
        bool floated = false;
        bool indicator = false;
        bool onFrame = false;
        bool atLeft = false;
        bool redocked = false;
        if (tools && primary && splitter) {
            // Earlier checks can leave the primary column hidden once its last
            // visible panel closed, and reordered to the left edge. Re-show a
            // panel and move the primary to the workspace's right so the left
            // band this check drags to is a bare workspace edge, not a column.
            primary->showPanel(QStringLiteral("layersPanel"), true);
            frame.movePanelColumn(primary, 1, nullptr);
            pumpFloat(4);
            pictura::PanelFloat* floatWindow = floatToolsColumnForTest(frame, tools);
            floated = floatWindow && floatWindow->isVisible();
            if (floated) {
                QWidget* central = frame.centralWidget();
                const int centralLeft = central->mapToGlobal(QPoint(0, 0)).x();
                const QPoint over(central->mapToGlobal(QPoint(2, central->height() / 2)));
                tools->beginColumnHeaderDragForTest(
                    floatWindow->mapToGlobal(QPoint(floatWindow->width() / 2, 8)));
                tools->dragColumnHeaderToForTest(over);
                indicator = primary->dropIndicatorVisibleForTest();
                onFrame = primary->edgeIndicatorOnFrameForTest();
                const QRect mark = primary->dropIndicatorGlobalGeometryForTest();
                atLeft = !mark.isNull() && qAbs(mark.left() - centralLeft) <= 2
                         && mark.height() > mark.width();
                redocked = tools->dropColumnHeaderForTest(over)
                           && tools->columnFloatForTest() == nullptr
                           && splitter->indexOf(tools) >= 0 && !tools->isWindow();
                pumpFloat(6);
            }
        }
        pictura::PanelFloat::setForceChildOverlayForTest(false);
        ST_BEGIN("float_column_drag_indicator");
        ST_PASS("float_column_drag_indicator floated=%d indicator=%d on_frame=%d "
                "at_left=%d redocked=%d",
                floated ? 1 : 0, indicator ? 1 : 0, onFrame ? 1 : 0, atLeft ? 1 : 0,
                redocked ? 1 : 0);
        if (!(floated && indicator && onFrame && atLeft && redocked)) {
            return pictura::selfTest().fail(443, "float column drag indicator");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // tools_side_mark (444): a widget panel dragged over the docked Tools body
    // draws no mark (atomic), but a drag into the new-column band immediately
    // beside the Tools column draws the new-column mark at that edge.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool bodyOff = false;
        bool sideOn = false;
        bool onFrame = false;
        if (tools && primary) {
            const QPoint body(tools->mapToGlobal(tools->rect().center()));
            const bool beganBody = primary->beginTabDragForTest(QStringLiteral("layersPanel"));
            primary->dragToForTest(body);
            bodyOff = beganBody && !tools->dropIndicatorVisibleForTest();
            primary->cancelDragForTest();
            pumpFloat(4);
            const QRect tr(tools->mapToGlobal(QPoint(0, 0)), tools->size());
            const QPoint beside(tr.right() + 8, tr.center().y());
            const bool beganSide = primary->beginTabDragForTest(QStringLiteral("layersPanel"));
            primary->dragToForTest(beside);
            sideOn = beganSide && tools->dropIndicatorVisibleForTest();
            onFrame = tools->edgeIndicatorOnFrameForTest();
            primary->cancelDragForTest();
            pumpFloat(4);
        }
        ST_BEGIN("tools_side_mark");
        ST_PASS("tools_side_mark body_off=%d side_on=%d on_frame=%d", bodyOff ? 1 : 0,
                sideOn ? 1 : 0, onFrame ? 1 : 0);
        if (!(bodyOff && sideOn && onFrame)) {
            return pictura::selfTest().fail(444, "tools side mark");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_icon_no_grip (445): a collapsed floating group hides the resize grip
    // and shrink-wraps both axes to the icon row; expanding restores the grip
    // and the shared normal minimum width.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool gripOff = false;
        bool sized = false;
        bool gripBack = false;
        if (column) {
            pictura::PanelGroup* group = liveGroup(column);
            pumpFloat(4);
            const QString panel = revealedPanel(column, group);
            pumpFloat(4);
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pumpFloat(8);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* fw = column->floatForTest(index);
                if (torn && fw && fw->group() == group) {
                    group->setCollapsedToIcons(true);
                    pumpFloat(8);
                    QWidget* grip = fw->sizeGripForTest();
                    gripOff = grip && !grip->isVisible();
                    const int iw = qMax(1, group->sizeHint().width());
                    const int ih = qMax(group->sizeHint().height(),
                                        pictura::PanelFloat::kFloatIconMinHeight);
                    sized = fw->width() <= iw + 2
                            && fw->height() >= pictura::PanelFloat::kFloatIconMinHeight
                            && fw->height() <= ih + 8;
                    group->setCollapsedToIcons(false);
                    pumpFloat(8);
                    gripBack = grip && grip->isVisible()
                               && fw->minimumWidth() >= pictura::PanelFloat::kFloatMinWidth;
                }
                closeFloats(column);
                pumpFloat(6);
            }
        }
        ST_BEGIN("float_icon_no_grip");
        ST_PASS("float_icon_no_grip torn=%d grip_off=%d sized=%d grip_back=%d", torn ? 1 : 0,
                gripOff ? 1 : 0, sized ? 1 : 0, gripBack ? 1 : 0);
        if (!(torn && gripOff && sized && gripBack)) {
            return pictura::selfTest().fail(445, "float icon no grip");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_icons_vertical (446): a floating group collapsed to icons stacks its
    // icons one per row in a vertical column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool stacked = false;
        int iconCount = 0;
        if (column) {
            // `applyPanelSessionForTest` does not rebuild the default groups, so
            // an earlier drag may have left a different arrangement. Pick the
            // group with the most tabs and show them all, so the fixture always
            // has a multi-panel group to collapse.
            pictura::PanelGroup* group = nullptr;
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate
                    && (!group || candidate->panels().size() > group->panels().size())) {
                    group = candidate;
                }
            }
            if (group) {
                const QList<QWidget*> members = group->panels();
                for (QWidget* panel : members) {
                    if (panel) {
                        column->showPanel(panel->objectName(), true);
                    }
                }
            }
            pumpFloat(8);
            if (group && group->visibleTitles().size() >= 2) {
                const QString panel = group->visiblePanels().first()->objectName();
                torn = column->tearOffForTest(panel);
                pumpFloat(8);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* fw = column->floatForTest(index);
                group = fw ? fw->group() : nullptr;
                if (torn && group) {
                    group->setCollapsedToIcons(true);
                    pumpFloat(8);
                    QList<QToolButton*> icons;
                    for (QToolButton* button : group->findChildren<QToolButton*>()) {
                        if (button
                            && button->objectName().startsWith(
                                QStringLiteral("panelGroupIcon_"))
                            && button->isVisible()) {
                            icons << button;
                        }
                    }
                    iconCount = icons.size();
                    stacked = icons.size() >= 2;
                    for (int i = 1; i < icons.size() && stacked; ++i) {
                        const QPoint a = icons.at(i - 1)->mapToGlobal(QPoint(0, 0));
                        const QPoint b = icons.at(i)->mapToGlobal(QPoint(0, 0));
                        stacked = b.y() > a.y() && qAbs(b.x() - a.x()) <= 4;
                    }
                }
                closeFloats(column);
                pumpFloat(6);
            }
        }
        ST_BEGIN("float_icons_vertical");
        ST_PASS("float_icons_vertical torn=%d stacked=%d n=%d", torn ? 1 : 0,
                stacked ? 1 : 0, iconCount);
        if (!(torn && stacked)) {
            return pictura::selfTest().fail(446, "float icons vertical");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_icon_drag (447): an icon in a floating collapsed group starts a panel
    // drag through the same tab-drag grammar the docked strip uses; a real press
    // + release below the drag threshold still opens the panel (the new event
    // filter does not swallow a click); and a released drag lands the panel in a
    // docked group.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool clicked = false;
        bool signalled = false;
        bool dragging = false;
        bool landed = false;
        if (column) {
            pictura::PanelGroup* group = liveGroup(column);
            pumpFloat(4);
            QString panel;
            if (group) {
                panel = revealedPanel(column, group);
                // Reveal a second tab so the icon drag moves a single panel; a
                // whole-group drag would consume the docked group and perturb the
                // later checks' accumulated layout.
                for (QWidget* candidate : group->panels()) {
                    if (candidate && candidate->objectName() != panel) {
                        column->showPanel(candidate->objectName(), true);
                        break;
                    }
                }
            }
            pumpFloat(6);
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pumpFloat(8);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* fw = column->floatForTest(index);
                group = fw ? fw->group() : nullptr;
                if (torn && group) {
                    group->setCollapsedToIcons(true);
                    pumpFloat(8);
                    auto iconFor = [group, &panel]() {
                        return group->findChild<QToolButton*>(
                            QStringLiteral("panelGroupIcon_") + panel);
                    };
                    QToolButton* icon = iconFor();
                    if (icon) {
                        // Below the threshold: press+release must still open the
                        // panel through the `clicked` -> `panelActivated` path.
                        const QPointF local(icon->rect().center());
                        const QPointF global(icon->mapToGlobal(icon->rect().center()));
                        QMouseEvent clickPress(QEvent::MouseButtonPress, local, global,
                                               Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(icon, &clickPress);
                        QMouseEvent clickRelease(QEvent::MouseButtonRelease, local, global,
                                                 Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(icon, &clickRelease);
                        pumpFloat(8);
                        clicked = column->iconFlyoutVisibleForTest();
                        column->triggerFlyoutCloseForTest();
                        pumpFloat(8);
                    }
                    // Closing the flyout rebuilds the icon row; re-fetch it.
                    icon = iconFor();
                    if (icon) {
                        // Park the source overlay so it cannot shadow the drop
                        // point, then reveal a docked group so the column has a
                        // real target surface.
                        column->floatClampedForTest(
                            index, frame.mapToGlobal(QPoint(frame.width() - 260, 60)));
                        pumpFloat(6);
                        for (pictura::PanelGroup* candidate : column->groups()) {
                            if (candidate && candidate->titleCountForTest() > 0) {
                                revealedPanel(column, candidate);
                                break;
                            }
                        }
                        pumpFloat(6);
                        const QPoint drop = column->boundaryPointForTest(1);
                        QString fired;
                        const QMetaObject::Connection conn = QObject::connect(
                            group, &pictura::PanelGroup::tabDragStarted, group,
                            [&fired](const QString& name, const QPoint&) { fired = name; });
                        const QPointF local(icon->rect().center());
                        const QPointF global(icon->mapToGlobal(icon->rect().center()));
                        QMouseEvent press(QEvent::MouseButtonPress, local, global,
                                          Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(icon, &press);
                        const QPointF moved(drop);
                        QMouseEvent move(QEvent::MouseMove, local, moved, Qt::NoButton,
                                         Qt::LeftButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(icon, &move);
                        signalled = fired == panel;
                        dragging = column->dragActiveForTest();
                        QMouseEvent release(QEvent::MouseButtonRelease, local, moved,
                                            Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(icon, &release);
                        pumpFloat(8);
                        QObject::disconnect(conn);
                        landed = column->groupForPanel(panel) != nullptr;
                    }
                }
                closeFloats(column);
                pumpFloat(6);
            }
        }
        ST_BEGIN("float_icon_drag");
        ST_PASS("float_icon_drag torn=%d clicked=%d signalled=%d dragging=%d landed=%d",
                torn ? 1 : 0, clicked ? 1 : 0, signalled ? 1 : 0, dragging ? 1 : 0,
                landed ? 1 : 0);
        if (!(torn && clicked && signalled && dragging && landed)) {
            return pictura::selfTest().fail(447, "float icon drag");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // group_body_outline (448): a whole group dragged over another column's group
    // body resolves a tabify target — the blue region outline is drawn around
    // that group and the release merges the two groups.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool made = false;
        bool shown = false;
        bool merged = false;
        if (primary) {
            QList<pictura::PanelGroup*> live;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && group->titleCountForTest() > 0) {
                    live << group;
                }
            }
            if (live.size() >= 2) {
                revealedPanel(primary, live.at(0));
                revealedPanel(primary, live.at(1));
                pumpFloat(6);
            }
            pictura::PanelColumn* dest =
                frame.createPanelColumn(pictura::PanelSide::Right, primary);
            pictura::PanelGroup* host = nullptr;
            if (dest) {
                for (pictura::PanelGroup* group : primary->groups()) {
                    if (group && !group->visibleTitles().isEmpty()) {
                        if (pictura::PanelGroup* taken =
                                primary->takeGroup(group->objectName())) {
                            dest->addGroup(taken);
                            host = taken;
                        }
                        break;
                    }
                }
            }
            pumpFloat(8);
            pictura::PanelGroup* source = nullptr;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && !group->visibleTitles().isEmpty()) {
                    source = group;
                    break;
                }
            }
            const QString srcPanel =
                source && !source->visiblePanels().isEmpty()
                    ? source->visiblePanels().first()->objectName()
                    : QString();
            made = dest && host && source && !srcPanel.isEmpty();
            if (made) {
                const QRect gr(host->mapToGlobal(QPoint(0, 0)), host->size());
                const QRect bar = host->tabBarGlobalRect();
                const QPoint body(gr.center().x(),
                                  (bar.bottom() + gr.bottom()) / 2);
                const bool began = primary->beginGroupDragForTest(srcPanel);
                primary->dragToForTest(body);
                shown = began && dest->outlineIndicatorVisibleForTest();
                const bool dropped = primary->dropForTest(body);
                pumpFloat(8);
                merged = dropped && dest->groupForPanel(srcPanel) == host;
            }
        }
        ST_BEGIN("group_body_outline");
        ST_PASS("group_body_outline made=%d shown=%d merged=%d", made ? 1 : 0,
                shown ? 1 : 0, merged ? 1 : 0);
        if (!(made && shown && merged)) {
            return pictura::selfTest().fail(448, "group body outline");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // min_width_300 (449): a normal-mode widget column and a floating overlay
    // each report a minimum width of at least 300.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool colOk = false;
        bool constOk = false;
        bool floatOk = false;
        int colMin = 0;
        int floatMin = 0;
        if (column) {
            colMin = column->minimumWidthForTest();
            colOk = colMin >= 300 && column->minimumWidthFloorForTest() >= 300;
            constOk = pictura::PanelFloat::kFloatMinWidth >= 300;
            pictura::PanelGroup* group = liveGroup(column);
            pumpFloat(4);
            const QString panel = revealedPanel(column, group);
            pumpFloat(4);
            if (!panel.isEmpty()) {
                column->tearOffForTest(panel);
                pumpFloat(8);
                pictura::PanelFloat* fw = column->floatForTest(column->floatCountForTest() - 1);
                floatMin = fw ? fw->minimumWidth() : 0;
                floatOk = fw && floatMin >= 300;
                closeFloats(column);
                pumpFloat(6);
            }
        }
        ST_BEGIN("min_width_300");
        ST_PASS("min_width_300 col=%d const=%d float=%d min=%d fmin=%d", colOk ? 1 : 0,
                constOk ? 1 : 0, floatOk ? 1 : 0, colMin, floatMin);
        if (!(colOk && constOk && floatOk)) {
            return pictura::selfTest().fail(449, "min width 300");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_icon_group_merge (450): a collapsed floating group dragged by the
    // group grip onto another floating overlay merges its panels into it.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool twoFloats = false;
        bool collapsed = false;
        bool merged = false;
        if (column) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : column->groups()) {
                if (group && group->titleCountForTest() > 0) {
                    groups << group;
                }
            }
            if (groups.size() >= 2) {
                pictura::PanelGroup* gA = groups.at(0);
                pictura::PanelGroup* gB = groups.at(1);
                const QString a = revealedPanel(column, gA);
                const QString b = revealedPanel(column, gB);
                pumpFloat(8);
                // Float the *target* group first and park it, so the source
                // overlay is created later and lands after it in the float list.
                // The resolver only inspects the first overlay under the cursor,
                // so the target must not be shadowed by the following source.
                const bool tB = !b.isEmpty() && column->tearOffForTest(b);
                pumpFloat(8);
                for (int i = 0; i < column->floatCountForTest(); ++i) {
                    pictura::PanelFloat* fw = column->floatForTest(i);
                    if (fw && fw->group() == gB) {
                        column->floatClampedForTest(
                            i, frame.mapToGlobal(QPoint(frame.width() - 260, 60)));
                        break;
                    }
                }
                pumpFloat(6);
                const bool tA = !a.isEmpty() && column->tearOffForTest(a);
                pumpFloat(8);
                pictura::PanelFloat* fA = nullptr;
                pictura::PanelFloat* fB = nullptr;
                for (int i = 0; i < column->floatCountForTest(); ++i) {
                    pictura::PanelFloat* fw = column->floatForTest(i);
                    if (fw && fw->group() == gA) {
                        fA = fw;
                    }
                    if (fw && fw->group() == gB) {
                        fB = fw;
                    }
                }
                twoFloats = tA && tB && fA && fB;
                if (twoFloats) {
                    gA->setCollapsedToIcons(true);
                    pumpFloat(8);
                    collapsed = gA->isCollapsedToIcons();
                    const QPoint target = gB->tabInsertionGlobalPointForTest(0);
                    const bool began = column->beginGroupDragForTest(a);
                    column->dragToForTest(target);
                    const bool dropped = column->dropForTest(target);
                    pumpFloat(8);
                    merged = began && dropped && fB->group()->containsPanel(a)
                             && column->floatCountForTest() == 1;
                }
            }
        }
        closeFloats(column);
        ST_BEGIN("float_icon_group_merge");
        ST_PASS("float_icon_group_merge two=%d collapsed=%d merged=%d", twoFloats ? 1 : 0,
                collapsed ? 1 : 0, merged ? 1 : 0);
        if (!(twoFloats && collapsed && merged)) {
            return pictura::selfTest().fail(450, "float icon group merge");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // group_body_outline_same_column (451): a whole-group drag over a *different*
    // group's body in the SAME column draws the blue region outline and merges on
    // release — also when the dragged group was torn off into a float — while a
    // drop on the dragged group's own body stays an above/below reorder boundary
    // (no outline, the thin insertion line).
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool ownBoundary = false;
        bool dockedMerge = false;
        bool floatMerge = false;
        if (primary) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && group->titleCountForTest() > 0) {
                    groups << group;
                }
            }
            if (groups.size() >= 2) {
                pictura::PanelGroup* target = groups.at(0);
                pictura::PanelGroup* source = groups.at(1);
                const QString t = revealedPanel(primary, target);
                const QString s = revealedPanel(primary, source);
                pumpFloat(8);
                if (!t.isEmpty() && !s.isEmpty()) {
                    // Own body: an above/below boundary, never the tabify outline.
                    const QRect own(source->mapToGlobal(QPoint(0, 0)), source->size());
                    const QRect ownBar = source->tabBarGlobalRect();
                    const int ownTop = qMax(own.top(), ownBar.bottom());
                    const QPoint ownBody(own.center().x(), ownTop + 8);
                    const bool beganOwn = primary->beginGroupDragForTest(s);
                    primary->dragToForTest(ownBody);
                    ownBoundary = beganOwn && primary->dropIndicatorVisibleForTest()
                                  && !primary->outlineIndicatorVisibleForTest();
                    primary->cancelDragForTest();
                    pumpFloat(6);

                    // A different group's body in the same column: outline + merge.
                    const QRect gr(target->mapToGlobal(QPoint(0, 0)), target->size());
                    const QRect bar = target->tabBarGlobalRect();
                    const int top = qMax(gr.top(), bar.bottom());
                    const QPoint body(gr.center().x(), top + (gr.bottom() - top) / 2);
                    const bool began = primary->beginGroupDragForTest(s);
                    primary->dragToForTest(body);
                    const bool outline = primary->outlineIndicatorVisibleForTest();
                    const bool dropped = primary->dropForTest(body);
                    pumpFloat(8);
                    dockedMerge = began && outline && dropped && target->containsPanel(s);
                }
            }
        }
        // The same body drop from a torn-off float back into the same column: the
        // user's tear-off-then-drop case.
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        primary = frame.panelColumn();
        if (primary) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && group->titleCountForTest() > 0) {
                    groups << group;
                }
            }
            if (groups.size() >= 2) {
                const QString s = revealedPanel(primary, groups.at(1));
                revealedPanel(primary, groups.at(0));
                pumpFloat(8);
                const bool torn = !s.isEmpty() && primary->tearOffForTest(s);
                pumpFloat(8);
                // Re-fetch the docked target: the tear-off re-lays out the column.
                pictura::PanelGroup* docked = nullptr;
                for (pictura::PanelGroup* group : primary->groups()) {
                    if (group && group->titleCountForTest() > 0) {
                        docked = group;
                        break;
                    }
                }
                if (torn && docked) {
                    revealedPanel(primary, docked);
                    pumpFloat(6);
                    const QRect gr(docked->mapToGlobal(QPoint(0, 0)), docked->size());
                    const QRect bar = docked->tabBarGlobalRect();
                    const int top = qMax(gr.top(), bar.bottom());
                    const QPoint body(gr.center().x(), top + (gr.bottom() - top) / 2);
                    const bool began = primary->beginGroupDragForTest(s);
                    primary->dragToForTest(body);
                    const bool outline = primary->outlineIndicatorVisibleForTest();
                    const bool dropped = primary->dropForTest(body);
                    pumpFloat(8);
                    floatMerge = began && outline && dropped && docked->containsPanel(s)
                                 && primary->floatCountForTest() == 0;
                }
                closeFloats(primary);
                pumpFloat(6);
            }
        }
        ST_BEGIN("group_body_outline_same_column");
        ST_PASS("group_body_outline_same_column own=%d docked=%d float=%d",
                ownBoundary ? 1 : 0, dockedMerge ? 1 : 0, floatMerge ? 1 : 0);
        if (!(ownBoundary && dockedMerge && floatMerge)) {
            return pictura::selfTest().fail(451, "group body same-column outline");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }
    // group_drag_float_follows (438): a group dragged over a resolved target
    // inside its own column stays docked and keeps the tabify indicator; only
    // when the pointer leaves every column does the group tear into an overlay;
    // the release re-docks the group's panels into the target group. (Overlay
    // cursor tracking is covered by tools_column_drag_float, where the exact
    // delta is measurable.)
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool began = false, inside = false, floated = false, indicator = false, redocked = false;
        if (primary) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* candidate : primary->groups()) {
                if (candidate && candidate->titleCountForTest() > 0
                    && !candidate->isCollapsedToIcons()) {
                    groups << candidate;
                }
            }
            if (groups.size() >= 2) {
                pictura::PanelGroup* source = groups.at(1);
                pictura::PanelGroup* target = groups.at(0);
                for (pictura::PanelGroup* group : {source, target}) {
                    if (group->visiblePanels().isEmpty() && !group->panels().isEmpty()
                        && group->panels().first()) {
                        primary->showPanel(group->panels().first()->objectName(), true);
                    }
                }
                pumpFloat(4);
                if (!source->visiblePanels().isEmpty() && source->visiblePanels().first()) {
                    const QString panel = source->visiblePanels().first()->objectName();
                    const QString targetPanel = target->visiblePanels().isEmpty()
                                                    ? QString()
                                                    : target->visiblePanels().first()->objectName();
                    const int floatsBefore = primary->floatCountForTest();
                    // The groups can be taller than the column viewport; reveal
                    // the source to start the drag, then scroll to the target
                    // before resolving the drop point.
                    primary->ensureGroupVisibleForTest(panel);
                    pumpFloat(4);
                    began = primary->beginGroupDragForTest(panel);
                    if (!targetPanel.isEmpty()) {
                        primary->ensureGroupVisibleForTest(targetPanel);
                        pumpFloat(4);
                    }
                    const QPoint a = target->tabBarGlobalRect().center();
                    primary->dragToForTest(a);
                    inside = primary->floatCountForTest() == floatsBefore
                             && (primary->outlineIndicatorVisibleForTest()
                                 || primary->dropIndicatorVisibleForTest());
                    indicator = primary->outlineIndicatorVisibleForTest()
                                || primary->dropIndicatorVisibleForTest();
                    // Leaving the workspace below the column tears the group off
                    // into an overlay (the left/right margins resolve a new-column
                    // target, so the free area below is the unambiguous outside).
                    const QPoint c =
                        primary->mapToGlobal(QPoint(primary->width() / 2, primary->height() + 40));
                    primary->dragToForTest(c);
                    const int index = primary->floatCountForTest() - 1;
                    pictura::PanelFloat* floatWindow = primary->floatForTest(index);
                    floated = index >= 0 && floatWindow
                              && primary->floatCountForTest() == floatsBefore + 1;
                    // Drop back on the tab bar: the whole group tabifies in, so
                    // every panel is docked again and no overlay remains.
                    const bool dropped = primary->dropForTest(a);
                    pumpFloat(6);
                    redocked = dropped && primary->floatCountForTest() == floatsBefore
                               && !primary->dragActiveForTest()
                               && primary->groupForPanel(panel) != nullptr;
                }
            }
        }
        ST_BEGIN("group_drag_float_follows");
        ST_PASS("group_drag_float_follows began=%d inside=%d floated=%d indicator=%d redocked=%d",
                began ? 1 : 0, inside ? 1 : 0, floated ? 1 : 0, indicator ? 1 : 0,
                redocked ? 1 : 0);
        if (!(began && inside && floated && indicator && redocked)) {
            return pictura::selfTest().fail(438, "group drag float follows");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }

    // float_widget_menu_close (453): the per-widget header menu's Close and
    // Close Group act on a floated group too — Close hides the active tab and
    // removes the emptied overlay, and Close Group removes the overlay instead of
    // leaving an empty ghost with a group no column can find.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool floatTorn = false, closedPanel = false, panelDocked = false, groupTorn = false,
             closedGroup = false, groupDocked = false;
        if (primary) {
            const int before = primary->floatCountForTest();
            pictura::PanelGroup* group = nullptr;
            for (pictura::PanelGroup* candidate : primary->groups()) {
                if (candidate && candidate->titleCountForTest() > 0) {
                    group = candidate;
                    break;
                }
            }
            const QString panel = revealedPanel(primary, group);
            pumpFloat(6);
            if (!panel.isEmpty() && primary->tearOffPanelForTest(panel)) {
                pumpFloat(8);
                const int index = primary->floatCountForTest() - 1;
                pictura::PanelFloat* overlay = primary->floatForTest(index);
                pictura::PanelGroup* floated = overlay ? overlay->group() : nullptr;
                floatTorn = index >= 0 && floated != nullptr;
                if (floated) {
                    const bool asked = floated->triggerPanelMenuForTest(QStringLiteral("Close"));
                    pumpFloat(8);
                    closedPanel = asked && primary->floatCountForTest() == before
                                  && !primary->isPanelVisible(panel);
                    panelDocked = primary->groupForPanel(panel) != nullptr;
                }
            }
            // Close Group on a whole floated group.
            if (!panel.isEmpty() && primary->tearOffForTest(panel)) {
                pumpFloat(8);
                const int index = primary->floatCountForTest() - 1;
                pictura::PanelFloat* overlay = primary->floatForTest(index);
                pictura::PanelGroup* floated = overlay ? overlay->group() : nullptr;
                groupTorn = index >= 0 && floated != nullptr;
                if (floated) {
                    const bool asked =
                        floated->triggerPanelMenuForTest(QStringLiteral("Close Tab Group"));
                    pumpFloat(8);
                    closedGroup = asked && primary->floatCountForTest() == before
                                  && !primary->isPanelVisible(panel);
                    groupDocked = primary->groupForPanel(panel) != nullptr;
                }
            }
        }
        ST_BEGIN("float_widget_menu_close");
        ST_PASS("float_widget_menu_close torn=%d panel=%d dock=%d groupTorn=%d group=%d gdock=%d",
                floatTorn ? 1 : 0, closedPanel ? 1 : 0, panelDocked ? 1 : 0, groupTorn ? 1 : 0,
                closedGroup ? 1 : 0, groupDocked ? 1 : 0);
        if (!(floatTorn && closedPanel && panelDocked && groupTorn && closedGroup && groupDocked)) {
            return pictura::selfTest().fail(453, "float widget menu close");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }
    // column_end_drop (454): a widget dropped at the very top or bottom of a
    // column lands as a new first/last group, not only in the gaps between the
    // existing groups.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool topOk = false, bottomOk = false;
        if (primary) {
            QStringList visible;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (!group) {
                    continue;
                }
                revealedPanel(primary, group);
            }
            pumpFloat(4);
            for (pictura::PanelGroup* group : primary->groups()) {
                if (!group) {
                    continue;
                }
                for (QWidget* panel : group->visiblePanels()) {
                    if (panel) {
                        visible << panel->objectName();
                    }
                }
            }
            const QString first = visible.value(0);
            const QString second = visible.value(1);
            if (!first.isEmpty()) {
                const QPoint top = primary->boundaryPointForTest(0);
                primary->beginTabDragForTest(first);
                primary->dragToForTest(top);
                const bool dropped = primary->dropForTest(top);
                pumpFloat(6);
                pictura::PanelGroup* landed = primary->groupForPanel(first);
                topOk = dropped && !primary->groups().isEmpty() && primary->groups().first() == landed;
            }
            if (!second.isEmpty()) {
                const QPoint bottom = primary->boundaryPointForTest(1000);
                primary->beginTabDragForTest(second);
                primary->dragToForTest(bottom);
                const bool dropped = primary->dropForTest(bottom);
                pumpFloat(6);
                pictura::PanelGroup* landed = primary->groupForPanel(second);
                bottomOk = dropped && !primary->groups().isEmpty()
                           && primary->groups().last() == landed;
            }
        }
        ST_BEGIN("column_end_drop");
        ST_PASS("column_end_drop top=%d bottom=%d", topOk ? 1 : 0, bottomOk ? 1 : 0);
        if (!(topOk && bottomOk)) {
            return pictura::selfTest().fail(454, "column end drop");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pumpFloat(6);
    }
    return 0;
}
