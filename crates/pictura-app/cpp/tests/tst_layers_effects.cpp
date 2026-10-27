// The Layers panel's nested layer-effects rows (#960): an "Effects" group and
// one row per effect under a styled layer, their eyes, the double-click that
// opens the Layer Style dialog, and the CS6 layer-style row menu.

#include <QtTest/QtTest>

#include "frame.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

#include <functional>

#include "qt_test_support.h"

class LayersEffectsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void nestedEffectRows();
    void effectsEyeTogglesMaster();
    void effectEyeTogglesOne();
    void newStyleStartsExpanded();
    void unstyledLayerHasNoGroup();
    void doubleClickEffectOpensItsPage();
    void doubleClickBadgeOpensBlendingOptions();
    void rowMenuListsLayerStyleCommands();
    void disableLayerEffectsIsOneState();
    void effectOrderingMatchesTheDialog();
    void adjustmentRowsShowNoFxBadge();

private:
    // A 10x10 white square lifted onto layer "1" over a white Background.
    void setupSquare();
    // Give layer "1" a drop shadow and refresh the panel.
    void addDropShadow();
    double styleValue(const char* key) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayersEffectsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayersEffectsTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

void LayersEffectsTest::setupSquare()
{
    QVERIFY(window_->newDocument(QStringLiteral("Effects"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    view_->select_rect(10, 10, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    panel_->refresh();
    QCoreApplication::processEvents();
}

void LayersEffectsTest::addDropShadow()
{
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    panel_->refresh();
    QCoreApplication::processEvents();
}

double LayersEffectsTest::styleValue(const char* key) const
{
    return pictura::layer_style_value(*view_, QStringLiteral("1"), QLatin1String(key));
}

void LayersEffectsTest::nestedEffectRows()
{
    setupSquare();
    addDropShadow();

    QVERIFY(panel_->rowExistsForTest(QStringLiteral("1/@fx")));
    QCOMPARE(panel_->rowKindForTest(QStringLiteral("1/@fx")), QStringLiteral("layer-effects"));
    QCOMPARE(panel_->rowNameForTest(QStringLiteral("1/@fx")), QStringLiteral("Effects"));
    QVERIFY(panel_->rowExistsForTest(QStringLiteral("1/@fx/dropShadow")));
    QCOMPARE(panel_->rowKindForTest(QStringLiteral("1/@fx/dropShadow")),
             QStringLiteral("layer-effect"));
    QCOMPARE(panel_->rowNameForTest(QStringLiteral("1/@fx/dropShadow")),
             QStringLiteral("Drop Shadow"));
    QVERIFY(panel_->rowVisibleForTest(QStringLiteral("1/@fx/dropShadow")));
}

void LayersEffectsTest::effectsEyeTogglesMaster()
{
    setupSquare();
    addDropShadow();
    QCOMPARE(styleValue("fx.visible"), 1.0);
    const int states = view_->history_count();

    QVERIFY(panel_->clickRowEyeForTest(QStringLiteral("1/@fx")));
    QCoreApplication::processEvents();
    QCOMPARE(styleValue("fx.visible"), 0.0);
    QCOMPARE(view_->history_count(), states + 1);
    QVERIFY(!panel_->rowVisibleForTest(QStringLiteral("1/@fx")));

    QVERIFY(panel_->clickRowEyeForTest(QStringLiteral("1/@fx")));
    QCoreApplication::processEvents();
    QCOMPARE(styleValue("fx.visible"), 1.0);
}

void LayersEffectsTest::effectEyeTogglesOne()
{
    setupSquare();
    addDropShadow();
    QCOMPARE(styleValue("dropShadow.on"), 1.0);
    const int states = view_->history_count();

    QVERIFY(panel_->clickRowEyeForTest(QStringLiteral("1/@fx/dropShadow")));
    QCoreApplication::processEvents();
    QCOMPARE(styleValue("dropShadow.on"), 0.0);
    QCOMPARE(view_->history_count(), states + 1);
    // The effect stays listed so it can be switched back on.
    QVERIFY(panel_->rowExistsForTest(QStringLiteral("1/@fx/dropShadow")));
    QVERIFY(!panel_->rowVisibleForTest(QStringLiteral("1/@fx/dropShadow")));
}

void LayersEffectsTest::newStyleStartsExpanded()
{
    setupSquare();
    QVERIFY(!panel_->rowExistsForTest(QStringLiteral("1/@fx")));
    addDropShadow();
    QVERIFY(panel_->rowExistsForTest(QStringLiteral("1/@fx")));
    QVERIFY(panel_->rowExpandedForTest(QStringLiteral("1")));
    QVERIFY(panel_->rowExpandedForTest(QStringLiteral("1/@fx")));
}

void LayersEffectsTest::unstyledLayerHasNoGroup()
{
    setupSquare();
    QVERIFY(!panel_->rowExistsForTest(QStringLiteral("1/@fx")));
}

void LayersEffectsTest::doubleClickEffectOpensItsPage()
{
    setupSquare();
    addDropShadow();
    QString opened = QStringLiteral("unset");
    panel_->setDialogDriverForTest([&](QDialog& dialog) {
        if (auto* style = qobject_cast<pictura::LayerStyleDialog*>(&dialog)) {
            opened = style->rowKeys().value(style->currentPage());
        }
        return false;
    });
    QVERIFY(panel_->doubleClickRowCenterForTest(QStringLiteral("1/@fx/dropShadow")));
    panel_->setDialogDriverForTest(nullptr);
    QCOMPARE(opened, QStringLiteral("dropShadow"));
}

void LayersEffectsTest::doubleClickBadgeOpensBlendingOptions()
{
    setupSquare();
    addDropShadow();
    int page = -1;
    QString key = QStringLiteral("unset");
    panel_->setDialogDriverForTest([&](QDialog& dialog) {
        if (auto* style = qobject_cast<pictura::LayerStyleDialog*>(&dialog)) {
            page = style->currentPage();
            key = style->rowKeys().value(page);
        }
        return false;
    });
    QVERIFY(panel_->doubleClickFxBadgeForTest(QStringLiteral("1")));
    panel_->setDialogDriverForTest(nullptr);
    QCOMPARE(page, 0);
    QCOMPARE(key, QString());
}

void LayersEffectsTest::rowMenuListsLayerStyleCommands()
{
    setupSquare();
    addDropShadow();
    const QStringList texts = panel_->layerStyleMenuTextsForTest(QStringLiteral("1"));
    QVERIFY(texts.contains(QStringLiteral("Disable Layer Effects")));
    QVERIFY(texts.contains(QStringLiteral("Blending Options…")));
    for (const QString& title :
         {QStringLiteral("Bevel & Emboss…"), QStringLiteral("Stroke…"),
          QStringLiteral("Inner Shadow…"), QStringLiteral("Inner Glow…"),
          QStringLiteral("Satin…"), QStringLiteral("Color Overlay…"),
          QStringLiteral("Gradient Overlay…"),
          QStringLiteral("Pattern Overlay…"), QStringLiteral("Outer Glow…"),
          QStringLiteral("Drop Shadow…")}) {
        QVERIFY2(texts.contains(title), qPrintable(title));
    }
    QVERIFY(texts.contains(QStringLiteral("Copy Layer Style")));
    QVERIFY(texts.contains(QStringLiteral("Paste Layer Style")));
    QVERIFY(texts.contains(QStringLiteral("Clear Layer Style")));
    QVERIFY(texts.contains(QStringLiteral("Global Light…")));
    QVERIFY(texts.contains(QStringLiteral("Create Layer")));
    QVERIFY(texts.contains(QStringLiteral("Hide All Effects")));
    QVERIFY(texts.contains(QStringLiteral("Scale Effects…")));
    QVERIFY(!panel_->layerStyleMenuEnabledForTest(QStringLiteral("1"),
                                                  QStringLiteral("Global Light…")));
    QVERIFY(!panel_->layerStyleMenuEnabledForTest(QStringLiteral("1"),
                                                  QStringLiteral("Create Layer")));
    QVERIFY(panel_->layerStyleMenuEnabledForTest(QStringLiteral("1"),
                                                 QStringLiteral("Copy Layer Style")));
}

void LayersEffectsTest::disableLayerEffectsIsOneState()
{
    setupSquare();
    addDropShadow();
    const int states = view_->history_count();
    QVERIFY(panel_->triggerLayerStyleMenuActionForTest(QStringLiteral("1"),
                                                       QStringLiteral("Disable Layer Effects")));
    QCoreApplication::processEvents();
    QCOMPARE(styleValue("fx.visible"), 0.0);
    QCOMPARE(view_->history_count(), states + 1);
}

void LayersEffectsTest::effectOrderingMatchesTheDialog()
{
    // The panel rows take their order from the engine's effect table and the
    // menu from the dialog's; the two must stay identical.
    QCOMPARE(pictura::layer_style_effect_names(), pictura::LayerStyleDialog::effectKeys());
}

// The fx badge tracks the layer's present effects, not the style block: an
// adjustment or fill row paints no badge, and a real effect object brings it
// back whether it is on or off (#983).
void LayersEffectsTest::adjustmentRowsShowNoFxBadge()
{
    setupSquare();

    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    QCoreApplication::processEvents();
    panel_->refresh();
    const QString adjustment = panel_->currentPath();
    QVERIFY2(!adjustment.isEmpty(), "adjustment row present");
    QVERIFY2(panel_->rowHasAdjustmentForTest(adjustment), "row is an adjustment");
    QVERIFY2(!panel_->rowHasStyleForTest(adjustment), "adjustment carries no style");
    QVERIFY2(panel_->rowFxRectForTest(adjustment).isEmpty(),
             "an adjustment row paints no fx badge");

    // The Alt-click-all-effects gesture needs an fx region; with none it is inert.
    const int before = view_->history_count();
    panel_->altClickRowFxForTest(adjustment);
    QCOMPARE(view_->history_count(), before);

    // A fill row is content too, so it also paints no fx badge.
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY2(!fill.isEmpty(), "fill row present");
    panel_->refresh();
    QVERIFY2(panel_->rowHasAdjustmentForTest(fill), "fill row reports adjustment content");
    QVERIFY2(panel_->rowFxRectForTest(fill).isEmpty(), "a fill row paints no fx badge");

    // A real layer style on the adjustment brings the badge back.
    QVERIFY(pictura::layer_style_set(*view_, adjustment, QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowHasStyleForTest(adjustment), "styled adjustment reports a style");
    QVERIFY2(!panel_->rowFxRectForTest(adjustment).isEmpty(),
             "a styled adjustment paints the fx badge");

    // Switching the effect off leaves its object in the style block, so it is
    // still a present effect and the badge stays (present means on or off).
    QVERIFY(pictura::layer_style_set(*view_, adjustment, QStringLiteral("colorOverlay.on"), 0.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowFxRectForTest(adjustment).isEmpty(),
             "an off effect is still present, so the badge stays");
}

QTEST_MAIN(LayersEffectsTest)
#include "tst_layers_effects.moc"
