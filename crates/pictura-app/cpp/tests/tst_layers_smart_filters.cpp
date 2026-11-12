#include <QtTest/QtTest>

#include <QtCore/QAbstractItemModel>
#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTreeView>

#include <functional>

#include "frame.h"
#include "commands.h"
#include "pictura_raw_dialog.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "api/layers_bridge.h"

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

class LayersSmartFiltersTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void treeShowsGroupAndToggles();
    void clearMenuCommandRemovesFilters();
    void smartFiltersGroupMenu();
    void smartFilterRowMenu();
    void deleteFilterMaskFollowsTheMask();
    void editSmartFilterIsGatedOnKind();
    void editSmartFilterRequestsTheDialog();
    void menuTogglesAndClearsFilters();
    void filterRowDisablesAndDeletes();
    void smartFiltersListToggleSitsOnTheLayerRow();
    void closingTheListHidesTheGroup();
    void nestedRowsFormOneCell();
    void syntheticRowsHaveNoThumbnail();
    void nestedEyeAlignsWithImageThumb();
    void hiddenFilterShowsFadedEye();
    void oneToggleControlsBothLists();
    void filterRowStillDrags();
    void noRuleBetweenGroups();
    void clickingFilterRowSelectsLayer();

private:
    // A smart object with one camera-raw smart filter; returns its layer path.
    QString filteredSmartObject();
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LayersSmartFiltersTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void LayersSmartFiltersTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

void LayersSmartFiltersTest::treeShowsGroupAndToggles()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartFilters"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->convert_to_smart_object(path));
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));

    const QString group = path + QStringLiteral("/@sf");
    const QString child = group + QStringLiteral("/0");

    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    panel->expandForTest(path);
    panel->expandForTest(group);
    QCoreApplication::processEvents();

    auto* tree = panel->findChild<QTreeView*>();
    QVERIFY(tree);
    QAbstractItemModel* model = tree->model();
    QVERIFY(model);

    std::function<QModelIndex(const QModelIndex&)> findText =
        [&](const QModelIndex& parent) -> QModelIndex {
        for (int r = 0; r < model->rowCount(parent); ++r) {
            const QModelIndex index = model->index(r, 0, parent);
            if (index.data(Qt::DisplayRole).toString() == QStringLiteral("Smart Filters")) {
                return index;
            }
            const QModelIndex found = findText(index);
            if (found.isValid()) {
                return found;
            }
        }
        return {};
    };
    const QModelIndex groupIndex = findText(QModelIndex());
    QVERIFY(groupIndex.isValid());
    QCOMPARE(model->rowCount(groupIndex), 1);
    QCOMPARE(model->index(0, 0, groupIndex).data(Qt::DisplayRole).toString(),
             QStringLiteral("Camera Raw Filter"));

    QVERIFY(pictura::layer_smart_filter_visible(*view, path, 0));
    QVERIFY(panel->clickSmartFilterEyeForTest(child));
    QCoreApplication::processEvents();
    QVERIFY(!pictura::layer_smart_filter_visible(*view, path, 0));

    QVERIFY(pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->clickSmartFilterEyeForTest(group));
    QCoreApplication::processEvents();
    QVERIFY(!pictura::layer_smart_filters_enabled(*view, path));
}

void LayersSmartFiltersTest::clearMenuCommandRemovesFilters()
{
    QVERIFY(window_->newDocument(QStringLiteral("ClearFilters"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->convert_to_smart_object(path));
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);

    panel->setView(view);
    panel->refresh();
    QVERIFY(panel->selectRowForTest(path));
    QCoreApplication::processEvents();

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QAction* clear = registry->action(QString::fromLatin1(pictura::command_ids::LayerSmartFiltersClear));
    QVERIFY(clear != nullptr);
    registry->refresh();
    QVERIFY2(clear->isEnabled(), "Clear Smart Filters is enabled when the layer has filters");

    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerSmartFiltersClear)));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    registry->refresh();
    QVERIFY2(!clear->isEnabled(), "Clear Smart Filters disables once the stack is empty");
}

QString LayersSmartFiltersTest::filteredSmartObject()
{
    if (!window_->newDocument(QStringLiteral("SmartMenus"), 64, 64, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return QString();
    }
    pictura::PictureView* view = window_->activeView();
    const QString path = view->add_solid_fill(0xff808080u);
    view->rasterize_fill_content(path);
    view->convert_to_smart_object(path);
    view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5"));
    return path;
}

void LayersSmartFiltersTest::smartFiltersGroupMenu()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QCOMPARE(panel->smartFiltersGroupMenuTextsForTest(path + QStringLiteral("/@sf")),
             QStringList({QStringLiteral("Disable Smart Filters"),
                          QStringLiteral("Delete Filter Mask"),
                          QStringLiteral("Clear Smart Filters")}));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::smartFilterRowMenu()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QCOMPARE(panel->smartFilterRowMenuTextsForTest(path + QStringLiteral("/@sf/0")),
             QStringList({QStringLiteral("Edit Smart Filter Blending Options…"),
                          QStringLiteral("Edit Smart Filter…"),
                          QStringLiteral("Disable Smart Filter"),
                          QStringLiteral("Delete Smart Filter")}));
    // Blending options is a follow-up: shown, disabled.
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/0"),
                                                  QStringLiteral("Edit Smart Filter Blending Options…")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::deleteFilterMaskFollowsTheMask()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    // The camera-raw filter carries no filter mask, so the item is disabled.
    QVERIFY(!pictura::layer_has_smart_filter_mask(*view, path));
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf"),
                                                  QStringLiteral("Delete Filter Mask")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::editSmartFilterIsGatedOnKind()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    // Camera Raw is editable; an out-of-range index (no dialog) is not.
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 0), 1);
    QVERIFY(panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/0"),
                                                 QStringLiteral("Edit Smart Filter…")));
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 9), 0);
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/9"),
                                                  QStringLiteral("Edit Smart Filter…")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::editSmartFilterRequestsTheDialog()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QString gotPath;
    int gotIndex = -1;
    connect(panel, &pictura::LayersPanel::smartFilterEditRequested, panel,
            [&](const QString& p, int i) {
                gotPath = p;
                gotIndex = i;
            });
    // The window re-dispatches the filter's own command, which opens the modal
    // dialog; close it from the event loop and record that it appeared.
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<pictura::PicturaRawDialog*>(widget);
                dialog && dialog->isVisible()) {
                opened = true;
                dialog->reject();
            }
        }
    });
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Edit Smart Filter…")));
    QCOMPARE(gotPath, path);
    QCOMPARE(gotIndex, 0);
    QVERIFY2(opened, "Edit Smart Filter… reopened the filter dialog");
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::menuTogglesAndClearsFilters()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Disable Smart Filters")));
    QVERIFY(!pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Enable Smart Filters")));
    QVERIFY(pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Clear Smart Filters")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::filterRowDisablesAndDeletes()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Disable Smart Filter")));
    QVERIFY(!pictura::layer_smart_filter_visible(*view, path, 0));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Delete Smart Filter")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    QVERIFY(window_->closeActiveDocument(false));
}

// The Smart Filters group row's card is a square (1:1) with no black outline on
// a non-square document (#982).
void LayersSmartFiltersTest::smartFiltersListToggleSitsOnTheLayerRow()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    const QString group = path + QStringLiteral("/@sf");
    auto* tree = panel->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), path);
    const QModelIndex header = indexForPath(tree->model(), group);
    QVERIFY(layer.isValid() && header.isValid());
    QVERIFY2(layer.data(pictura::FiltersListOpenRole).toBool(), "a filtered layer opens the list");
    QVERIFY2(!header.data(pictura::ExpandableRole).toBool(), "the header has no chevron");
    const QRect toggle = delegate->listToggleRect(tree->visualRect(layer), layer);
    QVERIFY2(!toggle.isEmpty(), "the layer row draws the list toggle");
    QVERIFY2(toggle.left() > tree->visualRect(layer).center().x(), "the toggle is right-aligned");

    const QImage open = panel->rowImageForTest(path);
    QVERIFY(panel->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    const QImage closed = panel->rowImageForTest(path);
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

void LayersSmartFiltersTest::closingTheListHidesTheGroup()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    const QString group = path + QStringLiteral("/@sf");
    QVERIFY(panel->rowExistsForTest(group));
    QVERIFY(panel->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(!panel->rowExistsForTest(group), "closing hides the header");
    QVERIFY2(!panel->rowExistsForTest(group + QStringLiteral("/0")), "closing hides the rows");
}

void LayersSmartFiltersTest::nestedRowsFormOneCell()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    const QString group = path + QStringLiteral("/@sf");
    const QImage header = panel->rowImageForTest(group);
    const QImage last = panel->rowImageForTest(group + QStringLiteral("/0"));
    QVERIFY(!header.isNull() && !last.isNull());
    QVERIFY2(header.height() >= 24, "a synthetic row keeps its minimum vertical padding");
    const QColor headerBottom =
        header.pixelColor(header.width() / 2, header.height() - 1);
    const QColor lastBottom = last.pixelColor(last.width() / 2, last.height() - 1);
    QVERIFY2(headerBottom != lastBottom, "the open list has no rule between its rows");
    QVERIFY2(headerBottom == header.pixelColor(header.width() / 2, header.height() - 2),
             "the group header does not draw a bottom rule");
    QVERIFY2(lastBottom.lightness() < headerBottom.lightness(),
             "the block closes after the last row");
    const int border = pictura::LayerRowDelegate::kEyeColumn - 1;
    QCOMPARE(header.pixelColor(border, header.height() / 2),
             last.pixelColor(border, last.height() / 2));
}

void LayersSmartFiltersTest::syntheticRowsHaveNoThumbnail()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    QVERIFY2(panel->rowThumbRectForTest(path + QStringLiteral("/@sf")).isEmpty(),
             "the Smart Filters header draws no thumbnail");
    QVERIFY2(panel->rowThumbRectForTest(path + QStringLiteral("/@sf/0")).isEmpty(),
             "a filter row draws no thumbnail");
    auto* tree = panel->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), path);
    const QModelIndex header = indexForPath(tree->model(), path + QStringLiteral("/@sf"));
    QVERIFY(layer.isValid() && header.isValid());
    const QRect hr = tree->visualRect(header);
    const QRect slot = delegate->imageThumbBox(QRect(0, 0, hr.width(), hr.height()), layer);
    QVERIFY(!slot.isEmpty());
    const QImage headerImage = panel->rowImageForTest(path + QStringLiteral("/@sf"));
    const QImage childImage = panel->rowImageForTest(path + QStringLiteral("/@sf/0"));
    QVERIFY(!headerImage.isNull() && !childImage.isNull());
    const QColor surface = headerImage.pixelColor(headerImage.width() - 4, 1);
    QCOMPARE(headerImage.pixelColor(slot.center()), surface);
    QCOMPARE(childImage.pixelColor(slot.center()), surface);
}

void LayersSmartFiltersTest::nestedEyeAlignsWithImageThumb()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    const QString group = path + QStringLiteral("/@sf");
    auto* tree = panel->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), path);
    const QModelIndex header = indexForPath(tree->model(), group);
    const QModelIndex child = indexForPath(tree->model(), group + QStringLiteral("/0"));
    QVERIFY(layer.isValid() && header.isValid() && child.isValid());
    const QRect hr = tree->visualRect(header);
    const QRect cr = tree->visualRect(child);
    const QRect he = delegate->eyeRect(hr, header);
    const QRect ce = delegate->eyeRect(cr, child);
    QCOMPARE(ce.left(), he.left() + pictura::LayerRowDelegate::kIndent);
    QCOMPARE(delegate->eyeColumnRect(hr, header).left(),
             delegate->imageSlotRight(hr, layer));
    const QRect name = delegate->nameRect(hr, header);
    QVERIFY2(name.left() - he.right() <= 7, "the eye-to-name spacing is halved");
}

void LayersSmartFiltersTest::oneToggleControlsBothLists()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_style_set(*window_->activeView(), path,
                                     QStringLiteral("dropShadow.on"), 1.0));
    pictura::layer_style_commit(*window_->activeView(), QStringLiteral("Layer Style"));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    auto* tree = panel->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex layer = indexForPath(tree->model(), path);
    QVERIFY(layer.isValid());
    QVERIFY2(delegate->hasListToggle(layer), "one toggle covers both lists");
    QCOMPARE(delegate->listToggleAdvance(layer),
             pictura::LayerRowDelegate::kChevronWidth + 3);
    QVERIFY2(layer.data(pictura::EffectsListOpenRole).toBool(), "effects are open");
    QVERIFY2(layer.data(pictura::FiltersListOpenRole).toBool(), "filters are open");
    QVERIFY(panel->rowExistsForTest(path + QStringLiteral("/@fx")));
    QVERIFY(panel->rowExistsForTest(path + QStringLiteral("/@sf")));
    QVERIFY(panel->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(!panel->rowExistsForTest(path + QStringLiteral("/@fx")), "closing hides effects");
    QVERIFY2(!panel->rowExistsForTest(path + QStringLiteral("/@sf")), "closing hides filters");
    QVERIFY(panel->clickListToggleForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(panel->rowExistsForTest(path + QStringLiteral("/@fx")), "reopening shows effects");
    QVERIFY2(panel->rowExistsForTest(path + QStringLiteral("/@sf")), "reopening shows filters");
}

void LayersSmartFiltersTest::noRuleBetweenGroups()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_style_set(*window_->activeView(), path,
                                     QStringLiteral("dropShadow.on"), 1.0));
    pictura::layer_style_commit(*window_->activeView(), QStringLiteral("Layer Style"));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    const QImage filter = panel->rowImageForTest(path + QStringLiteral("/@sf/0"));
    const QImage effect = panel->rowImageForTest(path + QStringLiteral("/@fx/dropShadow"));
    QVERIFY(!effect.isNull() && !filter.isNull());
    const QColor filterBottom = filter.pixelColor(filter.width() / 2, filter.height() - 1);
    const QColor filterInterior = filter.pixelColor(filter.width() / 2, filter.height() - 2);
    const QColor effectBottom = effect.pixelColor(effect.width() / 2, effect.height() - 1);
    const QColor effectInterior = effect.pixelColor(effect.width() / 2, effect.height() - 2);
    QCOMPARE(filterBottom, filterInterior);
    QVERIFY2(effectBottom.lightness() < effectInterior.lightness(),
             "the last list row closes the block");
}

void LayersSmartFiltersTest::clickingFilterRowSelectsLayer()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    // Start on a different layer so the click has to move the document target.
    panel->selectPaths({QStringLiteral("0")}, QStringLiteral("0"));
    QCOMPARE(panel->targetLayerPaths(), QStringList{QStringLiteral("0")});
    for (const QString& row :
         {path + QStringLiteral("/@sf"), path + QStringLiteral("/@sf/0")}) {
        panel->selectPaths({QStringLiteral("0")}, QStringLiteral("0"));
        QVERIFY2(panel->clickRowForTest(row), "click the synthetic row");
        QCoreApplication::processEvents();
        QCOMPARE(panel->currentPath(), path);
        QCOMPARE(panel->selectedPaths(), QStringList{path});
        QCOMPARE(panel->targetLayerPaths(), QStringList{path});
    }
}

void LayersSmartFiltersTest::hiddenFilterShowsFadedEye()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    const QString childPath = path + QStringLiteral("/@sf/0");
    auto* tree = panel->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex child = indexForPath(tree->model(), childPath);
    QVERIFY(child.isValid());
    const QRect vr = tree->visualRect(child);
    const QRect eye = delegate->eyeRect(QRect(0, 0, vr.width(), vr.height()), child);
    const QImage visible = panel->rowImageForTest(childPath);
    QVERIFY(!visible.isNull());
    QVERIFY(panel->clickSmartFilterEyeForTest(childPath));
    QCoreApplication::processEvents();
    const QImage hidden = panel->rowImageForTest(childPath);
    QVERIFY(!hidden.isNull());
    const QColor visibleEye = visible.pixelColor(eye.center());
    const QColor hiddenEye = hidden.pixelColor(eye.center());
    const QColor surface = hidden.pixelColor(hidden.width() - 4, 1);
    QVERIFY2(hiddenEye != surface, "the hidden row paints the eye-off glyph");
    QVERIFY2(hiddenEye != visibleEye, "the hidden glyph is faded");
    QVERIFY2(hiddenEye.lightness() > visibleEye.lightness(),
             "the 50% glyph blends toward the row surface");

    // The eye slot stays the toggle target: a second click re-enables the row.
    QVERIFY(panel->clickSmartFilterEyeForTest(childPath));
    QCoreApplication::processEvents();
    QVERIFY2(panel->rowVisibleForTest(childPath),
             "the second click switches the filter back on");
}

void LayersSmartFiltersTest::filterRowStillDrags()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    const QString child = path + QStringLiteral("/@sf/0");
    QCOMPARE(panel->dragPathsForSyntheticForTest(child), QStringList{child});
    QVERIFY2(panel->dragPathsForSyntheticForTest(path + QStringLiteral("/@sf")).isEmpty(),
             "the Smart Filters header does not drag");
    QCOMPARE(panel->dragPathsForSyntheticForTest(path), QStringList{path});
}

QTEST_MAIN(LayersSmartFiltersTest)
#include "tst_layers_smart_filters.moc"
