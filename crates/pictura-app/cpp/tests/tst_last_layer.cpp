// A document keeps its last layer (#670): with one layer left, the Layers
// panel's trash, the row menu's Delete Layer, and Layer > Delete > Layer are
// greyed out, and a delete that slips through (the Delete key) is refused, so
// the canvas never shows a stale image with no layers behind it.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QAction>

class LastLayerTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void theLastLayerCannotBeDeleted();
    void removeLayerKeepsTheLastLayer();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LastLayerTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

// ll_last_layer_disabled, ll_last_layer_refused
void LastLayerTest::theLastLayerCannotBeDeleted()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Last"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    pictura::CommandRegistry* registry = frame.registry();
    QVERIFY(registry != nullptr);
    const QAction* deleteCommand =
        registry->action(QString::fromLatin1(pictura::command_ids::LayerDeleteLayer));
    QVERIFY(deleteCommand != nullptr);

    // One extra layer: everything can delete it.
    const QString extra = view->add_layer_in(QString());
    QVERIFY(!extra.isEmpty());
    frame.selectLayerPath(extra);
    QCoreApplication::processEvents();
    panel->refresh();
    registry->refresh();
    QCOMPARE(view->layer_count(), 2);
    QVERIFY(panel->deleteButtonEnabledForTest());
    QVERIFY(panel->rowMenuEnabledForPathForTest(extra, QStringLiteral("Delete Layer")));
    QVERIFY(deleteCommand->isEnabled());

    panel->clickDeleteButtonForTest();
    QCoreApplication::processEvents();
    panel->refresh();
    registry->refresh();
    QCOMPARE(view->layer_count(), 1);

    // The last layer: every delete entry point is greyed out.
    const QString last = panel->currentPath();
    QVERIFY(!last.isEmpty());
    QVERIFY2(!panel->deleteButtonEnabledForTest(), "trash greyed out");
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(last, QStringLiteral("Delete Layer")),
             "row menu greyed out");
    QVERIFY2(!deleteCommand->isEnabled(), "Layer > Delete > Layer greyed out");

    // The Delete key path still reaches the bridge, which refuses.
    const int history = view->history_count();
    QCOMPARE(view->delete_layers(QStringList{last}), 0);
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), history);

    frame.closeDocument(frame.activeDocumentIndex(), false);
}

// ll_remove_layer_guarded: the control server's layer_op delete calls
// remove_layer, which applies the same guard and records nothing when refused.
void LastLayerTest::removeLayerKeepsTheLastLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Remove"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);
    QVERIFY(!view->add_layer_in(QString()).isEmpty());
    QCOMPARE(view->layer_count(), 2);
    const int history = view->history_count();

    QVERIFY(view->remove_layer(1));
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), history + 1);

    QVERIFY2(!view->remove_layer(0), "the last layer stays");
    QVERIFY2(!view->remove_layer(5), "an index out of range is refused");
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), history + 1);

    frame.closeDocument(frame.activeDocumentIndex(), false);
}

QTEST_MAIN(LastLayerTest)
#include "tst_last_layer.moc"
