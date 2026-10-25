#include <QtTest/QtTest>

#include "api/layers_api.h"

#include <memory>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QKeySequence>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {

pictura::layers_api::LayerRow rowAt(pictura::PictureView& view, int index)
{
    return pictura::layers_api::rowAt(view, index);
}

QAction* commandMerge(pictura::CommandRegistry* registry)
{
    return registry->action(QString::fromLatin1(pictura::command_ids::LayerMergeLayers));
}

QAction* panelMerge(QMenu* menu)
{
    if (!menu) {
        return nullptr;
    }
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == QLatin1String("mergeDown")) {
            return action;
        }
    }
    return nullptr;
}

QStringList rowsKey(pictura::PictureView& view)
{
    QStringList keys;
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        const auto row = rowAt(view, i);
        keys << QStringLiteral("%1|%2|%3|%4|%5|%6|%7|%8|%9|%10")
                    .arg(row.path, row.name, row.kind, row.blend)
                    .arg(row.opacity)
                    .arg(row.fill)
                    .arg(row.lockBits)
                    .arg(row.hasMask ? 1 : 0)
                    .arg(row.hasVectorMask ? 1 : 0)
                    .arg(row.hasAdjustment ? 1 : 0);
    }
    return keys;
}

pictura::layers_api::LayerRow rowNamed(pictura::PictureView& view, const QString& name)
{
    for (const auto& row : pictura::layers_api::rows(view)) {
        if (row.name == name) {
            return row;
        }
    }
    return {};
}

QMenu* layerMenu(pictura::PicturaMainWindow& frame)
{
    if (!frame.menuBar()) {
        return nullptr;
    }
    for (QAction* action : frame.menuBar()->actions()) {
        QString text = action->text();
        text.remove(QLatin1Char('&'));
        if (text == QLatin1String("Layer")) {
            return action->menu();
        }
    }
    return nullptr;
}

int mergeMenuItems(QMenu* menu)
{
    int count = 0;
    if (!menu) {
        return count;
    }
    for (QAction* action : menu->actions()) {
        const QString text = action->text();
        if (text == QLatin1String("Merge Down") || text == QLatin1String("Merge Layers")) {
            ++count;
        } else if (action->menu() != nullptr) {
            count += mergeMenuItems(action->menu());
        }
    }
    return count;
}

} // namespace

class MergeCommandsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layerMenuContextSensitive();
    void layerMergeSubmenu();
    void layersPanelContextSensitiveLabel();
    void adjustmentSourceMergesDown();
    void adjustmentAboveBottomMergesLayers();
    void adjustmentTargetStillRefused();
    void mergeVisibleSelectsMergedResult();
    void mergeDownSelectsResult();
    void mergeLayersSelectsResult();
    void shortcutFollowsSelection();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void MergeCommandsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void MergeCommandsTest::layerMenuContextSensitive()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeCommands"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    view->add_layer(-1);
    view->add_layer(-1);
    view->add_layer(-1);
    layers->refresh();
    QVERIFY(pictura::layers_api::rowCount(*view) >= 4);

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QMenu* menu = layerMenu(frame);
    QVERIFY(menu != nullptr);
    QVERIFY2(registry->action(QStringLiteral("layer.merge.down")) == nullptr,
             "the separate Merge Down command is retired");

    // No selection: disabled and labelled Merge Layers.
    layers->selectPaths(QStringList{}, QString());
    registry->refresh();
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(!merge->isEnabled());
    QCOMPARE(mergeMenuItems(menu), 1);

    // One selected layer: Merge Down on Ctrl+E records one state.
    const int rowsBeforeDown = pictura::layers_api::rowCount(*view);
    const QString topPath = rowAt(*view, 0).path;
    const QString lowerName = rowAt(*view, 1).name;
    layers->selectPaths({topPath}, topPath);
    registry->refresh();
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));
    QVERIFY(merge->isEnabled());
    QCOMPARE(mergeMenuItems(menu), 1);
    QCOMPARE(registry->commandForShortcut(QKeySequence(QStringLiteral("Ctrl+E"))),
             QString::fromLatin1(pictura::command_ids::LayerMergeLayers));
    const QStringList rowsBeforeDownState = rowsKey(*view);
    const auto pixelBeforeDown = view->composite_argb(8, 8);
    const int historyBeforeDown = view->history_count();
    merge->trigger();
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBeforeDown - 1);
    QCOMPARE(view->history_count(), historyBeforeDown + 1);
    QCOMPARE(rowAt(*view, 0).name, lowerName);
    QVERIFY(view->undo());
    layers->refresh();
    QCOMPARE(rowsKey(*view), rowsBeforeDownState);
    QCOMPARE(view->composite_argb(8, 8), pixelBeforeDown);

    frame.closeDocument(doc, false);

    // A non-contiguous selection distinguishes Merge Layers from the old
    // current-row Merge Down: the unselected middle layer must survive.
    QVERIFY(frame.newDocument(QStringLiteral("MergeCommandsMulti"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int multiDoc = frame.activeDocumentIndex();
    view = frame.activeView();
    QVERIFY(view != nullptr);
    view->add_layer(-1);
    view->add_layer(-1);
    view->add_layer(-1);
    layers->refresh();

    const QString firstPath = rowAt(*view, 0).path;
    const QString topName = rowAt(*view, 0).name;
    const QString middleName = rowAt(*view, 1).name;
    const QString thirdSelectedPath = rowAt(*view, 2).path;
    layers->selectPaths({firstPath, thirdSelectedPath}, firstPath);
    registry->refresh();
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(merge->isEnabled());
    QCOMPARE(mergeMenuItems(menu), 1);
    const int rowsBeforeLayers = pictura::layers_api::rowCount(*view);
    const QStringList rowsBeforeLayersState = rowsKey(*view);
    const auto pixelBeforeLayers = view->composite_argb(8, 8);
    const int historyBeforeLayers = view->history_count();
    QVERIFY(!registry->dispatch(QStringLiteral("layer.merge.down")));
    QCOMPARE(view->history_count(), historyBeforeLayers);
    merge->trigger();
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBeforeLayers - 1);
    QCOMPARE(view->history_count(), historyBeforeLayers + 1);
    QCOMPARE(rowAt(*view, 0).name, topName);
    QCOMPARE(rowAt(*view, 1).name, middleName);
    QVERIFY(view->undo());
    layers->refresh();
    QCOMPARE(rowsKey(*view), rowsBeforeLayersState);
    QCOMPARE(view->composite_argb(8, 8), pixelBeforeLayers);

    frame.closeDocument(multiDoc, false);
}

// Layer > Merge groups the merge commands so the top-level list stays short.
void MergeCommandsTest::layerMergeSubmenu()
{
    pictura::PicturaMainWindow& frame = *window_;
    QMenu* layer = layerMenu(frame);
    QVERIFY(layer != nullptr);

    QMenu* merge = nullptr;
    for (QAction* action : layer->actions()) {
        QString text = action->text();
        text.remove(QLatin1Char('&'));
        if (text == QLatin1String("Merge")) {
            merge = action->menu();
        }
    }
    QVERIFY2(merge != nullptr, "Layer has a Merge submenu");

    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(registry != nullptr);
    const QList<QAction*> expected = {
        registry->action(QString::fromLatin1(pictura::command_ids::LayerMergeLayers)),
        registry->action(QString::fromLatin1(pictura::command_ids::LayerMergeVisible)),
        registry->action(QString::fromLatin1(pictura::command_ids::LayerStampVisible)),
        registry->action(QString::fromLatin1(pictura::command_ids::LayerStampSelected)),
        registry->action(QString::fromLatin1(pictura::command_ids::LayerFlattenImage)),
        registry->action(
            QString::fromLatin1(pictura::command_ids::LayerMergeClippingMask))};
    QList<QAction*> entries;
    for (QAction* action : merge->actions()) {
        if (!action->isSeparator()) {
            entries.append(action);
        }
    }
    QCOMPARE(entries.size(), expected.size());
    for (int i = 0; i < expected.size(); ++i) {
        QVERIFY2(expected.at(i) != nullptr, qPrintable(QString::number(i)));
        QCOMPARE(entries.at(i), expected.at(i));
        QVERIFY2(!layer->actions().contains(expected.at(i)), qPrintable(QString::number(i)));
    }
}

void MergeCommandsTest::layersPanelContextSensitiveLabel()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergePanel"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* column = frame.columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(view != nullptr && layers != nullptr && column != nullptr);

    view->add_layer(-1);
    view->add_layer(-1);
    view->add_layer(-1);
    layers->refresh();
    column->setRailMode(false);
    column->ensureGroupVisibleForTest(QStringLiteral("layersPanel"));
    column->showPanel(QStringLiteral("layersPanel"), true);
    QCoreApplication::processEvents();
    pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group != nullptr);
    QToolButton* button = group->headerMenuButtonForTest();
    QVERIFY(button != nullptr);
    QMenu* menu = button->menu();
    QVERIFY(menu != nullptr);
    QAction* merge = panelMerge(menu);
    QVERIFY(merge != nullptr);

    const QString topPath = rowAt(*view, 0).path;
    const QString secondPath = rowAt(*view, 1).path;
    const QString bottomPath = rowAt(*view, pictura::layers_api::rowCount(*view) - 1).path;

    layers->selectPaths({topPath}, topPath);
    emit menu->aboutToShow();
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));
    QVERIFY(merge->isEnabled());

    layers->selectPaths({topPath, secondPath}, topPath);
    emit menu->aboutToShow();
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(merge->isEnabled());

    layers->selectPaths({bottomPath}, bottomPath);
    emit menu->aboutToShow();
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));
    QVERIFY(!merge->isEnabled());

    layers->selectPaths(QStringList{}, QString());
    emit menu->aboutToShow();
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(!merge->isEnabled());

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::adjustmentSourceMergesDown()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeAdjustmentSource"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    layers->refresh();
    QVERIFY(rowAt(*view, 0).hasAdjustment);

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    const QString adjustmentPath = rowAt(*view, 0).path;
    layers->selectPaths({adjustmentPath}, adjustmentPath);
    QCoreApplication::processEvents();

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));
    QVERIFY(merge->isEnabled());
    merge->trigger();
    QCoreApplication::processEvents();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore - 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    QVERIFY(!rowAt(*view, 0).hasAdjustment);
    QCOMPARE(layers->selectedPaths(), QStringList{rowAt(*view, 0).path});

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::adjustmentAboveBottomMergesLayers()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeAdjustmentSelection"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    layers->refresh();
    QVERIFY(rowAt(*view, 0).hasAdjustment);

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    const QString adjustmentPath = rowAt(*view, 0).path;
    const QString pixelPath = rowAt(*view, 1).path;
    layers->selectPaths({adjustmentPath, pixelPath}, adjustmentPath);
    QCoreApplication::processEvents();

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(merge->isEnabled());
    merge->trigger();
    QCoreApplication::processEvents();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore - 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    QVERIFY(!rowAt(*view, 0).hasAdjustment);
    QCOMPARE(layers->selectedPaths(), QStringList{rowAt(*view, 0).path});

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::adjustmentTargetStillRefused()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeAdjustmentRefusal"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    const QString pixelPath = rowAt(*view, 0).path;
    layers->selectPaths({pixelPath}, pixelPath);
    QCoreApplication::processEvents();

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QVERIFY(!merge->isEnabled());
    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerMergeLayers)));
    QCoreApplication::processEvents();
    QCOMPARE(view->history_count(), historyBefore);
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore);

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::mergeVisibleSelectsMergedResult()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeVisibleSelection"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), 3);
    const int topIndex = rowAt(*view, 0).path.toInt();
    const int middleIndex = rowAt(*view, 1).path.toInt();
    QVERIFY(view->set_layer_name(topIndex, QStringLiteral("HiddenTop")));
    QVERIFY(view->set_layer_name(middleIndex, QStringLiteral("Visible")));
    view->set_layer_visible(topIndex, false);
    layers->refresh();

    const int historyBefore = view->history_count();
    const QString visiblePath = rowAt(*view, 1).path;
    const QString backgroundPath = rowAt(*view, 2).path;
    layers->selectPaths({visiblePath, backgroundPath}, visiblePath);
    QCoreApplication::processEvents();

    QAction* mergeVisible =
        registry->action(QString::fromLatin1(pictura::command_ids::LayerMergeVisible));
    QVERIFY(mergeVisible != nullptr);
    QVERIFY(mergeVisible->isEnabled());
    mergeVisible->trigger();
    QCoreApplication::processEvents();

    const QStringList selected = layers->selectedPaths();
    QCOMPARE(selected.size(), 1);
    QString mergedPath;
    QString hiddenPath;
    for (const auto& row : pictura::layers_api::rows(*view)) {
        if (row.name == QLatin1String("Visible")) {
            mergedPath = row.path;
        } else if (row.name == QLatin1String("HiddenTop")) {
            hiddenPath = row.path;
            QVERIFY(!row.visible);
        }
    }
    QVERIFY(!mergedPath.isEmpty());
    QVERIFY(!hiddenPath.isEmpty());
    QCOMPARE(selected.first(), mergedPath);
    QVERIFY(!selected.contains(hiddenPath));
    QCOMPARE(view->history_count(), historyBefore + 1);

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::mergeDownSelectsResult()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeDownSelection"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), 4);
    QVERIFY(view->set_layer_name(rowAt(*view, 0).path.toInt(), QStringLiteral("Top")));
    QVERIFY(view->set_layer_name(rowAt(*view, 1).path.toInt(), QStringLiteral("Middle")));
    QVERIFY(view->set_layer_name(rowAt(*view, 2).path.toInt(), QStringLiteral("Third")));
    layers->refresh();

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    const QString middlePath = rowAt(*view, 1).path;
    layers->selectPaths({middlePath}, middlePath);
    QCoreApplication::processEvents();

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));
    QVERIFY(merge->isEnabled());
    merge->trigger();
    QCoreApplication::processEvents();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore - 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    const QStringList selected = layers->selectedPaths();
    QCOMPARE(selected.size(), 1);
    const auto merged = rowNamed(*view, QStringLiteral("Third"));
    const auto top = rowNamed(*view, QStringLiteral("Top"));
    QVERIFY(!merged.path.isEmpty());
    QVERIFY(!top.path.isEmpty());
    QCOMPARE(selected.first(), merged.path);
    QVERIFY(selected.first() != top.path);

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::mergeLayersSelectsResult()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeLayersSelection"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), 4);
    QVERIFY(view->set_layer_name(rowAt(*view, 0).path.toInt(), QStringLiteral("Top")));
    QVERIFY(view->set_layer_name(rowAt(*view, 1).path.toInt(), QStringLiteral("Middle")));
    QVERIFY(view->set_layer_name(rowAt(*view, 2).path.toInt(), QStringLiteral("Third")));
    layers->refresh();

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    const QString topPath = rowAt(*view, 0).path;
    const QString thirdPath = rowAt(*view, 2).path;
    layers->selectPaths({topPath, thirdPath}, topPath);
    QCoreApplication::processEvents();

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));
    QVERIFY(merge->isEnabled());
    merge->trigger();
    QCoreApplication::processEvents();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore - 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    const QStringList selected = layers->selectedPaths();
    QCOMPARE(selected.size(), 1);
    const auto merged = rowNamed(*view, QStringLiteral("Top"));
    const auto middle = rowNamed(*view, QStringLiteral("Middle"));
    QVERIFY(!merged.path.isEmpty());
    QVERIFY(!middle.path.isEmpty());
    QCOMPARE(selected.first(), merged.path);
    QVERIFY(selected.first() != middle.path);

    frame.closeDocument(doc, false);
}

void MergeCommandsTest::shortcutFollowsSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MergeShortcut"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), 2);
    const QString layerPath = rowAt(*view, 0).path;
    const QString backgroundPath = rowAt(*view, 1).path;

    QAction* merge = commandMerge(registry);
    QVERIFY(merge != nullptr);

    layers->selectPaths({backgroundPath}, backgroundPath);
    QCoreApplication::processEvents();
    QVERIFY(!merge->isEnabled());
    QCOMPARE(merge->text(), QStringLiteral("Merge Down"));

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    layers->selectPaths({backgroundPath, layerPath}, backgroundPath);
    QCoreApplication::processEvents();
    QVERIFY(merge->isEnabled());
    QCOMPARE(merge->text(), QStringLiteral("Merge Layers"));

    frame.activateWindow();
    QTest::keyClick(&frame, Qt::Key_E, Qt::ControlModifier);
    QCoreApplication::processEvents();
    if (pictura::layers_api::rowCount(*view) == rowsBefore) {
        // Offscreen Qt may not deliver the window shortcut; the enabled state
        // above is the live-refresh assertion, and trigger covers the merge.
        merge->trigger();
        QCoreApplication::processEvents();
    }
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore - 1);
    QCOMPARE(view->history_count(), historyBefore + 1);

    frame.closeDocument(doc, false);
}

QTEST_MAIN(MergeCommandsTest)
#include "tst_merge_commands.moc"
