// The Layers panel's nested layer-effects rows (#960): an "Effects" group and
// one row per effect under a styled layer, their eyes, the double-click that
// opens the Layer Style dialog, and the CS6 layer-style row menu.

#include <QtTest/QtTest>

#include "frame.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "api/layers_bridge.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QDialog>
#include <QtWidgets/QTreeView>

#include <functional>

#include "qt_test_support.h"

namespace {
QModelIndex indexForPath(QAbstractItemModel* model, const QString& path,
                         const QModelIndex& parent = QModelIndex())
{
    if (!model) {
        return {};
    }
    for (int row = 0; row < model->rowCount(parent); ++row) {
        const QModelIndex index = model->index(row, 0, parent);
        if (index.data(pictura::PathRole).toString() == path) {
            return index;
        }
        const QModelIndex child = indexForPath(model, path, index);
        if (child.isValid()) {
            return child;
        }
    }
    return {};
}
} // namespace

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
    void effectsListToggleSitsOnTheLayerRow();
    void closingTheListHidesTheGroup();
    void nestedRowsFormOneCell();
    void syntheticRowsHaveNoThumbnail();
    void nestedEyeAlignsWithImageThumb();
    void clickingEffectRowSelectsLayer();
    void hiddenEffectShowsFadedEye();
    void groupKeepsNestedLayersChevron();
    void shapeLayerWithEffectsShowsTheListToggle();
    void syntheticRowsAreNotSelectable();
    void syntheticRowsDoNotDrag();

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

// The Effects group row's card is a square (1:1) with no black outline on a
// non-square document (#982).
void LayersEffectsTest::effectsListToggleSitsOnTheLayerRow()
{
    setupSquare();
    addDropShadow();
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), QStringLiteral("1"));
    const QModelIndex group = indexForPath(tree->model(), QStringLiteral("1/@fx"));
    QVERIFY(layer.isValid() && group.isValid());
    QVERIFY2(layer.data(pictura::EffectsListOpenRole).toBool(), "a new style opens the list");
    QVERIFY2(!group.data(pictura::ExpandableRole).toBool(), "the group header has no chevron");
    const QRect toggle = delegate->listToggleRect(tree->visualRect(layer), layer);
    QVERIFY2(!toggle.isEmpty(), "the layer row draws the list toggle");
    QVERIFY2(toggle.left() > tree->visualRect(layer).center().x(), "the toggle is right-aligned");

    const QImage open = panel_->rowImageForTest(QStringLiteral("1"));
    QVERIFY(panel_->clickListToggleForTest(QStringLiteral("1")));
    QCoreApplication::processEvents();
    const QImage closed = panel_->rowImageForTest(QStringLiteral("1"));
    QVERIFY(!open.isNull() && !closed.isNull());
    bool flipped = false;
    for (int y = toggle.top(); y <= toggle.bottom() && !flipped; ++y) {
        for (int x = toggle.left(); x <= toggle.right(); ++x) {
            if (open.pixelColor(x, y) != closed.pixelColor(x, y)) {
                flipped = true;
                break;
            }
        }
    }
    QVERIFY2(flipped, "the list toggle points down when closed and up when open");
}

void LayersEffectsTest::closingTheListHidesTheGroup()
{
    setupSquare();
    addDropShadow();
    QVERIFY(panel_->rowExistsForTest(QStringLiteral("1/@fx")));
    QVERIFY(panel_->clickListToggleForTest(QStringLiteral("1")));
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->rowExistsForTest(QStringLiteral("1/@fx")), "closing hides the header");
    QVERIFY2(!panel_->rowExistsForTest(QStringLiteral("1/@fx/dropShadow")), "closing hides the rows");
}

void LayersEffectsTest::nestedRowsFormOneCell()
{
    setupSquare();
    addDropShadow();
    const QImage group = panel_->rowImageForTest(QStringLiteral("1/@fx"));
    const QImage last = panel_->rowImageForTest(QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(!group.isNull() && !last.isNull());
    QVERIFY2(group.height() >= 24, "a synthetic row keeps its minimum vertical padding");
    const QColor groupBottom =
        group.pixelColor(group.width() / 2, group.height() - 1);
    const QColor lastBottom = last.pixelColor(last.width() / 2, last.height() - 1);
    QVERIFY2(groupBottom != lastBottom, "the open list has no rule between its rows");
    QVERIFY2(groupBottom == group.pixelColor(group.width() / 2, group.height() - 2),
             "the group header does not draw a bottom rule");
    QVERIFY2(lastBottom.lightness() < groupBottom.lightness(),
             "the block closes after the last row");
    const int border = pictura::LayerRowDelegate::kEyeColumn - 1;
    QCOMPARE(group.pixelColor(border, group.height() / 2),
             last.pixelColor(border, last.height() / 2));
}

void LayersEffectsTest::syntheticRowsHaveNoThumbnail()
{
    setupSquare();
    addDropShadow();
    QVERIFY2(panel_->rowThumbRectForTest(QStringLiteral("1/@fx")).isEmpty(),
             "the Effects header draws no thumbnail");
    QVERIFY2(panel_->rowThumbRectForTest(QStringLiteral("1/@fx/dropShadow")).isEmpty(),
             "an effect row draws no thumbnail");
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), QStringLiteral("1"));
    const QModelIndex group = indexForPath(tree->model(), QStringLiteral("1/@fx"));
    QVERIFY(layer.isValid() && group.isValid());
    const QRect gr = tree->visualRect(group);
    const QRect slot =
        delegate->imageThumbBox(QRect(0, 0, gr.width(), gr.height()), layer);
    QVERIFY(!slot.isEmpty());
    const QImage groupImage = panel_->rowImageForTest(QStringLiteral("1/@fx"));
    const QImage childImage = panel_->rowImageForTest(QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(!groupImage.isNull() && !childImage.isNull());
    const QColor surface = groupImage.pixelColor(groupImage.width() - 4, 1);
    QCOMPARE(groupImage.pixelColor(slot.center()), surface);
    QCOMPARE(childImage.pixelColor(slot.center()), surface);
}

void LayersEffectsTest::nestedEyeAlignsWithImageThumb()
{
    setupSquare();
    addDropShadow();
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "add mask");
    panel_->refresh();
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), QStringLiteral("1"));
    const QModelIndex group = indexForPath(tree->model(), QStringLiteral("1/@fx"));
    const QModelIndex child = indexForPath(tree->model(), QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(layer.isValid() && group.isValid() && child.isValid());
    const QRect gr = tree->visualRect(group);
    const QRect cr = tree->visualRect(child);
    const QRect ge = delegate->eyeRect(gr, group);
    const QRect ce = delegate->eyeRect(cr, child);
    QCOMPARE(ce.left(), ge.left() + pictura::LayerRowDelegate::kIndent);
    QCOMPARE(delegate->eyeColumnRect(gr, group).left(),
             delegate->imageSlotRight(gr, layer));
    QVERIFY(delegate->eyeColumnRect(gr, group).left() < delegate->thumbsRight(gr, layer));
    const QRect name = delegate->nameRect(gr, group);
    QVERIFY2(name.left() - ge.right() <= 7, "the eye-to-name spacing is halved");
}

void LayersEffectsTest::clickingEffectRowSelectsLayer()
{
    setupSquare();
    addDropShadow();
    // Start on a different layer so the click has to move the document target,
    // not merely re-affirm an existing selection.
    panel_->selectPaths({QStringLiteral("0")}, QStringLiteral("0"));
    QCOMPARE(panel_->targetLayerPaths(), QStringList{QStringLiteral("0")});
    for (const QString& path :
         {QStringLiteral("1/@fx"), QStringLiteral("1/@fx/dropShadow")}) {
        panel_->selectPaths({QStringLiteral("0")}, QStringLiteral("0"));
        QVERIFY2(panel_->clickRowForTest(path), "click the synthetic row");
        QCoreApplication::processEvents();
        QCOMPARE(panel_->currentPath(), QStringLiteral("1"));
        const QStringList selected = panel_->selectedPaths();
        QCOMPARE(selected, QStringList{QStringLiteral("1")});
        QCOMPARE(panel_->targetLayerPaths(), QStringList{QStringLiteral("1")});
    }
}

void LayersEffectsTest::hiddenEffectShowsFadedEye()
{
    setupSquare();
    addDropShadow();
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex child = indexForPath(tree->model(), QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(child.isValid());
    const QRect vr = tree->visualRect(child);
    const QRect eye = delegate->eyeRect(QRect(0, 0, vr.width(), vr.height()), child);
    const QImage visible = panel_->rowImageForTest(QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(!visible.isNull());
    QVERIFY(panel_->clickRowEyeForTest(QStringLiteral("1/@fx/dropShadow")));
    QCoreApplication::processEvents();
    const QImage hidden = panel_->rowImageForTest(QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(!hidden.isNull());
    const QColor visibleEye = visible.pixelColor(eye.center());
    const QColor hiddenEye = hidden.pixelColor(eye.center());
    const QColor surface = hidden.pixelColor(hidden.width() - 4, 1);
    QVERIFY2(hiddenEye != surface, "the hidden row paints the eye-off glyph");
    QVERIFY2(hiddenEye != visibleEye, "the hidden glyph is faded");
    QVERIFY2(hiddenEye.lightness() > visibleEye.lightness(),
             "the 50% glyph blends toward the row surface");

    // The eye slot stays the toggle target: a second click re-enables the row.
    QVERIFY(panel_->clickRowEyeForTest(QStringLiteral("1/@fx/dropShadow")));
    QCoreApplication::processEvents();
    QVERIFY2(panel_->rowVisibleForTest(QStringLiteral("1/@fx/dropShadow")),
             "the second click switches the effect back on");
}

void LayersEffectsTest::groupKeepsNestedLayersChevron()
{
    QVERIFY(window_->newDocument(QStringLiteral("EffectsGroup"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    const QString child = view_->add_layer_in(group);
    QVERIFY(!child.isEmpty());
    QVERIFY(pictura::layer_style_set(*view_, child, QStringLiteral("dropShadow.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    panel_->setView(view_);
    panel_->refresh();
    QCoreApplication::processEvents();
    QVERIFY(panel_->rowExpandableForTest(group));
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex groupIndex = indexForPath(tree->model(), group);
    const QModelIndex childIndex = indexForPath(tree->model(), child);
    QVERIFY(groupIndex.isValid() && childIndex.isValid());
    QVERIFY2(delegate->listToggleRect(tree->visualRect(groupIndex), groupIndex).isEmpty(),
             "the group row has no list toggle");
    QVERIFY2(!delegate->listToggleRect(tree->visualRect(childIndex), childIndex).isEmpty(),
             "the styled child draws the list toggle");
}

void LayersEffectsTest::shapeLayerWithEffectsShowsTheListToggle()
{
    QVERIFY(window_->newDocument(QStringLiteral("ShapeEffects"), 40, 40, QStringLiteral("rgb"),
                                 8, QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    panel_->refresh();
    QCoreApplication::processEvents();

    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 5;
    square.y0 = 5;
    square.x1 = 20;
    square.y1 = 20;
    const QString path = pictura::shape_add_layer(*view_, square, 0xffff0000u);
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_style_set(*view_, path, QStringLiteral("dropShadow.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    panel_->refresh();
    QCoreApplication::processEvents();

    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), path);
    QVERIFY(layer.isValid());
    QVERIFY2(layer.data(pictura::LayerRowShapeRole).toBool(), "a shape row");
    const QRect vr = tree->visualRect(layer);
    QVERIFY2(delegate->fxRect(vr, layer).isEmpty(), "a shape row keeps the fx badge hidden");
    QVERIFY2(!delegate->listToggleRect(vr, layer).isEmpty(),
             "a shape layer with effects still exposes the list toggle");
    QVERIFY(panel_->rowExistsForTest(path + QStringLiteral("/@fx")));
    QVERIFY(panel_->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->rowExistsForTest(path + QStringLiteral("/@fx")),
             "the toggle closes the shape layer's Effects list");
    QVERIFY(panel_->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(panel_->rowExistsForTest(path + QStringLiteral("/@fx")),
             "the toggle reopens the shape layer's Effects list");
}

void LayersEffectsTest::syntheticRowsAreNotSelectable()
{
    setupSquare();
    addDropShadow();
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree);
    const QModelIndex layer = indexForPath(tree->model(), QStringLiteral("1"));
    const QModelIndex group = indexForPath(tree->model(), QStringLiteral("1/@fx"));
    const QModelIndex child = indexForPath(tree->model(), QStringLiteral("1/@fx/dropShadow"));
    QVERIFY(layer.isValid() && group.isValid() && child.isValid());
    QVERIFY2(!(group.flags() & Qt::ItemIsSelectable), "the Effects header is not selectable");
    QVERIFY2(!(child.flags() & Qt::ItemIsSelectable), "an effect row is not selectable");
    QVERIFY2(layer.flags() & Qt::ItemIsSelectable, "the owning layer stays selectable");
}

void LayersEffectsTest::syntheticRowsDoNotDrag()
{
    setupSquare();
    addDropShadow();
    QVERIFY2(panel_->dragPathsForSyntheticForTest(QStringLiteral("1/@fx")).isEmpty(),
             "the Effects header does not drag");
    QVERIFY2(panel_->dragPathsForSyntheticForTest(QStringLiteral("1/@fx/dropShadow")).isEmpty(),
             "an effect row does not drag");
    QCOMPARE(panel_->dragPathsForSyntheticForTest(QStringLiteral("1")),
             QStringList{QStringLiteral("1")});
}

QTEST_MAIN(LayersEffectsTest)
#include "tst_layers_effects.moc"
