// Layer-menu commands: Duplicate Layer follows the Layers panel selection,
// including nested layers and multi-selections.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>

#include "api/layers_api.h"

#include <memory>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeySequence>

#include "commands.h"
#include "frame.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "api/layers_bridge.h"

#include "qt_test_support.h"

namespace {

int rowOfName(pictura::PictureView& view, const QString& name)
{
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        if (pictura::layers_api::rowAt(view, i).name == name) {
            return i;
        }
    }
    return -1;
}

int countNamed(pictura::PictureView& view, const QString& name)
{
    int count = 0;
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        if (pictura::layers_api::rowAt(view, i).name == name) {
            ++count;
        }
    }
    return count;
}

} // namespace

class LayerCommandsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void duplicateNestedLayer();
    void duplicateMultiSelection();
    void duplicateWithoutSelection();
    void ctrlJCopiesSelection();
    void ctrlJDuplicatesGroup();
    void ctrlJDuplicatesLayerWithoutSelection();
    void ctrlJCutOutcomes();
    void solidFillCommandUsesForeground();
    void managementNoOpRecordsNoHistory();
    void managementCommandRecordsOneState();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LayerCommandsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayerCommandsTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void LayerCommandsTest::duplicateNestedLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("DuplicateNested"), 32, 32,
                              QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer);
    QAction* duplicate = registry->action(id);
    QVERIFY(duplicate != nullptr);

    const QString group = view->add_group_in(QString());
    const QString nested = view->add_layer_in(group);
    QVERIFY(!group.isEmpty() && !nested.isEmpty());
    QVERIFY(view->set_layer_name_path(nested, QStringLiteral("Nested")));
    layers->refresh();
    layers->expandForTest(group);
    layers->selectPaths({nested}, nested);
    registry->refresh();
    QVERIFY2(duplicate->isEnabled(), "enabled for a nested selection");

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    const int sourceRow = rowOfName(*view, QStringLiteral("Nested"));
    const int copyRow = rowOfName(*view, QStringLiteral("Nested copy"));
    QVERIFY2(sourceRow >= 0 && copyRow >= 0, "the nested copy exists");
    QCOMPARE(copyRow, sourceRow - 1);
    QCOMPARE(pictura::layers_api::rowAt(*view, copyRow).path.left(group.size() + 1),
             group + QLatin1Char('/'));
}

void LayerCommandsTest::duplicateMultiSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("DuplicateMulti"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer);
    QAction* duplicate = registry->action(id);
    QVERIFY(duplicate != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    QVERIFY(view->set_layer_name_path(pictura::layers_api::rowAt(*view, 0).path,
                                      QStringLiteral("Top")));
    QVERIFY(view->set_layer_name_path(pictura::layers_api::rowAt(*view, 1).path,
                                      QStringLiteral("Second")));
    layers->refresh();
    const QString topPath = pictura::layers_api::rowAt(*view, 0).path;
    const QString secondPath = pictura::layers_api::rowAt(*view, 1).path;
    layers->selectPaths({topPath, secondPath}, topPath);
    registry->refresh();
    QVERIFY2(duplicate->isEnabled(), "enabled for a multi-selection");

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 2);
    QCOMPARE(view->history_count(), historyBefore + 1);
    const int topRow = rowOfName(*view, QStringLiteral("Top"));
    const int topCopyRow = rowOfName(*view, QStringLiteral("Top copy"));
    const int secondRow = rowOfName(*view, QStringLiteral("Second"));
    const int secondCopyRow = rowOfName(*view, QStringLiteral("Second copy"));
    QVERIFY(topRow >= 0 && topCopyRow >= 0 && secondRow >= 0 && secondCopyRow >= 0);
    QCOMPARE(topCopyRow, topRow - 1);
    QCOMPARE(secondCopyRow, secondRow - 1);

    const QString topCopyPath = pictura::layers_api::rowAt(*view, topCopyRow).path;
    const QString secondCopyPath = pictura::layers_api::rowAt(*view, secondCopyRow).path;
    const QStringList target = pictura::selected_layer_paths(*view);
    QCOMPARE(target.size(), 2);
    QVERIFY2(target.contains(topCopyPath), "the top copy is targeted");
    QVERIFY2(target.contains(secondCopyPath), "the second copy is targeted");
    const QString anchor = pictura::layer_selection_anchor(*view);
    QVERIFY2(anchor == topCopyPath || anchor == secondCopyPath,
             "one copy is the active path");
    const QStringList shown = layers->selectedPaths();
    QCOMPARE(shown.size(), 2);
    QVERIFY2(shown.contains(topCopyPath), "the panel shows the top copy");
    QVERIFY2(shown.contains(secondCopyPath), "the panel shows the second copy");
}

void LayerCommandsTest::duplicateWithoutSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("DuplicateEmpty"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer);
    QAction* duplicate = registry->action(id);
    QVERIFY(duplicate != nullptr);

    layers->selectPaths({}, QString());
    registry->refresh();
    QVERIFY2(!duplicate->isEnabled(), "disabled with no selection");

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore);
    QCOMPARE(view->history_count(), historyBefore);
}

// Ctrl+J with a selection on a non-group target keeps the Layer via Copy
// behavior: a selection-shaped copy above the source, source untouched, and
// one undo state.
void LayerCommandsTest::ctrlJCopiesSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CtrlJCopy"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerNewLayerViaCopy);
    QAction* viaCopy = registry->action(id);
    QVERIFY(viaCopy != nullptr);

    QVERIFY(view->select_rect(0, 0, 2, 4, QStringLiteral("new"), 0.0));
    registry->refresh();
    QVERIFY2(viaCopy->isEnabled(), "enabled with a selection");
    QCOMPARE(viaCopy->shortcut(), QKeySequence(QStringLiteral("Ctrl+J")));
    QCOMPARE(viaCopy->text(), QStringLiteral("Layer via Copy"));

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    QCOMPARE(rowOfName(*view, QStringLiteral("Layer 0 copy")), 0);
    const QImage copy = view->layer_thumbnail(1, 4);
    const QImage source = view->layer_thumbnail(0, 4);
    QVERIFY(!copy.isNull() && !source.isNull());
    QCOMPARE(qAlpha(copy.pixel(0, 0)), 255);
    QCOMPARE(qAlpha(copy.pixel(3, 0)), 0);
    QCOMPARE(qAlpha(source.pixel(0, 0)), 255);
    QCOMPARE(qAlpha(source.pixel(3, 0)), 255);
}

// Ctrl+J on a selected group duplicates the whole group with or without a
// selection; the group copy is the new target and the source is untouched.
void LayerCommandsTest::ctrlJDuplicatesGroup()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CtrlJGroup"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerNewLayerViaCopy);
    QAction* viaCopy = registry->action(id);
    QVERIFY(viaCopy != nullptr);

    const QString group = view->add_group_in(QString());
    const QString child = view->add_layer_in(group);
    QVERIFY(!group.isEmpty() && !child.isEmpty());
    QVERIFY(view->set_layer_name_path(group, QStringLiteral("Folder")));
    QVERIFY(view->set_layer_name_path(child, QStringLiteral("Kid")));
    layers->refresh();
    layers->expandForTest(group);
    layers->selectPaths({group}, group);
    registry->refresh();
    QVERIFY2(!view->has_selection(), "no active selection");
    QVERIFY2(viaCopy->isEnabled(), "enabled for a group with no selection");
    QCOMPARE(registry->commandForShortcut(QKeySequence(QStringLiteral("Ctrl+J"))), id);

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();

    frame.activateWindow();
    QTest::keyClick(&frame, Qt::Key_J, Qt::ControlModifier);
    QCoreApplication::processEvents();
    if (pictura::layers_api::rowCount(*view) == rowsBefore) {
        // Offscreen Qt may not deliver the window shortcut; the enabled state
        // above is the live-refresh assertion, and trigger covers the dispatch.
        viaCopy->trigger();
        QCoreApplication::processEvents();
    }
    layers->refresh();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 2);
    QCOMPARE(view->history_count(), historyBefore + 1);
    QCOMPARE(countNamed(*view, QStringLiteral("Kid")), 2);
    const int copyRow = rowOfName(*view, QStringLiteral("Folder copy"));
    QVERIFY2(copyRow >= 0, "the group copy exists");
    QCOMPARE(rowOfName(*view, QStringLiteral("Folder")), copyRow + 2);
    QCOMPARE(rowOfName(*view, QStringLiteral("Kid")),
             copyRow + 1);
    const QString copyPath = pictura::layers_api::rowAt(*view, copyRow).path;
    const int copyKidRow = rowOfName(*view, QStringLiteral("Kid"));
    QCOMPARE(pictura::layers_api::rowAt(*view, copyKidRow).path.left(copyPath.size() + 1),
             copyPath + QLatin1Char('/'));

    const QStringList target = pictura::selected_layer_paths(*view);
    QCOMPARE(target, QStringList{copyPath});
    QCOMPARE(pictura::layer_selection_anchor(*view), copyPath);
    QVERIFY2(layers->selectedPaths().contains(copyPath), "the panel shows the copy");
    QVERIFY2(pictura::layers_api::rowForPath(*view, child).path == child,
             "the source child is unchanged");

    // An active selection no longer suppresses group duplication.
    QVERIFY(view->select_rect(0, 0, 8, 8, QStringLiteral("new"), 0.0));
    layers->selectPaths({group}, group);
    registry->refresh();
    QVERIFY(viaCopy->isEnabled());
    const int rowsSecond = pictura::layers_api::rowCount(*view);
    const int historySecond = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsSecond + 2);
    QCOMPARE(view->history_count(), historySecond + 1);

    // A mixed multi-selection follows the anchor's kind: the group copies whole
    // and the other selected layer copies alongside it, as one state.
    view->deselect();
    QVERIFY(!view->has_selection());
    const int baseRow = rowOfName(*view, QStringLiteral("Layer 0"));
    QVERIFY2(baseRow >= 0, "the plain layer is still present");
    const QString basePath = pictura::layers_api::rowAt(*view, baseRow).path;
    layers->selectPaths({group, basePath}, group);
    registry->refresh();
    const int rowsMixed = pictura::layers_api::rowCount(*view);
    const int historyMixed = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsMixed + 3);
    QCOMPARE(view->history_count(), historyMixed + 1);
    QCOMPARE(countNamed(*view, QStringLiteral("Kid")), 4);
    QVERIFY2(rowOfName(*view, QStringLiteral("Layer 0 copy")) >= 0,
             "the plain layer copied too");
}

// Ctrl+J with no selection duplicates the selected layer.
void LayerCommandsTest::ctrlJDuplicatesLayerWithoutSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CtrlJLayer"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerNewLayerViaCopy);
    QAction* viaCopy = registry->action(id);
    QVERIFY(viaCopy != nullptr);

    QVERIFY(view->add_layer(-1) >= 0);
    layers->refresh();
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    QVERIFY(view->set_layer_name_path(path, QStringLiteral("Plain")));
    layers->refresh();
    layers->selectPaths({path}, path);
    registry->refresh();
    QVERIFY2(!view->has_selection(), "no active selection");
    QVERIFY2(viaCopy->isEnabled(), "enabled with no selection");

    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    QCOMPARE(rowOfName(*view, QStringLiteral("Plain copy")),
             rowOfName(*view, QStringLiteral("Plain")) - 1);
    const QString copyPath = pictura::layers_api::rowAt(
        *view, rowOfName(*view, QStringLiteral("Plain copy"))).path;
    QCOMPARE(pictura::selected_layer_paths(*view), QStringList{copyPath});

    // A selection that covers no pixels counts as no selection.
    QVERIFY(view->select_rect(100, 100, 2, 2, QStringLiteral("new"), 0.0));
    QVERIFY2(view->has_selection(), "the empty selection is active");
    QCOMPARE(view->selection_count(), 0);
    const int rowsZero = pictura::layers_api::rowCount(*view);
    const int historyZero = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rowsZero + 1);
    QCOMPARE(view->history_count(), historyZero + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Duplicate Layer"));
}

// Layer via Cut needs a selection and a non-group target. It copies the
// selected pixels and clears them, and is unavailable for the other cases.
void LayerCommandsTest::ctrlJCutOutcomes()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CtrlJCut"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* layers = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && layers != nullptr && registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::LayerNewLayerViaCut);
    QAction* viaCut = registry->action(id);
    QVERIFY(viaCut != nullptr);

    QVERIFY(view->select_rect(0, 0, 2, 4, QStringLiteral("new"), 0.0));
    registry->refresh();
    QVERIFY2(viaCut->isEnabled(), "enabled for a raster target with a selection");
    const int rowsBefore = pictura::layers_api::rowCount(*view);
    const int historyBefore = view->history_count();
    QVERIFY(registry->dispatch(id));

    QCOMPARE(pictura::layers_api::rowCount(*view), rowsBefore + 1);
    QCOMPARE(view->history_count(), historyBefore + 1);
    const QImage cut = view->layer_thumbnail(1, 4);
    const QImage source = view->layer_thumbnail(0, 4);
    QVERIFY(!cut.isNull() && !source.isNull());
    QCOMPARE(qAlpha(cut.pixel(0, 0)), 255);
    QCOMPARE(qAlpha(cut.pixel(3, 0)), 0);
    QCOMPARE(qAlpha(source.pixel(0, 0)), 0);
    QCOMPARE(qAlpha(source.pixel(3, 0)), 255);

    // No selection: unavailable and a dispatch changes nothing.
    view->deselect();
    registry->refresh();
    QVERIFY(!view->has_selection());
    QVERIFY2(!viaCut->isEnabled(), "disabled with no selection");
    int rows = pictura::layers_api::rowCount(*view);
    int history = view->history_count();
    QVERIFY(registry->dispatch(id));
    QCOMPARE(pictura::layers_api::rowCount(*view), rows);
    QCOMPARE(view->history_count(), history);

    // A selection covering no pixels is also unavailable.
    QVERIFY(view->select_rect(100, 100, 2, 2, QStringLiteral("new"), 0.0));
    QVERIFY2(view->has_selection(), "the empty selection is active");
    QCOMPARE(view->selection_count(), 0);
    registry->refresh();
    QVERIFY2(!viaCut->isEnabled(), "disabled for a zero-coverage selection");

    // Group target: unavailable and a dispatch changes nothing.
    const QString group = view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QVERIFY(view->select_rect(0, 0, 2, 2, QStringLiteral("new"), 0.0));
    layers->refresh();
    layers->selectPaths({group}, group);
    registry->refresh();
    QVERIFY2(viaCut->isEnabled() == false, "disabled for a group target");
    rows = pictura::layers_api::rowCount(*view);
    history = view->history_count();
    QVERIFY(registry->dispatch(id));
    layers->refresh();
    QCOMPARE(pictura::layers_api::rowCount(*view), rows);
    QCOMPARE(view->history_count(), history);
}

// The Layer menu's Solid Color... fills with the current foreground colour,
// the same colour the Layers-panel fill menu uses.
void LayerCommandsTest::solidFillCommandUsesForeground()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("SolidFill"), 32, 32, QStringLiteral("rgb"), 8,
                               QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    auto* colors = frame.findChild<pictura::ColorState*>();
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(view != nullptr && colors != nullptr && registry != nullptr);
    // Set the colour last: the frame swaps the pair when the edit target
    // changes, so the state is checked again right before the command runs.
    colors->setForeground(QColor(0x22, 0x66, 0xcc));
    QVERIFY2(colors->foreground() == QColor(0x22, 0x66, 0xcc), "the foreground holds");

    const QString id = QString::fromLatin1(pictura::command_ids::LayerNewFillSolidColor);
    QVERIFY(registry->action(id) != nullptr);
    registry->refresh();
    QVERIFY2(registry->action(id)->isEnabled(), "enabled with a document open");
    QTimer timer;
    QObject::connect(&timer, &QTimer::timeout, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QDialog*>(widget);
            if (dialog && dialog->isVisible()) {
                dialog->accept();
                return;
            }
        }
    });
    timer.start(10);
    QVERIFY(registry->dispatch(id));
    timer.stop();
    QCOMPARE(view->sample_argb(0, 0), 0xff2266ccu);
}

// Merge Down on the bottom layer has nothing to merge into: refused, no state.
void LayerCommandsTest::managementNoOpRecordsNoHistory()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("NoOp"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);
    const int history = view->history_count();
    QVERIFY(view->merge_layers(QStringList{QStringLiteral("0")}).isEmpty());
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->layer_count(), 1);
}

// A content-neutral New Group and a recompositing Merge Down each record one
// state, and one undo restores the tree.
void LayerCommandsTest::managementCommandRecordsOneState()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OneState"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);

    int history = view->history_count();
    QVERIFY(!view->add_group_in(QStringLiteral("0")).isEmpty());
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->undo());
    QCOMPARE(view->layer_count(), 1);

    QVERIFY(!view->add_layer_in(QStringLiteral("0")).isEmpty());
    history = view->history_count();
    QVERIFY(!view->merge_layers(QStringList{QStringLiteral("1")}).isEmpty());
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->layer_count(), 1);
    QVERIFY(view->undo());
    QCOMPARE(view->layer_count(), 2);
}

QTEST_MAIN(LayerCommandsTest)
#include "tst_layer_commands.moc"
