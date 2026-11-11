// The Brush panel's Shape Dynamics, Scattering, Color Dynamics, Transfer, and
// Brush Pose sets: the Control model, padlocks, preset behaviour, and
// persistence.

#include <QtTest/QtTest>

#include "frame.h"
#include "panels/brush_panel.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "paint_tip.h"
#include "qt_test_support.h"
#include "tool_options_json.h"
#include "tools.h"

#include "api/paint_bridge.h"

#include <QtCore/QJsonObject>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <memory>

namespace {

pictura::PaintOptions* brushOptions(pictura::PicturaMainWindow& frame)
{
    auto* tools = frame.findChild<pictura::ToolController*>();
    return tools ? tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush) : nullptr;
}

QListWidget* optionSets(pictura::PicturaMainWindow& frame)
{
    return frame.findChild<QListWidget*>(QStringLiteral("brushOptionSets"));
}

QListWidgetItem* row(pictura::PicturaMainWindow& frame, const QString& name)
{
    QListWidget* sets = optionSets(frame);
    if (!sets) {
        return nullptr;
    }
    for (int i = 0; i < sets->count(); ++i) {
        if (sets->item(i)->text() == name) {
            return sets->item(i);
        }
    }
    return nullptr;
}

void selectSet(pictura::PicturaMainWindow& frame, const QString& name)
{
    QListWidget* sets = optionSets(frame);
    QListWidgetItem* item = row(frame, name);
    if (sets && item) {
        sets->setCurrentItem(item);
    }
}

template <typename T> T* field(pictura::PicturaMainWindow& frame, const char* name)
{
    return frame.findChild<T*>(QString::fromLatin1(name));
}

} // namespace

class BrushDynamicsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void shapeDynamicsPageOpens();
    void setCheckboxWritesTheModel();
    void lockedSetSurvivesPreset();
    void unlockedSetFollowsPreset();
    void dynamicsRoundTrip();
    void legacySettingsKeepDefaults();
    void roundnessReachesTheBrush();
    void flattenedTipPaintsWide();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void BrushDynamicsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void BrushDynamicsTest::shapeDynamicsPageOpens()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    selectSet(*window, QStringLiteral("Shape Dynamics"));
    auto* pages = window->findChild<QStackedWidget*>(QStringLiteral("brushDynamicsPages"));
    auto* sets = optionSets(*window);
    QVERIFY(pages && sets);
    QCOMPARE(pages->currentIndex(), sets->row(row(*window, QStringLiteral("Shape Dynamics"))));
    auto* sizeJitter = field<pictura::NumericField>(*window, "dynShapeSizeJitter");
    QVERIFY(sizeJitter);
    QVERIFY(!sizeJitter->isHidden());
    QVERIFY(sizeJitter->isEnabled());
}

void BrushDynamicsTest::setCheckboxWritesTheModel()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    QVERIFY(paint);
    pictura::BrushDynamics d = paint->dynamics();
    d.scatter = 123;
    d.scatterEnabled = true;
    paint->setDynamics(d);

    QListWidgetItem* item = row(*window, QStringLiteral("Scattering"));
    QVERIFY(item);
    QCOMPARE(item->checkState(), Qt::Checked);
    item->setCheckState(Qt::Unchecked);
    QVERIFY(!paint->dynamics().scatterEnabled);
    QCOMPARE(paint->dynamics().scatter, 123);
}

void BrushDynamicsTest::lockedSetSurvivesPreset()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    auto* tools = window->findChild<pictura::ToolController*>();
    QVERIFY(paint && tools);

    pictura::BrushDynamics d = paint->dynamics();
    d.colorEnabled = true;
    d.applyPerTip = true;
    d.hueJitter = 77;
    d.saturationJitter = 44;
    d.purity = -25;
    paint->setDynamics(d);

    selectSet(*window, QStringLiteral("Color Dynamics"));
    auto* lock = field<QToolButton>(*window, "brushSetLock");
    QVERIFY(lock && lock->isEnabled());
    lock->click();
    QVERIFY(paint->dynamics().lockColor);

    QListWidgetItem* item = row(*window, QStringLiteral("Color Dynamics"));
    QVERIFY(item && !item->icon().isNull());

    pictura::BrushPresetPicker picker(tools, window.get());
    picker.choosePresetForTest(0);
    const pictura::BrushDynamics after = paint->dynamics();
    QVERIFY(after.colorEnabled);
    QVERIFY(after.applyPerTip);
    QCOMPARE(after.hueJitter, 77);
    QCOMPARE(after.saturationJitter, 44);
    QCOMPARE(after.purity, -25);
}

void BrushDynamicsTest::unlockedSetFollowsPreset()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    auto* tools = window->findChild<pictura::ToolController*>();
    QVERIFY(paint && tools);

    pictura::BrushDynamics d = paint->dynamics();
    d.shapeEnabled = true;
    d.sizeJitter = 99;
    d.angleJitter = 99;
    d.roundnessJitter = 99;
    d.lockShape = false;
    paint->setDynamics(d);

    pictura::BrushPresetPicker picker(tools, window.get());
    picker.choosePresetForTest(0);
    const pictura::BrushDynamics after = paint->dynamics();
    QCOMPARE(after.sizeJitter, 0);
    QCOMPARE(after.angleJitter, 0);
    QCOMPARE(after.roundnessJitter, 0);
    QVERIFY(!after.shapeEnabled);
}

void BrushDynamicsTest::dynamicsRoundTrip()
{
    {
        auto window = pictura::test::makeMainWindow();
        QVERIFY(window);
        auto* paint = brushOptions(*window);
        QVERIFY(paint);
        pictura::BrushDynamics d = paint->dynamics();
        d.shapeEnabled = true;
        d.sizeControl = 2;
        d.sizeFade = 40;
        d.minDiameter = 25;
        d.angleControl = 7;
        d.scatterEnabled = true;
        d.scatterControl = 3;
        d.bothAxes = false;
        d.countJitter = 35;
        d.colorEnabled = true;
        d.applyPerTip = true;
        d.hueJitter = 60;
        d.purity = 30;
        d.transferEnabled = true;
        d.opacityControl = 2;
        d.opacityJitter = 50;
        d.poseEnabled = true;
        d.poseTiltX = 20;
        d.overridePressure = true;
        d.posePressure = 40;
        d.lockScatter = true;
        d.lockColor = true;
        paint->setDynamics(d);
        window->saveSession();
    }

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    QVERIFY(paint);
    const pictura::BrushDynamics d = paint->dynamics();
    QVERIFY(d.shapeEnabled);
    QCOMPARE(d.sizeControl, 2);
    QCOMPARE(d.sizeFade, 40);
    QCOMPARE(d.minDiameter, 25);
    QCOMPARE(d.angleControl, 7);
    QVERIFY(d.scatterEnabled);
    QCOMPARE(d.scatterControl, 3);
    QVERIFY(!d.bothAxes);
    QCOMPARE(d.countJitter, 35);
    QVERIFY(d.colorEnabled);
    QVERIFY(d.applyPerTip);
    QCOMPARE(d.hueJitter, 60);
    QCOMPARE(d.purity, 30);
    QVERIFY(d.transferEnabled);
    QCOMPARE(d.opacityControl, 2);
    QCOMPARE(d.opacityJitter, 50);
    QVERIFY(d.poseEnabled);
    QCOMPARE(d.poseTiltX, 20);
    QVERIFY(d.overridePressure);
    QCOMPARE(d.posePressure, 40);
    QVERIFY(d.lockScatter);
    QVERIFY(d.lockColor);
}

void BrushDynamicsTest::legacySettingsKeepDefaults()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* tools = window->findChild<pictura::ToolController*>();
    auto* paint = brushOptions(*window);
    QVERIFY(tools && paint);

    QJsonObject brush;
    brush.insert(QStringLiteral("scatter"), 77);
    brush.insert(QStringLiteral("count"), 3);
    brush.insert(QStringLiteral("sizeJitter"), 10);
    brush.insert(QStringLiteral("angleJitter"), 20);
    brush.insert(QStringLiteral("roundnessJitter"), 30);
    QJsonObject root;
    root.insert(QStringLiteral("brush"), brush);
    pictura::applyToolOptionsJson(*tools, root);

    const pictura::BrushDynamics d = paint->dynamics();
    QCOMPARE(d.scatter, 77);
    QCOMPARE(d.count, 3);
    QCOMPARE(d.sizeJitter, 10);
    QCOMPARE(d.angleJitter, 20);
    QCOMPARE(d.roundnessJitter, 30);
    QCOMPARE(d.sizeControl, 0);
    QVERIFY(!d.shapeEnabled);
    QVERIFY(!d.colorEnabled);
    QVERIFY(!d.transferEnabled);
    QVERIFY(!d.poseEnabled);
}

void BrushDynamicsTest::roundnessReachesTheBrush()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    auto* roundness = field<pictura::NumericField>(*window, "brushPanelRoundness");
    QVERIFY(paint && roundness);
    auto* edit = roundness->findChild<QLineEdit*>();
    QVERIFY(edit);
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("30"));
    QTest::keyClick(edit, Qt::Key_Return);
    QCOMPARE(paint->roundness(), 30);
    auto* panel = window->findChild<pictura::BrushPanel*>(QStringLiteral("brushPanel"));
    QVERIFY(panel && !panel->previewForTest().isNull());
}

void BrushDynamicsTest::flattenedTipPaintsWide()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    QVERIFY(paint);
    paint->setSize(20);
    paint->setHardness(100);
    paint->setRoundness(30);
    paint->setTipAngle(0);
    const pictura::PaintTip tip = pictura::paintTip(*paint);
    const ::rust::Vec<std::uint8_t> rgba = pictura::brush_dab_preview(tip, 64);
    QCOMPARE(int(rgba.size()), 64 * 64 * 4);
    int minX = 64, maxX = -1, minY = 64, maxY = -1;
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 64; ++x) {
            if (rgba[std::size_t(y * 64 + x) * 4 + 3] == 0) {
                continue;
            }
            minX = std::min(minX, x);
            maxX = std::max(maxX, x);
            minY = std::min(minY, y);
            maxY = std::max(maxY, y);
        }
    }
    QVERIFY(maxX >= minX && maxY >= minY);
    QVERIFY(maxX - minX > maxY - minY);
}

QTEST_MAIN(BrushDynamicsTest)
#include "tst_brush_dynamics.moc"
