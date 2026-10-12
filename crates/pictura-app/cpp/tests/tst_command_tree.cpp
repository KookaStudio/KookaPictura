#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "defringe_dialog.h"
#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/panel_column_internal.h"
#include "panels/panel_group.h"
#include "panels/properties_panel.h"
#include "theme.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"
#include "preferences_dialog.h"
#include "session.h"
#include "theme.h"
#include "tool_hint_bar.h"
#include "toolbox.h"

#include "qt_test_support.h"

class CommandTreeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menus();
    void dispatch();
    void menusPanel();
    void layerMaskMenu();
    void vectorMaskMenu();
    void menuCount();
    void menubarClear();
    void panelMenusToolsIcons();
    void toolboxSlotMetrics();
    void toolboxInteractions();
    void toolboxZoomMenu();
    void widgetmenuButton();
    void panelGroupChrome();
    void selfAnchorDock();
    void layerAdjustmentMenu();
    void layerMenuGroupsAndDelete();
    void layerContentOptions();
    void layerArrangeStamp();
    void layerLockCommands();
    void shapeLayerCommands();
    void layerMattingCommands();
    void railModeResizeGrip();
    void menuMoves();
    void preferencesPages();
    void toolHintsToggle();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CommandTreeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("CommandTree"), 512, 512, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    window_->resize(1100, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void CommandTreeTest::menus()
{
    const QStringList expectedMenus = {
        QStringLiteral("File"),  QStringLiteral("Edit"),   QStringLiteral("Image"),
        QStringLiteral("Layer"), QStringLiteral("Type"),   QStringLiteral("Select"),
        QStringLiteral("Filter"), QStringLiteral("View"),  QStringLiteral("Window"),
        QStringLiteral("Help")};
    QCOMPARE(window_->topLevelMenuTitles(), expectedMenus);
}

void CommandTreeTest::dispatch()
{
    pictura::PicturaMainWindow& frame = *window_;
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(registry != nullptr);
    QVERIFY2(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectAll)),
             "dispatch select all");
    QVERIFY2(view->has_selection(), "select all selects");
    QVERIFY2(!registry->dispatch(QStringLiteral("no.such.command")), "unknown id inert");
    view->deselect();
}

void CommandTreeTest::menusPanel()
{
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);

    const QStringList common = {
        QStringLiteral("Rename"),          QStringLiteral("New Layer"),
        QStringLiteral("New Group"),       QStringLiteral("Duplicate Layer(s)"),
        QStringLiteral("Delete Layer(s)"), QStringLiteral("Group Layers"),
        QStringLiteral("Ungroup Layers"),  QStringLiteral("Move Layer Up"),
        QStringLiteral("Move Layer Down")};

    QStringList expectedPixel = common;
    expectedPixel << QStringLiteral("Add Layer Mask") << QStringLiteral("Delete Layer Mask")
                  << QStringLiteral("Enable Layer Mask") << QStringLiteral("Disable Layer Mask")
                  << QStringLiteral("Blending Options…") << QStringLiteral("Copy Layer Style")
                  << QStringLiteral("Paste Layer Style") << QStringLiteral("Clear Layer Style")
                  << QStringLiteral("Export As…") << QStringLiteral("Quick Export as PNG")
                  << QStringLiteral("Color Label");

    QStringList expectedBackground = common;
    expectedBackground << QStringLiteral("Blending Options…") << QStringLiteral("Export As…")
                       << QStringLiteral("Quick Export as PNG") << QStringLiteral("Color Label");

    QStringList expectedGroup = common;
    expectedGroup << QStringLiteral("Blending Options…") << QStringLiteral("Color Label");

    QStringList expectedAdjustment = common;
    expectedAdjustment << QStringLiteral("Edit Adjustment…") << QStringLiteral("Color Label");

    QStringList expectedType = common;
    expectedType << QStringLiteral("Blending Options…") << QStringLiteral("Rasterize Type")
                 << QStringLiteral("Color Label");

    QCOMPARE(panel->rowMenuTextsForTest(QStringLiteral("pixel")), expectedPixel);
    QCOMPARE(panel->rowMenuTextsForTest(QStringLiteral("background")), expectedBackground);
    QCOMPARE(panel->rowMenuTextsForTest(QStringLiteral("group")), expectedGroup);
    QCOMPARE(panel->rowMenuTextsForTest(QStringLiteral("adjustment")), expectedAdjustment);
    QCOMPARE(panel->rowMenuTextsForTest(QStringLiteral("type")), expectedType);

    QVERIFY2(panel->rowMenuEnabledForTest(QStringLiteral("pixel"), QStringLiteral("Add Layer Mask")),
             "wired mask row is enabled");
    QVERIFY2(!panel->rowMenuEnabledForTest(QStringLiteral("pixel"),
                                           QStringLiteral("Blending Options…")),
             "a kind-only row with no editable path is disabled");
    QVERIFY2(!panel->rowMenuToolTipForTest(QStringLiteral("pixel"),
                                           QStringLiteral("Blending Options…"))
                  .contains(QStringLiteral("not implemented yet")),
             "Blending Options is implemented (no not-implemented tooltip)");
    QVERIFY2(panel->rowMenuEnabledForTest(QStringLiteral("pixel"), QStringLiteral("New Layer")),
             "wired pixel row is enabled");

    const QStringList expectedColor = {
        QStringLiteral("None"),  QStringLiteral("Red"),  QStringLiteral("Orange"),
        QStringLiteral("Yellow"), QStringLiteral("Green"), QStringLiteral("Blue"),
        QStringLiteral("Violet"), QStringLiteral("Gray")};
    QVERIFY2(panel->colorLabelTextsForTest() == expectedColor, "color label menu");
}

// Layer > Layer Mask: the leaves are implemented, enabled with a document, and
// wired through the layer-mask bridge for the active layer.
void CommandTreeTest::layerMaskMenu()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };
    registry->refresh();
    QVERIFY(action(pictura::command_ids::LayerMaskRevealAll)->isEnabled());
    QVERIFY(!action(pictura::command_ids::LayerMaskDelete)->isEnabled());

    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMaskRevealAll)));
    QVERIFY2(view->layer_row_has_mask(0), "reveal all adds a mask");
    registry->refresh();
    QVERIFY(!action(pictura::command_ids::LayerMaskRevealAll)->isEnabled());
    QVERIFY(action(pictura::command_ids::LayerMaskDelete)->isEnabled());
    QVERIFY(action(pictura::command_ids::LayerMaskDisable)->isEnabled());

    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMaskDelete)));
    QVERIFY2(!view->layer_row_has_mask(0), "delete clears the mask");
    registry->refresh();
    QVERIFY(action(pictura::command_ids::LayerMaskRevealAll)->isEnabled());

    auto* strip = window_->findChild<QToolButton*>(QStringLiteral("layersStripMask"));
    QVERIFY(strip != nullptr && strip->isEnabled());
    strip->click();
    QVERIFY2(view->layer_row_has_mask(0), "strip button adds a mask");
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMaskDelete)));
}

// Layer > Vector Mask and Layer > Rasterize > Vector Mask: the add leaves start
// enabled, Reveal All adds one state, Delete clears, and Rasterize consumes the
// vector mask into a layer mask.
void CommandTreeTest::vectorMaskMenu()
{
    pictura::PictureView* view = window_->activeView();
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view != nullptr && registry != nullptr);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };
    registry->refresh();
    QVERIFY(action(pictura::command_ids::LayerVectorMaskRevealAll)->isEnabled());
    QVERIFY(!action(pictura::command_ids::LayerVectorMaskDelete)->isEnabled());

    const int before = view->history_count();
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerVectorMaskRevealAll)));
    QVERIFY2(pictura::vector_mask_present(*view), "reveal all adds a vector mask");
    QCOMPARE(view->history_count(), before + 1);
    registry->refresh();
    QVERIFY(!action(pictura::command_ids::LayerVectorMaskRevealAll)->isEnabled());
    QVERIFY(action(pictura::command_ids::LayerVectorMaskDelete)->isEnabled());
    QVERIFY(action(pictura::command_ids::LayerRasterizeVectorMask)->isEnabled());

    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerVectorMaskDelete)));
    QVERIFY2(!pictura::vector_mask_present(*view), "delete clears the vector mask");
    registry->refresh();
    QVERIFY(!action(pictura::command_ids::LayerRasterizeVectorMask)->isEnabled());

    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerVectorMaskRevealAll)));
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerRasterizeVectorMask)));
    QVERIFY2(!pictura::vector_mask_present(*view), "rasterize dropped the vector mask");
    QVERIFY2(view->layer_row_has_mask(0), "rasterize produced a layer mask");
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMaskDelete)));
}

void CommandTreeTest::menuCount()
{
    pictura::PanelColumn* column = window_->panelColumn();
    QVERIFY(column != nullptr);
    const QStringList expectedMenu = {
        QStringLiteral("Close"),
        QStringLiteral("Close Panel Group"),
        QStringLiteral("Minimize"),
        QStringLiteral("Collapse to Icons"),
        QStringLiteral("Auto-Collapse Iconic Panels"),
        QStringLiteral("Auto-Show Hidden Panels"),
        QStringLiteral("Interface Options\u2026"),
    };
    QCOMPARE(column->tabMenuActionsForTest(), expectedMenu);
    const bool a0 = column->autoCollapseIconicForTest();
    column->triggerTabMenuForTest(QStringLiteral("Auto-Collapse Iconic Panels"));
    const bool a1 = column->autoCollapseIconicForTest();
    column->triggerTabMenuForTest(QStringLiteral("Auto-Collapse Iconic Panels"));
    const bool a2 = column->autoCollapseIconicForTest();
    const bool b0 = column->autoShowHiddenForTest();
    column->triggerTabMenuForTest(QStringLiteral("Auto-Show Hidden Panels"));
    const bool b1 = column->autoShowHiddenForTest();
    column->triggerTabMenuForTest(QStringLiteral("Auto-Show Hidden Panels"));
    const bool b2 = column->autoShowHiddenForTest();
    QVERIFY2(a1 != a0 && a2 == a0 && b1 != b0 && b2 == b0, "tab menu checkables toggle");
    bool optionsFired = false;
    QObject::connect(column, &pictura::PanelColumn::interfaceOptionsRequested, column,
                     [&optionsFired]() { optionsFired = true; });
    column->triggerTabMenuForTest(QStringLiteral("Interface Options\u2026"));
    QVERIFY2(optionsFired, "interface options signal");
}

void CommandTreeTest::menubarClear()
{
    QMenuBar* bar = window_->menuBar();
    QVERIFY(bar != nullptr);
    QCoreApplication::processEvents();
    bool menuClear = true;
    QString menuOffender;
    const QRect barRect(bar->mapToGlobal(QPoint(0, 0)), bar->size());
    for (QWidget* child : window_->findChildren<QWidget*>()) {
        if (!child || child == bar || !child->isVisible() || bar->isAncestorOf(child)
            || child->window() != window_.get()) {
            continue;
        }
        const QRect childRect(child->mapToGlobal(QPoint(0, 0)), child->size());
        if (childRect.intersects(barRect)) {
            menuClear = false;
            if (menuOffender.isEmpty()) {
                menuOffender = child->objectName().isEmpty()
                                   ? QString::fromLatin1(child->metaObject()->className())
                                   : child->objectName();
            }
        }
    }
    QVERIFY2(menuClear, qPrintable(QStringLiteral("menu-bar overlay=") + menuOffender));
    const bool staleDiscarded = !window_->restoreStoredLayout(
        QByteArrayLiteral("stale-layout"), window_->layoutRevisionForTest() - 1);
    QVERIFY2(staleDiscarded, "stale layout discarded");
    window_->saveSession();
    QVERIFY2(pictura::loadSession().layoutRevision == window_->layoutRevisionForTest(),
             "layout revision saved");
}

void CommandTreeTest::panelMenusToolsIcons()
{
    auto* toolbox = window_->findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(toolbox != nullptr);
    toolbox->setColumns(1);
    QCoreApplication::processEvents();
    const QList<QToolButton*> slotButtons = toolbox->slotButtons();
    bool toolIcons = !slotButtons.isEmpty();
    for (QToolButton* button : slotButtons) {
        if (button->minimumWidth() < 32 || button->iconSize().width() < 22) {
            toolIcons = false;
        }
    }
    auto* screenMode = toolbox->findChild<QToolButton*>(QStringLiteral("screenModeButton"));
    if (screenMode && !screenMode->icon().isNull()
        && (screenMode->minimumWidth() < 32 || screenMode->iconSize().width() < 22)) {
        toolIcons = false;
    }
    const int min1 = toolbox->minimumWidth();
    const int content1 = toolbox->contentWidthForTest();
    toolbox->setColumns(2);
    QCoreApplication::processEvents();
    const int min2 = toolbox->minimumWidth();
    const int content2 = toolbox->contentWidthForTest();
    const bool toolTight =
        min1 > 0 && min1 == content1 && min2 == content2 && min2 > min1;
    toolbox->setColumns(1);
    QCoreApplication::processEvents();
    QVERIFY2(toolIcons, "slot/screen-mode icons are large enough");
    QVERIFY2(toolTight, "one/two-column widths equal content width");
}

void CommandTreeTest::toolboxSlotMetrics()
{
    using pictura::Toolbox;
    QCOMPARE(Toolbox::slotSizeForDpi(96.0), QSize(36, 28));
    QCOMPARE(Toolbox::iconSizeForDpi(96.0), QSize(24, 20));
    QCOMPARE(Toolbox::slotSizeForDpi(192.0), QSize(72, 56));
    QCOMPARE(Toolbox::iconSizeForDpi(192.0), QSize(48, 40));

    auto* toolbox = window_->findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(toolbox != nullptr);
    const QList<QToolButton*> slotList = toolbox->slotButtons();
    QCOMPARE(int(slotList.size()), 21);
    QVERIFY(!slotList.isEmpty());
    const qreal dpi = toolbox->screen() ? toolbox->screen()->logicalDotsPerInch() : 96.0;
    // The fixed size follows logical DPI alone; Qt multiplies by the device
    // pixel ratio at paint time, so we must not fold that ratio in here.
    const QSize expected = Toolbox::slotSizeForDpi(dpi);
    for (QToolButton* button : slotList) {
        QCOMPARE(button->size(), expected);
        QCOMPARE(button->minimumWidth(), expected.width());
        QVERIFY(button->minimumWidth() >= 32);
        QVERIFY(button->iconSize().width() >= 22);
    }
}

void CommandTreeTest::toolboxInteractions()
{
    auto* toolbox = window_->findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(toolbox != nullptr);
    toolbox->setColumns(1);
    QCoreApplication::processEvents();

    // A flyout opens beside its button, never over it, and carries the icon
    // margin styling.
    toolbox->openSlotFlyoutForTest(2);
    QCoreApplication::processEvents();
    QMenu* flyout = toolbox->slotMenuForTest(2);
    QVERIFY(flyout != nullptr);
    QVERIFY(flyout->isVisible());
    QCOMPARE(flyout->objectName(), QStringLiteral("toolFlyoutMenu"));
    QVERIFY2(pictura::Theme::styleSheet(pictura::Theme::kDefaultLevel)
                 .contains(QStringLiteral("QMenu#toolFlyoutMenu::item")),
             "the flyout's icon margin styling lives in the global stylesheet");
    QToolButton* button2 = toolbox->slotButtons().value(1);
    QVERIFY(button2 != nullptr);
    const QPoint btnTopLeft = button2->mapToGlobal(QPoint(0, 0));
    const int btnRight = btnTopLeft.x() + button2->width();
    QVERIFY2(flyout->pos().x() >= btnRight
                 || flyout->pos().x() + flyout->width() <= btnTopLeft.x(),
             "flyout does not cover the button");

    // A real click on another slot both closes the flyout and switches the
    // tool; the app-level filter forwards the press past the open popup.
    QToolButton* button1 = toolbox->slotButtons().value(0);
    QVERIFY(button1 != nullptr);
    window_->setActiveTool(pictura::ToolId::Brush);
    QCoreApplication::processEvents();
    QCOMPARE(window_->activeTool(), pictura::ToolId::Brush);
    QTest::mouseClick(button1, Qt::LeftButton);
    QCoreApplication::processEvents();
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
    QVERIFY(!toolbox->slotMenuForTest(2)->isVisible());

    // The swap hit target is the top-left corner and a click exchanges fg/bg.
    auto* fgbg = toolbox->foregroundBackgroundForTest();
    QVERIFY(fgbg != nullptr);
    const QRect swap = fgbg->swapRectForTest();
    QVERIFY(swap.right() > fgbg->width() / 2);
    QVERIFY(swap.top() < fgbg->height() / 2);
    const int swapsBefore = fgbg->swapCountForTest();
    const QColor fgBefore = fgbg->foregroundForTest();
    const QColor bgBefore = fgbg->backgroundForTest();
    QMouseEvent swapPress(QEvent::MouseButtonPress, QPointF(swap.center()),
                          QPointF(fgbg->mapToGlobal(swap.center())), Qt::LeftButton,
                          Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(fgbg, &swapPress);
    QCoreApplication::processEvents();
    QCOMPARE(fgbg->swapCountForTest(), swapsBefore + 1);
    QCOMPARE(fgbg->foregroundForTest(), bgBefore);
    QCOMPARE(fgbg->backgroundForTest(), fgBefore);

    // The Paint Mask stub is checkable with a mapped icon, left of screen mode.
    auto* paintMask = toolbox->findChild<QToolButton*>(QStringLiteral("paintMaskButton"));
    auto* screenMode = toolbox->findChild<QToolButton*>(QStringLiteral("screenModeButton"));
    QVERIFY(paintMask != nullptr);
    QVERIFY(screenMode != nullptr);
    QVERIFY(paintMask->isCheckable());
    QVERIFY(!paintMask->icon().isNull());
    // One-column mode stacks the Quick Mask / Screen Mode pair vertically and
    // sizes them like the tool slots.
    QVERIFY(paintMask->y() < screenMode->y());
    QCOMPARE(paintMask->width(), screenMode->width());
    // Both footer buttons match a tool slot in either column mode.
    const QSize slotSize = toolbox->slotButtons().first()->size();
    const QSize slotIcon = toolbox->slotButtons().first()->iconSize();
    toolbox->setColumns(2);
    QCoreApplication::processEvents();
    QCOMPARE(paintMask->size(), slotSize);
    QCOMPARE(screenMode->size(), slotSize);
    QCOMPARE(paintMask->iconSize(), slotIcon);
    QCOMPARE(screenMode->iconSize(), slotIcon);
    QVERIFY2(toolbox->contentWidthForTest() >= paintMask->width() + screenMode->width(),
             "two-column footer fits the content width");
    toolbox->setColumns(1);
    QCoreApplication::processEvents();
    QSignalSpy maskSpy(paintMask, &QToolButton::toggled);
    paintMask->click();
    QCOMPARE(maskSpy.count(), 1);
    QVERIFY(paintMask->isChecked());

    // The screen-mode button opens an InstantPopup of the three registry
    // commands, and choosing one switches the frame.
    QMenu* modeMenu = screenMode->menu();
    QVERIFY(modeMenu != nullptr);
    QCOMPARE(screenMode->popupMode(), QToolButton::InstantPopup);
    QCOMPARE(int(modeMenu->actions().size()), 3);
    for (QAction* action : modeMenu->actions()) {
        QVERIFY(action->isCheckable());
        // Display-only F accelerator column; no key binding is added here.
        QVERIFY2(action->text().endsWith(QStringLiteral("\tF")),
                 "screen-mode items carry the F hint");
    }
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QAction* full =
        registry->action(QString::fromLatin1(pictura::command_ids::ViewScreenModeFull));
    QVERIFY(full != nullptr);
    QVERIFY(modeMenu->actions().contains(full));
    window_->setScreenMode(pictura::PicturaMainWindow::ScreenMode::Standard);
    const QImage standardIcon = screenMode->icon().pixmap(16, 16).toImage();
    // The trigger would otherwise raise the full-screen advisory modal; the
    // dialog itself is covered by tst_screen_modes.
    pictura::SessionState session = pictura::loadSession();
    session.showFullScreenMessage = false;
    QVERIFY(pictura::saveSession(session));
    full->trigger();
    QCoreApplication::processEvents();
    QCOMPARE(window_->screenMode(), pictura::PicturaMainWindow::ScreenMode::Full);
    QVERIFY(full->isChecked());
    // The footer button mirrors the active mode's registry icon.
    QVERIFY2(screenMode->icon().pixmap(16, 16).toImage() != standardIcon,
             "screen-mode button icon follows the mode");
    window_->setScreenMode(pictura::PicturaMainWindow::ScreenMode::Standard);
}

// The Zoom tool's right-click menu: the CS6 zoom presets, driving the active
// canvas (issue: Zoom tool context menu).
void CommandTreeTest::toolboxZoomMenu()
{
    auto* toolbox = window_->findChild<pictura::Toolbox*>(QStringLiteral("toolsPanel"));
    QVERIFY(toolbox != nullptr);
    const int zoomGroup = pictura::toolInfo(pictura::ToolId::Zoom).group;
    QMenu* menu = toolbox->slotMenuForTest(zoomGroup);
    QVERIFY2(menu != nullptr, "the Zoom slot has a right-click menu");
    QCOMPARE(menu->objectName(), QStringLiteral("zoomToolMenu"));
    QVERIFY2(!toolbox->hasFlyoutTriangleForTest(zoomGroup), "Zoom has no member flyout");

    QStringList texts;
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator()) {
            texts << action->text();
        }
    }
    // A right-click on the Zoom slot must open the menu.
    QToolButton* zoomButton = nullptr;
    const QList<QToolButton*> zoomButtons = toolbox->slotButtons();
    const QList<int> zoomGroups = toolbox->slotGroupsForTest();
    for (int i = 0; i < zoomGroups.size() && i < zoomButtons.size(); ++i) {
        if (zoomGroups.at(i) == zoomGroup) {
            zoomButton = zoomButtons.at(i);
        }
    }
    QVERIFY(zoomButton != nullptr);
    QTest::mouseClick(zoomButton, Qt::RightButton);
    QCoreApplication::processEvents();
    QVERIFY2(menu->isVisible(), "right-click on the Zoom slot opens the menu");
    menu->close();

    QCOMPARE(texts, (QStringList{QStringLiteral("Fit on Screen"), QStringLiteral("100%"),
                                 QStringLiteral("200%"), QStringLiteral("Print Size"),
                                 QStringLiteral("Zoom In"), QStringLiteral("Zoom Out")}));

    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const QPointF centre(canvas->width() / 2.0, canvas->height() / 2.0);

    // Each entry drives the active canvas.
    canvas->setZoom(4.0, centre);
    menu->actions().at(2)->trigger(); // 200%
    QVERIFY2(qAbs(canvas->zoom() - 2.0) < 1e-6, "200% sets the zoom");
    menu->actions().at(1)->trigger(); // 100%
    QVERIFY2(qAbs(canvas->zoom() - 1.0) < 1e-6, "100% sets the zoom");
    menu->actions().at(3)->trigger(); // Print Size (72 ppi default -> 96/72)
    QVERIFY2(qAbs(canvas->zoom() - 96.0 / 72.0) < 1e-6, "Print Size follows the resolution");
    canvas->setZoom(8.0, centre);
    menu->actions().at(0)->trigger(); // Fit on Screen
    QVERIFY2(canvas->zoom() < 8.0, "Fit on Screen fits the document");

    // The Zoom In / Zoom Out entries step the zoom.
    canvas->setZoom(1.0, centre);
    menu->actions().at(5)->trigger(); // Zoom In
    QVERIFY2(canvas->zoom() > 1.0, "Zoom In magnifies");
    menu->actions().at(6)->trigger(); // Zoom Out
    QVERIFY2(qAbs(canvas->zoom() - 1.0) < 1e-6, "Zoom Out reduces back to 100%");

    // With the Zoom tool active, a canvas right-click shows the same menu; with
    // another tool active it does not.
    const QPoint local(20, 20);
    const QPoint global = canvas->mapToGlobal(local);
    window_->setActiveTool(pictura::ToolId::Zoom);
    QContextMenuEvent zoomCtx(QContextMenuEvent::Mouse, local, global);
    QApplication::sendEvent(canvas, &zoomCtx);
    QCoreApplication::processEvents();
    QVERIFY2(menu->isVisible(), "canvas right-click shows the zoom menu with Zoom active");
    menu->close();

    window_->setActiveTool(pictura::ToolId::Move);
    QContextMenuEvent moveCtx(QContextMenuEvent::Mouse, local, global);
    QApplication::sendEvent(canvas, &moveCtx);
    QCoreApplication::processEvents();
    QVERIFY2(!menu->isVisible(), "canvas right-click shows no zoom menu with another tool");
}

void CommandTreeTest::widgetmenuButton()
{
    pictura::PanelColumn* column =
        window_->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column != nullptr);
    column->setRailMode(false);
    column->ensureGroupVisibleForTest(QStringLiteral("layersPanel"));
    QCoreApplication::processEvents();
    pictura::PanelGroup* layerGroup = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(layerGroup != nullptr);
    column->showPanel(QStringLiteral("layersPanel"), true);
    QCoreApplication::processEvents();
    QToolButton* btnPtr = layerGroup->headerMenuButtonForTest();
    QToolButton* hooked = column->widgetMenuButtonForTest(layerGroup->objectName());
    QVERIFY2(btnPtr && hooked == btnPtr && btnPtr->isVisible()
                 && layerGroup->headerMenuAtRightForTest(),
             "widget menu button at header right");

    const QStringList layerTexts = layerGroup->panelMenuTextsForTest();
    column->showPanel(QStringLiteral("channelsPanel"), true);
    QCoreApplication::processEvents();
    const QStringList channelTexts = layerGroup->panelMenuTextsForTest();
    const bool follows = layerGroup->headerMenuButtonForTest()
        && layerGroup->headerMenuButtonForTest()->objectName()
               == QStringLiteral("panelWidgetMenu_channelsPanel");
    const QStringList colorTexts =
        column->widgetMenuTextsForTest(QStringLiteral("colorPanel"));
    const bool firstsDiffer = !layerTexts.isEmpty() && !channelTexts.isEmpty()
        && !colorTexts.isEmpty() && layerTexts.first() != channelTexts.first()
        && layerTexts.first() != colorTexts.first();
    column->showPanel(QStringLiteral("layersPanel"), true);
    QCoreApplication::processEvents();
    const bool options = layerGroup->panelMenuEnabledForTest(QStringLiteral("Panel Options…"));
    QVERIFY2(follows && firstsDiffer && options, "per-panel widget menu");

    const bool copyDisabled = !layerGroup->panelMenuEnabledForTest(QStringLiteral("Copy CSS"));
    const bool blendDisabled =
        !layerGroup->panelMenuEnabledForTest(QStringLiteral("Blending Options…"));
    const bool triggerBlocked = !column->triggerWidgetMenuForTest(
        QStringLiteral("layersPanel"), QStringLiteral("Copy CSS"));
    QVERIFY2(copyDisabled && blendDisabled && triggerBlocked
                 && layerGroup->panelMenuToolTipForTest(QStringLiteral("Copy CSS"))
                        == QStringLiteral("Copy CSS — not implemented yet"),
             "disabled entries");
    pictura::PanelColumn* historyColumn =
        window_->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY2(column->widgetMenuHasCloseForTest(QStringLiteral("layersPanel"))
                 && column->widgetMenuHasCloseForTest(QStringLiteral("channelsPanel"))
                 && column->widgetMenuHasCloseForTest(QStringLiteral("colorPanel"))
                 && historyColumn->widgetMenuHasCloseForTest(QStringLiteral("historyPanel")),
             "close stays off the widget menu");
}

void CommandTreeTest::panelGroupChrome()
{
    pictura::PanelColumn* column =
        window_->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column != nullptr);
    column->setRailMode(false);
    column->showPanel(QStringLiteral("layersPanel"), true);
    QCoreApplication::processEvents();
    pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group != nullptr);
    const QString title = group->titleForPanel(QStringLiteral("layersPanel"));
    QVERIFY(!title.isEmpty());

    // Normal-mode tabs are iconless; the icon is retained for the iconic strip.
    QVERIFY2(!group->tabHasIconForTest(title), "normal tab carries no icon");
    QVERIFY2(!group->titleIconForTest(title).isNull(), "panel icon retained for iconic mode");
    group->setCollapsedToIconsForTest(true);
    QCoreApplication::processEvents();
    QVERIFY2(!group->iconicIconForTest(QStringLiteral("layersPanel")).isNull(),
             "iconic strip shows the panel icon");
    group->setCollapsedToIconsForTest(false);
    QCoreApplication::processEvents();

    // The corner action button draws the bundled `panel.menu` glyph, not text,
    // and keeps a right margin.
    QToolButton* corner = group->headerMenuButtonForTest();
    QVERIFY(corner != nullptr);
    QVERIFY2(!corner->icon().isNull(), "corner glyph resolves");
    QVERIFY2(corner->text().isEmpty(), "corner shows the glyph, not a character");
    QVERIFY2(corner->icon().pixmap(16, 16).toImage()
                 == pictura::icon(QStringLiteral("panel.menu")).pixmap(16, 16).toImage(),
             "corner uses panel.menu");
    QVERIFY2(group->headerCornerRightMarginForTest() > 0, "corner keeps a right margin");

    // The header band paints the inactive-tab surface behind the corner.
    QWidget* band = group->headerBandForTest();
    QWidget* cornerWidget = group->headerCornerForTest();
    QVERIFY(band != nullptr && cornerWidget != nullptr);
    QVERIFY(band->height() > 0);
    const QImage image = group->grab().toImage();
    const qreal dpr = image.devicePixelRatio();
    const QPoint gap = band->mapTo(group, QPoint(cornerWidget->x() + 2, 2));
    const QColor gapColour =
        image.pixelColor(int(gap.x() * dpr + 0.5), int(gap.y() * dpr + 0.5));
    QCOMPARE(gapColour.rgba(), QColor(QStringLiteral("#363636")).rgba());

    // The column header sits on the group-header shade with a one-step-darker
    // bottom rule; the container keeps side rules only.
    const QString sheet = qApp->styleSheet();
    const QString baseHex = QStringLiteral("#363636");
    const QString headerRuleHex =
        pictura::Theme::shade(QColor(baseHex), -1).name(QColor::HexRgb);
    QVERIFY2(sheet.contains(QStringLiteral("QWidget#panelColumnHeader { background: ")
                            + baseHex + QStringLiteral("; border: 0; border-bottom: 1px solid ")
                            + headerRuleHex + QStringLiteral("; }")),
             "column header uses the group-header shade and a darker bottom rule");
}

void CommandTreeTest::selfAnchorDock()
{
    // The pre-Essentials single column: its only right-hand column is
    // document-facing on the left, which this self-anchor drop check exercises.
    pictura::PanelColumn* primary = window_->restoreLegacyDefaultForTest();
    QVERIFY(primary != nullptr);
    primary->setRailMode(false);
    primary->showPanel(QStringLiteral("layersPanel"), true);
    QCoreApplication::processEvents();

    // A panel in the right-hand column docks to the document-facing (left) side
    // of its own column; the source keeps its other groups.
    QVERIFY2(primary->selfAnchorDockForTest(QStringLiteral("layersPanel"), false),
             "panel docks to the left of its own column");
    // And another panel docks to the opposite (right) side of its own column.
    QVERIFY2(primary->selfAnchorDockForTest(QStringLiteral("channelsPanel"), true),
             "panel docks to the right of its own column");
}

// Layer > New Adjustment Layer: every CS6 kind is implemented, enabled with a
// document open, and creates an adjustment layer named for its menu entry;
// Shadows/Highlights and HDR Toning are destructive Image commands, not kinds.
void CommandTreeTest::layerAdjustmentMenu()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);

    const QStringList leaves = {
        QStringLiteral("Brightness/Contrast"), QStringLiteral("Levels"),
        QStringLiteral("Curves"),              QStringLiteral("Exposure"),
        QStringLiteral("Vibrance"),            QStringLiteral("Hue/Saturation"),
        QStringLiteral("Color Balance"),       QStringLiteral("Black & White"),
        QStringLiteral("Photo Filter"),        QStringLiteral("Channel Mixer"),
        QStringLiteral("Color Lookup"),        QStringLiteral("Invert"),
        QStringLiteral("Posterize"),           QStringLiteral("Threshold"),
        QStringLiteral("Gradient Map"),        QStringLiteral("Selective Color")};
    for (const QString& name : leaves) {
        QAction* action = registry->action(pictura::commandIdForPath(
            {QStringLiteral("Layer"), QStringLiteral("New Adjustment Layer"), name}));
        QVERIFY2(action, qPrintable(name));
        registry->refresh();
        QVERIFY2(action->isEnabled(), qPrintable(name));
        const QString expected =
            name == QStringLiteral("Invert") ? name : name + QStringLiteral("\u2026");
        QCOMPARE(action->text(), expected);

        const int before = view->layer_row_count();
        action->trigger();
        QCOMPARE(view->layer_row_count(), before + 1);
        bool named = false;
        for (int i = 0; i < view->layer_row_count(); ++i) {
            if (view->layer_row_name(i) == name) {
                named = true;
            }
        }
        QVERIFY2(named, qPrintable(name));
    }

    for (const QString& gone :
         {QStringLiteral("Shadows/Highlights"), QStringLiteral("HDR Toning")}) {
        QVERIFY2(!registry->action(pictura::commandIdForPath(
                     {QStringLiteral("Layer"), QStringLiteral("New Adjustment Layer"), gone})),
                 qPrintable(gone));
    }

    // The Adjustments panel lists the same sixteen kinds in CS6 panel order,
    // then the two trailing toggles.
    pictura::PanelColumn* column = window_->panelColumn();
    QVERIFY(column != nullptr);
    const QStringList expectedPanel = {
        QStringLiteral("Brightness-Contrast"), QStringLiteral("Levels"),
        QStringLiteral("Curves"),              QStringLiteral("Exposure"),
        QStringLiteral("Vibrance"),            QStringLiteral("Hue-Saturation"),
        QStringLiteral("Color Balance"),       QStringLiteral("Black & White"),
        QStringLiteral("Photo Filter"),        QStringLiteral("Channel Mixer"),
        QStringLiteral("Color Lookup"),        QStringLiteral("Invert"),
        QStringLiteral("Posterize"),           QStringLiteral("Threshold"),
        QStringLiteral("Gradient Map"),        QStringLiteral("Selective Color"),
        QStringLiteral("Add Mask by Default"), QStringLiteral("Clip to Layer")};
    QCOMPARE(column->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel")), expectedPanel);
}

// Layer: separators split the top level into CS6's groups (#246), and Delete
// Layer is live, removing the Layers-panel selection.
void CommandTreeTest::layerMenuGroupsAndDelete()
{
    QMenu* layer = nullptr;
    for (QAction* top : window_->menuBar()->actions()) {
        if (top->text().remove(QLatin1Char('&')) == QStringLiteral("Layer")) {
            layer = top->menu();
        }
    }
    QVERIFY(layer != nullptr);
    QStringList groupEnds;
    const QList<QAction*> actions = layer->actions();
    for (int i = 1; i < actions.size(); ++i) {
        if (actions.at(i)->isSeparator()) {
            groupEnds.append(actions.at(i - 1)->text());
        }
    }
    const QStringList expectedEnds = {
        QStringLiteral("New"),                       QStringLiteral("Delete Hidden Layers"),
        QStringLiteral("Smart Filter"),              QStringLiteral("Layer Content Options…"),
        QStringLiteral("Release Clipping Mask"),     QStringLiteral("Rasterize"),
        QStringLiteral("New Layer-based Slice"),     QStringLiteral("Lock All Layers In Group…"),
        QStringLiteral("Merge Clipping Mask")};
    QCOMPARE(groupEnds, expectedEnds);

    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    pictura::CommandRegistry* registry = window_->registry();
    QAction* del = registry->action(QString::fromLatin1(pictura::command_ids::LayerDeleteLayer));
    QVERIFY(del != nullptr);
    QCOMPARE(del->text(), QStringLiteral("Delete Layer"));
    const QString doomed = view->add_layer_in(QString());
    panel->refresh();
    QVERIFY(panel->selectRowForTest(doomed));
    registry->refresh();
    QVERIFY(del->isEnabled());
    const int before = view->layer_row_count();
    del->trigger();
    QCOMPARE(view->layer_row_count(), before - 1);
    for (int i = 0; i < view->layer_row_count(); ++i) {
        QVERIFY(view->layer_row_path(i) != doomed);
    }
}

// Layer > Layer Content Options is enabled only for a fill/adjustment layer and
// opens that layer's Properties page.
void CommandTreeTest::layerContentOptions()
{
    QVERIFY(window_->newDocument(QStringLiteral("ContentOptions"), 32, 32,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = window_->activeDocumentIndex();
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* props =
        window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    pictura::PanelColumn* column = window_->panelColumn();
    QVERIFY(layers && props && column);
    QVERIFY(layers->selectRowForTest(view->layer_row_path(0)));

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerContentOptions);
    QAction* action = registry->action(id);
    QVERIFY(action);

    // A pixel current layer disables the command and invoking it is a no-op.
    column->showPanel(QStringLiteral("propertiesPanel"), false);
    registry->refresh();
    QVERIFY2(!action->isEnabled(), "disabled for a pixel layer");
    action->trigger();
    QVERIFY2(!column->isPanelVisible(QStringLiteral("propertiesPanel")),
             "no-op for a pixel layer");

    // An adjustment current layer enables the command and opens Properties.
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    QString path;
    for (int i = 0; i < view->layer_row_count(); ++i) {
        if (view->layer_row_has_adjustment(i)) {
            path = view->layer_row_path(i);
        }
    }
    QVERIFY2(!path.isEmpty(), "adjustment row");
    layers->refresh();
    QVERIFY(layers->selectRowForTest(path));
    registry->refresh();
    QVERIFY2(action->isEnabled(), "enabled for an adjustment layer");
    QVERIFY2(registry->dispatch(id), "dispatch Layer Content Options");
    QVERIFY2(column->isPanelVisible(QStringLiteral("propertiesPanel")), "Properties shown");
    QCOMPARE(props->pathForTest(), path);

    window_->closeDocument(doc, false);
}

// An iconic rail column is resizable from its workspace-facing edge whether it
// is docked left or right; the grip drives the frame's widget-column resize.
void CommandTreeTest::railModeResizeGrip(){
    pictura::PicturaMainWindow& frame = *window_;
    pictura::PanelColumn* right = frame.panelColumn();
    QVERIFY(right != nullptr);
    pictura::PanelColumn* left = frame.createPanelColumn(pictura::PanelSide::Left, nullptr);
    QVERIFY(left != nullptr);
    QCoreApplication::processEvents();

    struct Case {
        pictura::PanelColumn* column;
        bool leftDock;
    };
    const Case cases[] = {{right, false}, {left, true}};
    for (const Case& c : cases) {
        c.column->setRailMode(true);
        QCoreApplication::processEvents();
        QVERIFY2(c.column->railModeForTest(), "rail mode on");

        QWidget* grip = c.column->findChild<QWidget*>(QStringLiteral("panelResizeGrip"));
        QVERIFY2(grip != nullptr, "workspace-edge grip exists");
        QVERIFY2(grip->isVisible(), "grip stays visible in rail mode");
        // A right-docked column's workspace edge is its left side; a left-docked
        // column's is its right side.
        QCOMPARE(grip->x(), c.leftDock ? c.column->width() - grip->width() : 0);
        QVERIFY2(c.column->maximumWidth() > c.column->minimumWidth(),
                 "rail column is width-resizable");
        QCOMPARE(c.column->minimumWidth(), pictura::kIconStripMinWidth);

        const int before = c.column->width();
        const int globalX = c.column->mapToGlobal(QPoint(0, 0)).x();
        frame.beginWidgetColumnResize(c.column, globalX);
        // Grow by 30 px: drag a right-docked column's left edge left, a
        // left-docked column's right edge right.
        frame.updateWidgetColumnResize(globalX + (c.leftDock ? 30 : -30));
        frame.endWidgetColumnResize();
        QCoreApplication::processEvents();
        QCOMPARE(c.column->width(), before + 30);
    }
}

// The Window menu carries Options; the old View > Options and View > Use GPU
// Compute entries and the Window > 3D stub are gone, while the other 3D
// entries (Workspace/Panels/Edit ▸ Preferences) stay.
void CommandTreeTest::menuMoves()
{
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    bool windowOptions = false;
    bool viewOptions = false;
    bool viewGpu = false;
    bool window3d = false;
    for (const pictura::CommandInfo& info : registry->describe()) {
        if (info.id == QString::fromLatin1(pictura::command_ids::ViewOptions)) {
            windowOptions = info.path
                == QStringList({QStringLiteral("Window"), QStringLiteral("Options")});
            viewOptions = info.path
                == QStringList({QStringLiteral("View"), QStringLiteral("Options")});
        }
        if (info.id == QStringLiteral("view.gpuCompute")) {
            viewGpu = true;
        }
        if (info.path == QStringList({QStringLiteral("Window"), QStringLiteral("3D")})) {
            window3d = true;
        }
    }
    QVERIFY2(windowOptions, "Window > Options exists");
    QVERIFY2(!viewOptions, "View > Options is gone");
    QVERIFY2(!viewGpu, "View > Use GPU Compute is gone");
    QVERIFY2(!window3d, "Window > 3D is gone");
    // The unimplemented workspace placeholders are removed, not disabled.
    for (const QString& gone : {QStringLiteral("3D"), QStringLiteral("Advanced 3D"),
                                QStringLiteral("Motion"), QStringLiteral("New Features")}) {
        QVERIFY2(!registry->action(pictura::commandIdForPath(
                     {QStringLiteral("Window"), QStringLiteral("Workspace"), gone})),
                 qPrintable(QStringLiteral("Workspace > ") + gone + QStringLiteral(" is gone")));
    }
    QVERIFY2(registry->action(pictura::commandIdForPath(
                 {QStringLiteral("Window"), QStringLiteral("Panels"), QStringLiteral("3D")})),
             "Window > Panels > 3D stays");
    QVERIFY2(registry->action(pictura::commandIdForPath(
                 {QStringLiteral("Edit"), QStringLiteral("Preferences"), QStringLiteral("3D")})),
             "Edit > Preferences > 3D stays");
}

// The Preferences Performance page is real: it exposes the GPU-compute
// checkbox and the renamed rows still land on their own stack page.
void CommandTreeTest::preferencesPages()
{
    pictura::PreferencesDialog* prefs = window_->preferencesDialog();
    if (!prefs) {
        window_->registry()->dispatch(
            QString::fromLatin1(pictura::command_ids::EditPreferencesGeneral));
        QCoreApplication::processEvents();
        prefs = window_->preferencesDialog();
    }
    QVERIFY(prefs != nullptr);
    QVERIFY2(prefs->pagesForTest().contains(QStringLiteral("Performance")),
             "Performance is a real page");

    prefs->openOn(QStringLiteral("Performance"));
    QCOMPARE(prefs->currentPageForTest(), QStringLiteral("Performance"));
    prefs->openOn(QStringLiteral("General"));
    QCOMPARE(prefs->currentPageForTest(), QStringLiteral("General"));
    prefs->openOn(QStringLiteral("Interface"));
    QCOMPARE(prefs->currentPageForTest(), QStringLiteral("Interface"));
    // A disabled pane never becomes the shown page.
    prefs->openOn(QStringLiteral("Cursors"));
    QCOMPARE(prefs->currentPageForTest(), QStringLiteral("Interface"));

    QSignalSpy gpuSpy(prefs, &pictura::PreferencesDialog::gpuComputeChanged);
    prefs->setGpuCompute(true);
    QVERIFY(prefs->checkboxForTest(QStringLiteral("useGpuCompute")));
    prefs->setCheckboxForTest(QStringLiteral("useGpuCompute"), false);
    QCOMPARE(gpuSpy.count(), 1);
    QCOMPARE(gpuSpy.at(0).at(0).toBool(), false);
    prefs->setGpuComputeEnabled(false);
    prefs->close();
}

// View > Tool Hints is a checkable toggle, on by default; it hides the footer
// hint strip (once a document owns the footer) and restores it.
void CommandTreeTest::toolHintsToggle()
{
    QAction* action = window_->registry()->action(
        QString::fromLatin1(pictura::command_ids::ViewToolHints));
    QVERIFY2(action != nullptr, "Tool Hints command exists");
    QVERIFY2(action->isCheckable(), "Tool Hints is checkable");
    QVERIFY2(action->isChecked(), "Tool Hints defaults on");

    auto* hintBar = window_->findChild<pictura::ToolHintBar*>(QStringLiteral("toolHintBar"));
    QVERIFY2(hintBar != nullptr, "hint bar exists");

    QVERIFY(window_->newDocument(QStringLiteral("Hints"), 4, 3, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCoreApplication::processEvents();
    QVERIFY2(hintBar->isVisible(), "hints shown with a document");

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY2(!action->isChecked(), "toggled off");
    QVERIFY2(!hintBar->isVisible(), "hints hidden");

    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY2(action->isChecked() && hintBar->isVisible(), "hints restored");

    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

// Layer > Arrange / Reverse / Merge Down / Delete Layer / Stamp: the new
// leaves exist, enable where applicable, and stamp adds one layer while
// preserving the source layers.
void CommandTreeTest::layerArrangeStamp()
{
    QVERIFY(window_->newDocument(QStringLiteral("ArrangeStamp"), 32, 32,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = window_->activeDocumentIndex();
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view != nullptr);
    QVERIFY(layers != nullptr);
    QVERIFY(registry != nullptr);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };

    // Two ordinary layers above whatever the fresh document created.
    view->add_layer(-1);
    view->add_layer(-1);
    layers->refresh();
    QVERIFY(view->layer_row_count() >= 3);
    const QString topName = view->layer_row_name(0);
    const QString middleName = view->layer_row_name(1);

    for (const char* id : {pictura::command_ids::LayerArrangeFront,
                           pictura::command_ids::LayerArrangeForward,
                           pictura::command_ids::LayerArrangeBackward,
                           pictura::command_ids::LayerArrangeBack,
                           pictura::command_ids::LayerArrangeReverse,
                           pictura::command_ids::LayerMergeDown,
                           pictura::command_ids::LayerDeleteLayer,
                           pictura::command_ids::LayerStampVisible,
                           pictura::command_ids::LayerStampSelected}) {
        QVERIFY2(action(id) != nullptr, id);
    }

    // Arrange Forward swaps the middle layer with the one above it.
    layers->selectPaths({view->layer_row_path(1)}, view->layer_row_path(1));
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerArrangeForward)->isEnabled(), "forward enabled");
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerArrangeForward)));
    layers->refresh();
    QCOMPARE(view->layer_row_name(0), middleName);
    QCOMPARE(view->layer_row_name(1), topName);

    // Reverse flips the contiguous top-two run back.
    const QString run0 = view->layer_row_path(0);
    const QString run1 = view->layer_row_path(1);
    layers->selectPaths({run0, run1}, run0);
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerArrangeReverse)->isEnabled(), "reverse enabled");
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerArrangeReverse)));
    layers->refresh();
    QCOMPARE(view->layer_row_name(0), topName);
    QCOMPARE(view->layer_row_name(1), middleName);

    // Merge Down folds the active layer into the layer below it.
    layers->selectPaths({view->layer_row_path(0)}, view->layer_row_path(0));
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerMergeDown)->isEnabled(), "merge down enabled");
    const int beforeMerge = view->layer_row_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMergeDown)));
    QCOMPARE(view->layer_row_count(), beforeMerge - 1);

    // Stamp Visible adds exactly one layer and keeps every source.
    layers->refresh();
    layers->selectPaths({view->layer_row_path(0)}, view->layer_row_path(0));
    registry->refresh();
    const int beforeStamp = view->layer_row_count();
    QVERIFY2(action(pictura::command_ids::LayerStampVisible)->isEnabled(), "stamp visible enabled");
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerStampVisible)));
    QCOMPARE(view->layer_row_count(), beforeStamp + 1);

    // Delete Layer removes the selected (stamp) layer.
    layers->refresh();
    layers->selectPaths({view->layer_row_path(0)}, view->layer_row_path(0));
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerDeleteLayer)->isEnabled(), "delete enabled");
    const int beforeDelete = view->layer_row_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerDeleteLayer)));
    QCOMPARE(view->layer_row_count(), beforeDelete - 1);

    window_->closeDocument(doc, false);
}

// Layer > Lock Layers (All / Transparency / Image / Position) over the panel
// selection, and Lock All Layers In Group… over the current group.
void CommandTreeTest::layerLockCommands()
{
    QVERIFY(window_->newDocument(QStringLiteral("LockCmds"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const int doc = window_->activeDocumentIndex();
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view != nullptr);
    QVERIFY(layers != nullptr);
    QVERIFY(registry != nullptr);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };

    for (const char* id : {pictura::command_ids::LayerLockLayersAll,
                           pictura::command_ids::LayerLockLayersTransparency,
                           pictura::command_ids::LayerLockLayersImage,
                           pictura::command_ids::LayerLockLayersPosition,
                           pictura::command_ids::LayerLockAllInGroup}) {
        QVERIFY2(action(id) != nullptr, id);
    }

    const auto rowOf = [view](const QString& path) {
        for (int i = 0; i < view->layer_row_count(); ++i) {
            if (view->layer_row_path(i) == path) {
                return i;
            }
        }
        return -1;
    };

    // No selection disables the lock commands.
    layers->selectPaths({}, QString());
    registry->refresh();
    QVERIFY2(!action(pictura::command_ids::LayerLockLayersAll)->isEnabled(), "disabled empty");

    // Lock Transparency over the selection sets bit 0x01 in one state.
    view->add_layer(-1);
    layers->refresh();
    const QString top = view->layer_row_path(0);
    layers->selectPaths({top}, top);
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerLockLayersTransparency)->isEnabled(),
             "transparency enabled");
    const int before = view->history_count();
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerLockLayersTransparency)));
    QCOMPARE(view->history_count(), before + 1);
    QVERIFY((view->layer_row_lock(rowOf(top)) & 0x01) != 0);

    // Lock All sets the full set.
    layers->selectPaths({top}, top);
    registry->refresh();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerLockLayersAll)));
    QCOMPARE(view->layer_row_lock(rowOf(top)) & 0x0F, 0x0F);

    // Lock All Layers In Group… locks the group's descendants, not the group.
    const QString group = view->add_group_in(QString());
    const QString child = view->add_layer_in(group);
    layers->refresh();
    layers->selectPaths({child}, child);
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerLockAllInGroup)->isEnabled(), "group lock enabled");
    const int groupBase = view->history_count();
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerLockAllInGroup)));
    QCOMPARE(view->history_count(), groupBase + 1);
    QCOMPARE(view->layer_row_lock(rowOf(child)) & 0x0F, 0x0F);
    QCOMPARE(view->layer_row_lock(rowOf(group)) & 0x0F, 0x00);

    window_->closeDocument(doc, false);
}

// Layer > Rasterize Shape and Copy/Paste Shape Attributes: enabled over a
// current shape layer, paste only after a copy, and shape rasterize drops the
// shape.
void CommandTreeTest::shapeLayerCommands()
{
    QVERIFY(window_->newDocument(QStringLiteral("ShapeCmds"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const int doc = window_->activeDocumentIndex();
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && layers && registry);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };

    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 4;
    square.y0 = 4;
    square.x1 = 20;
    square.y1 = 20;
    const QString shape = pictura::shape_add_layer(*view, square, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    layers->refresh();
    QCoreApplication::processEvents();
    layers->selectPaths({shape}, shape);
    registry->refresh();

    QVERIFY2(action(pictura::command_ids::LayerRasterizeShape)->isEnabled(), "shape rasterize");
    QVERIFY2(action(pictura::command_ids::LayerCopyShapeAttributes)->isEnabled(), "copy attrs");
    QVERIFY2(!action(pictura::command_ids::LayerPasteShapeAttributes)->isEnabled(),
             "paste waits for a copy");

    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerCopyShapeAttributes)));
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerPasteShapeAttributes)->isEnabled(), "paste enabled");

    const int before = view->history_count();
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerRasterizeShape)));
    QCOMPARE(view->history_count(), before + 1);

    window_->closeDocument(doc, false);
}

// Layer > Matting: the three leaves exist, enable over an editable pixel layer,
// each is one undo state, and the Defringe dialog defaults to a 1-pixel width.
void CommandTreeTest::layerMattingCommands()
{
    QVERIFY(window_->newDocument(QStringLiteral("Matting"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const int doc = window_->activeDocumentIndex();
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && layers && registry);
    const auto action = [registry](const char* id) {
        return registry->action(QString::fromLatin1(id));
    };

    for (const char* id : {pictura::command_ids::LayerMattingDefringe,
                           pictura::command_ids::LayerMattingRemoveBlack,
                           pictura::command_ids::LayerMattingRemoveWhite}) {
        QVERIFY2(action(id) != nullptr, id);
    }

    view->add_layer(-1);
    layers->refresh();
    const QString top = view->layer_row_path(0);
    layers->selectPaths({top}, top);
    registry->refresh();
    QVERIFY2(action(pictura::command_ids::LayerMattingRemoveBlack)->isEnabled(),
             "remove black enabled");
    QVERIFY2(action(pictura::command_ids::LayerMattingRemoveWhite)->isEnabled(),
             "remove white enabled");
    QVERIFY2(action(pictura::command_ids::LayerMattingDefringe)->isEnabled(), "defringe enabled");

    const int before = view->history_count();
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerMattingRemoveBlack)));
    QCOMPARE(view->history_count(), before + 1);

    pictura::DefringeDialog dialog;
    auto* width = dialog.findChild<QSpinBox*>(QStringLiteral("defringeWidth"));
    QVERIFY(width != nullptr);
    QCOMPARE(width->value(), 1);

    // A zero-selection panel disables every Matting command.
    layers->selectPaths({}, QString());
    registry->refresh();
    QVERIFY2(!action(pictura::command_ids::LayerMattingRemoveBlack)->isEnabled(),
             "disabled without an active layer");

    window_->closeDocument(doc, false);
}

QTEST_MAIN(CommandTreeTest)
#include "tst_command_tree.moc"