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
    const int brushSize = tools->optionsFor<PaintOptions>(ToolId::Brush)->size();
    const int eraserMode = tools->optionsFor<EraserGroup>(ToolId::Eraser)->get().mode;
    QJsonObject json;
    json.insert(QStringLiteral("type"), QJsonObject{{QStringLiteral("size"), 99999.0}});
    json.insert(QStringLiteral("brush"), 5);
    json.insert(QStringLiteral("eraser"), QJsonObject{{QStringLiteral("mode"), QStringLiteral("x")}});
    json.insert(QStringLiteral("unknownTool"), QJsonObject{{QStringLiteral("a"), 1}});
    applyToolOptionsJson(*tools, json);
    QCOMPARE(tools->optionsFor<TypeGroup>(ToolId::HorizontalType)->get().size, 1296.0);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->size(), brushSize);
    QCOMPARE(tools->optionsFor<EraserGroup>(ToolId::Eraser)->get().mode, eraserMode);
}

QTEST_MAIN(ToolOptionsPersistenceTest)
#include "tst_tool_options_persistence.moc"
