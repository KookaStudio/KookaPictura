// Tool options survive a restart: the session store carries every tool's
// options-bar settings, and bad entries fall back to the defaults.

#include <QtTest/QtTest>

#include "options_bar.h"
#include "panels/numeric_field.h"
#include "tool_options_json.h"
#include "tools.h"

#include <QtCore/QJsonObject>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLineEdit>

#include <memory>

#include "qt_test_support.h"

class ToolOptionsPersistenceTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void optionsSurviveARestart();
    void quickSelectionToleranceFromOptionsBarPersists();
    void quickSelectionToleranceTypedWithoutEnterPersists();
    void badEntriesKeepDefaults();
    void restoreDoesNotNotify();
    void missingFieldsRestoreDefaults();
    void patchMergesAndNotifies();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ToolOptionsPersistenceTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void ToolOptionsPersistenceTest::optionsSurviveARestart()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        auto* tools = window->findChild<ToolController*>();
        QVERIFY(tools);
        MoveGroup* move = tools->optionsFor<MoveGroup>(ToolId::Move);
        MoveOptions m = move->get();
        m.showTransformControls = false;
        move->set(m);
        TypeGroup* type = tools->optionsFor<TypeGroup>(ToolId::HorizontalType);
        TypeOptions t = type->get();
        t.size = 48.0;
        type->set(t);
        PaintOptions* paint = tools->optionsFor<PaintOptions>(ToolId::Brush);
        paint->setSize(77);
        paint->setOpacity(40);
        paint->setGrain(75);
        paint->setPenTip(true);
        paint->setAliased(true);
        paint->setWetEdges(true);
        paint->setAirbrush(true);
        paint->setPressureSize(true);
        paint->setPressureOpacity(true);
        tools->optionsFor<SelectionOptions>(ToolId::Marquee)->setFeather(5.5);
        tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->setTolerance(77);
        RetouchGroup* blur = tools->optionsFor<RetouchGroup>(ToolId::Blur);
        RetouchOptions r = blur->get();
        r.strength = 80;
        blur->set(r);
        ToneGroup* burn = tools->optionsFor<ToneGroup>(ToolId::Burn);
        ToneOptions b = burn->get();
        b.amount = 20;
        burn->set(b);
        window->saveSession();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    QVERIFY(!tools->optionsFor<MoveGroup>(ToolId::Move)->get().showTransformControls);
    // The options bar starts from the restored values.
    auto* transform = window->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(transform && !transform->isChecked());
    QCOMPARE(tools->optionsFor<TypeGroup>(ToolId::HorizontalType)->get().size, 48.0);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->size(), 77);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->opacity(), 40);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->grain(), 75);
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->penTip());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->aliased());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->wetEdges());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->airbrush());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->pressureSize());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->pressureOpacity());
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::Marquee)->feather(), 5.5);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
    QCOMPARE(tools->optionsFor<RetouchGroup>(ToolId::Blur)->get().strength, 80);
    QCOMPARE(tools->optionsFor<ToneGroup>(ToolId::Burn)->get().amount, 20);
    // Each retouch and toning tool keeps its own options.
    QCOMPARE(tools->optionsFor<RetouchGroup>(ToolId::Sharpen)->get().strength,
             RetouchOptions{}.strength);
    QCOMPARE(tools->optionsFor<ToneGroup>(ToolId::Dodge)->get().amount, ToneOptions{}.amount);
}

void ToolOptionsPersistenceTest::quickSelectionToleranceFromOptionsBarPersists()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        window->show();
        auto* tools = window->findChild<ToolController*>();
        auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
        QVERIFY(tools && bar);
        window->setActiveTool(ToolId::QuickSelection);
        QWidget* page = bar->pageForTest(ToolId::QuickSelection);
        QVERIFY(page);
        auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
        QVERIFY(spin);
        auto* edit = spin->findChild<QLineEdit*>();
        QVERIFY(edit);
        edit->setFocus();
        edit->selectAll();
        QTest::keyClicks(edit, QStringLiteral("77"));
        QTest::keyClick(edit, Qt::Key_Return);
        QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
        window->saveSession();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(tools && bar);
    window->setActiveTool(ToolId::QuickSelection);
    QWidget* page = bar->pageForTest(ToolId::QuickSelection);
    QVERIFY(page);
    auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
    QVERIFY(spin);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
    QCOMPARE(spin->value(), 77.0);
}

void ToolOptionsPersistenceTest::quickSelectionToleranceTypedWithoutEnterPersists()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        window->show();
        auto* tools = window->findChild<ToolController*>();
        auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
        QVERIFY(tools && bar);
        window->setActiveTool(ToolId::QuickSelection);
        QWidget* page = bar->pageForTest(ToolId::QuickSelection);
        QVERIFY(page);
        auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
        QVERIFY(spin);
        auto* edit = spin->findChild<QLineEdit*>();
        QVERIFY(edit);
        edit->setFocus();
        edit->selectAll();
        QTest::keyClicks(edit, QStringLiteral("88"));
        window->close();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(tools && bar);
    window->setActiveTool(ToolId::QuickSelection);
    QWidget* page = bar->pageForTest(ToolId::QuickSelection);
    QVERIFY(page);
    auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
    QVERIFY(spin);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 88);
    QCOMPARE(spin->value(), 88.0);
}

void ToolOptionsPersistenceTest::badEntriesKeepDefaults()
{
    using namespace pictura;
    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    const int brushDefault = PaintOptions{}.size();
    QJsonObject json;
    json.insert(QStringLiteral("type"), QJsonObject{{QStringLiteral("size"), 99999.0}});
    json.insert(QStringLiteral("brush"), 5);
    json.insert(QStringLiteral("eraser"), QJsonObject{{QStringLiteral("mode"), QStringLiteral("x")}});
    json.insert(QStringLiteral("unknownTool"), QJsonObject{{QStringLiteral("a"), 1}});
    applyToolOptionsJson(*tools, json);
    QCOMPARE(tools->optionsFor<TypeGroup>(ToolId::HorizontalType)->get().size, 1296.0);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->size(), brushDefault);
    QCOMPARE(tools->optionsFor<EraserGroup>(ToolId::Eraser)->get().mode, EraserOptions{}.mode);
}

void ToolOptionsPersistenceTest::restoreDoesNotNotify()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    SelectionOptions* marquee = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
    SelectionOptions* quick = tools.optionsFor<SelectionOptions>(ToolId::QuickSelection);
    CropOptions* crop = tools.optionsFor<CropOptions>(ToolId::Crop);
    QVERIFY(paint && marquee && quick && crop);

    int brushSignals = 0;
    paint->sizeChanged = [&brushSignals](int) { ++brushSignals; };
    paint->tipChanged = [&brushSignals]() { ++brushSignals; };
    paint->paintSettingsChanged = [&brushSignals]() { ++brushSignals; };
    int selectionSignals = 0;
    marquee->combineModeChanged = [&selectionSignals](SelectionMode) { ++selectionSignals; };
    marquee->magneticWidthChanged = [&selectionSignals](int) { ++selectionSignals; };
    quick->combineModeChanged = [&selectionSignals](SelectionMode) { ++selectionSignals; };
    quick->magneticWidthChanged = [&selectionSignals](int) { ++selectionSignals; };
    int cropSignals = 0;
    crop->optionsChanged = [&cropSignals]() { ++cropSignals; };

    applyToolOptionsJson(
        tools,
        QJsonObject{
            {QStringLiteral("brush"),
             QJsonObject{{QStringLiteral("size"), 99},
                         {QStringLiteral("hardness"), 10},
                         {QStringLiteral("opacity"), 50},
                         {QStringLiteral("flow"), 60},
                         {QStringLiteral("mode"), QStringLiteral("multiply")},
                         {QStringLiteral("roundness"), 40},
                         {QStringLiteral("tipAngle"), 30},
                         {QStringLiteral("spacing"), 10},
                         {QStringLiteral("grain"), 20},
                         {QStringLiteral("flipX"), true},
                         {QStringLiteral("flipY"), true},
                         {QStringLiteral("penTip"), true},
                         {QStringLiteral("aliased"), true},
                         {QStringLiteral("wetEdges"), true},
                         {QStringLiteral("airbrush"), true},
                         {QStringLiteral("pressureSize"), true},
                         {QStringLiteral("pressureOpacity"), true},
                         {QStringLiteral("scatter"), 10},
                         {QStringLiteral("count"), 3},
                         {QStringLiteral("sizeJitter"), 10},
                         {QStringLiteral("angleJitter"), 20},
                         {QStringLiteral("roundnessJitter"), 30}}},
            {QStringLiteral("selection"),
             QJsonObject{{QStringLiteral("combineMode"), 1},
                         {QStringLiteral("magneticWidth"), 40}}},
            {QStringLiteral("quickSelection"),
             QJsonObject{{QStringLiteral("combineMode"), 2},
                         {QStringLiteral("magneticWidth"), 50}}},
            {QStringLiteral("crop"),
             QJsonObject{{QStringLiteral("shieldOpacity"), 40},
                         {QStringLiteral("overlayShow"), 1}}}});

    QCOMPARE(brushSignals, 0);
    QCOMPARE(selectionSignals, 0);
    QCOMPARE(cropSignals, 0);
    QCOMPARE(paint->size(), 99);
    QCOMPARE(paint->hardness(), 10);
    QCOMPARE(paint->opacity(), 50);
    QCOMPARE(paint->flow(), 60);
    QCOMPARE(paint->mode(), QStringLiteral("multiply"));
    QVERIFY(paint->airbrush());
    QVERIFY(paint->pressureSize());
    QVERIFY(paint->pressureOpacity());
    QCOMPARE(int(marquee->combineMode()), 1);
    QCOMPARE(marquee->magneticWidth(), 40);
    QCOMPARE(int(quick->combineMode()), 2);
    QCOMPARE(quick->magneticWidth(), 50);
    QCOMPARE(crop->settings().shieldOpacity, 40);
    QCOMPARE(crop->settings().overlayShow, int(CropSettings::AlwaysShow));
}

void ToolOptionsPersistenceTest::missingFieldsRestoreDefaults()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    SelectionOptions* marquee = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
    MoveGroup* move = tools.optionsFor<MoveGroup>(ToolId::Move);
    QVERIFY(paint && marquee && move);

    paint->setSize(77);
    paint->setHardness(10);
    paint->setOpacity(40);
    marquee->setTolerance(200);
    marquee->setMagneticWidth(40);
    MoveOptions options = move->get();
    options.autoSelectOn = true;
    move->set(options);

    applyToolOptionsJson(
        tools,
        QJsonObject{
            {QStringLiteral("brush"), QJsonObject{{QStringLiteral("size"), 33}}},
            {QStringLiteral("selection"), QJsonObject{{QStringLiteral("tolerance"), 100}}},
            {QStringLiteral("move"), QJsonObject{{QStringLiteral("alignTo"), 1}}}});

    QCOMPARE(paint->size(), 33);
    QCOMPARE(paint->hardness(), PaintOptions{}.hardness());
    QCOMPARE(paint->opacity(), PaintOptions{}.opacity());
    QCOMPARE(marquee->tolerance(), 100);
    QCOMPARE(marquee->magneticWidth(), SelectionOptions{}.magneticWidth());
    QVERIFY(!move->get().autoSelectOn);
    QCOMPARE(move->get().alignTo, 1);
}

void ToolOptionsPersistenceTest::patchMergesAndNotifies()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    QVERIFY(paint);
    paint->setSize(77);
    paint->setOpacity(40);

    int notified = 0;
    paint->sizeChanged = [&notified](int) { ++notified; };
    paint->paintSettingsChanged = [&notified]() { ++notified; };

    patchToolOptionsJson(tools,
                         QJsonObject{{QStringLiteral("brush"),
                                      QJsonObject{{QStringLiteral("size"), 123}}}});
    QCOMPARE(paint->size(), 123);
    QCOMPARE(paint->opacity(), 40);
    QCOMPARE(notified, 1);
}

QTEST_MAIN(ToolOptionsPersistenceTest)
#include "tst_tool_options_persistence.moc"
