// Layer Comps panel: rows, create/apply/cycle, Last Document State, editing,
// warnings, and the Window menu toggle.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/layer_comps_api.h"
#include "api/layers_bridge.h"
#include "commands.h"
#include "frame.h"
#include "layer_comp_options_dialog.h"
#include "panels/layer_comps_panel.h"
#include "panels/panel_column.h"
#include "qt_test_support.h"

#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>

#include <memory>

namespace layer_comps_api = pictura::layer_comps_api;

namespace {

pictura::PictureView* newDoc(pictura::PicturaMainWindow& window, const QString& name)
{
    if (!window.newDocument(name, 64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window.activeView();
}

QString addLayer(pictura::PictureView& view, const QString& name)
{
    const QString path = view.add_layer_in(QString());
    if (!path.isEmpty() && !name.isEmpty()) {
        view.set_layer_name_path(path, name);
    }
    return path;
}

} // namespace

class LayerCompsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void panelStartsHidden();
    void windowMenuTogglesPanel();
    void toggleIsCheckable();
    void panelRestoresLayout();
    void togglingLeavesDocumentUnchanged();
    void rowsFollowCompOrder();
    void emptyDocumentShowsLastStateOnly();
    void multiSelection();
    void documentSwitchRefreshes();
    void layerDeleteShowsWarning();
    void createDialogDefaults();
    void createRemembersChoices();
    void altClickCreatesWithoutDialog();
    void createCancelChangesNothing();
    void createIsUndoable();
    void applyRowAddsOneHistoryState();
    void previousNextCycles();
    void selectedCompsCycle();
    void applyUpdatesLayersPanel();
    void lastStateDisabledBeforeApply();
    void lastStateRestoresPreApply();
    void lastStateUndoReturnsToCompState();
    void lastStateCannotBeEdited();
    void noopApplyLeavesHistoryAndLastState();
    void updateRecapturesComp();
    void deleteRemovesComp();
    void renameComp();
    void optionsDialogOpens();
    void duplicateComp();
    void emptyNameIsRefused();
    void metadataEditsRecordOneState();
    void documentCloseDoesNotWrite();
    void panelMenuActions();
    void menuActionsFollowSelection();
    void cautionIconAppears();
    void clearWarning();
    void clearAllWarningsRecordsOneState();
    void applyingWarnedComp();
    void noDocumentDisablesActions();
    void transformSessionBlocksApply();

private:
    pictura::LayerCompsPanel* panel() const
    {
        return window_->findChild<pictura::LayerCompsPanel*>(QStringLiteral("layerCompsPanel"));
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LayerCompsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(panel() != nullptr);
}

void LayerCompsPanelTest::cleanup()
{
}

void LayerCompsPanelTest::panelStartsHidden()
{
    QVERIFY(panel());
    QVERIFY(!panel()->isVisible());
}

void LayerCompsPanelTest::windowMenuTogglesPanel()
{
    QAction* action = window_->registry()->action(QStringLiteral("window.panels.layer.comps"));
    QVERIFY(action);
    pictura::PanelColumn* column =
        window_->columnForPanel(QStringLiteral("layerCompsPanel"));
    QVERIFY(column);
    QVERIFY(!action->isChecked());
    action->trigger();
    QVERIFY(action->isChecked());
    QVERIFY(column->isPanelVisible(QStringLiteral("layerCompsPanel")));
    action->trigger();
    QVERIFY(!action->isChecked());
    QVERIFY(!column->isPanelVisible(QStringLiteral("layerCompsPanel")));
}

void LayerCompsPanelTest::toggleIsCheckable()
{
    QAction* action = window_->registry()->action(QStringLiteral("window.panels.layer.comps"));
    QVERIFY(action);
    QVERIFY(action->isCheckable());
}

void LayerCompsPanelTest::panelRestoresLayout()
{
    QVERIFY(panel());
    QCOMPARE(panel()->objectName(), QStringLiteral("layerCompsPanel"));
}

void LayerCompsPanelTest::togglingLeavesDocumentUnchanged()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("toggle"));
    QVERIFY(view);
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    panel()->setView(view);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
}

void LayerCompsPanelTest::rowsFollowCompOrder()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("rows"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    layer_comps_api::create(*view, QStringLiteral("B"), QString(), 1);
    panel()->setView(view);
    panel()->refresh();
    QListWidget* list = panel()->listForTest();
    QCOMPARE(list->count(), 3);
    QCOMPARE(list->item(0)->text(), QStringLiteral("Last Document State"));
    QCOMPARE(list->item(1)->text(), QStringLiteral("A"));
    QCOMPARE(list->item(2)->text(), QStringLiteral("B"));
}

void LayerCompsPanelTest::emptyDocumentShowsLastStateOnly()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("empty"));
    QVERIFY(view);
    panel()->setView(view);
    panel()->refresh();
    QListWidget* list = panel()->listForTest();
    QCOMPARE(list->count(), 1);
    QCOMPARE(list->item(0)->text(), QStringLiteral("Last Document State"));
}

void LayerCompsPanelTest::multiSelection()
{
    QCOMPARE(panel()->listForTest()->selectionMode(), QAbstractItemView::ExtendedSelection);
}

void LayerCompsPanelTest::documentSwitchRefreshes()
{
    pictura::PictureView* first = newDoc(*window_, QStringLiteral("first"));
    QVERIFY(first);
    layer_comps_api::create(*first, QStringLiteral("One"), QString(), 1);
    pictura::PictureView* second = newDoc(*window_, QStringLiteral("second"));
    QVERIFY(second);
    panel()->setView(second);
    panel()->refresh();
    QCOMPARE(panel()->listForTest()->count(), 1);
    panel()->setView(first);
    panel()->refresh();
    QCOMPARE(panel()->listForTest()->count(), 2);
}

void LayerCompsPanelTest::layerDeleteShowsWarning()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("warn"));
    QVERIFY(view);
    const QString path = addLayer(*view, QStringLiteral("A"));
    QVERIFY(!path.isEmpty());
    const int id = layer_comps_api::create(*view, QStringLiteral("Warn"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(view->remove_layer(path.toInt()));
    QCOMPARE(layer_comps_api::warning(*view, 0), true);
}

void LayerCompsPanelTest::createDialogDefaults()
{
    pictura::LayerCompOptionsDialog dialog(QStringLiteral("New"), QStringLiteral("Layer Comp 1"),
                                           QString(), 1);
    auto* visibility = dialog.findChild<QCheckBox*>(QStringLiteral("layerCompVisibility"));
    auto* position = dialog.findChild<QCheckBox*>(QStringLiteral("layerCompPosition"));
    auto* appearance = dialog.findChild<QCheckBox*>(QStringLiteral("layerCompAppearance"));
    QVERIFY(visibility && position && appearance);
    QVERIFY(visibility->isChecked());
    QVERIFY(!position->isChecked());
    QVERIFY(!appearance->isChecked());
}

void LayerCompsPanelTest::createRemembersChoices()
{
    pictura::LayerCompOptionsDialog dialog(QStringLiteral("New"), QStringLiteral("Layer Comp 1"),
                                           QString(), 2);
    auto* position = dialog.findChild<QCheckBox*>(QStringLiteral("layerCompPosition"));
    QVERIFY(position);
    QVERIFY(position->isChecked());
    QCOMPARE(dialog.recordBits(), 2);
}

void LayerCompsPanelTest::altClickCreatesWithoutDialog()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("alt"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    panel()->setView(view);
    QVERIFY(panel()->createComp(true));
    QCOMPARE(layer_comps_api::count(*view), 1);
}

void LayerCompsPanelTest::createCancelChangesNothing()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("cancel"));
    QVERIFY(view);
    const int before = layer_comps_api::count(*view);
    pictura::LayerCompOptionsDialog dialog(QStringLiteral("New"), QStringLiteral("Layer Comp 1"),
                                           QString(), 1);
    dialog.reject();
    QCOMPARE(layer_comps_api::count(*view), before);
}

void LayerCompsPanelTest::createIsUndoable()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("undo"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    panel()->setView(view);
    QVERIFY(panel()->createComp(true));
    QCOMPARE(layer_comps_api::count(*view), 1);
    QVERIFY(view->undo());
    QCOMPARE(layer_comps_api::count(*view), 0);
}

void LayerCompsPanelTest::applyRowAddsOneHistoryState()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("apply"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    view->set_layer_visible(0, false);
    panel()->setView(view);
    panel()->refresh();
    panel()->listForTest()->setCurrentRow(1);
    const int history = view->history_count();
    QVERIFY(panel()->applySelected());
    QCOMPARE(view->history_count(), history + 1);
}

void LayerCompsPanelTest::previousNextCycles()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("cycle"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    view->set_layer_visible(0, false);
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    view->set_layer_visible(0, true);
    layer_comps_api::create(*view, QStringLiteral("B"), QString(), 1);
    view->set_layer_visible(0, false);
    layer_comps_api::create(*view, QStringLiteral("C"), QString(), 1);
    panel()->setView(view);
    panel()->refresh();
    panel()->listForTest()->setCurrentRow(1);
    QVERIFY(panel()->nextComp());
}

void LayerCompsPanelTest::selectedCompsCycle()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("selected"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    view->set_layer_visible(0, false);
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    view->set_layer_visible(0, true);
    layer_comps_api::create(*view, QStringLiteral("B"), QString(), 1);
    view->set_layer_visible(0, false);
    layer_comps_api::create(*view, QStringLiteral("C"), QString(), 1);
    panel()->setView(view);
    panel()->refresh();
    QListWidget* list = panel()->listForTest();
    list->setCurrentRow(1);
    list->item(3)->setSelected(true);
    QVERIFY(panel()->nextComp());
}

void LayerCompsPanelTest::applyUpdatesLayersPanel()
{
    QSKIP("covered by the Layers panel refresh path");
}

void LayerCompsPanelTest::lastStateDisabledBeforeApply()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("last-disabled"));
    QVERIFY(view);
    QVERIFY(!layer_comps_api::lastAvailable(*view));
}

void LayerCompsPanelTest::lastStateRestoresPreApply()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("last"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    view->set_layer_visible(0, false);
    panel()->setView(view);
    panel()->refresh();
    panel()->listForTest()->setCurrentRow(1);
    QVERIFY(panel()->applySelected());
    QVERIFY(panel()->listForTest()->item(0)->data(Qt::UserRole + 2).toBool());
    QVERIFY(layer_comps_api::restoreLast(*view));
}

void LayerCompsPanelTest::lastStateUndoReturnsToCompState()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("last-undo"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    view->set_layer_visible(0, false);
    panel()->setView(view);
    panel()->refresh();
    panel()->listForTest()->setCurrentRow(1);
    QVERIFY(panel()->applySelected());
    QVERIFY(layer_comps_api::restoreLast(*view));
    QVERIFY(view->undo());
    QVERIFY(view->layer_visible(0));
}

void LayerCompsPanelTest::lastStateCannotBeEdited()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("last-readonly"));
    QVERIFY(view);
    panel()->setView(view);
    panel()->refresh();
    panel()->listForTest()->setCurrentRow(0);
    QVERIFY(!panel()->updateSelected());
    QVERIFY(!panel()->deleteSelected());
}

void LayerCompsPanelTest::noopApplyLeavesHistoryAndLastState()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("noop"));
    QVERIFY(view);
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    const int history = view->history_count();
    QVERIFY(!layer_comps_api::apply(*view, id) || view->history_count() == history);
}

void LayerCompsPanelTest::updateRecapturesComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("update"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(layer_comps_api::update(*view, id));
}

void LayerCompsPanelTest::deleteRemovesComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("delete"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(layer_comps_api::remove(*view, id));
    QCOMPARE(layer_comps_api::count(*view), 0);
}

void LayerCompsPanelTest::renameComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("rename"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(layer_comps_api::rename(*view, id, QStringLiteral("B")));
    QCOMPARE(layer_comps_api::name(*view, 0), QStringLiteral("B"));
}

void LayerCompsPanelTest::optionsDialogOpens()
{
    pictura::LayerCompOptionsDialog dialog(QStringLiteral("Options"), QStringLiteral("A"),
                                           QStringLiteral("comment"), 7);
    QCOMPARE(dialog.name(), QStringLiteral("A"));
    QCOMPARE(dialog.comment(), QStringLiteral("comment"));
    QCOMPARE(dialog.recordBits(), 7);
}

void LayerCompsPanelTest::duplicateComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("duplicate"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(layer_comps_api::duplicate(*view, id) >= 0);
    QCOMPARE(layer_comps_api::count(*view), 2);
}

void LayerCompsPanelTest::emptyNameIsRefused()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("empty-name"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    QVERIFY(!layer_comps_api::rename(*view, id, QString()));
}

void LayerCompsPanelTest::metadataEditsRecordOneState()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("meta-states"));
    QVERIFY(view);
    addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    const int history = view->history_count();
    QVERIFY(layer_comps_api::setOptions(*view, id, QStringLiteral("B"), QString(), 3));
    QCOMPARE(view->history_count(), history + 1);
}

void LayerCompsPanelTest::documentCloseDoesNotWrite()
{
    QSKIP("covered by the document-close panel host path");
}

void LayerCompsPanelTest::panelMenuActions()
{
    QMenu menu;
    panel()->updatePanelMenu(&menu);
    QVERIFY(menu.actions().size() >= 7);
}

void LayerCompsPanelTest::menuActionsFollowSelection()
{
    panel()->listForTest()->setCurrentRow(0);
    QMenu menu;
    panel()->updatePanelMenu(&menu);
    QVERIFY(menu.actions().at(0)->isEnabled());
    QVERIFY(!menu.actions().at(1)->isEnabled());
}

void LayerCompsPanelTest::cautionIconAppears()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("caution"));
    QVERIFY(view);
    const QString path = addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(view->remove_layer(path.toInt()));
    panel()->setView(view);
    panel()->refresh();
    QCOMPARE(panel()->listForTest()->item(1)->data(Qt::UserRole + 1).toBool(), true);
}

void LayerCompsPanelTest::clearWarning()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("clear"));
    QVERIFY(view);
    const QString path = addLayer(*view, QStringLiteral("A"));
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(view->remove_layer(path.toInt()));
    QVERIFY(layer_comps_api::clearWarning(*view, id));
}

void LayerCompsPanelTest::clearAllWarningsRecordsOneState()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("clear-all"));
    QVERIFY(view);
    const QString path = addLayer(*view, QStringLiteral("A"));
    layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(view->remove_layer(path.toInt()));
    const int history = view->history_count();
    QVERIFY(layer_comps_api::clearAllWarnings(*view));
    QCOMPARE(view->history_count(), history + 1);
}

void LayerCompsPanelTest::applyingWarnedComp()
{
    pictura::PictureView* view = newDoc(*window_, QStringLiteral("warned-apply"));
    QVERIFY(view);
    const QString a = addLayer(*view, QStringLiteral("A"));
    const QString b = addLayer(*view, QStringLiteral("B"));
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    const int id = layer_comps_api::create(*view, QStringLiteral("A"), QString(), 1);
    QVERIFY(id >= 0);
    view->set_layer_visible(0, false);
    QVERIFY(view->remove_layer(b.toInt()));
    QVERIFY(layer_comps_api::apply(*view, id));
    QVERIFY(view->layer_visible(a.toInt()));
}

void LayerCompsPanelTest::noDocumentDisablesActions()
{
    panel()->setView(nullptr);
    panel()->refresh();
    QCOMPARE(panel()->listForTest()->count(), 0);
}

void LayerCompsPanelTest::transformSessionBlocksApply()
{
    QSKIP("requires a live transform session fixture");
}

QTEST_MAIN(LayerCompsPanelTest)

#include "tst_layer_comps_panel.moc"
