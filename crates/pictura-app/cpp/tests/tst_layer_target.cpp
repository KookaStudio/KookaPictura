// The document layer target (#741): selected paths + anchor, its transitions,
// and the panel/property consumers that read it.

#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QCoreApplication>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QStringList>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTreeView>

#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipboard.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/auto_select.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_arrange.cxxqt.h"

#include "qt_test_support.h"

namespace {

QString nameAtPath(pictura::PictureView& view, const QString& path)
{
    for (int row = 0; row < pictura::layers_api::rowCount(view); ++row) {
        const auto entry = pictura::layers_api::rowAt(view, row);
        if (entry.path == path) {
            return entry.name;
        }
    }
    return {};
}

QString topPath(pictura::PictureView& view)
{
    return pictura::layers_api::rowCount(view) > 0 ? pictura::layers_api::rowAt(view, 0).path
                                                   : QString();
}

QString pathNamed(pictura::PictureView& view, const QString& name)
{
    for (int row = 0; row < pictura::layers_api::rowCount(view); ++row) {
        const auto entry = pictura::layers_api::rowAt(view, row);
        if (entry.name == name) {
            return entry.path;
        }
    }
    return {};
}

} // namespace

class LayerTargetTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void panelSelectionUpdatesTarget();
    void multiSelectionKeepsAnchor();
    void multiSelectionHasNoEditTarget();
    void deselectClearsTarget();
    void deselectSurvivesRefresh();
    void userDeselectClearsCurrentRow();
    void selectionDoesNotDirtyOrRecord();
    void singleSelectionIsEditTarget();
    void zeroOrMultiSelectionRefusesToolEdit();
    void multiSelectionKeepsStructuralCommands();
    void multiSelectionIsShownInFull();
    void repeatedRefreshKeepsTarget();
    void filterHidingTargetKeepsIt();
    void filteredTargetCommands();
    void moveAutoSelectHiddenLayer();
    void createdNodesBecomeTarget();
    void duplicateBecomesTarget();
    void deleteSelectsSurvivor();
    void groupAndUngroupSelectResult();
    void mergeSelectsResult();
    void reorderKeepsLayerSelected();
    void backgroundFromLayerKeepsTarget();
    void crossDocumentCopySelectsCopy();
    void refusedOperationLeavesTarget();
    void undoFirstEditAfterOpen();
    void undoRestoresStructuralTarget();
    void undoKeepsLaterSelection();
    void redoRestoresPostOperationTarget();
    void namedSnapshotRestoresTarget();
    void historyJumpRestoresTarget();
    void pasteBecomesTargetAndUndo();
    void moveToSelectsMovedLayer();
    void arrangeAndReverseKeepTarget();
    void mergeVisibleAndFlatten();
    void emptySelectPathsClearsTarget();
    void invalidActivePathClearsTarget();
    void documentSwitchRestoresTarget();
    void newDocumentSelectsFirstLayer();
    void refreshDoesNotRewriteTheTarget();
    void currentRowFollowsTheTarget();
    void selectSimilarKeepsActiveSelected();
    void selectLinkedSelectsMembers();
    void documentSwitchCurrentRow();
    void clickingEmptySpaceClearsCurrentRow();

private:
    bool newDoc(const QString& name, int layers);
    void select(const QStringList& paths, const QString& current);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayerTargetTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayerTargetTest::cleanup()
{
    if (window_ && window_->activeView()) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool LayerTargetTest::newDoc(const QString& name, int layers)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_) {
        return false;
    }
    while (view_->layer_count() < layers) {
        if (view_->add_layer_in(QString()).isEmpty()) {
            return false;
        }
    }
    panel_->refresh();
    QCoreApplication::processEvents();
    return true;
}

void LayerTargetTest::select(const QStringList& paths, const QString& current)
{
    panel_->selectPaths(paths, current);
    QCoreApplication::processEvents();
}

// lt_single / ltf_active_layer_push
void LayerTargetTest::panelSelectionUpdatesTarget()
{
    QVERIFY(newDoc(QStringLiteral("Single"), 2));
    QVERIFY(panel_->selectRowForTest(QStringLiteral("1")));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("1")});
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
    QCOMPARE(view_->active_layer_path(), QStringLiteral("1"));
}

// lt_multi
void LayerTargetTest::multiSelectionKeepsAnchor()
{
    QVERIFY(newDoc(QStringLiteral("Multi"), 3));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_edit_target_set(*view_, QStringLiteral("0")));
    select({QStringLiteral("0"), QStringLiteral("2")}, QStringLiteral("2"));
    QCOMPARE(pictura::selected_layer_paths(*view_),
             (QStringList{QStringLiteral("0"), QStringLiteral("2")}));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("2"));
    QVERIFY(pictura::mask_edit_target_get(*view_).isEmpty());
}

// lt_edit_multi
void LayerTargetTest::multiSelectionHasNoEditTarget()
{
    QVERIFY(newDoc(QStringLiteral("NoEdit"), 2));
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(view_->active_layer_path().isEmpty());
}

// lt_deselect
void LayerTargetTest::deselectClearsTarget()
{
    QVERIFY(newDoc(QStringLiteral("Deselect"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    pictura::clear_layer_target(*view_);
    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(pictura::layer_selection_anchor(*view_).isEmpty());
}

// lpr_active_deselect
void LayerTargetTest::deselectSurvivesRefresh()
{
    QVERIFY(newDoc(QStringLiteral("DeselectRefresh"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    pictura::clear_layer_target(*view_);
    panel_->refresh();
    panel_->refresh();
    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(panel_->selectedPaths().isEmpty());
}

// lt_deselect: a user-driven deselect clears the target and the tree's current
// row with it, so no row keeps an active highlight the document does not have.
void LayerTargetTest::userDeselectClearsCurrentRow()
{
    QVERIFY(newDoc(QStringLiteral("UserDeselect"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QVERIFY(tree->currentIndex().isValid());

    tree->selectionModel()->clearSelection();
    QCoreApplication::processEvents();

    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(pictura::layer_selection_anchor(*view_).isEmpty());
    QVERIFY(panel_->selectedPaths().isEmpty());
    QVERIFY(!tree->currentIndex().isValid());
    QVERIFY(panel_->currentPath().isEmpty());
}

// lt_no_history
void LayerTargetTest::selectionDoesNotDirtyOrRecord()
{
    QVERIFY(newDoc(QStringLiteral("Clean"), 2));
    const int history = view_->history_count();
    const bool dirty = view_->is_dirty();
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("1"));
    QCOMPARE(view_->history_count(), history);
    QCOMPARE(view_->is_dirty(), dirty);
}

// ltf_active_layer
void LayerTargetTest::singleSelectionIsEditTarget()
{
    QVERIFY(newDoc(QStringLiteral("EditTarget"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QCOMPARE(view_->active_layer_path(), QStringLiteral("1"));
    QVERIFY(view_->active_layer_visible());
}

// ltf_active_layer_refuse
void LayerTargetTest::zeroOrMultiSelectionRefusesToolEdit()
{
    QVERIFY(newDoc(QStringLiteral("Refuse"), 2));
    pictura::clear_layer_target(*view_);
    QVERIFY(view_->active_layer_path().isEmpty());
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("0"));
    QVERIFY(view_->active_layer_path().isEmpty());
}

// ltf_multi_commands
void LayerTargetTest::multiSelectionKeepsStructuralCommands()
{
    QVERIFY(newDoc(QStringLiteral("Commands"), 3));
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList selected = pictura::selected_layer_paths(*view_);
    QCOMPARE(selected.size(), 2);
    QVERIFY(view_->can_merge_layers(selected));
    QVERIFY(view_->active_layer_path().isEmpty());
}

// lpr_multi_visual
void LayerTargetTest::multiSelectionIsShownInFull()
{
    QVERIFY(newDoc(QStringLiteral("Visual"), 3));
    select({QStringLiteral("0"), QStringLiteral("2")}, QStringLiteral("2"));
    QCOMPARE(panel_->selectedPaths().size(), 2);
    QCOMPARE(panel_->currentPath(), QStringLiteral("2"));
}

void LayerTargetTest::repeatedRefreshKeepsTarget()
{
    QVERIFY(newDoc(QStringLiteral("Repeated"), 3));
    select({QStringLiteral("0"), QStringLiteral("2")}, QStringLiteral("2"));
    for (int i = 0; i < 3; ++i) {
        panel_->refresh();
        QCoreApplication::processEvents();
    }
    QCOMPARE(pictura::selected_layer_paths(*view_),
             (QStringList{QStringLiteral("0"), QStringLiteral("2")}));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("2"));
}

// lpr_filter_target
void LayerTargetTest::filterHidingTargetKeepsIt()
{
    QVERIFY(newDoc(QStringLiteral("FilterTarget"), 3));
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    panel_->refresh();
    const QString target = pictura::layers_api::rowAt(*view_, 0).path;
    select({target}, target);
    panel_->setFilterKindForTest({QStringLiteral("pixel")}, true);
    QCoreApplication::processEvents();
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{target});
    QCOMPARE(pictura::layer_selection_anchor(*view_), target);

    // Ctrl+arrow onto a displayed row must not rewrite the hidden target with
    // the rows the filter leaves visible.
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    const QModelIndex shown = tree->model()->index(0, 0);
    QVERIFY(shown.isValid());
    tree->selectionModel()->setCurrentIndex(shown, QItemSelectionModel::NoUpdate);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{target});
    QCOMPARE(pictura::layer_selection_anchor(*view_), target);

    panel_->setFilterKindForTest(QStringList{}, false);
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(panel_->selectedPaths(), QStringList{target});
}

// lpr_filter_command
void LayerTargetTest::filteredTargetCommands()
{
    QVERIFY(newDoc(QStringLiteral("FilterCommand"), 3));
    select({QStringLiteral("1"), QStringLiteral("2")}, QStringLiteral("2"));
    panel_->setFilterKindForTest({QStringLiteral("adjustment")}, true);
    QCoreApplication::processEvents();
    panel_->deleteSelection();
    QCoreApplication::processEvents();
    QCOMPARE(view_->layer_count(), 1);
}

// lt_autoselect_hidden
void LayerTargetTest::moveAutoSelectHiddenLayer()
{
    QVERIFY(newDoc(QStringLiteral("AutoSelect"), 2));
    const QString painted = view_->add_solid_fill(0xff3366ffu);
    QVERIFY(!painted.isEmpty());
    QVERIFY(view_->rasterize_fill_content(painted));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    panel_->setFilterKindForTest({QStringLiteral("adjustment")}, true);
    QCoreApplication::processEvents();
    const QString hit = pictura::auto_select_layer(*view_, 8, 8, false);
    QCOMPARE(hit, painted);
    window_->selectLayerPath(hit);
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layer_selection_anchor(*view_), painted);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{painted});
}

// lt_op_create
void LayerTargetTest::createdNodesBecomeTarget()
{
    QVERIFY(newDoc(QStringLiteral("Create"), 1));
    const QString layer = view_->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{layer});
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{group});
}

// lt_op_duplicate
void LayerTargetTest::duplicateBecomesTarget()
{
    QVERIFY(newDoc(QStringLiteral("Duplicate"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList created = view_->duplicate_layers({QStringLiteral("1")});
    QCOMPARE(created.size(), 1);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{created.first()});
}

// lt_op_delete
void LayerTargetTest::deleteSelectsSurvivor()
{
    QVERIFY(newDoc(QStringLiteral("Delete"), 3));
    const QString below = nameAtPath(*view_, QStringLiteral("0"));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QCOMPARE(view_->delete_layers({QStringLiteral("1")}), 1);
    QCOMPARE(view_->layer_count(), 2);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
    QCOMPARE(nameAtPath(*view_, QStringLiteral("0")), below);
}

// lt_op_group
void LayerTargetTest::groupAndUngroupSelectResult()
{
    QVERIFY(newDoc(QStringLiteral("Group"), 4));
    const QString outsider = nameAtPath(*view_, QStringLiteral("3"));
    select({QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("3")}, QStringLiteral("3"));
    const QString group = view_->group_layers({QStringLiteral("0"), QStringLiteral("1")});
    QVERIFY(!group.isEmpty());
    QStringList selected = pictura::selected_layer_paths(*view_);
    QCOMPARE(selected.size(), 2);
    QVERIFY(selected.contains(group));
    QVERIFY(selected.contains(pathNamed(*view_, outsider)));
    QCOMPARE(view_->ungroup_layers({group}), 1);
    selected = pictura::selected_layer_paths(*view_);
    QCOMPARE(selected.size(), 3);
    QVERIFY(selected.contains(QStringLiteral("0")));
    QVERIFY(selected.contains(QStringLiteral("1")));
    QVERIFY(selected.contains(pathNamed(*view_, outsider)));
}

// lt_op_merge
void LayerTargetTest::mergeSelectsResult()
{
    QVERIFY(newDoc(QStringLiteral("Merge"), 3));
    select({QStringLiteral("1"), QStringLiteral("2")}, QStringLiteral("2"));
    const QString merged = view_->merge_layers({QStringLiteral("1"), QStringLiteral("2")});
    QVERIFY(!merged.isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{merged});
    QCOMPARE(view_->layer_count(), 2);
}

// lt_op_reorder
void LayerTargetTest::reorderKeepsLayerSelected()
{
    QVERIFY(newDoc(QStringLiteral("Reorder"), 3));
    const QString name = nameAtPath(*view_, QStringLiteral("1"));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(view_->move_layer_path(QStringLiteral("1"), 1));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("2"));
    QCOMPARE(nameAtPath(*view_, QStringLiteral("2")), name);
}

// lt_op_background
void LayerTargetTest::backgroundFromLayerKeepsTarget()
{
    QVERIFY(newDoc(QStringLiteral("Background"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(view_->background_from_layer(QStringLiteral("1")));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
}

// lt_op_cross_document
void LayerTargetTest::crossDocumentCopySelectsCopy()
{
    QVERIFY(newDoc(QStringLiteral("Source"), 1));
    pictura::PictureView* source = view_;
    const QString sourcePath = topPath(*view_);
    QVERIFY(newDoc(QStringLiteral("Destination"), 1));
    const int before = view_->layer_count();
    QVERIFY(window_->copyLayerFromDocument(source, sourcePath));
    QCOMPARE(view_->layer_count(), before + 1);
    QCOMPARE(pictura::selected_layer_paths(*view_),
             QStringList{topPath(*view_)});
}

// lt_op_refused
void LayerTargetTest::refusedOperationLeavesTarget()
{
    QVERIFY(newDoc(QStringLiteral("Refused"), 1));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QCOMPARE(view_->delete_layers({QStringLiteral("0")}), 0);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("0")});
}

// lt_undo_first_edit
void LayerTargetTest::undoFirstEditAfterOpen()
{
    QVERIFY(newDoc(QStringLiteral("FirstEdit"), 1));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
    const QString created = view_->add_layer_in(QString());
    QVERIFY(!created.isEmpty());
    QCOMPARE(pictura::layer_selection_anchor(*view_), created);
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
}

// lt_undo_structural
void LayerTargetTest::undoRestoresStructuralTarget()
{
    QVERIFY(newDoc(QStringLiteral("Undo"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(!view_->duplicate_layers({QStringLiteral("1")}).isEmpty());
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));

    QCOMPARE(view_->delete_layers({QStringLiteral("1")}), 1);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
}

// lt_undo_pixel (structural stand-in: the target changed after the edit)
void LayerTargetTest::undoKeepsLaterSelection()
{
    QVERIFY(newDoc(QStringLiteral("UndoLater"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(!view_->duplicate_layers({QStringLiteral("1")}).isEmpty());
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
}

// lt_redo
void LayerTargetTest::redoRestoresPostOperationTarget()
{
    QVERIFY(newDoc(QStringLiteral("Redo"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList created = view_->duplicate_layers({QStringLiteral("1")});
    QVERIFY(view_->undo());
    QVERIFY(view_->redo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), created.first());
}

// lt_undo_named_snapshot
void LayerTargetTest::namedSnapshotRestoresTarget()
{
    QVERIFY(newDoc(QStringLiteral("Snapshot"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    QVERIFY(view_->history_add_snapshot(QStringLiteral("Named")));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QVERIFY(view_->history_restore_snapshot(0));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
    const QString created = view_->add_layer_in(QString());
    QVERIFY(!created.isEmpty());
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
}


void LayerTargetTest::historyJumpRestoresTarget()
{
    QVERIFY(newDoc(QStringLiteral("Jump"), 1));
    QVERIFY(!view_->add_layer_in(QString()).isEmpty());
    QVERIFY(!view_->add_layer_in(QString()).isEmpty());
    QVERIFY(view_->history_jump(0));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
    QVERIFY(view_->history_jump(2));
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("2"));
}

void LayerTargetTest::pasteBecomesTargetAndUndo()
{
    QVERIFY(newDoc(QStringLiteral("Paste"), 2));
    // A copy needs pixels: rasterize a solid fill into a layer of its own.
    const QString filled = view_->add_solid_fill(0xff2244aau);
    QVERIFY(!filled.isEmpty());
    QVERIFY(view_->rasterize_fill_content(filled));
    select({filled}, filled);
    QVERIFY(pictura::clipboard_copy(*view_, filled, false));
    const QString pasted =
        pictura::clipboard_paste(*view_, filled, pictura::PasteKind::Plain, 0.0, 0.0);
    QVERIFY(!pasted.isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{pasted});
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_selection_anchor(*view_), filled);
    QVERIFY(view_->redo());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{pasted});
}

void LayerTargetTest::moveToSelectsMovedLayer()
{
    QVERIFY(newDoc(QStringLiteral("MoveTo"), 3));
    const QString moved = nameAtPath(*view_, QStringLiteral("0"));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QVERIFY(view_->move_layer_to(QStringLiteral("0"), QString(), 0));
    QCOMPARE(nameAtPath(*view_, QStringLiteral("2")), moved);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("2")});
}

void LayerTargetTest::arrangeAndReverseKeepTarget()
{
    QVERIFY(newDoc(QStringLiteral("Arrange"), 3));
    const QString front = nameAtPath(*view_, QStringLiteral("0"));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    QVERIFY(pictura::arrange_layer_run(*view_, QStringLiteral("0"), 0));
    QCOMPARE(nameAtPath(*view_, QStringLiteral("2")), front);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("2")});

    QVERIFY(newDoc(QStringLiteral("Reverse"), 4));
    const QString bottom = nameAtPath(*view_, QStringLiteral("0"));
    const QString next = nameAtPath(*view_, QStringLiteral("1"));
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("0"));
    QVERIFY(pictura::reverse_layers_run(*view_,
                                        QStringList{QStringLiteral("0"), QStringLiteral("1")}));
    QCOMPARE(nameAtPath(*view_, QStringLiteral("0")), next);
    QCOMPARE(nameAtPath(*view_, QStringLiteral("1")), bottom);
    const QStringList selected = pictura::selected_layer_paths(*view_);
    QCOMPARE(selected.size(), 2);
    QVERIFY(selected.contains(QStringLiteral("0")));
    QVERIFY(selected.contains(QStringLiteral("1")));
}

void LayerTargetTest::mergeVisibleAndFlatten()
{
    QVERIFY(newDoc(QStringLiteral("MergeVisible"), 3));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    const QString merged = view_->merge_visible(QStringLiteral("0"));
    QVERIFY(!merged.isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{merged});

    QVERIFY(newDoc(QStringLiteral("Flatten"), 3));
    QVERIFY(view_->flatten_image() > 0);
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("0")});
}

void LayerTargetTest::emptySelectPathsClearsTarget()
{
    QVERIFY(newDoc(QStringLiteral("EmptySelect"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->selectPaths({}, {});
    QCoreApplication::processEvents();
    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(panel_->selectedPaths().isEmpty());
    panel_->refresh();
    QVERIFY(panel_->selectedPaths().isEmpty());
}

void LayerTargetTest::invalidActivePathClearsTarget()
{
    QVERIFY(newDoc(QStringLiteral("InvalidActive"), 2));
    view_->set_active_layer(QStringLiteral("9"));
    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(pictura::layer_selection_anchor(*view_).isEmpty());
}

// lt_switch
void LayerTargetTest::documentSwitchRestoresTarget()
{
    QVERIFY(newDoc(QStringLiteral("A"), 2));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    const int first = window_->activeDocumentIndex();
    QVERIFY(newDoc(QStringLiteral("B"), 1));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    const int second = window_->activeDocumentIndex();
    window_->setActiveDocumentIndex(first);
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
    window_->setActiveDocumentIndex(second);
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
}

// lt_new
void LayerTargetTest::newDocumentSelectsFirstLayer()
{
    QVERIFY(newDoc(QStringLiteral("New"), 1));
    QCOMPARE(pictura::selected_layer_paths(*view_), QStringList{QStringLiteral("0")});
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));
}

void LayerTargetTest::refreshDoesNotRewriteTheTarget()
{
    QVERIFY(newDoc(QStringLiteral("NoRewrite"), 2));
    select({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList created = view_->duplicate_layers(
        {QStringLiteral("0"), QStringLiteral("1")});
    QCOMPARE(created.size(), 2);
    panel_->refresh();
    const QStringList target = pictura::selected_layer_paths(*view_);
    QCOMPARE(target.size(), 2);
    QVERIFY(target.contains(created[0]));
    QVERIFY(target.contains(created[1]));
    const QString anchor = pictura::layer_selection_anchor(*view_);
    panel_->refresh();
    QCOMPARE(pictura::selected_layer_paths(*view_), target);
    QCOMPARE(pictura::layer_selection_anchor(*view_), anchor);
}

void LayerTargetTest::currentRowFollowsTheTarget()
{
    QVERIFY(newDoc(QStringLiteral("CurrentRow"), 2));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    const QStringList created = view_->duplicate_layers({QStringLiteral("0")});
    QCOMPARE(created.size(), 1);
    const QString copy = created.first();
    panel_->refresh();
    QCoreApplication::processEvents();
    QCOMPARE(panel_->currentPath(), copy);

    auto* properties =
        window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    QVERIFY(properties != nullptr);
    QCOMPARE(properties->pathForTest(), copy);
}

void LayerTargetTest::selectSimilarKeepsActiveSelected()
{
    QVERIFY(newDoc(QStringLiteral("SelectSimilar"), 3));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList matched = view_->select_similar(QStringLiteral("1"));
    QVERIFY(!matched.contains(QStringLiteral("1")));
    QVERIFY(matched.size() >= 1);
    QStringList expected = matched;
    expected.prepend(QStringLiteral("1"));
    QCOMPARE(pictura::selected_layer_paths(*view_), expected);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
    panel_->refresh();
    const QStringList shown = panel_->selectedPaths();
    QCOMPARE(shown.size(), expected.size());
    for (const QString& path : expected) {
        QVERIFY(shown.contains(path));
    }
    QCOMPARE(panel_->currentPath(), QStringLiteral("1"));
}

void LayerTargetTest::selectLinkedSelectsMembers()
{
    QVERIFY(newDoc(QStringLiteral("SelectLinked"), 3));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("2")}, true), 2);
    select({QStringLiteral("0")}, QStringLiteral("0"));
    const QStringList linked = view_->select_linked(QStringLiteral("0"));
    QCOMPARE(linked.size(), 2);
    QCOMPARE(pictura::selected_layer_paths(*view_), linked);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("0"));

    select({QStringLiteral("1")}, QStringLiteral("1"));
    const QStringList before = pictura::selected_layer_paths(*view_);
    QVERIFY(view_->select_linked(QStringLiteral("1")).isEmpty());
    QCOMPARE(pictura::selected_layer_paths(*view_), before);
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("1"));
}

void LayerTargetTest::documentSwitchCurrentRow()
{
    QVERIFY(newDoc(QStringLiteral("SwitchA"), 3));
    select({QStringLiteral("1")}, QStringLiteral("1"));
    const int first = window_->activeDocumentIndex();

    QVERIFY(newDoc(QStringLiteral("SwitchB"), 2));
    select({QStringLiteral("0")}, QStringLiteral("0"));
    const int second = window_->activeDocumentIndex();

    window_->setActiveDocumentIndex(first);
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    QVERIFY(view_ != nullptr);
    panel_->setView(view_);
    panel_->refresh();
    QCOMPARE(panel_->currentPath(), QStringLiteral("1"));

    window_->setActiveDocumentIndex(second);
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    QVERIFY(view_ != nullptr);
    panel_->setView(view_);
    panel_->refresh();
    QCOMPARE(panel_->currentPath(), QStringLiteral("0"));
}

// lt_deselect: clicking the empty space below the rows is the panel's explicit
// deselect gesture; it leaves no current row behind.
void LayerTargetTest::clickingEmptySpaceClearsCurrentRow()
{
    QVERIFY(newDoc(QStringLiteral("EmptyClick"), 2));
    window_->show();
    QCoreApplication::processEvents();
    select({QStringLiteral("1")}, QStringLiteral("1"));
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QAbstractItemModel* model = tree->model();
    QVERIFY(model != nullptr);
    const QModelIndex last = model->index(model->rowCount() - 1, 0);
    QVERIFY(last.isValid());
    const int y = tree->visualRect(last).bottom() + 10;
    QVERIFY2(y < tree->viewport()->height(), "the tree leaves no empty space below the rows");

    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier, QPoint(10, y));
    QCoreApplication::processEvents();

    QVERIFY(pictura::selected_layer_paths(*view_).isEmpty());
    QVERIFY(pictura::layer_selection_anchor(*view_).isEmpty());
    QVERIFY(!tree->currentIndex().isValid());
    QVERIFY(panel_->currentPath().isEmpty());
}

QTEST_MAIN(LayerTargetTest)
#include "tst_layer_target.moc"
