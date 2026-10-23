// Crop Settings menu (#104): the menu items, the Classic-mode gating, the P / H
// keys, the shield (enable, colour, opacity, Auto Adjust Opacity), Show Cropped
// Area, Auto Center Preview and persistence.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "panels/numeric_field.h"
#include "session.h"
#include "tool_options_json.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include <cmath>

class CropSettingsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void menuListsItsItems();
    void classicModeGatesTheModernItems();
    void boxesTurnOverTheFixedImageInClassic();
    void rotationGestureTurnsTheBoxWithThePointer();
    void classicDragMovesTheBox();
    void keysToggleClassicAndShowArea();
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

    void key(Qt::Key k)
    {
        QKeyEvent event(QEvent::KeyPress, k, Qt::NoModifier);
        QApplication::sendEvent(window_.get(), &event);
    }

    QMenu* cogMenu()
    {
        auto* button = window_->findChild<QToolButton*>(QStringLiteral("optionsCropCog"));
        return button ? button->menu() : nullptr;
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

void CropSettingsTest::keysToggleClassicAndShowArea()
{
    openDocument(100);
    QVERIFY(crop_->classicMode());
    key(Qt::Key_P);
    QVERIFY(!crop_->classicMode());
    QVERIFY(crop_->settings().showCroppedArea);
    key(Qt::Key_H);
    QVERIFY(!crop_->settings().showCroppedArea);
    key(Qt::Key_H);
    QVERIFY(crop_->settings().showCroppedArea);
    key(Qt::Key_P);
    QVERIFY(crop_->classicMode());
    key(Qt::Key_H);  // Classic mode ignores H
    QVERIFY(crop_->settings().showCroppedArea);
    QCOMPARE(tools_->activeTool(), pictura::ToolId::Crop);
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
    crop_->setSettings(s);

    const QJsonObject stored =
        pictura::loadSession().toolOptions.value(QStringLiteral("crop")).toObject();
    QCOMPARE(stored.value(QStringLiteral("shieldOpacity")).toInt(), 33);

    crop_->applySettings(pictura::CropSettings{});
    QCOMPARE(crop_->settings().shieldOpacity, 75);
    pictura::applyToolOptionsJson(*tools_, pictura::loadSession().toolOptions);
    const pictura::CropSettings& back = crop_->settings();
    QCOMPARE(back.shieldOpacity, 33);
    QVERIFY(back.shieldCustomColor);
    QCOMPARE(back.shieldColor, QColor(1, 2, 3));
    QVERIFY(!back.shieldEnabled);
    QVERIFY(!back.autoAdjustOpacity);
    QVERIFY(!back.showCroppedArea);
    QVERIFY(!back.autoCenterPreview);
}

QTEST_MAIN(CropSettingsTest)
#include "tst_crop_settings.moc"
