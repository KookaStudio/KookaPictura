#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QScopeGuard>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QScreen>
#include <QtWidgets/QApplication>
#include <QtWidgets/QToolButton>

#include "frame.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "session.h"
#include "theme.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

void pump(int rounds = 4)
{
    for (int i = 0; i < rounds; ++i) {
        QCoreApplication::processEvents();
    }
}

QJsonArray rectJson(const QRect& rect)
{
    QJsonArray array;
    array.append(rect.x());
    array.append(rect.y());
    array.append(rect.width());
    array.append(rect.height());
    return array;
}

QJsonObject groupSpec(const QStringList& order, const QStringList& visible,
                      bool minimized = false, bool collapsed = false,
                      bool floating = false, const QRect& geometry = QRect())
{
    QJsonObject group;
    group.insert(QStringLiteral("order"), QJsonArray::fromStringList(order));
    group.insert(QStringLiteral("visible"), QJsonArray::fromStringList(visible));
    group.insert(QStringLiteral("minimized"), minimized);
    group.insert(QStringLiteral("collapsed"), collapsed);
    if (floating) {
        group.insert(QStringLiteral("floating"), true);
    }
    if (geometry.isValid()) {
        group.insert(QStringLiteral("floatGeometry"), rectJson(geometry));
    }
    return group;
}

QJsonObject columnSpec(const QString& side, int order, int width, const QString& railMode,
                       const QJsonArray& groups, bool floating = false,
                       const QRect& geometry = QRect())
{
    QJsonObject column;
    column.insert(QStringLiteral("side"), side);
    column.insert(QStringLiteral("order"), order);
    column.insert(QStringLiteral("width"), width);
    column.insert(QStringLiteral("railMode"), railMode);
    column.insert(QStringLiteral("groups"), groups);
    if (floating) {
        column.insert(QStringLiteral("floating"), true);
    }
    if (geometry.isValid()) {
        column.insert(QStringLiteral("floatGeometry"), rectJson(geometry));
    }
    return column;
}

QPoint documentTabsCenter(PicturaMainWindow* window)
{
    QWidget* tabs = window->findChild<QWidget*>(QStringLiteral("documentTabs"));
    return tabs ? tabs->mapToGlobal(tabs->rect().center()) : window->rect().center();
}

bool beginColumnFloat(PanelColumn* column)
{
    column->beginColumnHeaderDragForTest(
        column->mapToGlobal(QPoint(qMax(1, column->width() / 2), 8)));
    const QPoint outside = column->mapToGlobal(QPoint(column->width() / 2, -40));
    column->dragColumnHeaderToForTest(outside);
    return column->dropColumnHeaderForTest(outside);
}

void dragWidgetBy(QWidget* widget, const QPoint& delta)
{
    const QPoint local = widget->rect().center();
    const QPoint start = widget->mapToGlobal(local);
    const QPoint end = start + delta;
    QMouseEvent press(QEvent::MouseButtonPress, QPointF(local), QPointF(start),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(widget, &press);
    QMouseEvent move(QEvent::MouseMove, QPointF(local + delta), QPointF(end),
                     Qt::NoButton, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(widget, &move);
    QMouseEvent release(QEvent::MouseButtonRelease, QPointF(local + delta), QPointF(end),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(widget, &release);
    pump();
}

bool anyVisibleFlyout(PicturaMainWindow* window)
{
    const QList<QWidget*> flyouts =
        window->findChildren<QWidget*>(QStringLiteral("panelIconFlyout"));
    for (QWidget* flyout : flyouts) {
        if (flyout && flyout->isVisible()) {
            return true;
        }
    }
    return false;
}

} // namespace

class FloatingWorkspacesTest : public QObject {
    Q_OBJECT

private slots:
    void init() { PanelFloat::setForceChildOverlayForTest(false); }
    void cleanup() { PanelFloat::setForceChildOverlayForTest(false); }

    void flyoutHostsWholeGroup();
    void flyoutSurvivesOutsideClick();
    void flyoutTogglesAndSwitches();
    void escapeDoesNotCloseFlyout();
    void floatingIconRowOpensFlyout();
    void floatingIconRowShowsOneHeader();
    void flyoutGroupLeavesNoStaleOverlay();
    void flyoutFloatingRowDragLeavesNoStale();
    void floatingRowFlyoutCloseGroupNoStale();
    void flyoutClosesOnHideAll();
    void flyoutCloseButtonPlacement();
    void flyoutCloseButtonHidesFlyout();
    void flyoutCloseButtonOnlyInFlyout();
    void flyoutResizesFromLeftEdge();
    void flyoutResizesFromRightEdge();
    void flyoutResizesBesideFloatingRow();
    void flyoutResizeKeepsMinimum();
    void flyoutOpensAtMinimumWidth();
    void flyoutChildModeStaysInFrame();
    void floatHeaderMatchesDockedHeader();
    void floatCornerMenuPainted();
    void iconicColumnFloatKeepsTwoThirdsHeight();
    void normalColumnFloatKeepsTwoThirdsHeight();
    void iconicColumnSnapsWhenRailToggled();
    void workspaceRoundTripsGroupFloat();
    void workspaceRoundTripsColumnFloat();
    void workspaceRoundTripsFloatInsideFloat();
    void workspaceRoundTripsEmptyFloatingColumn();
    void workspaceSwitchRemembersFloats();
    void workspaceWithoutFloatsDocksEverything();
    void restartRestoresFloats();
    void restartRestoresAutoFlags();
    void restoredFloatIsClamped();
    void toolsFloatRestores();
    void toolsFloatSurvivesWorkspaceApply();
    void legacyStoreDocksEverything();
    void schema4StoreKeepsDefaultGroups();
    void startupChildFloatUsesFrameGeometry();
};

void FloatingWorkspacesTest::flyoutHostsWholeGroup()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QCOMPARE(column->iconFlyoutGroupForTest(), group);
    QCOMPARE(group->currentPanelName(), QStringLiteral("layersPanel"));
    QVERIFY(group->titles().size() >= 2);
    QCOMPARE(column->activeIconNameForTest(), QStringLiteral("layersPanel"));

    QVERIFY(column->triggerFlyoutCloseForTest());
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    QCOMPARE(column->groupForPanel(QStringLiteral("layersPanel")), group);
}

void FloatingWorkspacesTest::flyoutSurvivesOutsideClick()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QWidget* flyout = column->iconFlyoutForTest();
    QVERIFY(flyout);
    QCOMPARE(flyout->windowType(), Qt::Tool);

    QTest::mouseClick(window->centralWidget(), Qt::LeftButton, Qt::NoModifier, QPoint(10, 10));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutTogglesAndSwitches()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    PanelGroup* layersGroup = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(layersGroup);

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("channelsPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QCOMPARE(column->iconFlyoutGroupForTest(), layersGroup);
    QCOMPARE(layersGroup->currentPanelName(), QStringLiteral("channelsPanel"));

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("colorPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QVERIFY(column->iconFlyoutGroupForTest()->containsPanel(QStringLiteral("colorPanel")));
    QCOMPARE(column->groupForPanel(QStringLiteral("layersPanel")), layersGroup);
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::escapeDoesNotCloseFlyout()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QWidget* flyout = column->iconFlyoutForTest();
    QVERIFY(flyout);
    QKeyEvent escape(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(flyout, &escape);
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::floatingIconRowOpensFlyout()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    QCOMPARE(column->floatCountForTest(), 1);
    PanelGroup* floatGroup = column->floatForTest(0)->group();
    QVERIFY(floatGroup);
    floatGroup->setCollapsedToIconsForTest(true);
    pump();

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QCOMPARE(column->iconFlyoutGroupForTest(), floatGroup);
    QTest::mouseClick(window->centralWidget(), Qt::LeftButton, Qt::NoModifier, QPoint(10, 10));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    column->triggerFlyoutCloseForTest();
    pump();
    QVERIFY(floatGroup->isCollapsedToIcons());
    QCOMPARE(column->floatCountForTest(), 1);
}

void FloatingWorkspacesTest::floatingIconRowShowsOneHeader()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    PanelGroup* floatGroup = column->floatForTest(0)->group();
    QVERIFY(floatGroup);
    floatGroup->setCollapsedToIconsForTest(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QVERIFY(!floatGroup->floatHeaderVisibleForTest());
    QVERIFY(floatGroup->floatCloseButton());
    QVERIFY(!floatGroup->floatCloseButton()->isVisible());
    QToolButton* flyoutClose = floatGroup->flyoutCloseButtonForTest();
    QVERIFY(flyoutClose && flyoutClose->isVisible());
    column->triggerFlyoutCloseForTest();
    pump();
    QVERIFY(floatGroup->floatHeaderVisibleForTest());
}

void FloatingWorkspacesTest::flyoutGroupLeavesNoStaleOverlay()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();

    // Drag the hosted group out of the flyout.
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    QVERIFY(column->beginGroupDragForTest(QStringLiteral("layersPanel")));
    const QPoint outside =
        window->mapToGlobal(QPoint(window->width() / 2, window->height() + 400));
    column->dragToForTest(outside);
    QVERIFY(column->dropForTest(outside));
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    QCOMPARE(column->floatCountForTest(), 1);
    column->closeFloatForTest(0);
    pump();

    // Closing the last visible panel of a hosted group that was not floated.
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("adjustmentsPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    PanelGroup* hosted = column->iconFlyoutGroupForTest();
    QVERIFY(hosted);
    const QList<QWidget*> hostedPanels = hosted->panels();
    for (QWidget* panel : hostedPanels) {
        if (panel && hosted->isPanelVisible(panel->objectName())) {
            QVERIFY(column->showPanel(panel->objectName(), false));
        }
    }
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());

    // Removing a dynamic owner column while its flyout is open.
    QVERIFY(window->newColumnDropForTest(QStringLiteral("stylesPanel"),
                                         QStringLiteral("left")));
    pump();
    PanelColumn* dynamic = window->columnForPanel(QStringLiteral("stylesPanel"));
    QVERIFY(dynamic);
    dynamic->setRailMode(true);
    pump();
    QVERIFY(dynamic->openIconFlyoutForTest(QStringLiteral("stylesPanel")));
    pump();
    PanelGroup* dynamicGroup = dynamic->groupForPanel(QStringLiteral("stylesPanel"));
    QVERIFY(dynamicGroup);
    dynamic->closeGroup(dynamicGroup);
    pump();
    QVERIFY(!anyVisibleFlyout(window.get()));

    // Rebuilding the layout while a flyout is open.
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("colorPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    window->applyWorkspaceLayoutForTest(QJsonArray{});
    pump();
    QVERIFY(!anyVisibleFlyout(window.get()));
}

void FloatingWorkspacesTest::flyoutFloatingRowDragLeavesNoStale()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    PanelGroup* floating = column->floatForTest(0)->group();
    QVERIFY(floating);
    floating->setCollapsedToIconsForTest(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());

    QVERIFY(column->beginGroupDragForTest(QStringLiteral("layersPanel")));
    const QPoint outside =
        window->mapToGlobal(QPoint(window->width() / 2, window->height() + 400));
    column->dragToForTest(outside);
    QVERIFY(column->dropForTest(outside));
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    QCOMPARE(column->floatCountForTest(), 1);
    QVERIFY(column->floatForTest(0)->group()->containsPanel(QStringLiteral("layersPanel")));
}

void FloatingWorkspacesTest::floatingRowFlyoutCloseGroupNoStale()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    PanelGroup* floating = column->floatForTest(0)->group();
    QVERIFY(floating);
    floating->setCollapsedToIconsForTest(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());

    column->closeGroup(floating);
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    QVERIFY(!anyVisibleFlyout(window.get()));
}

void FloatingWorkspacesTest::flyoutClosesOnHideAll()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    window->setPanelsHidden(true);
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    window->setPanelsHidden(false);
    pump();
}

void FloatingWorkspacesTest::flyoutCloseButtonPlacement()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutVisibleForTest());
    PanelGroup* group = column->iconFlyoutGroupForTest();
    QVERIFY(group);
    QToolButton* close = group->flyoutCloseButtonForTest();
    QToolButton* menu = group->headerMenuButtonForTest();
    QVERIFY(close && menu);
    QVERIFY(close->isVisible());
    QVERIFY(close->mapTo(group, QPoint(0, 0)).x()
            < menu->mapTo(group, QPoint(0, 0)).x());
    QVERIFY(group->headerCornerWidthForTest() >= 16 + 18 + 18 + 2);

    QToolButton* stripButton =
        column->findChild<QToolButton*>(QStringLiteral("panelIcon_layersPanel"));
    QVERIFY(stripButton);
    const QRect buttonRect(stripButton->mapToGlobal(QPoint(0, 0)), stripButton->size());
    const QRect flyoutRect = column->iconFlyoutGeometryForTest();
    QVERIFY(flyoutRect.isValid());
    QVERIFY(flyoutRect.right() < buttonRect.left());
    QVERIFY(!flyoutRect.intersects(buttonRect));
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutCloseButtonHidesFlyout()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    PanelGroup* group = column->iconFlyoutGroupForTest();
    QVERIFY(group);
    QToolButton* close = group->flyoutCloseButtonForTest();
    QVERIFY(close && close->isVisible());
    close->click();
    pump();
    QVERIFY(!column->iconFlyoutVisibleForTest());
    QCOMPARE(column->groupForPanel(QStringLiteral("layersPanel")), group);
}

void FloatingWorkspacesTest::flyoutCloseButtonOnlyInFlyout()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* docked = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(docked);
    QVERIFY(docked->flyoutCloseButtonForTest());
    QVERIFY(!docked->flyoutCloseButtonForTest()->isVisible());

    QVERIFY(column->tearOffForTest(docked->objectName()));
    pump();
    PanelGroup* floating = column->floatForTest(0)->group();
    QVERIFY(floating);
    QVERIFY(!floating->flyoutCloseButtonForTest()->isVisible());

    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutGroupForTest()->flyoutCloseButtonForTest()->isVisible());
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutResizesFromLeftEdge()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QCOMPARE(column->flyoutSideForTest(), QStringLiteral("left"));
    QWidget* grip = column->iconFlyoutResizeGripForTest();
    QVERIFY(grip && grip->isVisible());
    const QRect before = column->iconFlyoutGeometryForTest();
    dragWidgetBy(grip, QPoint(-40, 0));
    const QRect after = column->iconFlyoutGeometryForTest();
    QVERIFY(after.width() > before.width());
    QVERIFY(qAbs(after.right() - before.right()) <= 1);
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutResizesFromRightEdge()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    QVERIFY(window->newColumnDropForTest(QStringLiteral("stylesPanel"), QStringLiteral("left")));
    pump();
    PanelColumn* column = window->columnForPanel(QStringLiteral("stylesPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QCOMPARE(column->flyoutSideForTest(), QStringLiteral("right"));
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("stylesPanel")));
    pump();
    QWidget* grip = column->iconFlyoutResizeGripForTest();
    QVERIFY(grip && grip->isVisible());
    const QRect before = column->iconFlyoutGeometryForTest();
    dragWidgetBy(grip, QPoint(40, 0));
    const QRect after = column->iconFlyoutGeometryForTest();
    QVERIFY(after.width() > before.width());
    QVERIFY(qAbs(after.left() - before.left()) <= 1);
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutResizesBesideFloatingRow()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    QJsonArray leftGroups{
        groupSpec({QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")},
                  {QStringLiteral("colorPanel"), QStringLiteral("swatchesPanel")}),
        groupSpec({QStringLiteral("layersPanel"), QStringLiteral("channelsPanel")},
                  {QStringLiteral("layersPanel"), QStringLiteral("channelsPanel")})};
    QJsonArray rightGroups{
        groupSpec({QStringLiteral("adjustmentsPanel")},
                  {QStringLiteral("adjustmentsPanel")})};
    window->applyWorkspaceLayoutForTest(
        QJsonArray{columnSpec(QStringLiteral("left"), 0, 300, QStringLiteral("normal"),
                              leftGroups),
                   columnSpec(QStringLiteral("right"), 1, 300, QStringLiteral("normal"),
                              rightGroups)});
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(column);
    QCOMPARE(column->side(), PanelSide::Left);
    QVERIFY(column->tearOffPanelForTest(QStringLiteral("colorPanel")));
    pump();
    column = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(column);
    QCOMPARE(column->side(), PanelSide::Left);
    QCOMPARE(column->floatCountForTest(), 1);
    PanelGroup* floating = column->floatForTest(0)->group();
    QVERIFY(floating);
    floating->setCollapsedToIconsForTest(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("colorPanel")));
    pump();
    QCOMPARE(column->flyoutSideForTest(), QStringLiteral("right"));
    QWidget* grip = column->iconFlyoutResizeGripForTest();
    QVERIFY(grip && grip->isVisible());
    const QRect before = column->iconFlyoutGeometryForTest();
    dragWidgetBy(grip, QPoint(40, 0));
    const QRect after = column->iconFlyoutGeometryForTest();
    QVERIFY(after.width() > before.width());
    QVERIFY(qAbs(after.left() - before.left()) <= 1);
    column->triggerFlyoutCloseForTest();
    pump();
}
void FloatingWorkspacesTest::flyoutResizeKeepsMinimum()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QWidget* grip = column->iconFlyoutResizeGripForTest();
    QVERIFY(grip);
    dragWidgetBy(grip, QPoint(500, 0));
    QCOMPARE(column->iconFlyoutGeometryForTest().width(), PanelColumn::kMinNormalWidth);
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutOpensAtMinimumWidth()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    QVERIFY(column->iconFlyoutGeometryForTest().width() >= PanelColumn::kMinNormalWidth);
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::flyoutChildModeStaysInFrame()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    PanelFloat::setForceChildOverlayForTest(true);
    auto reset = qScopeGuard([] { PanelFloat::setForceChildOverlayForTest(false); });
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    pump();
    const QRect frameRect(window->mapToGlobal(QPoint(0, 0)), window->size());
    QRect flyout = column->iconFlyoutGeometryForTest();
    QVERIFY(flyout.left() >= frameRect.left() && flyout.right() <= frameRect.right());
    QVERIFY(flyout.top() >= frameRect.top() && flyout.bottom() <= frameRect.bottom());
    QWidget* grip = column->iconFlyoutResizeGripForTest();
    QVERIFY(grip);
    dragWidgetBy(grip, QPoint(-40, 0));
    flyout = column->iconFlyoutGeometryForTest();
    QVERIFY(flyout.left() >= frameRect.left() && flyout.right() <= frameRect.right());
    column->triggerFlyoutCloseForTest();
    pump();
}

void FloatingWorkspacesTest::floatHeaderMatchesDockedHeader()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    QWidget* floatHeader = group->floatHeaderForTest();
    QVERIFY(floatHeader && floatHeader->width() > 4 && floatHeader->height() > 4);
    const QColor floatColor = floatHeader->grab().toImage().pixelColor(4, 4);
    QWidget* dockedHeader = column->findChild<QWidget*>(QStringLiteral("panelColumnHeader"));
    QVERIFY(dockedHeader);
    const QColor dockedColor = dockedHeader->grab().toImage().pixelColor(4, 4);
    QCOMPARE(floatColor, dockedColor);
    QCOMPARE(floatColor, Theme::tokenColor(QStringLiteral("surface-header")));
}

void FloatingWorkspacesTest::floatCornerMenuPainted()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    PanelFloat* floatWindow = column->floatForTest(0);
    QVERIFY(floatWindow);
    QToolButton* button = group->headerMenuButtonForTest();
    QVERIFY(button && button->isVisible());
    const QRect rect(button->mapTo(floatWindow, QPoint(0, 0)), button->size());
    QVERIFY(floatWindow->rect().contains(rect));
    const QImage image = floatWindow->grab().toImage();
    const QColor background = Theme::tokenColor(QStringLiteral("surface-header"));
    bool painted = false;
    for (int y = rect.top() + 2; y <= rect.bottom() - 2 && !painted; ++y) {
        for (int x = rect.left() + 2; x <= rect.right() - 2; ++x) {
            if (image.pixelColor(x, y) != background) {
                painted = true;
                break;
            }
        }
    }
    QVERIFY(painted);
}

void FloatingWorkspacesTest::iconicColumnFloatKeepsTwoThirdsHeight()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    column->setRailMode(true);
    pump();
    const int docked = column->height();
    QVERIFY(docked > 0);
    QVERIFY(beginColumnFloat(column));
    pump();
    PanelFloat* floatWindow = column->columnFloatForTest();
    QVERIFY(floatWindow);
    const int expected = qMax(PanelFloat::kFloatMinHeight, (docked * 2) / 3);
    QVERIFY(qAbs(floatWindow->height() - expected) <= 2);
    QVERIFY(floatWindow->height() < docked);
}

void FloatingWorkspacesTest::normalColumnFloatKeepsTwoThirdsHeight()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QVERIFY(!column->railMode());
    const int docked = column->height();
    QVERIFY(docked > 0);
    QVERIFY(beginColumnFloat(column));
    pump();
    PanelFloat* floatWindow = column->columnFloatForTest();
    QVERIFY(floatWindow);
    const int expected = qMax(PanelFloat::kFloatMinHeight, (docked * 2) / 3);
    QVERIFY(qAbs(floatWindow->height() - expected) <= 2);
    QVERIFY(floatWindow->height() < docked);
}

void FloatingWorkspacesTest::iconicColumnSnapsWhenRailToggled()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QVERIFY(beginColumnFloat(column));
    pump();
    PanelFloat* floatWindow = column->columnFloatForTest();
    QVERIFY(floatWindow);
    const int before = floatWindow->height();
    column->setRailMode(true);
    pump();
    QVERIFY(floatWindow->height() <= before);
    QVERIFY(floatWindow->height()
            <= qMax(column->minimumSizeHint().height(), PanelFloat::kFloatIconMinHeight) + 2);
}

void FloatingWorkspacesTest::workspaceRoundTripsGroupFloat()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(column);
    QVERIFY(column->tearOffPanelForTest(QStringLiteral("colorPanel")));
    pump();
    QCOMPARE(column->floatCountForTest(), 1);
    const QRect before = column->floatGeometryForTest(0);

    const QJsonArray layout = window->workspaceLayoutForTest();
    window->applyWorkspaceLayoutForTest(layout);
    pump();

    PanelColumn* restored = window->columnForPanel(QStringLiteral("colorPanel"));
    QVERIFY(restored);
    QCOMPARE(restored->floatCountForTest(), 1);
    QVERIFY(restored->floatForTest(0)->group()->containsPanel(QStringLiteral("colorPanel")));
    const QRect after = restored->floatGeometryForTest(0);
    QVERIFY(qAbs(after.left() - before.left()) <= 2);
    QVERIFY(qAbs(after.top() - before.top()) <= 2);
    QVERIFY(qAbs(after.size().width() - before.size().width()) <= 2);
    QVERIFY(qAbs(after.size().height() - before.size().height()) <= 2);
    QVERIFY(window->columnForPanel(QStringLiteral("swatchesPanel")));
}

void FloatingWorkspacesTest::workspaceRoundTripsColumnFloat()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QVERIFY(beginColumnFloat(column));
    pump();
    QVERIFY(column->isColumnFloating());
    const QRect before = column->columnFloatGeometry();

    const QJsonArray layout = window->workspaceLayoutForTest();
    window->applyWorkspaceLayoutForTest(layout);
    pump();

    PanelColumn* restored = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(restored);
    QVERIFY(restored->isColumnFloating());
    const QRect after = restored->columnFloatGeometry();
    QVERIFY(qAbs(after.left() - before.left()) <= 2);
    QVERIFY(qAbs(after.top() - before.top()) <= 2);
    QVERIFY(qAbs(after.width() - before.width()) <= 2);
    QVERIFY(qAbs(after.height() - before.height()) <= 2);
}

void FloatingWorkspacesTest::workspaceRoundTripsFloatInsideFloat()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QVERIFY(beginColumnFloat(column));
    pump();
    QVERIFY(column->isColumnFloating());
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    QCOMPARE(column->floatCountForTest(), 1);

    const QJsonArray layout = window->workspaceLayoutForTest();
    window->applyWorkspaceLayoutForTest(layout);
    pump();
    PanelColumn* restored = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(restored);
    QVERIFY(restored->isColumnFloating());
    QCOMPARE(restored->floatCountForTest(), 1);
}

void FloatingWorkspacesTest::workspaceRoundTripsEmptyFloatingColumn()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    QJsonObject group =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")},
                  false, false, true, QRect(150, 120, 300, 200));
    QJsonArray layout{columnSpec(QStringLiteral("right"), 0, 300,
                                 QStringLiteral("normal"), QJsonArray{group}, true,
                                 QRect(100, 80, 300, 400))};
    window->applyWorkspaceLayoutForTest(layout);
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QVERIFY(column->isColumnFloating());
    QCOMPARE(column->floatCountForTest(), 1);
    QVERIFY(column->floatForTest(0)->group()->containsPanel(QStringLiteral("layersPanel")));
}

void FloatingWorkspacesTest::workspaceSwitchRemembersFloats()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    QVERIFY(window->createWorkspaceForTest(QStringLiteral("Float Workspace")));
    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    QCOMPARE(column->floatCountForTest(), 1);
    const QRect before = column->floatGeometryForTest(0);

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    pump();
    window->switchWorkspaceForTest(QStringLiteral("Float Workspace"));
    pump();
    PanelColumn* restored = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(restored);
    QCOMPARE(restored->floatCountForTest(), 1);
    const QRect after = restored->floatGeometryForTest(0);
    QVERIFY(qAbs(after.left() - before.left()) <= 2);
    QVERIFY(qAbs(after.top() - before.top()) <= 2);
}

void FloatingWorkspacesTest::workspaceWithoutFloatsDocksEverything()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group);
    QVERIFY(column->tearOffForTest(group->objectName()));
    pump();
    QCOMPARE(column->floatCountForTest(), 1);

    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    pump();
    for (PanelColumn* candidate : window->allPanelColumns()) {
        QVERIFY(candidate);
        QCOMPARE(candidate->floatCountForTest(), 0);
        QVERIFY(!candidate->isColumnFloating());
    }
}

void FloatingWorkspacesTest::restartRestoresFloats()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QRect savedGroup;
    QRect savedColumn;
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
        QVERIFY(column);
        QVERIFY(column->tearOffPanelForTest(QStringLiteral("colorPanel")));
        pump();
        savedGroup = column->floatGeometryForTest(0);
        window->saveSession();
    }
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* column = window->columnForPanel(QStringLiteral("colorPanel"));
        QVERIFY(column);
        QCOMPARE(column->floatCountForTest(), 1);
        QVERIFY(column->floatForTest(0)->group()->containsPanel(QStringLiteral("colorPanel")));
        const QRect restoredGroup = column->floatGeometryForTest(0);
        QVERIFY(qAbs(restoredGroup.left() - savedGroup.left()) <= 2);
        QVERIFY(qAbs(restoredGroup.top() - savedGroup.top()) <= 2);
        PanelColumn* primary = window->columnForPanel(QStringLiteral("layersPanel"));
        QVERIFY(primary);
        QVERIFY(beginColumnFloat(primary));
        pump();
        QVERIFY(primary->isColumnFloating());
        savedColumn = primary->columnFloatGeometry();
        window->saveSession();
    }
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
        QVERIFY(column);
        QVERIFY(column->isColumnFloating());
        QVERIFY(column->floatCountForTest() >= 1);
        const QRect restoredColumn = column->columnFloatGeometry();
        QVERIFY(qAbs(restoredColumn.left() - savedColumn.left()) <= 2);
        QVERIFY(qAbs(restoredColumn.top() - savedColumn.top()) <= 2);
    }
}

void FloatingWorkspacesTest::restartRestoresAutoFlags()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* column = window->panelColumn();
        QVERIFY(column);
        column->setAutoCollapseIconic(true);
        column->setAutoShowHidden(true);
        window->saveSession();
    }
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* column = window->panelColumn();
        QVERIFY(column);
        QVERIFY(column->autoCollapseIconicForTest());
        QVERIFY(column->autoShowHiddenForTest());
    }
}

void FloatingWorkspacesTest::restoredFloatIsClamped()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    QJsonObject hugeGroup =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")},
                  false, false, true, QRect(10000, 10000, 5000, 5000));
    QJsonArray layout{columnSpec(QStringLiteral("right"), 0, 300, QStringLiteral("normal"),
                                 QJsonArray{hugeGroup})};
    window->applyWorkspaceLayoutForTest(layout);
    pump();
    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QCOMPARE(column->floatCountForTest(), 1);
    PanelFloat* floatWindow = column->floatForTest(0);
    QVERIFY(floatWindow);
    const QRect bounds = PanelFloat::overlayUsesTopLevel()
                             ? window->screen()->availableGeometry()
                             : window->rect();
    QVERIFY(floatWindow->width() >= PanelColumn::kMinNormalWidth);
    QVERIFY(floatWindow->height() >= PanelFloat::kFloatMinHeight);
    QVERIFY(floatWindow->width() <= bounds.width());
    QVERIFY(floatWindow->height() <= bounds.height());
    QVERIFY(floatWindow->geometry().right() <= bounds.right());
    QVERIFY(floatWindow->geometry().bottom() <= bounds.bottom());

    QJsonObject tinyGroup =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")},
                  false, false, true, QRect(100, 100, 20, 10));
    QJsonArray tinyLayout{columnSpec(QStringLiteral("right"), 0, 300,
                                     QStringLiteral("normal"), QJsonArray{tinyGroup})};
    window->applyWorkspaceLayoutForTest(tinyLayout);
    pump();
    column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    floatWindow = column->floatForTest(0);
    QVERIFY(floatWindow);
    QVERIFY(floatWindow->width() >= PanelColumn::kMinNormalWidth);
    QVERIFY(floatWindow->height() >= PanelFloat::kFloatMinHeight);

    QJsonObject columnGroup =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")});
    QJsonArray columnLayout{columnSpec(QStringLiteral("right"), 0, 300,
                                       QStringLiteral("normal"),
                                       QJsonArray{columnGroup}, true,
                                       QRect(10000, 10000, 5000, 5000))};
    window->applyWorkspaceLayoutForTest(columnLayout);
    pump();
    PanelColumn* floatingColumn = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(floatingColumn);
    QVERIFY(floatingColumn->isColumnFloating());
    const QRect columnGeometry = floatingColumn->columnFloatGeometry();
    QVERIFY(columnGeometry.width() <= bounds.width());
    QVERIFY(columnGeometry.height() <= bounds.height());
    QVERIFY(columnGeometry.right() <= bounds.right());
    QVERIFY(columnGeometry.bottom() <= bounds.bottom());
}

void FloatingWorkspacesTest::toolsFloatRestores()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QRect savedGeometry;
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* tools = window->toolsColumn();
        QVERIFY(tools);
        QVERIFY(tools->beginColumnHeaderDragForTest(
            tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8))));
        const QPoint outside = documentTabsCenter(window.get());
        tools->dragColumnHeaderToForTest(outside);
        QVERIFY(tools->dropColumnHeaderForTest(outside));
        pump();
        QVERIFY(tools->isColumnFloating());
        savedGeometry = tools->columnFloatGeometry();
        const QJsonArray layout = window->workspaceLayoutForTest();
        bool found = false;
        for (const QJsonValue& value : layout) {
            const QJsonObject entry = value.toObject();
            if (!entry.value(QStringLiteral("tools")).toBool()) {
                continue;
            }
            found = true;
            QVERIFY(entry.value(QStringLiteral("floating")).toBool());
            QCOMPARE(entry.value(QStringLiteral("floatGeometry")).toArray().size(), 4);
            QVERIFY(entry.value(QStringLiteral("groups")).toArray().isEmpty());
        }
        QVERIFY(found);
        window->saveSession();
    }
    {
        auto window = pictura::test::makeMainWindow();
        window->resize(1100, 700);
        window->show();
        pump();
        PanelColumn* tools = window->toolsColumn();
        QVERIFY(tools);
        QVERIFY(tools->isColumnFloating());
        QCOMPARE(tools->columnFloatGeometry().topLeft(), savedGeometry.topLeft());
    }
}

void FloatingWorkspacesTest::toolsFloatSurvivesWorkspaceApply()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools);
    QVERIFY(tools->beginColumnHeaderDragForTest(
        tools->mapToGlobal(QPoint(qMax(1, tools->width() / 2), 8))));
    const QPoint outside = documentTabsCenter(window.get());
    tools->dragColumnHeaderToForTest(outside);
    QVERIFY(tools->dropColumnHeaderForTest(outside));
    pump();
    QVERIFY(tools->isColumnFloating());
    const QRect before = tools->columnFloatGeometry();
    window->switchWorkspaceForTest(QStringLiteral("Painting"));
    pump();
    QVERIFY(tools->isColumnFloating());
    QCOMPARE(tools->columnFloatGeometry().topLeft(), before.topLeft());
}

void FloatingWorkspacesTest::legacyStoreDocksEverything()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    pictura::SessionState state;
    state.schemaVersion = 9;
    QJsonObject legacyGroup =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")},
                  false, false, true);
    state.panelColumns =
        QJsonArray{columnSpec(QStringLiteral("right"), 0, 300,
                              QStringLiteral("normal"), QJsonArray{legacyGroup},
                              true)};
    QVERIFY(pictura::saveSession(state));

    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();
    PanelColumn* column = window->panelColumn();
    QVERIFY(column);
    QCOMPARE(column->floatCountForTest(), 0);
    QVERIFY(!column->isColumnFloating());
}

void FloatingWorkspacesTest::schema4StoreKeepsDefaultGroups()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    const QString path = pictura::sessionFilePath();
    QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
    QFile store(path);
    QVERIFY(store.open(QIODevice::WriteOnly | QIODevice::Truncate));
    store.write("{\"schemaVersion\":4}");
    store.close();

    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump();
    QVERIFY(window->columnForPanel(QStringLiteral("layersPanel")));
    QVERIFY(window->columnForPanel(QStringLiteral("colorPanel")));
    PanelColumn* layers = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(layers);
    QVERIFY(layers->groupForPanel(QStringLiteral("layersPanel")));
}

void FloatingWorkspacesTest::startupChildFloatUsesFrameGeometry()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    PanelFloat::setForceChildOverlayForTest(true);
    auto reset = qScopeGuard([] { PanelFloat::setForceChildOverlayForTest(false); });

    pictura::SessionState state;
    state.schemaVersion = 10;
    QJsonObject group =
        groupSpec({QStringLiteral("layersPanel")}, {QStringLiteral("layersPanel")},
                  false, false, true, QRect(900, 600, 300, 200));
    state.panelColumns =
        QJsonArray{columnSpec(QStringLiteral("right"), 0, 300,
                              QStringLiteral("normal"), QJsonArray{group})};
    QVERIFY(pictura::saveSession(state));

    auto window = pictura::test::makeMainWindow();
    window->resize(1100, 700);
    window->show();
    pump(8);
    PanelColumn* column = window->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column);
    QCOMPARE(column->floatCountForTest(), 1);
    PanelFloat* floatWindow = column->floatForTest(0);
    QVERIFY(floatWindow);
    window->resize(500, 300);
    pump(8);
    QVERIFY(floatWindow->geometry().right() < window->width());
    QVERIFY(floatWindow->geometry().bottom() < window->height());
}

int main(int argc, char *argv[])
{
    return pictura::test::shardedMain<FloatingWorkspacesTest>(argc, argv);
}
#include "tst_floating_workspaces.moc"
