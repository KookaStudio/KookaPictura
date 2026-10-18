#include <QtTest/QtTest>

#include <QtCore/QPoint>
#include <QtGui/QColor>
#include <QtWidgets/QToolButton>

#include "panels/color_panel.h"
#include "panels/swatches_panel.h"

#include "qt_test_support.h"

class SwatchesPanelTest : public QObject {
    Q_OBJECT

private slots:
    void gridReflowsToWidth();
    void indexAtMapsPointToSwatch();
    void clickModifiersSetColorsAndDelete();
    void footerNewAddsSwatch();
    void newSwatchUsesGeneratedName();
    void freshStoreUsesDefaultPalette();
    void addedSwatchesSurviveARestart();
    void emptyPanelSurvivesARestart();
    void resetRestoresDefaults();
};

void SwatchesPanelTest::gridReflowsToWidth()
{
    pictura::SwatchGrid grid;
    QCOMPARE(grid.swatches().size(), 48);
    QCOMPARE(grid.swatches().at(0).color.name(), QStringLiteral("#ff0000"));
    QCOMPARE(grid.swatches().at(35).name, QStringLiteral("Dark Rose"));
    QCOMPARE(grid.swatches().at(35).color.name(), QColor::fromHsv(330, 255, 160).name());
    QCOMPARE(grid.columnsForWidth(20), 1);
    QCOMPARE(grid.columnsForWidth(3 + 5 * 18), 5);
    QVERIFY2(grid.heightFor(20) > grid.heightFor(3 + 5 * 18), "narrower needs more rows");
}

void SwatchesPanelTest::indexAtMapsPointToSwatch()
{
    pictura::SwatchGrid grid;
    grid.resize(3 + 5 * 18, grid.heightFor(3 + 5 * 18));
    QCOMPARE(grid.indexAt(QPoint(3, 3)), 0);
    QCOMPARE(grid.indexAt(QPoint(2 + 4 * 18 + 3, 3)), 4);
    QCOMPARE(grid.indexAt(QPoint(3, 2 + 18 + 3)), 5);
    QCOMPARE(grid.indexAt(QPoint(3, 2 + 10 * 18)), -1);
}

void SwatchesPanelTest::clickModifiersSetColorsAndDelete()
{
    pictura::ColorState state;
    pictura::SwatchesPanel panel(&state);
    pictura::SwatchGrid* grid = panel.grid();
    grid->resize(3 + 5 * 18, grid->heightFor(3 + 5 * 18));
    const QColor first = grid->swatches().at(0).color;

    QTest::mouseClick(grid, Qt::LeftButton, Qt::NoModifier, QPoint(3, 3));
    QCOMPARE(state.foreground().name(), first.name());

    QTest::mouseClick(grid, Qt::LeftButton, Qt::ControlModifier, QPoint(3, 3));
    QCOMPARE(state.background().name(), first.name());

    const int before = grid->swatches().size();
    QTest::mouseClick(grid, Qt::LeftButton, Qt::AltModifier, QPoint(3, 3));
    QCOMPARE(grid->swatches().size(), before - 1);
}

void SwatchesPanelTest::footerNewAddsSwatch()
{
    pictura::ColorState state;
    state.setForeground(QColor(1, 2, 3));
    pictura::SwatchesPanel panel(&state);
    pictura::SwatchGrid* grid = panel.grid();
    const int before = grid->swatches().size();

    panel.addSwatchFromForeground();
    QCOMPARE(grid->swatches().size(), before + 1);
    QCOMPARE(grid->swatches().last().color.name(), QStringLiteral("#010203"));
    QVERIFY2(panel.findChild<QToolButton*>(QStringLiteral("swatchesNew")) != nullptr,
             "footer New button present");
}

void SwatchesPanelTest::newSwatchUsesGeneratedName()
{
    pictura::ColorState state;
    pictura::SwatchesPanel panel(&state);
    pictura::SwatchGrid* grid = panel.grid();

    panel.addSwatchFromForeground();
    QCOMPARE(grid->swatches().last().name, QStringLiteral("Swatch 1"));
    panel.addSwatchFromForeground();
    QCOMPARE(grid->swatches().last().name, QStringLiteral("Swatch 2"));
}

void SwatchesPanelTest::freshStoreUsesDefaultPalette()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    auto window = pictura::test::makeMainWindow();
    auto* panel = window->findChild<pictura::SwatchesPanel*>(QStringLiteral("swatchesPanel"));
    QVERIFY(panel);
    QCOMPARE(panel->grid()->swatches().size(), 48);
}

void SwatchesPanelTest::addedSwatchesSurviveARestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    {
        auto window = pictura::test::makeMainWindow();
        auto* panel = window->findChild<pictura::SwatchesPanel*>(QStringLiteral("swatchesPanel"));
        auto* colors = window->findChild<pictura::ColorState*>();
        QVERIFY(panel && colors);
        colors->setForeground(QColor(1, 2, 3));
        panel->addSwatchFromForeground();
        QCOMPARE(panel->grid()->swatches().size(), 49);
        QCOMPARE(panel->grid()->swatches().last().name, QStringLiteral("Swatch 1"));
    }

    {
        auto window = pictura::test::makeMainWindow();
        auto* panel = window->findChild<pictura::SwatchesPanel*>(QStringLiteral("swatchesPanel"));
        QVERIFY(panel);
        QCOMPARE(panel->grid()->swatches().size(), 49);
        QCOMPARE(panel->grid()->swatches().last().color, QColor(1, 2, 3));
        QCOMPARE(panel->grid()->swatches().last().name, QStringLiteral("Swatch 1"));
    }
}

void SwatchesPanelTest::emptyPanelSurvivesARestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    {
        auto window = pictura::test::makeMainWindow();
        auto* panel = window->findChild<pictura::SwatchesPanel*>(QStringLiteral("swatchesPanel"));
        QVERIFY(panel);
        panel->grid()->setSwatches({});
        QCOMPARE(panel->grid()->swatches().size(), 0);
    }

    {
        auto window = pictura::test::makeMainWindow();
        auto* panel = window->findChild<pictura::SwatchesPanel*>(QStringLiteral("swatchesPanel"));
        QVERIFY(panel);
        QCOMPARE(panel->grid()->swatches().size(), 0);
    }
}

void SwatchesPanelTest::resetRestoresDefaults()
{
    pictura::ColorState state;
    pictura::SwatchesPanel panel(&state);
    pictura::SwatchGrid* grid = panel.grid();

    panel.addSwatchFromForeground();
    QVERIFY(grid->removeSwatch(0));
    QCOMPARE(grid->swatches().size(), 48);

    panel.resetSwatches();
    QCOMPARE(grid->swatches().size(), 48);
    QCOMPARE(grid->swatches().at(0).color.name(), QStringLiteral("#ff0000"));
}

QTEST_MAIN(SwatchesPanelTest)
#include "tst_swatches_panel.moc"
