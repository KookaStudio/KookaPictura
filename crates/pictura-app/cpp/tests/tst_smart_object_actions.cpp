#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QStringList>

#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {

QString id(const char* command)
{
    return QString::fromLatin1(command);
}

} // namespace

// The advanced Smart Object actions: Reset Transform, Convert to Layers, and
// New Smart Object via Copy. The engine behavior is unit-tested in
// pictura-render; these cover the command/menu wiring and the one-undo-state
// contract.
class SmartObjectActionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rowMenuOffersSmartRows();
    void rowMenuOffersSmartObjectContents();
    void commandsRequireASmartObject();
    void backgroundRowConvertsToSmartObject();
    void resetTransformRecordsOneState();
    void convertToLayersConsumesObject();
    void newViaCopyAddsIndependentLayer();

private:
    QString makeSmartLayer(pictura::PictureView* view, pictura::LayersPanel* panel);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmartObjectActionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void SmartObjectActionsTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString SmartObjectActionsTest::makeSmartLayer(pictura::PictureView* view,
                                               pictura::LayersPanel* panel)
{
    view->add_layer(-1);
    panel->refresh();
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    panel->selectPaths({path}, path);
    window_->registry()->refresh();
    if (!view->convert_to_smart_object(path)) {
        return QString();
    }
    panel->refresh();
    window_->registry()->refresh();
    return path;
}

void SmartObjectActionsTest::rowMenuOffersSmartRows()
{
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    // Presence is kind-based: a `pixel` row carrying the smart flag offers them.
    const QStringList smart = panel->smartRowMenuTextsForTest();
    QVERIFY2(smart.contains(QStringLiteral("Reset Transform")), "Reset Transform row");
    QVERIFY2(smart.contains(QStringLiteral("Convert to Layers")), "Convert to Layers row");
    QVERIFY2(smart.contains(QStringLiteral("New Smart Object via Copy")),
             "New Smart Object via Copy row");
    QVERIFY2(smart.contains(QStringLiteral("Edit Contents")), "Edit Contents row");
    QVERIFY2(smart.contains(QStringLiteral("Replace Contents…")), "Replace Contents row");
    QVERIFY2(smart.contains(QStringLiteral("Export Contents…")), "Export Contents row");
    QVERIFY2(smart.contains(QStringLiteral("Relink to File…")), "Relink to File row");
    QVERIFY2(smart.contains(QStringLiteral("Convert to Linked…")), "Convert to Linked row");

    const QStringList pixel = panel->rowMenuTextsForTest(QStringLiteral("pixel"));
    QVERIFY2(!pixel.contains(QStringLiteral("Reset Transform")), "plain pixel omits Reset");
    QVERIFY2(!pixel.contains(QStringLiteral("Convert to Layers")), "plain pixel omits Convert");
    QVERIFY2(!pixel.contains(QStringLiteral("New Smart Object via Copy")),
             "plain pixel omits via Copy");
    QVERIFY2(!pixel.contains(QStringLiteral("Edit Contents")), "plain pixel omits Edit Contents");
    QVERIFY2(!pixel.contains(QStringLiteral("Replace Contents…")),
             "plain pixel omits Replace Contents");
    QVERIFY2(!pixel.contains(QStringLiteral("Export Contents…")),
             "plain pixel omits Export Contents");
    QVERIFY2(!pixel.contains(QStringLiteral("Relink to File…")), "plain pixel omits Relink");
    QVERIFY2(!pixel.contains(QStringLiteral("Convert to Linked…")),
             "plain pixel omits Convert to Linked");
    QVERIFY2(pixel.contains(QStringLiteral("Convert to Smart Object")),
             "plain pixel offers Convert to Smart Object");

    // Enablement follows the target: disabled on a plain layer, enabled once the
    // layer is a smart object.
    QVERIFY(window_->newDocument(QStringLiteral("SmartRows"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    view->add_layer(-1);
    panel->refresh();
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Reset Transform")),
             "a plain layer disables Reset Transform");
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Smart Object")),
             "a plain layer enables Convert to Smart Object");
    QVERIFY2(view->convert_to_smart_object(path), "convert to smart object");
    panel->refresh();
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Reset Transform")),
             "a smart layer enables Reset Transform");
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Layers")),
             "a smart layer enables Convert to Layers");
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("New Smart Object via Copy")),
             "a smart layer enables New Smart Object via Copy");
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Smart Object")),
             "an existing smart object disables Convert to Smart Object");
}

// The contents rows: Convert to Smart Object acts on the row directly, the
// Edit/Replace/Export rows run on the row's path through the frame, and the
// linked rows are present but disabled until #113 lands.
void SmartObjectActionsTest::rowMenuOffersSmartObjectContents()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartContents"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);

    // Convert to Smart Object from the row menu converts the row and selects the
    // new object, recording one state.
    view->add_layer(-1);
    panel->refresh();
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    const int before = view->history_count();
    QVERIFY2(panel->performRowActionForTest(QStringLiteral("convertToSmartObject"), path),
             "the row menu converts the layer");
    QCOMPARE(view->history_count(), before + 1);
    QVERIFY2(!view->layer_smart_object_state(path).isEmpty(), "the row became a smart object");

    const QStringList rows = panel->rowMenuTextsForPathForTest(path);
    QVERIFY2(rows.contains(QStringLiteral("Edit Contents")), "edit contents row present");
    QVERIFY2(rows.contains(QStringLiteral("Replace Contents…")), "replace contents row present");
    QVERIFY2(rows.contains(QStringLiteral("Export Contents…")), "export contents row present");
    QVERIFY2(rows.contains(QStringLiteral("Relink to File…")), "relink row present");
    QVERIFY2(rows.contains(QStringLiteral("Convert to Linked…")), "convert to linked row present");

    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Edit Contents")),
             "an editable source enables Edit Contents");
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Replace Contents…")),
             "a replaceable object enables Replace Contents");
    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Export Contents…")),
             "a payload enables Export Contents");
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Relink to File…")),
             "Relink to File is not implemented");
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Linked…")),
             "Convert to Linked is not implemented");

    // The contents rows carry the row's path to the frame through the signal,
    // and the frame acts on it: Edit Contents opens a new untitled editor tab
    // (no file dialog), which proves the signal is connected end to end.
    QSignalSpy spy(panel, &pictura::LayersPanel::smartObjectContentsRequested);
    QVERIFY(spy.isValid());
    const int tabsBefore = window_->documentCount();
    QVERIFY(panel->performRowActionForTest(QStringLiteral("editContents"), path));
    QCOMPARE(spy.count(), 1);
    QCOMPARE(spy.takeFirst().at(0).toString(), QStringLiteral("edit"));
    QCOMPARE(window_->documentCount(), tabsBefore + 1);
}

void SmartObjectActionsTest::commandsRequireASmartObject()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartCmds"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && panel && registry);

    // A transparent pixel layer is not yet a smart object, so reset/convert are
    // disabled while convert-to-smart-object is enabled.
    view->add_layer(-1);
    panel->refresh();
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    panel->selectPaths({path}, path);
    registry->refresh();
    QVERIFY(registry->action(id(pictura::command_ids::LayerSmartObjectConvertTo))->isEnabled());
    QVERIFY(!registry->action(id(pictura::command_ids::LayerSmartObjectResetTransform))->isEnabled());
    QVERIFY(
        !registry->action(id(pictura::command_ids::LayerSmartObjectConvertToLayers))->isEnabled());
    QVERIFY(!registry->action(id(pictura::command_ids::LayerSmartObjectNewViaCopy))->isEnabled());

    QVERIFY(view->convert_to_smart_object(path));
    panel->refresh();
    registry->refresh();
    QVERIFY(registry->action(id(pictura::command_ids::LayerSmartObjectResetTransform))->isEnabled());
    QVERIFY(
        registry->action(id(pictura::command_ids::LayerSmartObjectConvertToLayers))->isEnabled());
    QVERIFY(
        registry->action(id(pictura::command_ids::LayerSmartObjectNewViaCopy))->isEnabled());
}

void SmartObjectActionsTest::resetTransformRecordsOneState()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartReset"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && panel && registry);
    const QString path = makeSmartLayer(view, panel);
    QVERIFY2(!path.isEmpty(), "smart layer prepared");

    const int before = view->history_count();
    QVERIFY(registry->dispatch(id(pictura::command_ids::LayerSmartObjectResetTransform)));
    QCOMPARE(view->history_count(), before + 1);
    QVERIFY2(!view->layer_smart_object_state(path).isEmpty(), "stays a smart object");
}

void SmartObjectActionsTest::convertToLayersConsumesObject()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartLayers"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && panel && registry);
    const QString path = makeSmartLayer(view, panel);
    QVERIFY2(!path.isEmpty(), "smart layer prepared");

    const int before = view->history_count();
    QVERIFY(registry->dispatch(id(pictura::command_ids::LayerSmartObjectConvertToLayers)));
    QCOMPARE(view->history_count(), before + 1);
    QVERIFY2(view->layer_smart_object_state(path).isEmpty(), "object consumed into layers");
    QVERIFY(!registry->action(id(pictura::command_ids::LayerSmartObjectConvertToLayers))
                 ->isEnabled());
}

void SmartObjectActionsTest::newViaCopyAddsIndependentLayer()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartCopy"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && panel && registry);
    const QString path = makeSmartLayer(view, panel);
    QVERIFY2(!path.isEmpty(), "smart layer prepared");

    const int layersBefore = pictura::layers_api::rowCount(*view);
    const int before = view->history_count();
    QVERIFY(registry->dispatch(id(pictura::command_ids::LayerSmartObjectNewViaCopy)));
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(pictura::layers_api::rowCount(*view), layersBefore + 1);
    QVERIFY2(!view->layer_smart_object_state(path).isEmpty(), "original keeps its source");
}

// A Background is a convertible target: the row menu and the command offer
// Convert to Smart Object, the conversion unlocks it (no longer a Background)
// and records one state, and a group row still omits the row.
void SmartObjectActionsTest::backgroundRowConvertsToSmartObject()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartBackground"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(view && panel && registry);

    // The bridge newDocument makes a plain layer; flag it as the Background.
    const QString path = pictura::layers_api::rowAt(*view, 0).path;
    QVERIFY2(view->background_from_layer(path), "the layer becomes the Background");
    panel->refresh();
    QCOMPARE(view->layer_kind(path.toInt()), QStringLiteral("background"));

    QVERIFY2(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Smart Object")),
             "a Background row enables Convert to Smart Object");
    panel->selectPaths({path}, path);
    registry->refresh();
    QVERIFY(registry->action(id(pictura::command_ids::LayerSmartObjectConvertTo))->isEnabled());

    // A group row does not offer the command.
    const QString group = view->add_group_in(QString());
    panel->refresh();
    QVERIFY2(!panel->rowMenuTextsForPathForTest(group)
                  .contains(QStringLiteral("Convert to Smart Object")),
             "a group row omits Convert to Smart Object");

    panel->selectPaths({path}, path);
    registry->refresh();
    const int before = view->history_count();
    QVERIFY2(panel->performRowActionForTest(QStringLiteral("convertToSmartObject"), path),
             "the Background row converts");
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(before), QStringLiteral("Convert to Smart Object"));
    QVERIFY2(view->layer_kind(path.toInt()) != QStringLiteral("background"),
             "the row is no longer a Background");
    QVERIFY2(!view->layer_smart_object_state(path).isEmpty(), "the Background became a smart object");
    panel->refresh();
    QVERIFY2(!panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Smart Object")),
             "an existing smart object disables Convert to Smart Object");
}

QTEST_MAIN(SmartObjectActionsTest)
#include "tst_smart_object_actions.moc"
