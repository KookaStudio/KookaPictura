#include "selftest_shell_round4.h"
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
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

namespace {

// One bounded event pump so offscreen geometry is real.
void pump4(int count)
{
    for (int i = 0; i < count; ++i) {
        QCoreApplication::processEvents();
    }
}

// A floating overlay is a frameless Qt::Tool top-level window parented to the
// main window: no decorations, no taskbar entry.
bool toolFloatWindow(QWidget* w, QWidget* parent)
{
    return w && w->isWindow() && w->windowFlags().testFlag(Qt::Tool)
           && w->windowFlags().testFlag(Qt::FramelessWindowHint)
           && w->parentWidget() == parent;
}

} // namespace

int pictura::runShellRound4Checks(pictura::PicturaMainWindow& frame)
{
    // lsc_column_float (420): a whole widget column dragged by its header with no
    // resolved column target tears off into a frameless `Qt::Tool` overlay
    // (parented to the main window, never a decorated OS window) that keeps the
    // shared minimum width and a resize grip; a later release on a workspace edge
    // re-docks the whole column into the splitter and removes the overlay.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        for (int i = 0; i < 6; ++i) {
            QCoreApplication::processEvents();
        }

        pictura::PanelColumn* column = frame.panelColumn();
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        QWidget* tabs = frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
        QWidget* central = frame.centralWidget();

        bool floated = false;
        bool toolWindow = false;
        bool minWidth = false;
        bool grip = false;
        bool redocked = false;
        if (column && splitter && tabs && central) {
            const QPoint start =
                column->mapToGlobal(QPoint(qMax(1, column->width() / 2), 8));
            column->beginColumnHeaderDragForTest(start);
            // The document area resolves no column and no workspace edge, so the
            // whole column follows the cursor as a frameless tool window.
            const QPoint empty = tabs->mapToGlobal(tabs->rect().center());
            column->dragColumnHeaderToForTest(empty);
            pictura::PanelFloat* floatWindow = column->columnFloatForTest();
            floated = floatWindow != nullptr && splitter->indexOf(column) < 0;
            toolWindow = floated && toolFloatWindow(floatWindow, &frame)
                         && floatWindow->content() == column;
            minWidth = floated && floatWindow->minimumWidth() >= pictura::PanelFloat::kFloatMinWidth
                       && floatWindow->width() >= pictura::PanelFloat::kFloatMinWidth;
            grip = floated && floatWindow->sizeGripForTest() != nullptr;

            // A release in the workspace outer left band re-docks the column.
            const QPoint edge(central->mapToGlobal(QPoint(2, central->height() / 2)));
            redocked = column->dropColumnHeaderForTest(edge)
                       && column->columnFloatForTest() == nullptr
                       && splitter->indexOf(column) >= 0 && column->isVisible();
        }

        ST_BEGIN("lsc_column_float");
        ST_PASS("lsc_column_float floated=%d window=%d min=%d grip=%d redocked=%d",
                floated ? 1 : 0, toolWindow ? 1 : 0, minWidth ? 1 : 0, grip ? 1 : 0,
                redocked ? 1 : 0);
        if (!(floated && toolWindow && minWidth && grip && redocked)) {
            return pictura::selfTest().fail(420, "column float");
        }

        frame.applyPanelSessionForTest(pictura::SessionState{});
        for (int i = 0; i < 6; ++i) {
            QCoreApplication::processEvents();
        }
    }

    // tools_column_outer_band (421): an outer-band release of the tools column
    // commits a real sibling column (the indicator matches the result), not a
    // dock area: the column lands at the splitter tail and no overlay is left.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        QWidget* central = frame.centralWidget();
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool indicator = false;
        bool committed = false;
        bool landedAtTail = false;
        if (splitter && central && tools && primary) {
            const int countBefore = splitter->count();
            const QPoint edge(central->mapToGlobal(
                QPoint(central->width() - 2, central->height() / 2)));
            tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            tools->dragColumnHeaderToForTest(edge);
            indicator = primary->dropIndicatorVisibleForTest();
            const bool dropped = tools->dropColumnHeaderForTest(edge);
            pump4(6);
            committed = dropped && tools->columnFloatForTest() == nullptr
                        && splitter->indexOf(tools) >= 0 && splitter->count() == countBefore;
            landedAtTail = committed && splitter->indexOf(tools) == splitter->count() - 1;
        }
        ST_BEGIN("tools_column_outer_band");
        ST_PASS("tools_column_outer_band indicator=%d committed=%d tail=%d",
                indicator ? 1 : 0, committed ? 1 : 0, landedAtTail ? 1 : 0);
        if (!(indicator && committed && landedAtTail)) {
            return pictura::selfTest().fail(421, "tools outer band");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_column_float_tabless (422): the tools column floats as a frameless
    // tool window, is tabless (no `PanelGroup`s), and re-docks on a valid column
    // release.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        QWidget* tabs = frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool floated = false;
        bool tabless = false;
        bool redocked = false;
        if (splitter && tabs && tools && primary) {
            tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            tools->dragColumnHeaderToForTest(tabs->mapToGlobal(tabs->rect().center()));
            pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
            floated = floatWindow && toolFloatWindow(floatWindow, &frame)
                      && floatWindow->content() == tools && splitter->indexOf(tools) < 0;
            tabless = tools->isToolsColumn() && tools->groups().isEmpty()
                      && !tools->railMode();
            const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
            const QPoint left(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
            redocked = tools->dropColumnHeaderForTest(left)
                       && tools->columnFloatForTest() == nullptr
                       && splitter->indexOf(tools) >= 0 && !tools->isWindow();
        }
        ST_BEGIN("tools_column_float_tabless");
        ST_PASS("tools_column_float_tabless float=%d tabless=%d redock=%d",
                floated ? 1 : 0, tabless ? 1 : 0, redocked ? 1 : 0);
        if (!(floated && tabless && redocked)) {
            return pictura::selfTest().fail(422, "tools column float");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_column_atomic (423): atomic both ways. A widget-panel drag onto the
    // tools column resolves no indicator and does not combine; a tools-column
    // header drag onto a widget group resolves a sibling column, never
    // `IntoGroup` (the tools column never grows groups).
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool panelRejected = false;
        bool sibling = false;
        bool neverGroup = false;
        if (splitter && tools && primary) {
            const QPoint over(tools->mapToGlobal(tools->rect().center()));
            const bool began = primary->beginTabDragForTest(QStringLiteral("layersPanel"));
            primary->dragToForTest(over);
            const bool noIndicator = !tools->dropIndicatorVisibleForTest();
            const bool noCombine = !primary->dragIsPanelForTest() || tools->groups().isEmpty();
            primary->cancelDragForTest();
            pump4(4);
            panelRejected = began && noIndicator && noCombine && tools->groups().isEmpty();

            const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
            const QPoint onPrimary(pr.center());
            tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            tools->dragColumnHeaderToForTest(onPrimary);
            const bool toolsIndicator = primary->dropIndicatorVisibleForTest();
            const bool dropped = tools->dropColumnHeaderForTest(onPrimary);
            pump4(6);
            sibling = dropped && toolsIndicator && splitter->indexOf(tools) >= 0
                      && !tools->isWindow();
            neverGroup = tools->groups().isEmpty();
        }
        ST_BEGIN("tools_column_atomic");
        ST_PASS("tools_column_atomic panel_rejected=%d sibling=%d never_group=%d",
                panelRejected ? 1 : 0, sibling ? 1 : 0, neverGroup ? 1 : 0);
        if (!(panelRejected && sibling && neverGroup)) {
            return pictura::selfTest().fail(423, "tools atomic");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_column_boundary_rejected (424): a widget panel dropped above or
    // below the tools column is rejected (atomic): no indicator, no group, no
    // insertion into the tools column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool aboveRejected = false;
        bool belowRejected = false;
        bool atomic = false;
        if (tools && primary) {
            const QPoint above(tools->mapToGlobal(QPoint(tools->width() / 2, 4)));
            const QPoint below(tools->mapToGlobal(QPoint(tools->width() / 2,
                                                          tools->height() - 4)));
            primary->beginTabDragForTest(QStringLiteral("swatchesPanel"));
            primary->dragToForTest(above);
            aboveRejected = !tools->dropIndicatorVisibleForTest();
            primary->cancelDragForTest();
            pump4(4);
            primary->beginTabDragForTest(QStringLiteral("swatchesPanel"));
            primary->dragToForTest(below);
            belowRejected = !tools->dropIndicatorVisibleForTest();
            primary->cancelDragForTest();
            pump4(4);
            atomic = tools->isToolsColumn() && tools->groups().isEmpty();
        }
        ST_BEGIN("tools_column_boundary_rejected");
        ST_PASS("tools_column_boundary_rejected above=%d below=%d atomic=%d",
                aboveRejected ? 1 : 0, belowRejected ? 1 : 0, atomic ? 1 : 0);
        if (!(aboveRejected && belowRejected && atomic)) {
            return pictura::selfTest().fail(424, "tools boundary rejected");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_column_toggle (425): the tools header toggle switches the tool grid
    // between one and two columns (not normal/iconic rail mode).
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        auto* tools = frame.toolsColumn();
        auto* toolbox = frame.findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
        bool toggled = false;
        bool restored = false;
        bool railUnchanged = false;
        if (tools && toolbox) {
            QToolButton* toggle = tools->panelColumnToggleForTest();
            const int before = toolbox->columns();
            const bool railBefore = tools->railMode();
            if (toggle) {
                toggle->click();
                pump4(6);
            }
            const int after = toolbox->columns();
            toggled = toggle && before != after
                      && ((before == 1 && after == 2) || (before == 2 && after == 1));
            railUnchanged = !tools->railMode() && tools->railMode() == railBefore;
            if (toggle) {
                toggle->click();
                pump4(6);
            }
            restored = toolbox->columns() == before;
        }
        ST_BEGIN("tools_column_toggle");
        ST_PASS("tools_column_toggle toggled=%d restored=%d rail=%d",
                toggled ? 1 : 0, restored ? 1 : 0, railUnchanged ? 1 : 0);
        if (!(toggled && restored && railUnchanged)) {
            return pictura::selfTest().fail(425, "tools column toggle");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // panel_float_tabify (426): a dragged panel and a dragged group both drop
    // into an existing in-window float's group as tabs, with the shared
    // insertion indicator shown on the float's tab bar, and one float remains.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool floated = false;
        bool panelTabbed = false;
        bool indicator = false;
        bool groupMerged = false;
        bool oneFloat = false;
        if (primary && primary->groups().size() < 3) {
            primary = frame.restoreLegacyDefaultForTest();
            pump4(6);
        }
        if (primary) {
            // Pick three live groups: one to float, one to feed a panel, one to
            // merge whole. The accumulated test session group layout is not fixed.
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && !group->visibleTitles().isEmpty()) {
                    groups << group;
                }
            }
            if (groups.size() >= 3) {
                pictura::PanelGroup* floatSrc = groups.at(0);
                pictura::PanelGroup* panelSrc = groups.at(1);
                pictura::PanelGroup* groupSrc = groups.at(2);
                const QString floatPanel = floatSrc->visiblePanels().first()->objectName();
                const QString panelName = panelSrc->visiblePanels().first()->objectName();
                QStringList groupPanels;
                for (QWidget* panel : groupSrc->visiblePanels()) {
                    if (panel) {
                        groupPanels << panel->objectName();
                    }
                }

                floated = primary->tearOffForTest(floatPanel);
                pump4(6);
                pictura::PanelFloat* floatWindow = primary->floatForTest(0);
                pictura::PanelGroup* floatGroup =
                    floatWindow ? floatWindow->group() : nullptr;
                if (floated && floatGroup) {
                    // A panel from another docked group tabifies into the float,
                    // with the shared indicator drawn on the float's tab bar.
                    const QPoint target = floatGroup->tabInsertionGlobalPointForTest(0);
                    const bool beganPanel = primary->beginTabDragForTest(panelName);
                    primary->dragToForTest(target);
                    indicator = primary->floatTabIndicatorVisibleForTest(0);
                    const bool droppedPanel = primary->dropForTest(target);
                    pump4(6);
                    const QStringList afterPanel = primary->floatPanelNamesForTest(0);
                    panelTabbed = beganPanel && droppedPanel && indicator
                                  && afterPanel.contains(panelName)
                                  && afterPanel.contains(floatPanel);

                    // A whole docked group merges into the float as tabs.
                    pictura::PanelFloat* float2 = primary->floatForTest(0);
                    pictura::PanelGroup* group2 = float2 ? float2->group() : nullptr;
                    const QPoint target2 = group2
                        ? group2->tabInsertionGlobalPointForTest(group2->titleCountForTest())
                        : QPoint();
                    const bool beganGroup = primary->beginGroupDragForTest(groupPanels.first());
                    primary->dragToForTest(target2);
                    const bool droppedGroup = primary->dropForTest(target2);
                    pump4(6);
                    const QStringList afterGroup = primary->floatPanelNamesForTest(0);
                    groupMerged = beganGroup && droppedGroup && !groupPanels.isEmpty();
                    for (const QString& name : groupPanels) {
                        groupMerged = groupMerged && afterGroup.contains(name);
                    }
                }
                oneFloat = primary->floatCountForTest() == 1;
            }
        }
        ST_BEGIN("panel_float_tabify");
        ST_PASS("panel_float_tabify floated=%d panel=%d indicator=%d group=%d one_float=%d",
                floated ? 1 : 0, panelTabbed ? 1 : 0, indicator ? 1 : 0,
                groupMerged ? 1 : 0, oneFloat ? 1 : 0);
        if (!(floated && panelTabbed && groupMerged && oneFloat)) {
            return pictura::selfTest().fail(426, "float tabify");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // panel_group_tabify (427): a whole group dropped on another group's tab bar
    // merges the source's panels into the target as tabs; the source group is
    // emptied and exactly one group remains for that content.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool began = false;
        bool dropped = false;
        bool merged = false;
        bool sourceGone = false;
        bool oneGroup = false;
        if (primary) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : primary->groups()) {
                // A group collapsed to icons has no tab bar to drop onto.
                if (group && group->titleCountForTest() > 0
                    && !group->isCollapsedToIcons()) {
                    groups << group;
                }
            }
            if (groups.size() >= 2) {
                pictura::PanelGroup* target = groups.at(0);
                pictura::PanelGroup* source = groups.at(1);
                // The reset session may leave a group's tabs hidden; reveal one
                // panel in each so both groups are laid out and visible.
                for (pictura::PanelGroup* group : {target, source}) {
                    if (group->visiblePanels().isEmpty()) {
                        QWidget* panel = group->panels().value(0);
                        if (panel) {
                            primary->showPanel(panel->objectName(), true);
                        }
                    }
                }
                pump4(4);
                const QString sourcePanel =
                    source->visiblePanels().first()->objectName();
                if (!target->visiblePanels().isEmpty()) {
                    primary->ensureGroupVisibleForTest(
                        target->visiblePanels().first()->objectName());
                    pump4(4);
                }
                const int targetBefore = target->titleCountForTest();
                const int sourceCount = source->titleCountForTest();
                // Aim at the tab-bar centre: the bar's left edge can fall in a
                // neighbouring column's new-column edge band.
                const QPoint point = target->tabBarGlobalRect().center();
                began = primary->beginGroupDragForTest(sourcePanel);
                primary->dragToForTest(point);
                dropped = primary->dropForTest(point);
                // Read the result before the event pump can run the emptied
                // source group's deferred delete.
                merged = target->titleCountForTest() == targetBefore + sourceCount;
                sourceGone = source->titleCountForTest() == 0;
                oneGroup = primary->groupForPanel(sourcePanel) == target;
            }
        }
        ST_BEGIN("panel_group_tabify");
        ST_PASS("panel_group_tabify began=%d dropped=%d merged=%d source_gone=%d one=%d",
                began ? 1 : 0, dropped ? 1 : 0, merged ? 1 : 0, sourceGone ? 1 : 0,
                oneGroup ? 1 : 0);
        if (!(began && dropped && merged && sourceGone && oneGroup)) {
            return pictura::selfTest().fail(427, "group tabify");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // panel_group_outline (428): the blue region outline is drawn around the
    // resolved target group while a whole group drag hovers its tab bar, and is
    // cleared on cancel.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool shown = false;
        bool cleared = false;
        if (primary) {
            QList<pictura::PanelGroup*> groups;
            for (pictura::PanelGroup* group : primary->groups()) {
                if (group && group->titleCountForTest() > 0
                    && !group->isCollapsedToIcons()) {
                    groups << group;
                }
            }
            if (groups.size() >= 2) {
                pictura::PanelGroup* target = groups.at(0);
                pictura::PanelGroup* source = groups.at(1);
                for (pictura::PanelGroup* group : {target, source}) {
                    if (group->visiblePanels().isEmpty()) {
                        QWidget* panel = group->panels().value(0);
                        if (panel) {
                            primary->showPanel(panel->objectName(), true);
                        }
                    }
                }
                pump4(4);
                const QString sourcePanel =
                    source->visiblePanels().first()->objectName();
                if (!target->visiblePanels().isEmpty()) {
                    primary->ensureGroupVisibleForTest(
                        target->visiblePanels().first()->objectName());
                    pump4(4);
                }
                const QPoint point = target->tabBarGlobalRect().center();
                const bool began = primary->beginGroupDragForTest(sourcePanel);
                primary->dragToForTest(point);
                shown = began && primary->outlineIndicatorVisibleForTest();
                primary->cancelDragForTest();
                pump4(4);
                cleared = !primary->outlineIndicatorVisibleForTest();
            }
        }
        ST_BEGIN("panel_group_outline");
        ST_PASS("panel_group_outline shown=%d cleared=%d", shown ? 1 : 0,
                cleared ? 1 : 0);
        if (!(shown && cleared)) {
            return pictura::selfTest().fail(428, "group outline");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // panel_group_header_background (429): the group header paints one
    // continuous band across the full group width and tab-bar height, including
    // the strip above the corner grip/menu button, and the corner reaches the
    // group's right edge.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        bool checked = false;
        bool bandCoversHeader = false;
        bool cornerAtRight = false;
        if (primary) {
            for (pictura::PanelGroup* group : primary->groups()) {
                if (!group || group->isCollapsedToIcons()
                    || group->titleCountForTest() == 0) {
                    continue;
                }
                if (group->visiblePanels().isEmpty()) {
                    QWidget* panel = group->panels().value(0);
                    if (panel) {
                        primary->showPanel(panel->objectName(), true);
                    }
                }
                pump4(4);
                QTabBar* bar = group->tabBar();
                QWidget* corner = group->headerCornerForTest();
                QWidget* band = group->headerBandForTest();
                if (!bar || !corner || !band || !corner->isVisible()
                    || bar->height() <= 0 || group->width() <= 0) {
                    continue;
                }
                checked = true;
                cornerAtRight =
                    corner->mapTo(group, QPoint(corner->width(), 0)).x()
                    >= group->width() - 2;

                // The band spans the full group width over the tab-bar row, so
                // it covers the corner and the strip above it that the tab bar
                // does not reach.
                const QRect bandRect(band->mapTo(group, QPoint(0, 0)),
                                      band->size());
                const QRect cornerRect(corner->mapTo(group, QPoint(0, 0)),
                                       corner->size());
                const bool spans = bandRect.width() >= group->width() - 2
                                   && bandRect.height() >= bar->height()
                                   && bandRect.contains(cornerRect);

                // The former gap above the corner is the band's `${surface-header}`
                // shade, which is now the inactive-tab surface.
                const QColor headerShade(QStringLiteral("#363636"));
                const QImage image = group->grab().toImage();
                const qreal dpr = image.devicePixelRatio();
                const QPoint gap = band->mapTo(group, QPoint(corner->x() + 2, 2));
                const QColor gapColour = image.pixelColor(int(gap.x() * dpr + 0.5),
                                                          int(gap.y() * dpr + 0.5));
                bandCoversHeader =
                    spans && gapColour.rgba() == headerShade.rgba();
                break;
            }
        }
        ST_BEGIN("panel_group_header_background");
        ST_PASS("panel_group_header_background checked=%d band=%d corner_right=%d",
                checked ? 1 : 0, bandCoversHeader ? 1 : 0, cornerAtRight ? 1 : 0);
        if (!(checked && bandCoversHeader && cornerAtRight)) {
            return pictura::selfTest().fail(429, "header background");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // panel_drag_whole_dim (430): a group drag dims its docked source for the
    // whole drag and does not tear the group into an overlay while the pointer
    // is still over a column (here the atomic tools column, which resolves no
    // valid target); a cancel restores full opacity, re-docks the group, and
    // leaves no overlay behind.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        pictura::PanelColumn* tools = frame.toolsColumn();
        bool began = false, dimmed = false, floated = false, restored = false;
        if (primary && tools) {
            pictura::PanelGroup* group = nullptr;
            QString panelName;
            for (pictura::PanelGroup* candidate : primary->groups()) {
                if (candidate && candidate->titleCountForTest() > 0) {
                    group = candidate;
                    break;
                }
            }
            if (group && group->visiblePanels().isEmpty() && !group->panels().isEmpty()
                && group->panels().first()) {
                primary->showPanel(group->panels().first()->objectName(), true);
                pump4(4);
            }
            if (group && !group->visiblePanels().isEmpty() && group->visiblePanels().first()) {
                panelName = group->visiblePanels().first()->objectName();
            }
            if (!panelName.isEmpty()) {
                // Earlier shell checks leave their floats rehomed onto the
                // primary column, so the baseline is the number of live overlays.
                const int floatsBefore = primary->floatCountForTest();
                began = primary->beginGroupDragForTest(panelName);
                // The tools column resolves no valid target (D4 atomic); the
                // group stays docked and dimmed instead of tearing off, because
                // the pointer has not left every column yet.
                const QPoint overTools = tools->mapToGlobal(tools->rect().center());
                primary->dragToForTest(overTools);
                pump4(4);
                dimmed = began && primary->dragDimOpacityForTest() < 0.7;
                floated = primary->floatCountForTest() != floatsBefore;
                primary->cancelDragForTest();
                pump4(6);
                restored = primary->dragDimOpacityForTest() > 0.9
                           && primary->floatCountForTest() == floatsBefore
                           && !primary->dragActiveForTest();
            }
        }
        ST_BEGIN("panel_drag_whole_dim");
        ST_PASS("panel_drag_whole_dim began=%d dimmed=%d floated=%d restored=%d",
                began ? 1 : 0, dimmed ? 1 : 0, floated ? 1 : 0, restored ? 1 : 0);
        if (!(began && dimmed && !floated && restored)) {
            return pictura::selfTest().fail(430, "whole-drag dim");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // float_icon_flyout (431): clicking an icon in a floating compact/icon row
    // opens the same popup flyout a docked strip icon opens, hosting that same
    // group with the clicked panel current, and closing returns the group to its
    // overlay (re-collapsed) rather than docking it.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool opened = false;
        bool sameGroup = false;
        bool active = false;
        bool parity = false;
        bool restored = false;
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
                pump4(4);
            }
            const QString panel = group && !group->visiblePanels().isEmpty()
                                      && group->visiblePanels().first()
                                  ? group->visiblePanels().first()->objectName()
                                  : QString();
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pump4(6);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* floatWindow = column->floatForTest(index);
                pictura::PanelGroup* floatGroup =
                    floatWindow ? floatWindow->group() : nullptr;
                if (torn && floatGroup) {
                    floatGroup->setCollapsedToIconsForTest(true);
                    pump4(6);
                    QToolButton* icon = floatGroup->findChild<QToolButton*>(
                        QStringLiteral("panelGroupIcon_") + panel);
                    if (icon) {
                        icon->click();
                        pump4(8);
                        opened = column->iconFlyoutVisibleForTest();
                        sameGroup = column->iconFlyoutGroupForTest() == floatGroup;
                        active = floatGroup->currentPanelName() == panel;
                        parity = column->popupStyleParityForTest(panel);
                        const bool closed = column->triggerFlyoutCloseForTest();
                        pump4(8);
                        restored = closed && !column->iconFlyoutVisibleForTest()
                                   && floatGroup->parentWidget() == floatWindow
                                   && floatGroup->isCollapsedToIcons();
                    }
                    column->closeFloatForTest(index);
                    pump4(6);
                }
            }
        }
        ST_BEGIN("float_icon_flyout");
        ST_PASS("float_icon_flyout torn=%d opened=%d same=%d active=%d parity=%d restored=%d",
                torn ? 1 : 0, opened ? 1 : 0, sameGroup ? 1 : 0, active ? 1 : 0,
                parity ? 1 : 0, restored ? 1 : 0);
        if (!(torn && opened && sameGroup && active && parity && restored)) {
            return pictura::selfTest().fail(431, "float icon flyout");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // float_grip_drag (432): the floating compact row's group grip starts a
    // whole-group drag through the existing docked-strip grammar, and releasing
    // it over a no-target point leaves no extra overlay and restores opacity.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool began = false;
        bool groupDrag = false;
        bool noGhost = false;
        bool intact = false;
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
                pump4(4);
            }
            const QString panel = group && !group->visiblePanels().isEmpty()
                                      && group->visiblePanels().first()
                                  ? group->visiblePanels().first()->objectName()
                                  : QString();
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pump4(6);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* floatWindow = column->floatForTest(index);
                pictura::PanelGroup* floatGroup =
                    floatWindow ? floatWindow->group() : nullptr;
                if (torn && floatGroup) {
                    floatGroup->setCollapsedToIconsForTest(true);
                    pump4(6);
                    const int floatsBefore = column->floatCountForTest();
                    QWidget* grip =
                        floatGroup->findChild<QWidget*>(QStringLiteral("panelIconGroupGrip"));
                    if (grip) {
                        const QPointF local(grip->rect().center());
                        const QPointF global(grip->mapToGlobal(grip->rect().center()));
                        QMouseEvent press(QEvent::MouseButtonPress, local, global,
                                          Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(grip, &press);
                        QWidget* tabs =
                            frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
                        const QPoint outsidePoint =
                            tabs ? tabs->mapToGlobal(tabs->rect().center())
                                 : column->mapToGlobal(QPoint(-40, column->height() / 2));
                        const QPointF outside(outsidePoint);
                        QMouseEvent move(QEvent::MouseMove, outside, outside, Qt::NoButton,
                                         Qt::LeftButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(grip, &move);
                        pump4(6);
                        began = column->dragActiveForTest();
                        groupDrag = began && !column->dragIsPanelForTest();
                        QMouseEvent release(QEvent::MouseButtonRelease, outside, outside,
                                            Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
                        QCoreApplication::sendEvent(grip, &release);
                        pump4(10);
                        noGhost = column->floatCountForTest() == floatsBefore
                                  && !column->dragActiveForTest()
                                  && column->dragDimOpacityForTest() > 0.9;
                        intact = floatWindow->group() == floatGroup
                                 && floatGroup->containsPanel(panel);
                    }
                    column->closeFloatForTest(index);
                    pump4(6);
                }
            }
        }
        ST_BEGIN("float_grip_drag");
        ST_PASS("float_grip_drag torn=%d began=%d group=%d no_ghost=%d intact=%d",
                torn ? 1 : 0, began ? 1 : 0, groupDrag ? 1 : 0, noGhost ? 1 : 0,
                intact ? 1 : 0);
        if (!(torn && began && groupDrag && noGhost && intact)) {
            return pictura::selfTest().fail(432, "float grip drag");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // float_grip_resize (433): dragging the overlay's corner grip resizes the
    // overlay itself and leaves the main window's size unchanged.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool resized = false;
        bool windowUnchanged = false;
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
                pump4(4);
            }
            // A collapsed-to-icons group floats without a resize grip, so expand
            // it first; the grip check below needs the expanded chrome.
            if (group && group->isCollapsedToIcons()) {
                group->setCollapsedToIcons(false);
                pump4(4);
            }
            const QString panel = group && !group->visiblePanels().isEmpty()
                                      && group->visiblePanels().first()
                                  ? group->visiblePanels().first()->objectName()
                                  : QString();
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pump4(8);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* floatWindow = column->floatForTest(index);
                QWidget* grip = floatWindow ? floatWindow->sizeGripForTest() : nullptr;
                if (torn && floatWindow && grip && grip->isVisible()) {
                    const QSize mainBefore = frame.size();
                    const QSize floatBefore = floatWindow->size();
                    const QPointF local(grip->rect().center());
                    const QPointF global(grip->mapToGlobal(grip->rect().center()));
                    QMouseEvent press(QEvent::MouseButtonPress, local, global, Qt::LeftButton,
                                      Qt::LeftButton, Qt::NoModifier);
                    QCoreApplication::sendEvent(grip, &press);
                    const QPointF moved = global + QPointF(40, 40);
                    QMouseEvent move(QEvent::MouseMove, local, moved, Qt::NoButton,
                                     Qt::LeftButton, Qt::NoModifier);
                    QCoreApplication::sendEvent(grip, &move);
                    QMouseEvent release(QEvent::MouseButtonRelease, local, moved, Qt::LeftButton,
                                        Qt::NoButton, Qt::NoModifier);
                    QCoreApplication::sendEvent(grip, &release);
                    pump4(6);
                    resized = floatWindow->width() > floatBefore.width()
                              && floatWindow->height() > floatBefore.height();
                    windowUnchanged = frame.size() == mainBefore;
                }
                column->closeFloatForTest(index);
                pump4(6);
            }
        }
        ST_BEGIN("float_grip_resize");
        ST_PASS("float_grip_resize torn=%d resized=%d window=%d", torn ? 1 : 0,
                resized ? 1 : 0, windowUnchanged ? 1 : 0);
        if (!(torn && resized && windowUnchanged)) {
            return pictura::selfTest().fail(433, "float grip resize");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_float_fixed (434): the floating tools column has no resize grip and
    // is sized to the minimum the tool grid and its content need.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        QWidget* tabs = frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
        bool floated = false;
        bool fixed = false;
        bool sized = false;
        if (tools && primary && tabs) {
            const int wantW = qMax(tools->minimumWidth(), tools->minimumSizeHint().width());
            const int wantH = qMax(tools->minimumHeight(), tools->minimumSizeHint().height());
            tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            tools->dragColumnHeaderToForTest(tabs->mapToGlobal(tabs->rect().center()));
            pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
            floated = floatWindow && toolFloatWindow(floatWindow, &frame);
            if (floated) {
                QWidget* grip = floatWindow->sizeGripForTest();
                fixed = !floatWindow->resizableForTest() && (!grip || !grip->isVisible());
                sized = qAbs(floatWindow->width() - wantW) <= 2
                        && floatWindow->height() <= wantH + 2;
            }
            const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
            const QPoint left(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
            tools->dropColumnHeaderForTest(left);
            pump4(6);
        }
        ST_BEGIN("tools_float_fixed");
        ST_PASS("tools_float_fixed float=%d fixed=%d sized=%d", floated ? 1 : 0,
                fixed ? 1 : 0, sized ? 1 : 0);
        if (!(floated && fixed && sized)) {
            return pictura::selfTest().fail(434, "tools float fixed");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // column_float_height (435): a floating widget column opens at about two
    // thirds of its docked height, strictly less than the docked column, and a
    // release on a workspace edge re-docks it so no whole-column float leaks
    // into later checks.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* column = frame.panelColumn();
        QWidget* tabs = frame.findChild<QWidget*>(QStringLiteral("documentTabs"));
        QWidget* central = frame.centralWidget();
        bool floated = false;
        bool shorter = false;
        bool redocked = false;
        if (column && tabs && central) {
            column->setRailMode(false);
            pump4(4);
            const int dockedHeight = column->height();
            column->beginColumnHeaderDragForTest(
                column->mapToGlobal(QPoint(qMax(1, column->width() / 2), 8)));
            column->dragColumnHeaderToForTest(tabs->mapToGlobal(tabs->rect().center()));
            pictura::PanelFloat* floatWindow = column->columnFloatForTest();
            floated = floatWindow && toolFloatWindow(floatWindow, &frame);
            if (floated) {
                const int floatHeight = floatWindow->height();
                shorter = floatHeight < dockedHeight
                          && floatHeight >= pictura::PanelFloat::kFloatMinHeight;
            }
            const QPoint edge(central->mapToGlobal(QPoint(2, central->height() / 2)));
            redocked = column->dropColumnHeaderForTest(edge)
                       && column->columnFloatForTest() == nullptr;
            pump4(6);
        }
        ST_BEGIN("column_float_height");
        ST_PASS("column_float_height float=%d shorter=%d redock=%d", floated ? 1 : 0,
                shorter ? 1 : 0, redocked ? 1 : 0);
        if (!(floated && shorter && redocked)) {
            return pictura::selfTest().fail(435, "column float height");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // float_icon_width_snap (436): a collapsed-to-icons floating group snaps to
    // the icon-row height and shrinks its width, leaving no normal-width body.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool snapped = false;
        bool widthShrunk = false;
        int expandedWidth = 0;
        int collapsedWidth = 0;
        int iconWidth = 0;
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
                pump4(4);
            }
            const QString panel = group && !group->visiblePanels().isEmpty()
                                      && group->visiblePanels().first()
                                  ? group->visiblePanels().first()->objectName()
                                  : QString();
            if (!panel.isEmpty()) {
                if (group->isCollapsedToIcons()) {
                    group->setCollapsedToIcons(false);
                    pump4(4);
                }
                torn = column->tearOffForTest(panel);
                pump4(8);
                const int index = column->floatCountForTest() - 1;
                pictura::PanelFloat* floatWindow = column->floatForTest(index);
                if (torn && floatWindow && floatWindow->group() == group) {
                    expandedWidth = floatWindow->width();
                    group->setCollapsedToIcons(true);
                    pump4(8);
                    const int iconHeight = group->sizeHint().height();
                    iconWidth = qMax(pictura::PanelFloat::kFloatMinWidth,
                                     group->sizeHint().width());
                    collapsedWidth = floatWindow->width();
                    snapped = group->isCollapsedToIcons()
                              && floatWindow->height() >= pictura::PanelFloat::kFloatIconMinHeight
                              && floatWindow->height() <= iconHeight + 8;
                    widthShrunk = collapsedWidth < expandedWidth
                                  && collapsedWidth <= iconWidth + 2;
                }
                column->closeFloatForTest(index);
                pump4(6);
            }
        }
        ST_BEGIN("float_icon_width_snap");
        ST_PASS("float_icon_width_snap torn=%d snapped=%d shrink=%d w=%d->%d icon=%d",
                torn ? 1 : 0, snapped ? 1 : 0, widthShrunk ? 1 : 0, expandedWidth,
                collapsedWidth, iconWidth);
        if (!(torn && snapped && widthShrunk)) {
            return pictura::selfTest().fail(436, "float icon width snap");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // float_screen_bounds (437): a float can be moved outside the main window
    // rect, because movement is clamped to the screen rather than the window.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        // Shrink the frame below the offscreen screen so a screen-clamped float
        // has room outside it; the old central-rect clamp would pull it back.
        const QSize frameBefore = frame.size();
        frame.resize(700, 500);
        pump4(8);
        pictura::PanelColumn* column = frame.panelColumn();
        bool torn = false;
        bool onScreen = false;
        bool outsideFrame = false;
        int screenBottom = 0;
        int frameBottom = 0;
        int screenRight = 0;
        int frameRight = 0;
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
                pump4(4);
            }
            const QString panel = group && !group->visiblePanels().isEmpty()
                                      && group->visiblePanels().first()
                                  ? group->visiblePanels().first()->objectName()
                                  : QString();
            if (!panel.isEmpty()) {
                torn = column->tearOffForTest(panel);
                pump4(8);
                const int index = column->floatCountForTest() - 1;
                const QRect screen = column->floatHostRectForTest();
                const QRect frameRect(frame.mapToGlobal(QPoint(0, 0)), frame.size());
                screenBottom = screen.bottom();
                frameBottom = frameRect.bottom();
                screenRight = screen.right();
                frameRight = frameRect.right();
                const bool lowBand = screen.bottom() > frameRect.bottom();
                const bool rightBand = screen.right() > frameRect.right();
                if (torn && (lowBand || rightBand)) {
                    const QPoint want = lowBand
                        ? QPoint(screen.center().x(), screen.bottom() + 400)
                        : QPoint(screen.right() + 400, screen.center().y());
                    const bool clamped = column->floatClampedForTest(index, want);
                    const QRect after = column->floatGeometryForTest(index);
                    onScreen = clamped && screen.contains(after);
                    outsideFrame = lowBand ? after.bottom() > frameRect.bottom()
                                           : after.right() > frameRect.right();
                }
                column->closeFloatForTest(index);
                pump4(6);
            }
        }
        ST_BEGIN("float_screen_bounds");
        ST_PASS("float_screen_bounds torn=%d on_screen=%d outside=%d sb=%d fb=%d sr=%d fr=%d",
                torn ? 1 : 0, onScreen ? 1 : 0, outsideFrame ? 1 : 0, screenBottom,
                frameBottom, screenRight, frameRight);
        if (!(torn && onScreen && outsideFrame)) {
            return pictura::selfTest().fail(437, "float screen bounds");
        }
        frame.resize(frameBefore);
        pump4(6);
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }

    // tools_column_drag_float (439): the docked Tools column header drag tears
    // the column into a following overlay on the first move, even while the
    // pointer is still over a resolved sibling target inside the workspace (the
    // edge indicator keeps showing), instead of only floating once outside the
    // workspace; the release re-docks the column into the splitter.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        auto* splitter = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        bool floated = false, indicator = false, follows = false, redocked = false;
        if (tools && primary && splitter) {
            // Pressing the header tears the whole column off immediately; the
            // splitter then re-lays out, so the sibling target point is measured
            // against the primary column's post-float geometry.
            tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            pump4(4);
            const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
            const QPoint a(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
            const QPoint b = a + QPoint(30, 0);
            tools->dragColumnHeaderToForTest(a);
            pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
            floated = floatWindow && floatWindow->isVisible() && splitter->indexOf(tools) < 0;
            indicator = primary->dropIndicatorVisibleForTest();
            const QRect atA = floatWindow
                ? QRect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size())
                : QRect();
            tools->dragColumnHeaderToForTest(b);
            const QRect atB = floatWindow
                ? QRect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size())
                : QRect();
            follows = atA.isValid() && atB.isValid()
                      && atB.topLeft() - atA.topLeft() == b - a;
            redocked = tools->dropColumnHeaderForTest(b) && tools->columnFloatForTest() == nullptr
                       && splitter->indexOf(tools) >= 0 && !tools->isWindow();
            pump4(6);
        }
        ST_BEGIN("tools_column_drag_float");
        ST_PASS("tools_column_drag_float floated=%d indicator=%d follows=%d redocked=%d",
                floated ? 1 : 0, indicator ? 1 : 0, follows ? 1 : 0, redocked ? 1 : 0);
        if (!(floated && indicator && follows && redocked)) {
            return pictura::selfTest().fail(439, "tools column drag float");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump4(6);
    }
    if (const int fc = pictura::runShellRound4FloatCheck(frame); fc != 0) { return fc; }
    return 0;
}
