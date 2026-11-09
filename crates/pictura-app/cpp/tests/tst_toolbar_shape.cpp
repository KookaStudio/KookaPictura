#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtGui/QAction>
#include <QtGui/QMouseEvent>
#include <QtGui/QScreen>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QToolButton>

#include <memory>

#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "icons.h"
#include "panels/panel_column.h"
#include "toolbox.h"

#include "qt_test_support.h"

using pictura::PanelColumn;
using pictura::PanelSide;
using pictura::Toolbox;

namespace {

void pump()
{
    QCoreApplication::processEvents();
    QCoreApplication::processEvents();
}

bool beginGripDrag(PanelColumn* tools, const QPoint& target)
{
    QWidget* grip = tools ? tools->toolsGripForTest() : nullptr;
    if (!grip) {
        return false;
    }
    const QPoint start = grip->mapToGlobal(grip->rect().center());
    QMouseEvent press(QEvent::MouseButtonPress, grip->mapFromGlobal(start), start,
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(grip, &press);
    const QPoint step = start + QPoint(QApplication::startDragDistance() + 5, 0);
    QMouseEvent move(QEvent::MouseMove, grip->mapFromGlobal(step), step, Qt::NoButton,
                     Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(grip, &move);
    if (target != step) {
        QMouseEvent moveTo(QEvent::MouseMove, grip->mapFromGlobal(target), target,
                           Qt::NoButton, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(grip, &moveTo);
    }
    return tools->columnFloatForTest() != nullptr;
}

QPoint centralEdge(pictura::PicturaMainWindow* window, bool left)
{
    QWidget* central = window->centralWidget();
    const int x = left ? 2 : qMax(2, central->width() - 2);
    return central->mapToGlobal(QPoint(x, central->height() / 2));
}

} // namespace

class ToolbarShapeTest : public QObject {
    Q_OBJECT

private slots:
    void compactLayoutHasOneColumnAndNoScrollbar();
    void doubleArrowExpandsToTwoColumns();
    void gripDragFloatsOutsideTheWindow();
    void bothLayoutsDockLeftAndRight();
    void toolbarVisibleOnStartupWhenWorkspaceIsShort();
    void closeHidesAndWindowMenuRestores();
    void floatingModeFlipKeepsFullHeight();
    void floatingExpandedToCompactKeepsAllTools();
    void closeFloatingToolbarRemovesOverlay();
    void floatingDragOverAppWindowStaysFloating();
    void floatingDragBesideAppWindowStaysFloating();
};

void ToolbarShapeTest::compactLayoutHasOneColumnAndNoScrollbar()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    tools->show();
    toolbox->setColumns(1);
    pump();

    QCOMPARE(toolbox->columns(), 1);
    QCOMPARE(tools->toolsScrollBarPolicyForTest(),
             static_cast<int>(Qt::ScrollBarAlwaysOff));
    const int contentWidth = toolbox->contentWidthForTest();
    QTRY_VERIFY2(tools->width() > 0 && tools->width() <= contentWidth + 4,
                 "the tools column hugs the toolbar instead of leaving a grey strip");
    QVERIFY(tools->toolsGripForTest() != nullptr);
    QVERIFY(!tools->toolsGripForTest()->isHidden());
    auto* grip = qobject_cast<QLabel*>(tools->toolsGripForTest());
    QVERIFY2(grip != nullptr, "the tools grip is the panel-group `•••` label");
    QCOMPARE(grip->text(), QStringLiteral("\u2022\u2022\u2022"));
    QVERIFY(tools->toolsCloseButtonForTest() != nullptr);
    QVERIFY(!tools->toolsCloseButtonForTest()->isHidden());
    QToolButton* toggle = tools->panelColumnToggleForTest();
    QVERIFY(toggle != nullptr);
    QCOMPARE(toggle->icon().pixmap(16, 16).toImage(),
             pictura::icon(QStringLiteral("panel.columnsTwo")).pixmap(16, 16).toImage());
}

void ToolbarShapeTest::doubleArrowExpandsToTwoColumns()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    tools->show();
    toolbox->setColumns(1);
    pump();
    QToolButton* toggle = tools->panelColumnToggleForTest();
    QVERIFY(toggle != nullptr);
    const int compactWidth = toolbox->contentWidthForTest();

    toggle->click();
    pump();

    QCOMPARE(toolbox->columns(), 2);
    QVERIFY(toolbox->contentWidthForTest() > compactWidth);
    QCOMPARE(tools->toolsScrollBarPolicyForTest(),
             static_cast<int>(Qt::ScrollBarAlwaysOff));
    QVERIFY(!tools->toolsGripForTest()->isHidden());
    QVERIFY(!tools->toolsCloseButtonForTest()->isHidden());
    QCOMPARE(toggle->icon().pixmap(16, 16).toImage(),
             pictura::icon(QStringLiteral("panel.columnsOne")).pixmap(16, 16).toImage());

    toggle->click();
    pump();
    QCOMPARE(toolbox->columns(), 1);
    const int slotWidth = toolbox->slotButtons().first()->width();
    QTRY_VERIFY(tools->width() >= slotWidth + 4);
    QTRY_VERIFY(toolbox->width() >= slotWidth);
}

void ToolbarShapeTest::gripDragFloatsOutsideTheWindow()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QScreen* screen = QGuiApplication::primaryScreen();
    QVERIFY(screen != nullptr);
    const QRect available = screen->availableGeometry();
    window->move(available.topLeft());
    window->resize(500, 400);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    tools->show();
    pump();

    const QRect frameRect = window->frameGeometry();
    QPoint outside;
    if (frameRect.right() + 20 < available.right()) {
        outside = QPoint(frameRect.right() + 40, frameRect.center().y());
    } else if (frameRect.bottom() + 20 < available.bottom()) {
        outside = QPoint(frameRect.center().x(), frameRect.bottom() + 40);
    } else if (frameRect.left() - 20 > available.left()) {
        outside = QPoint(frameRect.left() - 40, frameRect.center().y());
    } else if (frameRect.top() - 20 > available.top()) {
        outside = QPoint(frameRect.center().x(), frameRect.top() - 40);
    }
    QVERIFY2(!frameRect.contains(outside),
             qPrintable(QStringLiteral("outside=%1,%2 frame=%3,%4,%5,%6 available=%7,%8,%9,%10")
                            .arg(outside.x()).arg(outside.y())
                            .arg(frameRect.x()).arg(frameRect.y())
                            .arg(frameRect.width()).arg(frameRect.height())
                            .arg(available.x()).arg(available.y())
                            .arg(available.width()).arg(available.height())));

    QVERIFY(beginGripDrag(tools, outside));
    pump();
    auto* floatWindow = tools->columnFloatForTest();
    QVERIFY(floatWindow != nullptr);
    QVERIFY(floatWindow->isWindow());
    QVERIFY2(floatWindow->height() >= tools->toolsRequiredHeight(),
             "the floating toolbar keeps the full tool content height");
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(toolbox != nullptr);
    QVERIFY2(floatWindow->width() <= toolbox->contentWidthForTest() + 4,
             "the floating toolbar hugs its content width");
    auto* scroll = window->findChild<QScrollArea*>(QStringLiteral("toolsScroll"));
    QVERIFY(scroll != nullptr);
    QVERIFY2(scroll->viewport()->height() >= toolbox->minimumSizeHint().height(),
             qPrintable(QStringLiteral("viewport=%1 toolboxMin=%2 toolboxHint=%3 float=%4 required=%5")
                            .arg(scroll->viewport()->height())
                            .arg(toolbox->minimumSizeHint().height())
                            .arg(toolbox->sizeHint().height())
                            .arg(floatWindow->height())
                            .arg(tools->toolsRequiredHeight())));
    const QRect floatRect(floatWindow->mapToGlobal(QPoint(0, 0)), floatWindow->size());
    QVERIFY(!window->frameGeometry().contains(floatRect.center()));

    QVERIFY(tools->dropColumnHeaderForTest(outside));
    pump();
    QVERIFY(tools->columnFloatForTest() != nullptr);
}

void ToolbarShapeTest::bothLayoutsDockLeftAndRight()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);

    for (int columns : {1, 2}) {
        toolbox->setColumns(columns);
        tools->show();
        pump();

        QVERIFY(beginGripDrag(tools, centralEdge(window.get(), true)));
        QVERIFY(tools->dropColumnHeaderForTest(centralEdge(window.get(), true)));
        pump();
        QCOMPARE(window->sideOf(tools), PanelSide::Left);
        QCOMPARE(toolbox->columns(), columns);

        QVERIFY(beginGripDrag(tools, centralEdge(window.get(), false)));
        QVERIFY(tools->dropColumnHeaderForTest(centralEdge(window.get(), false)));
        pump();
        QCOMPARE(window->sideOf(tools), PanelSide::Right);
        QCOMPARE(toolbox->columns(), columns);
    }
}

void ToolbarShapeTest::toolbarVisibleOnStartupWhenWorkspaceIsShort()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    QVERIFY2(tools->isVisible(), "the Tools panel is visible on startup without maximizing");

    // The compact layout is taller than a short workspace. The panel must stay
    // visible and clip at the bottom, never hide or grow a scrollbar.
    toolbox->setColumns(1);
    tools->show();
    pump();
    window->resize(1100, 450);
    pump();
    QTRY_VERIFY(tools->isVisible());
    auto* scroll = window->findChild<QScrollArea*>(QStringLiteral("toolsScroll"));
    QVERIFY(scroll != nullptr);
    QCOMPARE(scroll->verticalScrollBarPolicy(), Qt::ScrollBarAlwaysOff);
    QVERIFY(scroll->isVisible());
    QVERIFY2(scroll->viewport()->height() < toolbox->minimumSizeHint().height(),
             "the short workspace clips the tool content instead of hiding it");

    window->resize(1100, 1200);
    pump();
    QVERIFY(tools->isVisible());
    QCOMPARE(toolbox->columns(), 1);
}

void ToolbarShapeTest::closeHidesAndWindowMenuRestores()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    toolbox->setColumns(2);
    tools->show();
    pump();

    QWidget* documentArea = window->findChild<QWidget*>(QStringLiteral("documentArea"));
    QVERIFY(documentArea != nullptr);
    const int documentWidth = documentArea->width();

    QToolButton* close = tools->toolsCloseButtonForTest();
    QVERIFY(close != nullptr);
    close->click();
    pump();
    QVERIFY(!tools->isVisible());
    QVERIFY2(documentArea->width() > documentWidth,
             "closing the docked toolbar gives its space back to the canvas");

    QAction* action =
        window->registry()->action(QString::fromLatin1(pictura::command_ids::WindowPanelsTools));
    QVERIFY(action != nullptr);
    action->trigger();
    pump();
    QVERIFY(tools->isVisible());
    QCOMPARE(toolbox->columns(), 2);
}

void ToolbarShapeTest::floatingModeFlipKeepsFullHeight()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 1200);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    toolbox->setColumns(1);
    tools->show();
    pump();

    const QPoint target = window->mapToGlobal(window->rect().center());
    QVERIFY(beginGripDrag(tools, target));
    pump();
    auto* floatWindow = tools->columnFloatForTest();
    QVERIFY(floatWindow != nullptr);
    QVERIFY2(floatWindow->height() >= tools->toolsRequiredHeight(),
             "the one-column float keeps the full tool content height");

    QToolButton* toggle = tools->panelColumnToggleForTest();
    QVERIFY(toggle != nullptr);
    const int slotWidth = toolbox->slotButtons().first()->width();
    toggle->click();
    pump();
    QTRY_COMPARE(toolbox->columns(), 2);
    QTRY_VERIFY(floatWindow->height() >= tools->toolsRequiredHeight());
    QTRY_VERIFY(floatWindow->width() >= 2 * slotWidth);

    toggle->click();
    pump();
    QTRY_COMPARE(toolbox->columns(), 1);
    QTRY_VERIFY(floatWindow->height() >= tools->toolsRequiredHeight());
    QTRY_VERIFY(floatWindow->width() >= slotWidth + 4);
    QTRY_VERIFY(toolbox->width() >= slotWidth);
}

void ToolbarShapeTest::floatingExpandedToCompactKeepsAllTools()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 1200);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    auto* toolbox = window->findChild<Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(tools != nullptr);
    QVERIFY(toolbox != nullptr);
    toolbox->setColumns(2);
    tools->show();
    pump();

    const QPoint target = window->mapToGlobal(window->rect().center());
    QVERIFY(beginGripDrag(tools, target));
    pump();
    auto* floatWindow = tools->columnFloatForTest();
    QVERIFY(floatWindow != nullptr);

    QToolButton* toggle = tools->panelColumnToggleForTest();
    QVERIFY(toggle != nullptr);
    toggle->click();
    pump();
    QTRY_COMPARE(toolbox->columns(), 1);
    QTRY_VERIFY(floatWindow->height() >= tools->toolsRequiredHeight());
    QWidget* lastSlot = toolbox->slotButtons().last();
    QVERIFY(lastSlot != nullptr);
    const QRect lastRect(lastSlot->mapTo(floatWindow, QPoint(0, 0)), lastSlot->size());
    QVERIFY2(floatWindow->rect().contains(lastRect),
             "the compact float must show the last tool, not clip it");
    for (const QString& name :
         {QStringLiteral("paintMaskButton"), QStringLiteral("screenModeButton")}) {
        auto* button = toolbox->findChild<QToolButton*>(name);
        QVERIFY(button != nullptr);
        const QRect rect(button->mapTo(floatWindow, QPoint(0, 0)), button->size());
        QVERIFY2(floatWindow->rect().contains(rect),
                 "the compact float must show the whole footer, not clip it");
    }
}

void ToolbarShapeTest::closeFloatingToolbarRemovesOverlay()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 1200);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    tools->show();
    pump();

    const QPoint target = window->mapToGlobal(window->rect().center());
    QVERIFY(beginGripDrag(tools, target));
    pump();
    QVERIFY(tools->columnFloatForTest() != nullptr);

    QToolButton* close = tools->toolsCloseButtonForTest();
    QVERIFY(close != nullptr);
    close->click();
    pump();
    QVERIFY(!tools->isVisible());
    QVERIFY2(tools->columnFloatForTest() == nullptr,
             "closing a floating toolbar removes its overlay");

    QAction* action =
        window->registry()->action(QString::fromLatin1(pictura::command_ids::WindowPanelsTools));
    QVERIFY(action != nullptr);
    action->trigger();
    pump();
    QVERIFY(tools->isVisible());
    auto* splitter = window->findChild<QSplitter*>(QStringLiteral("centerSplitter"));
    QVERIFY(splitter != nullptr);
    QVERIFY(splitter->indexOf(tools) >= 0);
}

void ToolbarShapeTest::floatingDragOverAppWindowStaysFloating()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->resize(1100, 1200);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    tools->show();
    pump();

    const QPoint center = window->mapToGlobal(window->rect().center());
    QVERIFY(beginGripDrag(tools, center));
    pump();
    QVERIFY(tools->columnFloatForTest() != nullptr);

    // A second drag over the app window must move the float, not dock it. The
    // outer workspace edge is the only re-dock target for a popped-out panel.
    tools->beginColumnHeaderDragForTest(center);
    tools->dragColumnHeaderToForTest(center + QPoint(30, 30));
    QVERIFY(!tools->dropIndicatorVisibleForTest());
    QVERIFY(tools->dropColumnHeaderForTest(center + QPoint(30, 30)));
    pump();
    QVERIFY2(tools->columnFloatForTest() != nullptr,
             "a floating toolbar dragged over the app window stays floating");
    auto* splitter = window->findChild<QSplitter*>(QStringLiteral("centerSplitter"));
    QVERIFY(splitter != nullptr);
    QVERIFY(splitter->indexOf(tools) < 0);
}

void ToolbarShapeTest::floatingDragBesideAppWindowStaysFloating()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QScreen* screen = QGuiApplication::primaryScreen();
    QVERIFY(screen != nullptr);
    const QRect available = screen->availableGeometry();
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    // Leave desktop room to the left of the window so a point just outside the
    // frame still lines up with the workspace's left edge band.
    window->move(available.left() + 300, available.top());
    window->resize(500, 700);
    window->show();
    pump();

    PanelColumn* tools = window->toolsColumn();
    QVERIFY(tools != nullptr);
    tools->show();
    pump();

    const QPoint center = window->mapToGlobal(window->rect().center());
    QVERIFY(beginGripDrag(tools, center));
    pump();
    QVERIFY(tools->columnFloatForTest() != nullptr);

    QWidget* central = window->centralWidget();
    QVERIFY(central != nullptr);
    const QRect centralRect(central->mapToGlobal(QPoint(0, 0)), central->size());
    const QPoint besideWindow(centralRect.left() - 10, centralRect.center().y());
    QVERIFY2(besideWindow.x() >= available.left(),
             "the test point must stay on screen to be a real desktop drag");

    tools->beginColumnHeaderDragForTest(center);
    tools->dragColumnHeaderToForTest(besideWindow);
    QVERIFY2(!tools->dropIndicatorVisibleForTest(),
             "a point on the desktop outside the workspace draws no dock mark");
    QVERIFY(tools->dropColumnHeaderForTest(besideWindow));
    pump();
    QVERIFY2(tools->columnFloatForTest() != nullptr,
             "a floating toolbar dragged beside the app window stays floating");
    auto* splitter = window->findChild<QSplitter*>(QStringLiteral("centerSplitter"));
    QVERIFY(splitter != nullptr);
    QVERIFY(splitter->indexOf(tools) < 0);

    // Inside the workspace edge band is a deliberate re-dock target.
    const QPoint edge(centralRect.left() + 2, centralRect.center().y());
    tools->beginColumnHeaderDragForTest(center);
    tools->dragColumnHeaderToForTest(edge);
    QVERIFY(window->panelColumn()->dropIndicatorVisibleForTest());
    QVERIFY(tools->dropColumnHeaderForTest(edge));
    pump();
    QVERIFY(tools->columnFloatForTest() == nullptr);
    QVERIFY(splitter->indexOf(tools) >= 0);
    QCOMPARE(window->sideOf(tools), PanelSide::Left);
}

QTEST_MAIN(ToolbarShapeTest)

#include "tst_toolbar_shape.moc"
