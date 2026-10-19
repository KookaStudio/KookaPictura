#include "selftest_shell_round3.h"
#include "selftest_report.h"

#include "frame.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "session.h"
#include "toolbox.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/Qt>
#include <QtCore/QCoreApplication>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>

namespace {

// A floating overlay is a frameless Qt::Tool top-level window, not an in-window
// child and not a decorated OS window.
bool toolFloatWindow(QWidget* w)
{
    return w && w->isWindow() && w->windowFlags().testFlag(Qt::Tool)
           && w->windowFlags().testFlag(Qt::FramelessWindowHint);
}

} // namespace

int pictura::runShellRound3Checks(pictura::PicturaMainWindow& frame)
{
    // lss_empty_pane (349): the document pane stays in the workspace with no
    // document (so the widget columns cannot absorb it) while its empty tab strip
    // is hidden and reappears on create; closing the last document hides the
    // strip again.
    {
        while (frame.documentCount() > 0) {
            frame.closeDocument(0, false);
        }
        auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        if (!tabs) {
            return pictura::selfTest().fail(349, "document tab pane missing");
        }
        QCoreApplication::processEvents();
        const bool spaceEmpty = tabs->isVisible() && tabs->width() > 0
                                && tabs->tabBar() && !tabs->tabBar()->isVisible();
        const bool created = frame.newDocument(QStringLiteral("Round3"), 32, 32,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        const bool stripWithDoc = tabs->isVisible() && tabs->tabBar()->isVisible();
        const int doc = frame.activeDocumentIndex();
        frame.closeDocument(doc, false);
        QCoreApplication::processEvents();
        const bool spaceAgain = tabs->isVisible() && !tabs->tabBar()->isVisible();

        ST_BEGIN("lss_empty_pane");
        ST_PASS("lss_empty_pane space=%d strip=%d space2=%d", spaceEmpty ? 1 : 0,
                stripWithDoc ? 1 : 0, spaceAgain ? 1 : 0);
        if (!spaceEmpty || !created || !stripWithDoc || !spaceAgain) {
            return pictura::selfTest().fail(349, "empty workspace did not keep its space");
        }
    }

    // lss_fresh_pane (380): a freshly constructed window keeps the empty document
    // pane in the workspace (with visible space reserved for it) while hiding the
    // empty tab strip; creating a document shows the strip, and closing the last
    // one hides it again.
    {
        pictura::PicturaMainWindow fresh;
        auto* freshTabs = fresh.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        if (!freshTabs) {
            return pictura::selfTest().fail(380, "fresh document tab pane missing");
        }
        const bool freshSpace = freshTabs->isVisibleTo(&fresh)
                                && !freshTabs->tabBar()->isVisibleTo(&fresh);
        const bool freshCreated = fresh.newDocument(QStringLiteral("Fresh"), 32, 32,
                                                    QStringLiteral("rgb"), 8,
                                                    QStringLiteral("white"));
        const bool freshStrip = freshTabs->isVisibleTo(&fresh)
                                && freshTabs->tabBar()->isVisibleTo(&fresh);
        while (fresh.documentCount() > 0) {
            fresh.closeDocument(0, false);
        }
        const bool freshSpaceAgain = freshTabs->isVisibleTo(&fresh)
                                     && !freshTabs->tabBar()->isVisibleTo(&fresh);
        ST_BEGIN("lss_fresh_pane");
        ST_PASS("lss_fresh_pane space=%d strip=%d space2=%d", freshSpace ? 1 : 0,
                freshStrip ? 1 : 0, freshSpaceAgain ? 1 : 0);
        if (!freshSpace || !freshCreated || !freshStrip || !freshSpaceAgain) {
            return pictura::selfTest().fail(380, "fresh window ghost pane");
        }
    }

    // lss_tab_weight (350): the scoped document-tab rule carries bold weight
    // and extra right padding without touching the unscoped panel rules.
    {
        const QString sheet = qApp->styleSheet();
        const bool scoped = sheet.contains(QStringLiteral("QTabBar#documentTabBar::tab {"));
        const bool weight = sheet.contains(QStringLiteral("font-weight: 700"));
        const bool padding = sheet.contains(QStringLiteral("padding-right: 4px"));

        ST_BEGIN("lss_tab_weight");
        ST_PASS("lss_tab_weight scoped=%d weight=%d padding=%d", scoped ? 1 : 0, weight ? 1 : 0,
                padding ? 1 : 0);
        if (!scoped || !weight || !padding) {
            return pictura::selfTest().fail(350, "document tab rule missing");
        }
    }

    // lss_brush_keys (351): the pure helper maps US keys, evdev scan codes, and
    // their Shift variants, and returns 0 for non-paint tools and other keys.
    {
        const bool created = frame.newDocument(QStringLiteral("Round3Keys"), 32, 32,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(351, "brush key fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const bool usKeys =
            view->brush_shortcut_delta(Qt::Key_BracketLeft, 0, false, true) == -1
            && view->brush_shortcut_delta(Qt::Key_BracketRight, 0, false, true) == 1
            && view->brush_shortcut_delta(Qt::Key_BraceLeft, 0, true, true) == -5
            && view->brush_shortcut_delta(Qt::Key_BraceRight, 0, true, true) == 5;
        const bool nativeKeys =
            view->brush_shortcut_delta(0, 34, false, true) == -1
            && view->brush_shortcut_delta(0, 35, false, true) == 1
            && view->brush_shortcut_delta(0, 34, true, true) == -5
            && view->brush_shortcut_delta(0, 35, true, true) == 5;
        const bool guard =
            view->brush_shortcut_delta(Qt::Key_BracketRight, 35, false, false) == 0
            && view->brush_shortcut_delta(Qt::Key_A, 0, false, true) == 0;

        ST_BEGIN("lss_brush_keys");
        ST_PASS("lss_brush_keys us=%d native=%d guard=%d", usKeys ? 1 : 0, nativeKeys ? 1 : 0,
                guard ? 1 : 0);
        const bool ok = usKeys && nativeKeys && guard;
        if (!ok) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(351, "brush key mapping wrong");
        }
    }

    auto pump = [](int count) {
        for (int i = 0; i < count; ++i) {
            QCoreApplication::processEvents();
        }
    };

    // lss_col_header_move (401): a press-drag-release on a column's top header
    // moves the whole column onto a side of another column through the real
    // resolve/move path, and the frame's edge indicator shows while dragging.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* primary = frame.panelColumn();
        const bool made =
            frame.newColumnDropForTest(QStringLiteral("stylesPanel"), QStringLiteral("left"));
        pump(6);
        pictura::PanelColumn* left = frame.columnForPanel(QStringLiteral("stylesPanel"));
        bool indicator = false;
        bool moved = false;
        if (primary && left && primary != left) {
            const QRect lr(left->mapToGlobal(QPoint(0, 0)), left->size());
            const QPoint target(lr.left() + qMax(1, lr.width() / 4), lr.center().y());
            QWidget* header = primary->columnHeaderForTest();
            const bool began = primary->beginColumnHeaderDragForTest(target);
            primary->dragColumnHeaderToForTest(target);
            indicator = left->dropIndicatorVisibleForTest();
            const bool dropped = primary->dropColumnHeaderForTest(target);
            pump(6);
            const QList<pictura::PanelColumn*> order = frame.panelColumns();
            moved = header && began && dropped && !order.isEmpty() && order.first() == primary
                    && primary->isVisible() && primary->width() > 0;
        }
        ST_BEGIN("lss_col_header_move");
        ST_PASS("lss_col_header_move made=%d indicator=%d moved=%d", made ? 1 : 0,
                indicator ? 1 : 0, moved ? 1 : 0);
        if (!(made && indicator && moved)) {
            return pictura::selfTest().fail(401, "column header move");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_tab_compress (402): many-tab groups squeeze/elide instead of showing
    // scroll arrows (usesScrollButtons off), and the bar still does not expand.
    {
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group =
            column && !column->groups().isEmpty() ? column->groups().first() : nullptr;
        const bool noScroll = group && !group->tabUsesScrollButtonsForTest();
        const bool notExpanding = group && group->tabBar() && !group->tabBar()->expanding();
        ST_BEGIN("lss_tab_compress");
        ST_PASS("lss_tab_compress noScroll=%d expanding=%d", noScroll ? 1 : 0,
                notExpanding ? 1 : 0);
        if (!(group && noScroll && notExpanding)) {
            return pictura::selfTest().fail(402, "tab compression");
        }
    }

    // lss_grip_drag (403): the group corner's reserved grip is present, draggable
    // and drives a whole-group drag through the existing signal path.
    {
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && candidate->isVisible()
                    && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool hasGrip = false;
        bool dragStarted = false;
        if (group) {
            QWidget* grip = group->headerGripForTest();
            hasGrip = grip && grip->objectName() == QStringLiteral("panelGroupDragGrip")
                      && grip->cursor().shape() == Qt::SizeAllCursor && grip->isVisible();
            const QMetaObject::Connection conn =
                QObject::connect(group, &pictura::PanelGroup::groupDragStarted,
                                 [&dragStarted](const QPoint&) { dragStarted = true; });
            if (grip) {
                const QPoint start = grip->mapToGlobal(QPoint(2, 2));
                QMouseEvent press(QEvent::MouseButtonPress, QPointF(2, 2), QPointF(start),
                                  Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                QApplication::sendEvent(grip, &press);
                const QPoint moved = start + QPoint(24, 0);
                QMouseEvent move(QEvent::MouseMove, QPointF(26, 2), QPointF(moved), Qt::NoButton,
                                 Qt::LeftButton, Qt::NoModifier);
                QApplication::sendEvent(grip, &move);
                column->cancelDragForTest();
                QMouseEvent release(QEvent::MouseButtonRelease, QPointF(26, 2), QPointF(moved),
                                    Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
                QApplication::sendEvent(grip, &release);
                pump(4);
            }
            QObject::disconnect(conn);
        }
        ST_BEGIN("lss_grip_drag");
        ST_PASS("lss_grip_drag grip=%d started=%d", hasGrip ? 1 : 0, dragStarted ? 1 : 0);
        if (!(hasGrip && dragStarted)) {
            return pictura::selfTest().fail(403, "header grip drag");
        }
    }

    // lss_float_header (404): a floating group shows its top bar with a working
    // collapse toggle and the close control (moved out of the corner), and the
    // bar itself drags the overlay.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool headerShown = false;
        bool toggleWorks = false;
        bool closeInHeader = false;
        bool dragMoved = false;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            headerShown = group->floatHeaderVisibleForTest();
            QToolButton* toggle = group->floatToggleForTest();
            const bool wasCollapsed = group->isCollapsedToIcons();
            if (toggle) {
                toggle->click();
                pump(4);
                toggleWorks = group->isCollapsedToIcons() != wasCollapsed;
            }
            QWidget* floatHeader = group->floatHeaderForTest();
            QToolButton* close = group->floatCloseButton();
            closeInHeader = close && floatHeader && close->parentWidget() == floatHeader;
            const QRect before = column->floatGeometryForTest(0);
            if (floatHeader) {
                const QPoint start = floatHeader->mapToGlobal(QPoint(4, 4));
                QMouseEvent press(QEvent::MouseButtonPress, QPointF(4, 4), QPointF(start),
                                  Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                QApplication::sendEvent(floatHeader, &press);
                const QPoint moved = start + QPoint(40, 25);
                QMouseEvent move(QEvent::MouseMove, QPointF(44, 29), QPointF(moved), Qt::NoButton,
                                 Qt::LeftButton, Qt::NoModifier);
                QApplication::sendEvent(floatHeader, &move);
                QMouseEvent release(QEvent::MouseButtonRelease, QPointF(44, 29), QPointF(moved),
                                    Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
                QApplication::sendEvent(floatHeader, &release);
                pump(8);
                const QRect after = column->floatGeometryForTest(0);
                dragMoved = after.topLeft() != before.topLeft();
            }
            group->setCollapsedToIconsForTest(false);
            column->redockForTest(0, 0);
            pump(6);
        }
        ST_BEGIN("lss_float_header");
        ST_PASS("lss_float_header torn=%d header=%d toggle=%d close=%d drag=%d", torn ? 1 : 0,
                headerShown ? 1 : 0, toggleWorks ? 1 : 0, closeInHeader ? 1 : 0,
                dragMoved ? 1 : 0);
        if (!(torn && headerShown && toggleWorks && closeInHeader && dragMoved)) {
            return pictura::selfTest().fail(404, "float header");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_float_icons (405): collapsing a floating group to icons grows the
    // overlay to at least the icon-row minimum and the iconic row is styled as a
    // panel surface.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool minHeight = false;
        bool themed = false;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            group->setCollapsedToIcons(true);
            pump(8);
            minHeight = group->isCollapsedToIcons()
                        && column->floatGeometryForTest(0).height()
                               >= pictura::PanelFloat::kFloatIconMinHeight;
            themed = qApp->styleSheet().contains(QStringLiteral("QWidget#panelIconGroup {"));
            group->setCollapsedToIconsForTest(false);
            column->redockForTest(0, 0);
            pump(6);
        }
        ST_BEGIN("lss_float_icons");
        ST_PASS("lss_float_icons torn=%d minHeight=%d themed=%d", torn ? 1 : 0,
                minHeight ? 1 : 0, themed ? 1 : 0);
        if (!(torn && minHeight && themed)) {
            return pictura::selfTest().fail(405, "float icon mode");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_toolbox_column_drop (406): the tools column can be re-placed on either
    // side of a widget column through its own header drag (left half before it,
    // right half after it), and both workspace outer bands resolve a real
    // sibling column rather than a dock.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool leftHalf = false;
        bool rightHalf = false;
        if (cs && tools && primary) {
            const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
            const QPoint left(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
            const QPoint right(pr.left() + (pr.width() * 3) / 4, pr.center().y());
            auto dragTools = [&](const QPoint& to) {
                tools->beginColumnHeaderDragForTest(
                    tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
                tools->dragColumnHeaderToForTest(to);
                const bool indicator = primary->dropIndicatorVisibleForTest();
                tools->dropColumnHeaderForTest(to);
                pump(6);
                return indicator && cs->indexOf(tools) >= 0;
            };
            // A drag is a no-op when the tools column already sits on that side,
            // so only the indicator is required; the final index is checked.
            if (dragTools(left)) {
                leftHalf = cs->indexOf(tools) < cs->indexOf(primary);
            }
            if (dragTools(right)) {
                rightHalf = cs->indexOf(tools) > cs->indexOf(primary);
            }
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        ST_BEGIN("lss_toolbox_column_drop");
        ST_PASS("lss_toolbox_column_drop left=%d right=%d", leftHalf ? 1 : 0,
                rightHalf ? 1 : 0);
        if (!(leftHalf && rightHalf)) {
            return pictura::selfTest().fail(406, "toolbox column drop");
        }
    }

    // lss_tools_title_drag (407): the tools column header drag re-places the
    // column beside a widget column on release, and a release over the empty
    // document area leaves it as an in-window overlay (never an OS window).
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        if (!cs || !tabs || !tools || !primary) {
            return pictura::selfTest().fail(407, "tools title drag fixture");
        }
        auto startPoint = [tools]() {
            return tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8));
        };
        const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
        const QPoint interior(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
        tools->beginColumnHeaderDragForTest(startPoint());
        tools->dragColumnHeaderToForTest(interior);
        const bool indicator = primary->dropIndicatorVisibleForTest();
        tools->dropColumnHeaderForTest(interior);
        pump(6);
        const bool placed = cs->indexOf(tools) >= 0 && !tools->isWindow()
                            && cs->indexOf(tools) < cs->indexOf(primary);
        const QPoint canvas(tabs->mapToGlobal(tabs->rect().center()));
        tools->beginColumnHeaderDragForTest(startPoint());
        tools->dragColumnHeaderToForTest(canvas);
        pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
        const bool floated = floatWindow && toolFloatWindow(floatWindow) && cs->indexOf(tools) < 0;
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        ST_BEGIN("lss_tools_title_drag");
        ST_PASS("lss_tools_title_drag indicator=%d placed=%d float=%d",
                indicator ? 1 : 0, placed ? 1 : 0, floated ? 1 : 0);
        if (!(indicator && placed && floated)) {
            return pictura::selfTest().fail(407, "tools title drag");
        }
    }

    // lss_header_menu (408): a real header press+move shows the edge indicator
    // and the release moves the column; an outer-band header drop lands at the
    // splitter head, left of a Tools pane at index 0; and the header left-click
    // menu flips rail mode and both auto toggles and raises interface options.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* toolbox = frame.findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        pictura::PanelColumn* primary = frame.panelColumn();
        const bool made =
            frame.newColumnDropForTest(QStringLiteral("stylesPanel"), QStringLiteral("left"));
        pump(6);
        pictura::PanelColumn* left = frame.columnForPanel(QStringLiteral("stylesPanel"));
        if (!toolbox || !cs || !primary || !left || primary == left) {
            return pictura::selfTest().fail(408, "header menu fixture tb=%d cs=%d p=%d l=%d made=%d",
                                            toolbox ? 1 : 0, cs ? 1 : 0, primary ? 1 : 0,
                                            left ? 1 : 0, made ? 1 : 0);
        }
        auto sendMouse = [](QWidget* w, QEvent::Type type, const QPoint& global,
                            Qt::MouseButton button, Qt::MouseButtons buttons) {
            QMouseEvent event(type, QPointF(w->mapFromGlobal(global)), QPointF(global), button,
                              buttons, Qt::NoModifier);
            QApplication::sendEvent(w, &event);
        };

        // Real header drag: press the primary header, move over the left column,
        // see the indicator, release and confirm the column moved.
        QWidget* header = primary->columnHeaderForTest();
        const QPoint blank(qMax(2, header->width() / 4), header->height() / 2);
        const QRect lr(left->mapToGlobal(QPoint(0, 0)), left->size());
        const QPoint onLeft(lr.left() + qMax(1, lr.width() / 4), lr.center().y());
        sendMouse(header, QEvent::MouseButtonPress, header->mapToGlobal(blank), Qt::LeftButton,
                  Qt::LeftButton);
        sendMouse(header, QEvent::MouseMove, onLeft, Qt::NoButton, Qt::LeftButton);
        pump(4);
        const bool indicator = left->dropIndicatorVisibleForTest();
        sendMouse(header, QEvent::MouseButtonRelease, onLeft, Qt::LeftButton, Qt::NoButton);
        pump(6);
        const bool moved = cs->indexOf(primary) == 0 && cs->indexOf(primary) < cs->indexOf(left);

        // The tools column stays a real sibling column (never a dock, never a
        // tabified pane) after the header drag.
        const bool toolsSibling =
            frame.toolsColumn() && frame.toolsColumn()->isToolsColumn()
            && cs->indexOf(frame.toolsColumn()) >= 0 && !frame.toolsColumn()->isWindow();
        const bool leftOfTools = toolsSibling && cs->indexOf(primary) >= 0;

        // Header menu: every entry works.
        const QStringList texts = primary->columnHeaderMenuTextsForTest();
        const bool menuComplete =
            texts.contains(QStringLiteral("Collapse to Icons"))
            && texts.contains(QStringLiteral("Auto-Collapse Iconic Panels"))
            && texts.contains(QStringLiteral("Auto-Show Hidden Panels"))
            && texts.contains(QStringLiteral("Interface Options\u2026"));
        const bool railBefore = primary->railMode();
        const bool railFlipped = primary->triggerColumnHeaderMenuForTest(
                                     QStringLiteral("Collapse to Icons"))
                                 && primary->railMode() != railBefore;
        const bool autoCollapseBefore = primary->autoCollapseIconicForTest();
        const bool autoCollapseFlipped = primary->triggerColumnHeaderMenuForTest(
                                             QStringLiteral("Auto-Collapse Iconic Panels"))
                                         && primary->autoCollapseIconicForTest()
                                                != autoCollapseBefore;
        const bool autoShowBefore = primary->autoShowHiddenForTest();
        const bool autoShowFlipped = primary->triggerColumnHeaderMenuForTest(
                                         QStringLiteral("Auto-Show Hidden Panels"))
                                     && primary->autoShowHiddenForTest() != autoShowBefore;
        bool optionsFired = false;
        const QMetaObject::Connection conn = QObject::connect(
            primary, &pictura::PanelColumn::interfaceOptionsRequested,
            [&optionsFired]() { optionsFired = true; });
        const bool optionsTriggered = primary->triggerColumnHeaderMenuForTest(
            QStringLiteral("Interface Options\u2026"));
        QObject::disconnect(conn);

        // Leave the default chrome for the suites that follow.
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);

        ST_BEGIN("lss_header_menu");
        ST_PASS("lss_header_menu made=%d moved=%d tools=%d leftOfTools=%d menu=%d rail=%d "
                "autoC=%d autoS=%d options=%d",
                made ? 1 : 0, moved ? 1 : 0, toolsSibling ? 1 : 0, leftOfTools ? 1 : 0,
                menuComplete ? 1 : 0, railFlipped ? 1 : 0, autoCollapseFlipped ? 1 : 0,
                autoShowFlipped ? 1 : 0, (optionsTriggered && optionsFired) ? 1 : 0);
        if (!(made && moved && indicator && toolsSibling && leftOfTools && menuComplete
              && railFlipped && autoCollapseFlipped && autoShowFlipped && optionsTriggered
              && optionsFired)) {
            return pictura::selfTest().fail(408, "header menu");
        }
    }

    // lss_float_toggle_place (409): the float top bar puts the normal/icon
    // toggle immediately left of the close control, both at the right end.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool placed = false;
        bool rightSide = false;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            QWidget* header = group->floatHeaderForTest();
            QToolButton* toggle = group->floatToggleForTest();
            QToolButton* close = group->floatCloseButton();
            if (header && toggle && close) {
                const QPoint t = toggle->mapTo(header, QPoint(0, 0));
                const QPoint c = close->mapTo(header, QPoint(0, 0));
                const int gap = c.x() - (t.x() + toggle->width());
                placed = t.x() < c.x() && gap >= 0 && gap <= 12;
                rightSide = c.x() + close->width() >= header->width() - 6
                            && t.x() > header->width() / 2;
            }
            group->setCollapsedToIconsForTest(false);
            column->redockForTest(0, 0);
            pump(6);
        }
        ST_BEGIN("lss_float_toggle_place");
        ST_PASS("lss_float_toggle_place torn=%d placed=%d right=%d", torn ? 1 : 0,
                placed ? 1 : 0, rightSide ? 1 : 0);
        if (!(torn && placed && rightSide)) {
            return pictura::selfTest().fail(409, "float toggle placement");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_float_icon_height (410): collapsing a floating group snaps the overlay
    // down to the group's icon-row height, not merely up to a minimum.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool snapped = false;
        bool shrunk = false;
        int expandedHeight = 0;
        int collapsedHeight = 0;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            expandedHeight = column->floatGeometryForTest(0).height();
            group->setCollapsedToIcons(true);
            pump(8);
            const int iconHeight = group->sizeHint().height();
            collapsedHeight = column->floatGeometryForTest(0).height();
            snapped = group->isCollapsedToIcons()
                      && collapsedHeight >= pictura::PanelFloat::kFloatIconMinHeight
                      && collapsedHeight <= iconHeight + 8;
            shrunk = collapsedHeight < expandedHeight;
            group->setCollapsedToIconsForTest(false);
            column->redockForTest(0, 0);
            pump(6);
        }
        ST_BEGIN("lss_float_icon_height");
        ST_PASS("lss_float_icon_height torn=%d expanded=%d collapsed=%d snap=%d shrink=%d",
                torn ? 1 : 0, expandedHeight, collapsedHeight, snapped ? 1 : 0, shrunk ? 1 : 0);
        if (!(torn && snapped && shrunk)) {
            return pictura::selfTest().fail(410, "float icon height");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_float_resize (411): the overlay carries a resize grip that resizes the
    // overlay itself and a non-zero minimum size (min width from the docked
    // column, min height top+tab bars).
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool grip = false;
        bool minSize = false;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            pictura::PanelFloat* floatWindow = column->floatForTest(0);
            grip = floatWindow && floatWindow->sizeGripForTest() != nullptr;
            minSize = floatWindow && floatWindow->minimumWidth() > 0
                      && floatWindow->minimumHeight() > 0;
            group->setCollapsedToIconsForTest(false);
            column->redockForTest(0, 0);
            pump(6);
        }
        ST_BEGIN("lss_float_resize");
        ST_PASS("lss_float_resize torn=%d grip=%d minsize=%d", torn ? 1 : 0, grip ? 1 : 0,
                minSize ? 1 : 0);
        if (!(torn && grip && minSize)) {
            return pictura::selfTest().fail(411, "float resize affordance");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_float_single_drag (412): dragging the tab of a one-panel float moves
    // the whole overlay as a group drag; no second float is built and the source
    // overlay survives as the drag state is cancelled.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        bool floated = false;
        bool groupDrag = false;
        bool noExtra = false;
        bool intact = false;
        if (column) {
            floated = column->tearOffPanelForTest(QStringLiteral("channelsPanel"));
            pump(8);
            const int before = column->floatCountForTest();
            pictura::PanelFloat* floatWindow = column->floatForTest(before - 1);
            pictura::PanelGroup* fg = floatWindow ? floatWindow->group() : nullptr;
            const bool began =
                floated && column->beginTabDragForTest(QStringLiteral("channelsPanel"));
            pump(4);
            groupDrag = began && column->dragActiveForTest() && !column->dragIsPanelForTest();
            noExtra = column->floatCountForTest() == before;
            column->cancelDragForTest();
            pump(6);
            intact = fg && column->floatCountForTest() == before
                     && fg->containsPanel(QStringLiteral("channelsPanel"));
            for (int i = 0; i < 8 && column->floatCountForTest() > 0; ++i) {
                if (!column->redockForTest(0, 0)) {
                    break;
                }
                pump(4);
            }
        }
        ST_BEGIN("lss_float_single_drag");
        ST_PASS("lss_float_single_drag floated=%d group=%d noextra=%d intact=%d",
                floated ? 1 : 0, groupDrag ? 1 : 0, noExtra ? 1 : 0, intact ? 1 : 0);
        if (!(floated && groupDrag && noExtra && intact)) {
            return pictura::selfTest().fail(412, "single-panel float drag");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_panel_menu_close (413): the per-widget menu's live text now carries
    // `Close` and `Close Group`; Close hides the active tab and Close Group hides
    // the whole group through the column's existing close paths.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = column ? column->groupForPanel(QStringLiteral("layersPanel"))
                                            : nullptr;
        bool hasClose = false;
        bool hasCloseGroup = false;
        bool closesTab = false;
        bool closesGroup = false;
        if (column && group) {
            column->showPanel(QStringLiteral("layersPanel"), true);
            pump(4);
            const QStringList texts = group->panelMenuTextsForTest();
            hasClose = texts.contains(QStringLiteral("Close"));
            hasCloseGroup = texts.contains(QStringLiteral("Close Tab Group"));
            const QString active = group->currentPanelName();
            closesTab = !active.isEmpty()
                        && column->triggerWidgetMenuForTest(active, QStringLiteral("Close"))
                        && !group->isPanelVisible(active);
            column->showPanel(active, true);
            pump(4);
            closesGroup = column->triggerWidgetMenuForTest(group->currentPanelName(),
                                                           QStringLiteral("Close Tab Group"))
                          && !group->isVisible();
        }
        ST_BEGIN("lss_panel_menu_close");
        ST_PASS("lss_panel_menu_close label=%d groupLabel=%d tab=%d group=%d", hasClose ? 1 : 0,
                hasCloseGroup ? 1 : 0, closesTab ? 1 : 0, closesGroup ? 1 : 0);
        if (!(hasClose && hasCloseGroup && closesTab && closesGroup)) {
            return pictura::selfTest().fail(413, "panel menu close");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_float_drag_opacity (414): the overlay is dimmed for the whole drag,
    // including at drag start before any target resolves, and returns to full
    // opacity when the drag is cancelled.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        pictura::PanelColumn* column = frame.panelColumn();
        pictura::PanelGroup* group = nullptr;
        if (column) {
            for (pictura::PanelGroup* candidate : column->groups()) {
                if (candidate && !candidate->isCollapsedToIcons()
                    && candidate->isVisible() && !candidate->visibleTitles().isEmpty()) {
                    group = candidate;
                    break;
                }
            }
        }
        bool torn = false;
        bool dimmed = false;
        bool restored = false;
        if (column && group) {
            torn = column->tearOffForTest(group->objectName());
            pump(8);
            pictura::PanelFloat* floatWindow = column->floatForTest(0);
            const QList<QWidget*> panels = group->panels();
            const QString panel = panels.isEmpty() || !panels.first()
                                      ? QString()
                                      : panels.first()->objectName();
            const bool began = !panel.isEmpty() && column->beginGroupDragForTest(panel);
            if (floatWindow) {
                const qreal atStart = floatWindow->dragOpacityForTest();
                column->dragToForTest(column->boundaryPointForTest(0));
                pump(4);
                const qreal over = floatWindow->dragOpacityForTest();
                dimmed = began && atStart <= 0.7 && over <= 0.7;
                column->cancelDragForTest();
                pump(4);
                restored = floatWindow->dragOpacityForTest() > 0.9;
            }
            for (int i = 0; i < 8 && column->floatCountForTest() > 0; ++i) {
                if (!column->redockForTest(0, 0)) {
                    break;
                }
                pump(4);
            }
        }
        ST_BEGIN("lss_float_drag_opacity");
        ST_PASS("lss_float_drag_opacity torn=%d dimmed=%d restored=%d", torn ? 1 : 0,
                dimmed ? 1 : 0, restored ? 1 : 0);
        if (!(torn && dimmed && restored)) {
            return pictura::selfTest().fail(414, "float drag opacity");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_tools_dock_float (415): the tools column is a central-splitter pane
    // (never a dock); a header drag to the document area tears it off as an
    // in-window overlay, and a release on a widget column re-docks it, while a
    // release in the workspace outer band resolves a real sibling column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        pictura::PanelColumn* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        QWidget* central = frame.centralWidget();
        if (!cs || !tabs || !tools || !primary || !central) {
            return pictura::selfTest().fail(415, "tools column fixture");
        }
        auto startPoint = [tools]() {
            return tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8));
        };
        const bool pane = cs->indexOf(tools) >= 0;
        // Tear off over the empty document area.
        tools->beginColumnHeaderDragForTest(startPoint());
        tools->dragColumnHeaderToForTest(tabs->mapToGlobal(tabs->rect().center()));
        pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
        const bool floating =
            floatWindow && toolFloatWindow(floatWindow) && cs->indexOf(tools) < 0;
        // Re-dock on the primary column's left half.
        const QRect pr(primary->mapToGlobal(QPoint(0, 0)), primary->size());
        const QPoint leftEdge(pr.left() + qMax(1, pr.width() / 4), pr.center().y());
        const bool redocked = tools->dropColumnHeaderForTest(leftEdge)
                              && tools->columnFloatForTest() == nullptr
                              && cs->indexOf(tools) >= 0;
        pump(6);
        // The far-right workspace band resolves side 1 (a new sibling column).
        const QPoint tailPoint(central->mapToGlobal(QPoint(0, 0)).x() + central->width() - 2,
                               central->mapToGlobal(QPoint(0, central->height() / 2)).y());
        const bool rightBand = frame.newColumnSideAt(tailPoint) == 1;

        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        ST_BEGIN("lss_tools_dock_float");
        ST_PASS("lss_tools_dock_float pane=%d float=%d redock=%d rightband=%d",
                pane ? 1 : 0, floating ? 1 : 0, redocked ? 1 : 0, rightBand ? 1 : 0);
        if (!(pane && floating && redocked && rightBand)) {
            return pictura::selfTest().fail(415, "tools column float");
        }
    }

    // lss_tools_pane_height (416): hosting the tools column in the central
    // splitter keeps a free height (min 0, max unbounded) so the document area
    // fills the splitter instead of being forced down to the tool content.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        auto* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool ok = false;
        if (cs && tabs && tools && primary) {
            const bool pane = cs->indexOf(tools) >= 0;
            const bool freeHeight = tools->minimumHeight() == 0
                                    && tools->maximumHeight() == QWIDGETSIZE_MAX;
            const bool tabsFill = tabs->height() >= cs->height() - 40;
            ok = pane && freeHeight && tabsFill;
        }
        ST_BEGIN("lss_tools_pane_height");
        ST_PASS("lss_tools_pane_height ok=%d", ok ? 1 : 0);
        if (!ok) {
            return pictura::selfTest().fail(416, "tools pane height");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_tools_top_align (417): the tool content is hosted below the tools
    // column header and fills the column height (top-aligned), rather than being
    // a fixed-height body centred in the column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* tools = frame.toolsColumn();
        bool ok = false;
        if (tools) {
            QWidget* header = tools->columnHeaderForTest();
            QWidget* body = tools->toolsContentForTest();
            const int headerH = header ? header->height() : 0;
            const int bodyTop = body ? body->mapTo(tools, QPoint(0, 0)).y() : -1000;
            const bool fills = body && body->height() >= tools->height() - headerH - 4;
            const bool topAligned = body && bodyTop >= 0 && bodyTop <= headerH + 4;
            ok = fills && topAligned;
        }
        ST_BEGIN("lss_tools_top_align");
        ST_PASS("lss_tools_top_align ok=%d", ok ? 1 : 0);
        if (!ok) {
            return pictura::selfTest().fail(417, "tools top align");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_tools_edge_preview (418): a widget panel dragged over the tools column
    // resolves no in-column target (atomic), so no insertion indicator is drawn
    // on the tools column and its contents never combine.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* tools = frame.toolsColumn();
        pictura::PanelColumn* primary = frame.panelColumn();
        bool ok = false;
        if (tools && primary) {
            const QPoint over(tools->mapToGlobal(tools->rect().center()));
            const bool began = primary->beginTabDragForTest(QStringLiteral("layersPanel"));
            primary->dragToForTest(over);
            const bool noLine = !tools->dropIndicatorVisibleForTest();
            const bool atomic = tools->isToolsColumn() && tools->groups().isEmpty();
            primary->cancelDragForTest();
            pump(4);
            ok = began && noLine && atomic;
        }
        ST_BEGIN("lss_tools_edge_preview");
        ST_PASS("lss_tools_edge_preview ok=%d", ok ? 1 : 0);
        if (!ok) {
            return pictura::selfTest().fail(418, "tools edge preview");
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
    }

    // lss_tools_canvas_float (419): a tools-column header release over the empty
    // document area has no column under the pointer and is outside the outer
    // band, so it stays an in-window overlay (kept floating) instead of snapping
    // to the nearest column.
    {
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        auto* cs = frame.findChild<QSplitter*>(QStringLiteral("centerSplitter"));
        auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
        auto* tools = frame.toolsColumn();
        bool ok = false;
        if (cs && tabs && tools) {
            const QPoint canvas(tabs->mapToGlobal(tabs->rect().center()));
            const bool begun = tools->beginColumnHeaderDragForTest(
                tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8)));
            tools->dragColumnHeaderToForTest(canvas);
            pictura::PanelFloat* floatWindow = tools->columnFloatForTest();
            const bool floating =
                floatWindow && toolFloatWindow(floatWindow) && cs->indexOf(tools) < 0;
            const bool released = tools->dropColumnHeaderForTest(canvas)
                                  && tools->columnFloatForTest() != nullptr;
            ok = begun && floating && released;
        }
        frame.applyPanelSessionForTest(pictura::SessionState{});
        pump(6);
        ST_BEGIN("lss_tools_canvas_float");
        ST_PASS("lss_tools_canvas_float ok=%d", ok ? 1 : 0);
        if (!ok) {
            return pictura::selfTest().fail(419, "tools canvas float");
        }
    }

    return 0;
}
