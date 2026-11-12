// Bristle tips: the Brush Tip Shape controls, presets, and persistence.

#include <QtTest/QtTest>

#include "frame.h"
#include "options_bar.h"
#include "panels/brush_panel.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "qt_test_support.h"
#include "selftest_paint_fixture.h"
#include "tool_options_json.h"
#include "tools.h"

#include <QtCore/QJsonObject>
#include <QtGui/QImage>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QStackedWidget>

#include <memory>

namespace {

using paint_fixture::Fixture;

QImage seedImage()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

pictura::PaintOptions* brushOptions(pictura::PicturaMainWindow& frame)
{
    auto* tools = frame.findChild<pictura::ToolController*>();
    return tools ? tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush) : nullptr;
}

template <typename T> T* field(pictura::PicturaMainWindow& frame, const char* name)
{
    return frame.findChild<T*>(QString::fromLatin1(name));
}

void typeInto(pictura::NumericField* field, const QString& text)
{
    auto* edit = field->findChild<QLineEdit*>();
    QVERIFY(edit);
    edit->selectAll();
    QTest::keyClicks(edit, text);
    QTest::keyClick(edit, Qt::Key_Return);
}

pictura::BrushPresetPicker* buildPicker(pictura::PicturaMainWindow& frame)
{
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* canvas = frame.imageView();
    if (!tools || !canvas) {
        return nullptr;
    }
    tools->routeCanvasContextMenu(canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = frame.findChild<pictura::BrushPresetPicker*>();
    if (picker) {
        picker->hide();
    }
    return picker;
}

bool choosePreset(pictura::BrushPresetPicker* picker, const QString& name)
{
    auto* grid = picker ? picker->findChild<QListWidget*>(QStringLiteral("brushPickerGrid")) : nullptr;
    if (!grid) {
        return false;
    }
    for (int i = 0; i < grid->count(); ++i) {
        if (grid->item(i)->toolTip() == name) {
            picker->choosePresetForTest(i);
            return true;
        }
    }
    return false;
}

} // namespace

class BristleBrushesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void brushPanelKeepsSetsOnTheLeft();
    void bristleTipPageShowsControls();
    void bristleControlsWriteTheModel();
    void bristleDisablesHardness();
    void bristlePresetApplies();
    void roundPresetClearsBristle();
    void bristleRoundTrip();
    void legacySettingsKeepSmoothTip();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void BristleBrushesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void BristleBrushesTest::brushPanelKeepsSetsOnTheLeft()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* panel = window->findChild<pictura::BrushPanel*>(QStringLiteral("brushPanel"));
    QVERIFY(panel);
    auto* body = panel->findChild<QHBoxLayout*>(QStringLiteral("brushPanelBody"));
    auto* sets = panel->findChild<QListWidget*>(QStringLiteral("brushOptionSets"));
    auto* pages = panel->findChild<QStackedWidget*>(QStringLiteral("brushDynamicsPages"));
    QVERIFY(body && sets && pages);
    QCOMPARE(body->count(), 2);
    QVERIFY(body->itemAt(0)->layout());
    QVERIFY(body->itemAt(0)->layout()->indexOf(sets) >= 0);
    QCOMPARE(body->itemAt(1)->widget(), pages);
}

void BristleBrushesTest::bristleTipPageShowsControls()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* sets = window->findChild<QListWidget*>(QStringLiteral("brushOptionSets"));
    auto* pages = window->findChild<QStackedWidget*>(QStringLiteral("brushDynamicsPages"));
    QVERIFY(sets && pages);
    sets->setCurrentRow(0);
    QCOMPARE(pages->currentIndex(), 0);
    QWidget* tipPage = pages->widget(0);
    QVERIFY(tipPage);
    auto* enabled = tipPage->findChild<QCheckBox*>(QStringLiteral("brushBristleEnabled"));
    QVERIFY(enabled);
    QVERIFY(!enabled->isHidden());
    QCOMPARE(pages->currentWidget(), tipPage);
}

void BristleBrushesTest::bristleControlsWriteTheModel()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* paint = brushOptions(*window);
    QVERIFY(paint);

    auto* enabled = field<QCheckBox>(*window, "brushBristleEnabled");
    auto* shape = field<QComboBox>(*window, "brushBristleShape");
    auto* density = field<pictura::NumericField>(*window, "brushBristleDensity");
    auto* length = field<pictura::NumericField>(*window, "brushBristleLength");
    auto* thickness = field<pictura::NumericField>(*window, "brushBristleThickness");
    auto* stiffness = field<pictura::NumericField>(*window, "brushBristleStiffness");
    QVERIFY(enabled && shape && density && length && thickness && stiffness);

    enabled->setChecked(true);
    shape->setCurrentIndex(4);
    typeInto(density, QStringLiteral("70"));
    typeInto(length, QStringLiteral("65"));
    typeInto(thickness, QStringLiteral("45"));
    typeInto(stiffness, QStringLiteral("30"));

    QVERIFY(paint->bristleEnabled());
    QCOMPARE(paint->bristleShape(), 4);
    QCOMPARE(paint->bristleDensity(), 70);
    QCOMPARE(paint->bristleLength(), 65);
    QCOMPARE(paint->bristleThickness(), 45);
    QCOMPARE(paint->bristleStiffness(), 30);

    auto* panel = window->findChild<pictura::BrushPanel*>(QStringLiteral("brushPanel"));
    QVERIFY(panel && !panel->previewForTest().isNull());
}

void BristleBrushesTest::bristleDisablesHardness()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* enabled = field<QCheckBox>(*window, "brushBristleEnabled");
    auto* hardness = field<pictura::NumericField>(*window, "brushPanelHardness");
    auto* roundness = field<pictura::NumericField>(*window, "brushPanelRoundness");
    auto* density = field<pictura::NumericField>(*window, "brushBristleDensity");
    QVERIFY(enabled && hardness && roundness && density);

    enabled->setChecked(true);
    QVERIFY(!hardness->isEnabled());
    QVERIFY(!roundness->isEnabled());
    QVERIFY(density->isEnabled());

    enabled->setChecked(false);
    QVERIFY(hardness->isEnabled());
    QVERIFY(roundness->isEnabled());
    QVERIFY(!density->isEnabled());
}

void BristleBrushesTest::bristlePresetApplies()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seedImage(), QStringLiteral("bristle_preset"));
    QVERIFY2(f.ok(), "bristle fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.frame);
    QVERIFY(picker);
    auto* paint = brushOptions(f.frame);
    QVERIFY(paint);

    QVERIFY(choosePreset(picker, QStringLiteral("Bristle Flat 40")));
    QVERIFY(paint->bristleEnabled());
    QCOMPARE(paint->bristleShape(), 3);
    QCOMPARE(paint->size(), 40);
    QCOMPARE(paint->bristleDensity(), 60);
    QCOMPARE(paint->bristleLength(), 50);
    QCOMPARE(paint->bristleThickness(), 45);
    QCOMPARE(paint->bristleStiffness(), 55);
}

void BristleBrushesTest::roundPresetClearsBristle()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seedImage(), QStringLiteral("bristle_round"));
    QVERIFY2(f.ok(), "bristle fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.frame);
    QVERIFY(picker);
    auto* paint = brushOptions(f.frame);
    QVERIFY(paint);

    QVERIFY(choosePreset(picker, QStringLiteral("Bristle Round 30")));
    QVERIFY(paint->bristleEnabled());
    QVERIFY(choosePreset(picker, QStringLiteral("Soft Round 5")));
    QVERIFY(!paint->bristleEnabled());
}

void BristleBrushesTest::bristleRoundTrip()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* tools = window->findChild<pictura::ToolController*>();
    auto* paint = brushOptions(*window);
    QVERIFY(tools && paint);

    paint->setBristleEnabled(true);
    paint->setBristleShape(5);
    paint->setBristleDensity(77);
    paint->setBristleLength(66);
    paint->setBristleThickness(44);
    paint->setBristleStiffness(22);
    const QJsonObject json = pictura::toolOptionsToJson(*tools);

    paint->setBristleEnabled(false);
    paint->setBristleShape(0);
    paint->setBristleDensity(0);
    paint->setBristleLength(0);
    paint->setBristleThickness(0);
    paint->setBristleStiffness(0);

    pictura::applyToolOptionsJson(*tools, json);
    QVERIFY(paint->bristleEnabled());
    QCOMPARE(paint->bristleShape(), 5);
    QCOMPARE(paint->bristleDensity(), 77);
    QCOMPARE(paint->bristleLength(), 66);
    QCOMPARE(paint->bristleThickness(), 44);
    QCOMPARE(paint->bristleStiffness(), 22);
}

void BristleBrushesTest::legacySettingsKeepSmoothTip()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    auto* tools = window->findChild<pictura::ToolController*>();
    auto* paint = brushOptions(*window);
    QVERIFY(tools && paint);

    paint->setBristleEnabled(true);
    paint->setBristleShape(4);
    paint->setBristleDensity(90);
    QJsonObject json = pictura::toolOptionsToJson(*tools);
    QJsonObject brush = json.value(QStringLiteral("brush")).toObject();
    for (const char* key : {"bristleEnabled", "bristleShape", "bristleDensity", "bristleLength",
                            "bristleThickness", "bristleStiffness"}) {
        brush.remove(QString::fromLatin1(key));
    }
    json.insert(QStringLiteral("brush"), brush);

    pictura::applyToolOptionsJson(*tools, json);
    QVERIFY(!paint->bristleEnabled());
    QCOMPARE(paint->bristleShape(), 0);
    QCOMPARE(paint->bristleDensity(), 50);
    QCOMPARE(paint->bristleLength(), 50);
    QCOMPARE(paint->bristleThickness(), 50);
    QCOMPARE(paint->bristleStiffness(), 50);
}

QTEST_MAIN(BristleBrushesTest)
#include "tst_bristle_brushes.moc"
