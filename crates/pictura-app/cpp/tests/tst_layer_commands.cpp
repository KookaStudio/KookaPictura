// Layer-menu commands: Duplicate Layer follows the Layers panel selection,
// including nested layers and multi-selections.

#include <QtTest/QtTest>

#include "api/layers_api.h"

#include <memory>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QColor>

#include "commands.h"
#include "frame.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"

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

} // namespace

class LayerCommandsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void duplicateNestedLayer();
    void duplicateMultiSelection();
    void duplicateWithoutSelection();
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
    QVERIFY(registry->dispatch(id));
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
