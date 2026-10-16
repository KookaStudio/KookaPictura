// The canvas right-click context menus: routing to the active tool, the
// pasteboard fallback, and each tool's menu contents.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"
#include "qt_test_support.h"

#include "panels/brush_preset_picker.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMenu>

namespace {

using paint_fixture::Fixture;

QAction* actionNamed(QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator() && action->text() == text) {
            return action;
        }
    }
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator() && action->text().contains(text)) {
            return action;
        }
    }
    return nullptr;
}

int nonSeparatorActions(QMenu* menu)
{
    int count = 0;
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator()) {
            ++count;
        }
    }
    return count;
}

QImage seedImage()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::green);
    return seed;
}

} // namespace

class CanvasContextMenusTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void handOpensNothing();
    void moveListsLayers();
    void moveChooseSelects();
    void multiDocumentTargetsClicked();
    void moveIsolation();
    void eyeDropperMenuOpens();
    void eyeDropperSize();
    void eyeDropperScope();
    void cropToggleDelete();
    void cropReset();
    void pasteboardBackgroundMenu();
    void backgroundColour();
    void brushPickerOpens();
    void brushPresetApplies();
    void brushPickerIsNotACapturingPopup();
    void brushPickerKeepsBrushHover();
    void brushPickerDismissesOnOutsidePress();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CanvasContextMenusTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CanvasContextMenusTest::handOpensNothing()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_hand"));
    QVERIFY2(f.ok(), "hand fixture");
    f.frame.setActiveTool(pictura::ToolId::Hand);
    const QPointF over = f.canvas->widgetToImage(
        QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    QVERIFY(f.tools->contextMenuForTest(f.canvas, over) == nullptr);
}

void CanvasContextMenusTest::moveListsLayers()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_move_list"));
    QVERIFY2(f.ok(), "move fixture");
    f.frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.view->duplicate_layer(0) >= 0);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    QVERIFY(nonSeparatorActions(menu) >= 3); // two layers + Isolation Mode
    QVERIFY(actionNamed(menu, QStringLiteral("Isolation Mode")) != nullptr);
    delete menu;
}

void CanvasContextMenusTest::moveChooseSelects()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_move_choose"));
    QVERIFY2(f.ok(), "move fixture");
    f.frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.view->duplicate_layer(0) >= 0);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    // The copy (layer 1) is topmost; its row is listed first.
    const QString topName = f.view->layer_name(1);
    QAction* top = actionNamed(menu, topName);
    QVERIFY(top != nullptr);
    top->trigger();
    QCOMPARE(f.view->active_layer_path(), QStringLiteral("1"));
    delete menu;
}

void CanvasContextMenusTest::multiDocumentTargetsClicked()
{
    Fixture a(*window_, seedImage(), QStringLiteral("ccm_multi_a"));
    QVERIFY2(a.ok(), "first fixture");
    Fixture b(*window_, seedImage(), QStringLiteral("ccm_multi_b"));
    QVERIFY2(b.ok(), "second fixture");
    QVERIFY(a.doc != b.doc);
    QCOMPARE(b.frame.activeDocumentIndex(), b.doc);
    b.frame.setActiveTool(pictura::ToolId::Move);
    // Right-click the first document's canvas: it becomes the active document.
    b.tools->routeCanvasContextMenu(a.canvas, QPointF(2, 2), QPoint(0, 0));
    QCOMPARE(b.frame.activeDocumentIndex(), a.doc);
    QCOMPARE(b.frame.activeView(), a.view);
}

void CanvasContextMenusTest::moveIsolation()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_move_iso"));
    QVERIFY2(f.ok(), "move fixture");
    f.frame.setActiveTool(pictura::ToolId::Move);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    QAction* isolate = actionNamed(menu, QStringLiteral("Isolation Mode"));
    QVERIFY(isolate != nullptr);
    QSignalSpy spy(f.tools, &pictura::ToolController::isolationModeRequested);
    isolate->trigger();
    QCOMPARE(spy.count(), 1);
    delete menu;
}

void CanvasContextMenusTest::eyeDropperMenuOpens()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_eyedropper"));
    QVERIFY2(f.ok(), "eyedropper fixture");
    f.frame.setActiveTool(pictura::ToolId::Eyedropper);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    QCOMPARE(nonSeparatorActions(menu), 2);
    QVERIFY(actionNamed(menu, QStringLiteral("Sample Size")) != nullptr);
    QVERIFY(actionNamed(menu, QStringLiteral("Sample")) != nullptr);
    delete menu;
}

void CanvasContextMenusTest::eyeDropperSize()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_eyedropper_size"));
    QVERIFY2(f.ok(), "eyedropper fixture");
    f.frame.setActiveTool(pictura::ToolId::Eyedropper);
    auto* options = f.tools->optionsFor<pictura::EyedropperGroup>(pictura::ToolId::Eyedropper);
    QVERIFY(options != nullptr);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    QMenu* sizeMenu = actionNamed(menu, QStringLiteral("Sample Size"))->menu();
    QVERIFY(sizeMenu != nullptr);
    QAction* five = actionNamed(sizeMenu, QStringLiteral("5 by 5"));
    QVERIFY(five != nullptr);
    five->trigger();
    QCOMPARE(options->get().sampleSize, 5);
    auto* combo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->currentData().toInt(), 5);
    delete menu;
}

void CanvasContextMenusTest::eyeDropperScope()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_eyedropper_scope"));
    QVERIFY2(f.ok(), "eyedropper fixture");
    f.frame.setActiveTool(pictura::ToolId::Eyedropper);
    auto* options = f.tools->optionsFor<pictura::EyedropperGroup>(pictura::ToolId::Eyedropper);
    QVERIFY(options != nullptr);
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(2, 2));
    QVERIFY(menu != nullptr);
    QMenu* sampleMenu = actionNamed(menu, QStringLiteral("Sample"))->menu();
    QVERIFY(sampleMenu != nullptr);
    QAction* current = actionNamed(sampleMenu, QStringLiteral("Current Layer"));
    QVERIFY(current != nullptr);
    current->trigger();
    QCOMPARE(options->get().scope, 0);
    auto* combo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSample"));
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->currentIndex(), 0);
    delete menu;
}

void CanvasContextMenusTest::cropToggleDelete()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_crop_delete"));
    QVERIFY2(f.ok(), "crop fixture");
    f.frame.setActiveTool(pictura::ToolId::Crop);
    // Draw an active box (the tool starts with only a preview box).
    f.drag({QPointF(40, 40), QPointF(20, 20)});
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(30, 30));
    QVERIFY(menu != nullptr);
    QAction* del = actionNamed(menu, QStringLiteral("Delete Cropped Pixels"));
    QVERIFY(del != nullptr);
    QVERIFY(del->isCheckable());
    const bool before = del->isChecked();
    del->trigger();
    QCOMPARE(del->isChecked(), !before);
    auto* checkbox = window_->findChild<QCheckBox*>(QStringLiteral("optionsCropDelete"));
    QVERIFY(checkbox != nullptr);
    QCOMPARE(checkbox->isChecked(), !before);
    delete menu;
}

void CanvasContextMenusTest::cropReset()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_crop_reset"));
    QVERIFY2(f.ok(), "crop fixture");
    f.frame.setActiveTool(pictura::ToolId::Crop);
    f.drag({QPointF(40, 40), QPointF(20, 20)});
    QMenu* menu = f.tools->contextMenuForTest(f.canvas, QPointF(30, 30));
    QVERIFY(menu != nullptr);
    QAction* reset = actionNamed(menu, QStringLiteral("Reset Crop"));
    QVERIFY(reset != nullptr);
    reset->trigger();
    // Reset returns the box to the whole canvas.
    auto* session = f.tools->cropBoxSession();
    QVERIFY(session != nullptr);
    QCOMPARE(session->pendingCropRect(), QRect(0, 0, 64, 64));
    delete menu;
}

void CanvasContextMenusTest::pasteboardBackgroundMenu()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_background"));
    QVERIFY2(f.ok(), "background fixture");
    QMenu* menu = window_->canvasBackgroundMenuForTest();
    QVERIFY(menu != nullptr);
    QCOMPARE(nonSeparatorActions(menu), 4);
    delete menu;
}

void CanvasContextMenusTest::backgroundColour()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_background_pick"));
    QVERIFY2(f.ok(), "background fixture");
    QMenu* menu = window_->canvasBackgroundMenuForTest();
    QVERIFY(menu != nullptr);
    QAction* second = menu->actions().at(1);
    second->trigger();
    QCOMPARE(window_->canvasColorIndex(), 1);
    delete menu;
}

void CanvasContextMenusTest::brushPickerOpens()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_brush"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    QSignalSpy spy(f.tools, &pictura::ToolController::brushPickerRequested);
    f.tools->routeCanvasContextMenu(f.canvas, QPointF(2, 2), QPoint(10, 10));
    QCOMPARE(spy.count(), 1);
}

void CanvasContextMenusTest::brushPresetApplies()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_brush_apply"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    // The options bar builds the picker lazily; drive the real request path so
    // this test stands alone.
    f.tools->routeCanvasContextMenu(f.canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = window_->findChild<pictura::BrushPresetPicker*>();
    QVERIFY(picker != nullptr);
    QVERIFY(picker->presetCount() > 0);
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint != nullptr);
    // Force a differing size so applying a preset is observable.
    paint->setSize(paint->size() == 7 ? 8 : 7);
    QSignalSpy tipSpy(f.tools, &pictura::ToolController::brushTipChanged);
    QSignalSpy sizeSpy(f.tools, &pictura::ToolController::brushSizeChanged);
    picker->choosePresetForTest(0);
    QVERIFY(tipSpy.count() + sizeSpy.count() > 0);
}

void CanvasContextMenusTest::brushPickerIsNotACapturingPopup()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_brush_nograb"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    f.tools->routeCanvasContextMenu(f.canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = window_->findChild<pictura::BrushPresetPicker*>();
    QVERIFY(picker != nullptr);
    // A non-grabbing in-window overlay, not a Qt::Popup: no implicit pointer
    // grab, so the canvas keeps receiving hover and the first press paints.
    QVERIFY(!picker->isWindow());
    QVERIFY(!picker->windowFlags().testFlag(Qt::Popup));
    picker->hide();
}

void CanvasContextMenusTest::brushPickerKeepsBrushHover()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_brush_hover"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    f.tools->routeCanvasContextMenu(f.canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = window_->findChild<pictura::BrushPresetPicker*>();
    QVERIFY(picker != nullptr);
    // With the picker open the canvas still resolves hover to the brush ring.
    f.canvas->mouseMoved(QPointF(12, 12));
    QVERIFY(f.canvas->hasBrushOutlineForTest());
    picker->hide();
}

void CanvasContextMenusTest::brushPickerDismissesOnOutsidePress()
{
    Fixture f(*window_, seedImage(), QStringLiteral("ccm_brush_dismiss"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    window_->show();
    QCoreApplication::processEvents();
    f.tools->routeCanvasContextMenu(f.canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = window_->findChild<pictura::BrushPresetPicker*>();
    QVERIFY(picker != nullptr);
    QVERIFY(picker->isVisible());
    // A press well outside the overlay, delivered to the canvas: the overlay
    // dismisses itself without consuming the press, so the canvas still gets it.
    const QPoint outside =
        picker->mapToGlobal(QPoint(picker->width() + 40, picker->height() + 40));
    QMouseEvent press(QEvent::MouseButtonPress, f.canvas->mapFromGlobal(outside), outside,
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QSignalSpy pressSpy(f.canvas, &pictura::ImageView::mousePressed);
    QCoreApplication::sendEvent(f.canvas, &press);
    QCOMPARE(pressSpy.count(), 1);
    QVERIFY(!picker->isVisible());
}

QTEST_MAIN(CanvasContextMenusTest)
#include "tst_canvas_context_menus.moc"
