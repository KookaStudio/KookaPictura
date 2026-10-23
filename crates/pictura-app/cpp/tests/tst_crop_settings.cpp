// Crop Settings menu (#104): the menu items, the CS6 defaults, the Classic-mode
// gating, the P / H / O keys through the real shortcut path, the grid menu's
// show modes and cycle commands, the shield (enable, colour, opacity, Auto
// Adjust Opacity), Show Cropped Area, Auto Center Preview, a turned Classic
// commit, and persistence.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "panels/numeric_field.h"
#include "session.h"
#include "tool_options_json.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QKeyEvent>
#include <QtGui/QShortcut>
#include <QtGui/QWindow>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include <cmath>

class CropSettingsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void menuListsItsItems();
    void defaultsMatchCs6();
    void classicModeGatesTheModernItems();
    void boxesTurnOverTheFixedImageInClassic();
    void turnedClassicCommitTakesTheContentUnderTheBox();
    void rotationGestureTurnsTheBoxWithThePointer();
    void classicDragMovesTheBox();
    void keysToggleClassicAndShowArea();
    void gridMenuDrivesTheOverlay();
    void runtimePatchRefreshesTheMenu();
    void toolSwitchAsksToCrop();
    void perspectiveSwitchAsksToCrop();
    void hidingTheOutsideIsOpaque();
    void shieldCanBeTurnedOff();
    void opacitySetsTheAlpha();
    void customColourTintsTheShield();
    void autoAdjustThinsADrag();
    void autoCenterPansToTheBox();
    void settingsPersist();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::ToolController* tools_ = nullptr;
    pictura::CropOptions* crop_ = nullptr;
    pictura::ImageView* canvas_ = nullptr;

    void openDocument(int size)
    {
        pictura::PicturaMainWindow& frame = *window_;
        QVERIFY(frame.newDocument(QStringLiteral("Settings"), size, size, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
        canvas_ = frame.imageView();
        QVERIFY(canvas_);
        canvas_->actualPixels();
        frame.setActiveTool(pictura::ToolId::Crop);
    }

    void drag(const QPointF& from, const QPointF& to)
    {
        canvas_->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas_->mouseMoved(to);
        canvas_->mouseReleased(to);
    }

    QMenu* cogMenu()
    {
        auto* button = window_->findChild<QToolButton*>(QStringLiteral("optionsCropCog"));
        return button ? button->menu() : nullptr;
    }

    QAction* gridAction(const QString& text)
    {
        auto* button = window_->findChild<QToolButton*>(QStringLiteral("optionsCropGrid"));
        if (!button || !button->menu()) {
            return nullptr;
        }
        for (QAction* a : button->menu()->actions()) {
            if (a->text().section(QLatin1Char('\t'), 0, 0) == text) {
                return a;
            }
        }
        return nullptr;
    }

    // A key typed into the window, through the shortcut map like a real one.
    void typed(Qt::Key k, Qt::KeyboardModifiers mods = Qt::NoModifier)
    {
        QTest::keyClick(window_->windowHandle(), k, mods);
    }
};

void CropSettingsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    tools_ = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools_);
    crop_ = tools_->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    QVERIFY(crop_);
}

void CropSettingsTest::init()
{
    crop_->setClassicMode(true);
    crop_->setSettings(pictura::CropSettings{});
}

void CropSettingsTest::menuListsItsItems()
{
    QMenu* menu = cogMenu();
    QVERIFY(menu);
    for (const char* name : {"cropClassicMode", "cropShowArea", "cropAutoCenter",
                             "cropShieldEnabled", "cropShieldAutoAdjust"}) {
        QVERIFY2(menu->findChild<QCheckBox*>(QLatin1String(name)), name);
    }
    QVERIFY(menu->findChild<QComboBox*>(QStringLiteral("cropShieldColor")));
    auto* opacity = menu->findChild<pictura::NumericField*>(QStringLiteral("cropShieldOpacity"));
    QVERIFY(opacity);
    QCOMPARE(opacity->value(), 75.0);
    // Opacity is the shared slider field: its arrow opens the tracking popup.
    QVERIFY(opacity->findChild<QToolButton*>(QStringLiteral("cropShieldOpacityArrow")));
    bool sawP = false;
    bool sawH = false;
    for (QLabel* label : menu->findChildren<QLabel*>()) {
        sawP |= label->text() == QLatin1String("P");
        sawH |= label->text() == QLatin1String("H");
    }
    QVERIFY(sawP && sawH);
}

// CS6's defaults: Modern mode with Show Cropped Area and Auto Center Preview on,
// and the overlay shown automatically.
void CropSettingsTest::defaultsMatchCs6()
{
    const pictura::CropOptions fresh;
    QVERIFY(!fresh.classicMode());
    QVERIFY(!pictura::SessionState{}.cropClassicMode);
    const pictura::CropSettings d;
    QVERIFY(d.showCroppedArea);
    QVERIFY(d.autoCenterPreview);
    QCOMPARE(d.overlayShow, int(pictura::CropSettings::AutoShow));
    QCOMPARE(d.overlayOrientation, 0);
}

void CropSettingsTest::classicModeGatesTheModernItems()
{
    QMenu* menu = cogMenu();
    QVERIFY(menu);
    auto* show = menu->findChild<QCheckBox*>(QStringLiteral("cropShowArea"));
    auto* center = menu->findChild<QCheckBox*>(QStringLiteral("cropAutoCenter"));
    QVERIFY(crop_->classicMode());
    QVERIFY(!show->isEnabled() && !show->isChecked());
    QVERIFY(!center->isEnabled() && !center->isChecked());

    crop_->setClassicMode(false);
    QVERIFY(show->isEnabled() && show->isChecked());
    QVERIFY(center->isEnabled() && center->isChecked());

    crop_->setClassicMode(true);
    QVERIFY(!show->isEnabled() && !show->isChecked());
}

void CropSettingsTest::boxesTurnOverTheFixedImageInClassic()
{
    openDocument(200);
    crop_->setClassicMode(true);
    drag(QPointF(100, 100), QPointF(80, 20));
    QCOMPARE(tools_->pendingCropRect(), QRect(80, 20, 20, 80));
    // A quarter turn: the 20 x 80 box shows as 80 x 20 about its centre (90, 60).
    crop_->setAngle(90.0);
    QCOMPARE(canvas_->cropStraightenForTest(), 90.0);
    // Dragging the turned box on screen moves it along the screen axes: 10 px
    // down is 10 px along the model's -x.
    drag(QPointF(110, 60), QPointF(110, 70));
    QCOMPARE(tools_->pendingCropRect(), QRect(70, 20, 20, 80));
    crop_->setAngle(0.0);
    window_->setActiveTool(pictura::ToolId::Move);
}

// A 20 x 80 box turned a quarter turn lies across the edge between a 100 x 100
// white canvas and the area Canvas Size added beside it; the commit is that
// box's content, turned upright.
void CropSettingsTest::turnedClassicCommitTakesTheContentUnderTheBox()
{
    QVERIFY(window_->newDocument(QStringLiteral("Turned"), 100, 100, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    QVERIFY(pictura::canvas_size_apply(*view, QStringLiteral("top-left"), 200, 100, 0xff0000u));
    const QRgb left = view->composite_argb(50, 50);
    const QRgb right = view->composite_argb(150, 50);
    QVERIFY(left != right);
    canvas_ = window_->imageView();
    canvas_->actualPixels();
    window_->setActiveTool(pictura::ToolId::Crop);
    crop_->setClassicMode(true);
    drag(QPointF(110, 90), QPointF(90, 10));
    QCOMPARE(tools_->pendingCropRect(), QRect(90, 10, 20, 80));
    crop_->setAngle(90.0);
    QVERIFY(tools_->cropCommitSession()->commitCrop());

    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 80);
    const QRgb first = view->composite_argb(10, 2);
    const QRgb last = view->composite_argb(10, 77);
    QVERIFY2((first == left && last == right) || (first == right && last == left),
             "the turned box's two ends hold the two halves of the canvas");
    // Away from the seam each row is one colour: the box held no outside area.
    for (int y = 0; y < 80; ++y) {
        if (y >= 36 && y < 44) {
            continue;
        }
        for (int x = 0; x < 20; ++x) {
            QCOMPARE(view->composite_argb(x, y), y < 40 ? first : last);
        }
    }
    crop_->setAngle(0.0);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::rotationGestureTurnsTheBoxWithThePointer()
{
    openDocument(200);
    crop_->setClassicMode(true);
    drag(QPointF(120, 120), QPointF(80, 80));  // a 40 x 40 box centred on (100, 100)
    // Grab outside the right edge and sweep a quarter turn clockwise on screen:
    // the box turns clockwise, which is -90 degrees of content in the model.
    canvas_->mousePressed(QPointF(130, 100), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(100, 130));
    QCOMPARE(crop_->angle(), -90.0);
    canvas_->mouseReleased(QPointF(100, 130));
    // Modern turns the content with the pointer instead.
    crop_->setAngle(0.0);
    crop_->setClassicMode(false);
    canvas_->mousePressed(QPointF(130, 100), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(100, 130));
    QCOMPARE(crop_->angle(), 90.0);
    canvas_->mouseReleased(QPointF(100, 130));
    crop_->setAngle(0.0);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::classicDragMovesTheBox()
{
    openDocument(100);
    crop_->setClassicMode(true);
    drag(QPointF(50, 50), QPointF(10, 10));
    QCOMPARE(tools_->pendingCropRect(), QRect(10, 10, 40, 40));
    drag(QPointF(30, 30), QPointF(40, 40));
    QCOMPARE(tools_->pendingCropRect(), QRect(20, 20, 40, 40));
    window_->setActiveTool(pictura::ToolId::Move);
}

// P, H and O are also tool letters (Pen, Hand, Dodge), held by window
// shortcuts that see a real key before keyPressEvent does; while Crop is
// active they are the Crop tool's.
void CropSettingsTest::keysToggleClassicAndShowArea()
{
    openDocument(100);
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
    bool letterP = false;
    for (QShortcut* shortcut : window_->findChildren<QShortcut*>()) {
        letterP |= shortcut->key() == QKeySequence(QStringLiteral("P"));
    }
    QVERIFY2(letterP, "P is a tool letter too, so the shortcut path is exercised");

    QVERIFY(crop_->classicMode());
    typed(Qt::Key_P);
    QVERIFY(!crop_->classicMode());
    QCOMPARE(tools_->activeTool(), pictura::ToolId::Crop);
    QVERIFY(crop_->settings().showCroppedArea);
    typed(Qt::Key_H);
    QVERIFY(!crop_->settings().showCroppedArea);
    typed(Qt::Key_H);
    QVERIFY(crop_->settings().showCroppedArea);
    typed(Qt::Key_P);
    QVERIFY(crop_->classicMode());
    typed(Qt::Key_H);  // Classic mode takes H as a no-op
    QVERIFY(crop_->settings().showCroppedArea);
    QCOMPARE(tools_->activeTool(), pictura::ToolId::Crop);

    crop_->setGridOverlay(0);
    typed(Qt::Key_O);
    QCOMPARE(crop_->gridOverlay(), 1);
    typed(Qt::Key_O, Qt::ShiftModifier);  // Grid has no orientation
    QCOMPARE(crop_->settings().overlayOrientation, 0);
    crop_->setGridOverlay(3);
    typed(Qt::Key_O, Qt::ShiftModifier);
    QCOMPARE(crop_->settings().overlayOrientation, 1);
    crop_->setGridOverlay(5);
    typed(Qt::Key_O);
    QCOMPARE(crop_->gridOverlay(), 0);
    QCOMPARE(tools_->activeTool(), pictura::ToolId::Crop);
    window_->setActiveTool(pictura::ToolId::Move);
}

// The grid menu's show modes decide when the guides are drawn, and its cycle
// commands step the overlay and, for Triangle and Golden Spiral, its
// orientation.
void CropSettingsTest::gridMenuDrivesTheOverlay()
{
    openDocument(100);
    crop_->setGridOverlay(0);
    drag(QPointF(50, 50), QPointF(10, 10));
    QCOMPARE(canvas_->cropOverlayForTest(), -1);  // Auto: hidden at rest
    canvas_->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(32, 32));
    QCOMPARE(canvas_->cropOverlayForTest(), 0);  // shown while adjusting
    canvas_->mouseReleased(QPointF(32, 32));
    QCOMPARE(canvas_->cropOverlayForTest(), -1);

    QAction* always = gridAction(QStringLiteral("Always Show Overlay"));
    QAction* never = gridAction(QStringLiteral("Never Show Overlay"));
    QAction* cycle = gridAction(QStringLiteral("Cycle Overlay"));
    QAction* orient = gridAction(QStringLiteral("Cycle Orientation"));
    QVERIFY(always && never && cycle && orient && gridAction(QStringLiteral("Auto Show Overlay")));
    always->trigger();
    QCOMPARE(crop_->settings().overlayShow, int(pictura::CropSettings::AlwaysShow));
    QCOMPARE(canvas_->cropOverlayForTest(), 0);
    never->trigger();
    canvas_->mousePressed(QPointF(32, 32), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(34, 34));
    QCOMPARE(canvas_->cropOverlayForTest(), -1);
    canvas_->mouseReleased(QPointF(34, 34));

    always->trigger();
    cycle->trigger();
    QCOMPARE(crop_->gridOverlay(), 1);
    QCOMPARE(canvas_->cropOverlayForTest(), 1);
    QVERIFY(!orient->isEnabled());
    crop_->setGridOverlay(5);
    QVERIFY(orient->isEnabled());
    orient->trigger();
    orient->trigger();
    QCOMPARE(crop_->settings().overlayOrientation, 2);
    QCOMPARE(canvas_->cropOverlayOrientationForTest(), 2);
    crop_->setGridOverlay(0);
    window_->setActiveTool(pictura::ToolId::Move);
}

// The control server patches options at runtime through applyToolOptionsJson;
// the open menu and the canvas follow the patch.
void CropSettingsTest::runtimePatchRefreshesTheMenu()
{
    openDocument(100);
    drag(QPointF(50, 50), QPointF(10, 10));
    QMenu* menu = cogMenu();
    QVERIFY(menu);
    auto* opacity = menu->findChild<pictura::NumericField*>(QStringLiteral("cropShieldOpacity"));
    QVERIFY(opacity);
    QSignalSpy changed(tools_, &pictura::ToolController::cropOptionsChanged);
    pictura::applyToolOptionsJson(
        *tools_, QJsonObject{{QStringLiteral("crop"),
                              QJsonObject{{QStringLiteral("shieldOpacity"), 40}}}});
    QVERIFY(changed.count() >= 1);
    QCOMPARE(opacity->value(), 40.0);
    QCOMPARE(canvas_->cropShieldAlpha(), 102);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::toolSwitchAsksToCrop()
{
    openDocument(100);
    crop_->setClassicMode(true);
    drag(QPointF(50, 50), QPointF(10, 10));
    QVERIFY(tools_->hasPendingCrop());

    // Cancel keeps the crop and the tool.
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::Cancel; });
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Crop);
    QVERIFY(tools_->hasPendingCrop());

    // Don't Crop leaves without cropping.
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::DontCrop; });
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
    QCOMPARE(canvas_->documentSize(), QSize(100, 100));

    // Crop commits the box first.
    window_->setActiveTool(pictura::ToolId::Crop);
    drag(QPointF(50, 50), QPointF(10, 10));
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::Crop; });
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
    QCOMPARE(canvas_->documentSize(), QSize(40, 40));

    // A default box is not a pending crop, so nothing is asked.
    int asked = 0;
    tools_->setCropDecisionPrompt([&asked]() {
        ++asked;
        return pictura::CropDecision::Cancel;
    });
    window_->setActiveTool(pictura::ToolId::Crop);
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(asked, 0);
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::DontCrop; });
}

void CropSettingsTest::perspectiveSwitchAsksToCrop()
{
    openDocument(120);
    window_->setActiveTool(pictura::ToolId::PerspectiveCrop);
    for (const QPointF& p : {QPointF(30, 30), QPointF(90, 30), QPointF(90, 90), QPointF(30, 90)}) {
        canvas_->mousePressed(p, Qt::LeftButton, int(Qt::NoModifier));
        canvas_->mouseReleased(p);
    }
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::Cancel; });
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(window_->activeTool(), pictura::ToolId::PerspectiveCrop);
    tools_->setCropDecisionPrompt([]() { return pictura::CropDecision::DontCrop; });
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
}

void CropSettingsTest::hidingTheOutsideIsOpaque()
{
    openDocument(100);
    drag(QPointF(50, 50), QPointF(10, 10));
    QCOMPARE(canvas_->cropShieldAlpha(), 191);
    crop_->setClassicMode(false);
    pictura::CropSettings s = crop_->settings();
    s.showCroppedArea = false;
    crop_->setSettings(s);
    QCOMPARE(canvas_->cropShieldAlpha(), 255);
    // Classic mode always dims, whatever the stored value.
    crop_->setClassicMode(true);
    QCOMPARE(canvas_->cropShieldAlpha(), 191);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::shieldCanBeTurnedOff()
{
    openDocument(100);
    drag(QPointF(50, 50), QPointF(10, 10));
    pictura::CropSettings s = crop_->settings();
    s.shieldEnabled = false;
    crop_->setSettings(s);
    QCOMPARE(canvas_->cropShieldAlpha(), 0);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::opacitySetsTheAlpha()
{
    openDocument(100);
    drag(QPointF(50, 50), QPointF(10, 10));
    pictura::CropSettings s = crop_->settings();
    s.shieldOpacity = 40;
    crop_->setSettings(s);
    QCOMPARE(canvas_->cropShieldAlpha(), 102);
    s.shieldOpacity = 250;
    crop_->setSettings(s);
    QCOMPARE(crop_->settings().shieldOpacity, 100);
    QCOMPARE(canvas_->cropShieldAlpha(), 255);
    s.shieldOpacity = -5;
    crop_->setSettings(s);
    QCOMPARE(canvas_->cropShieldAlpha(), 0);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::customColourTintsTheShield()
{
    openDocument(100);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    canvas_->fitOnScreen();
    drag(QPointF(50, 50), QPointF(10, 10));
    QCoreApplication::processEvents();

    const QPoint outside = canvas_->imageToWidget(QPointF(80, 80)).toPoint();
    const QRgb matched = canvas_->grab().toImage().pixel(outside);

    pictura::CropSettings s = crop_->settings();
    s.shieldCustomColor = true;
    s.shieldColor = Qt::red;
    crop_->setSettings(s);
    QCoreApplication::processEvents();
    const QRgb red = canvas_->grab().toImage().pixel(outside);
    QVERIFY2(qRed(red) > qBlue(red) + 50, "a red shield tints the white canvas red");
    QVERIFY(red != matched);

    s.shieldCustomColor = false;
    crop_->setSettings(s);
    QCoreApplication::processEvents();
    QCOMPARE(canvas_->grab().toImage().pixel(outside), matched);
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::autoAdjustThinsADrag()
{
    openDocument(100);
    drag(QPointF(50, 50), QPointF(10, 10));
    QCOMPARE(canvas_->cropShieldAlpha(), 191);

    canvas_->mousePressed(QPointF(30, 30), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(35, 35));
    QCOMPARE(canvas_->cropShieldAlpha(), 96);
    canvas_->mouseReleased(QPointF(35, 35));
    QCOMPARE(canvas_->cropShieldAlpha(), 191);

    pictura::CropSettings s = crop_->settings();
    s.autoAdjustOpacity = false;
    crop_->setSettings(s);
    canvas_->mousePressed(QPointF(35, 35), Qt::LeftButton, int(Qt::NoModifier));
    canvas_->mouseMoved(QPointF(38, 38));
    QCOMPARE(canvas_->cropShieldAlpha(), 191);
    canvas_->mouseReleased(QPointF(38, 38));
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::autoCenterPansToTheBox()
{
    openDocument(2000);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    canvas_->actualPixels();
    crop_->setClassicMode(false);

    const QPointF view = QRectF(canvas_->rect()).center();
    const auto offBy = [&](const QPointF& image) {
        const QPointF d = canvas_->imageToWidget(image) - view;
        return std::hypot(d.x(), d.y());
    };
    const QPointF boxCentre(900, 900);
    drag(QPointF(1000, 1000), QPointF(800, 800));
    QVERIFY2(offBy(boxCentre) < 2.0, "the box is centred after the gesture");

    pictura::CropSettings s = crop_->settings();
    s.autoCenterPreview = false;
    crop_->setSettings(s);
    canvas_->panBy(QPointF(60, 40));
    const double before = offBy(boxCentre);
    drag(QPointF(900, 900), QPointF(890, 890));
    QVERIFY2(offBy(boxCentre) >= before - 1.0, "with Auto Center off the view stays put");
    window_->setActiveTool(pictura::ToolId::Move);
}

void CropSettingsTest::settingsPersist()
{
    pictura::CropSettings s;
    s.shieldOpacity = 33;
    s.shieldCustomColor = true;
    s.shieldColor = QColor(1, 2, 3);
    s.shieldEnabled = false;
    s.autoAdjustOpacity = false;
    s.showCroppedArea = false;
    s.autoCenterPreview = false;
    s.overlayShow = pictura::CropSettings::NeverShow;
    s.overlayOrientation = 3;
    crop_->setSettings(s);

    // Resetting saves the session too, so keep the stored options first.
    const QJsonObject options = pictura::loadSession().toolOptions;
    QCOMPARE(options.value(QStringLiteral("crop")).toObject().value(QStringLiteral("shieldOpacity")).toInt(),
             33);

    crop_->setSettings(pictura::CropSettings{});
    QCOMPARE(crop_->settings().shieldOpacity, 75);
    pictura::applyToolOptionsJson(*tools_, options);
    const pictura::CropSettings& back = crop_->settings();
    QCOMPARE(back.shieldOpacity, 33);
    QVERIFY(back.shieldCustomColor);
    QCOMPARE(back.shieldColor, QColor(1, 2, 3));
    QVERIFY(!back.shieldEnabled);
    QVERIFY(!back.autoAdjustOpacity);
    QVERIFY(!back.showCroppedArea);
    QVERIFY(!back.autoCenterPreview);
    QCOMPARE(back.overlayShow, int(pictura::CropSettings::NeverShow));
    QCOMPARE(back.overlayOrientation, 3);
}

QTEST_MAIN(CropSettingsTest)
#include "tst_crop_settings.moc"
