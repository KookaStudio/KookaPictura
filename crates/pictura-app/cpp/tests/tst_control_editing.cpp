// The control server's editing surface: parameters, layer paths, export,
// history, and the guards against stale state.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>

#include "api/history_api.h"
#include "api/paint_bridge.h"
#include "api/transform_api.h"
#include "control_server.h"
#include "control_test_client.h"
#include "frame.h"
#include "image_view.h"
#include "tools.h"
#include "api/view_bridge.h"

#include "qt_test_support.h"

using pictura::test::ControlClient;
using pictura::test::errorCode;
using pictura::test::succeeded;
namespace history_api = pictura::history_api;

class ControlEditingTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void filterTakesParameters();
    void badFilterParametersAreRefused();
    void filtersAreListed();
    void adjustmentApplies();
    void unknownAdjustmentIsRefused();
    void adjustmentsAreListed();
    void resizeRunsWithoutADialog();
    void unlistedCommandIsRefused();
    void unlistedKeyIsRefused();
    void oversizedResizeIsRefused();
    void orientationRunsById();
    void layerTreeReportsPaths();
    void layerDigestSeesEveryPixel();
    void activeLayerIsSetByPath();
    void exportLeavesTheDocumentAlone();
    void exportIsGatedLikeASave();
    void historyListsAndJumps();
    void staleUndoIsRefused();
    void staleDocumentIsRefused();
    void eachMutatorAddsOneState_data();
    void eachMutatorAddsOneState();
    void lassoIsOneCall();
    void freeTransformScalesAndRotates();
    void freeTransformIsRefusedDuringASession();
    void keyReachesTheCanvasOfAnInactiveWindow();
    void freeTransformWarpsTheLayer();
    void newDocumentCanHaveABackgroundLayer();

private:
    void paintCentreSquare();
    QJsonObject call(const QString& method, const QJsonObject& params = {});
    pictura::PictureView& view() { return *window_->activeView(); }

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlEditingTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    server_ = std::make_unique<pictura::ControlServer>(
        window_.get(), dir_.filePath(QStringLiteral("control.sock")));
    QVERIFY(server_->listen());
}

void ControlEditingTest::init()
{
    QVERIFY(window_->newDocument(QStringLiteral("Editing"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY(succeeded(call(QStringLiteral("layer_op"),
                           {{QStringLiteral("op"), QStringLiteral("add")},
                            {QStringLiteral("index"), 0}})));
    const QJsonArray layers =
        call(QStringLiteral("layer_tree")).value(QStringLiteral("result")).toObject()
            .value(QStringLiteral("layers")).toArray();
    view().set_active_layer(layers.first().toObject().value(QStringLiteral("path")).toString());
}

QJsonObject ControlEditingTest::call(const QString& method, const QJsonObject& params)
{
    return server_->dispatch(method, params);
}

void ControlEditingTest::filterTakesParameters()
{
    const int before = history_api::count(view());
    QVERIFY(succeeded(call(QStringLiteral("filter"),
                           {{QStringLiteral("kind"), QStringLiteral("gaussian-blur")},
                            {QStringLiteral("params"), QJsonArray{2.0}}})));
    QCOMPARE(history_api::count(view()), before + 1);
}

void ControlEditingTest::badFilterParametersAreRefused()
{
    const int before = history_api::count(view());
    const auto attempt = [this](const QJsonArray& values) {
        return errorCode(call(QStringLiteral("filter"),
                              {{QStringLiteral("kind"), QStringLiteral("gaussian-blur")},
                               {QStringLiteral("params"), values}}));
    };
    QCOMPARE(attempt({}), QStringLiteral("invalid_param"));
    QCOMPARE(attempt({999999.0}), QStringLiteral("invalid_param"));
    QCOMPARE(attempt({QStringLiteral("two")}), QStringLiteral("invalid_param"));
    QCOMPARE(history_api::count(view()), before);
}

void ControlEditingTest::filtersAreListed()
{
    const QJsonArray filters = call(QStringLiteral("list_filters"))
                                   .value(QStringLiteral("result")).toObject()
                                   .value(QStringLiteral("filters")).toArray();
    QVERIFY(!filters.isEmpty());
    bool sawBlur = false;
    bool sawDisplace = false;
    for (const QJsonValue& value : filters) {
        const QJsonObject filter = value.toObject();
        const QString kind = filter.value(QStringLiteral("kind")).toString();
        if (kind == QStringLiteral("gaussian-blur")) {
            sawBlur = filter.value(QStringLiteral("available")).toBool();
            QVERIFY(!filter.value(QStringLiteral("params")).toArray().isEmpty());
        } else if (kind == QStringLiteral("displace")) {
            sawDisplace = !filter.value(QStringLiteral("available")).toBool();
        }
    }
    QVERIFY(sawBlur);
    QVERIFY(sawDisplace);
}

void ControlEditingTest::adjustmentApplies()
{
    const int before = history_api::count(view());
    QVERIFY(succeeded(call(QStringLiteral("adjust_apply"),
                           {{QStringLiteral("kind"), QStringLiteral("posterize")}})));
    QCOMPARE(history_api::count(view()), before + 1);
}

void ControlEditingTest::unknownAdjustmentIsRefused()
{
    QCOMPARE(errorCode(call(QStringLiteral("adjust_apply"),
                            {{QStringLiteral("kind"), QStringLiteral("nonsense")}})),
             QStringLiteral("invalid_param"));
}

void ControlEditingTest::adjustmentsAreListed()
{
    const QJsonArray adjustments = call(QStringLiteral("list_adjustments"))
                                       .value(QStringLiteral("result")).toObject()
                                       .value(QStringLiteral("adjustments")).toArray();
    QCOMPARE(adjustments.size(), pictura::ControlServer::adjustmentKinds().size());
    for (const QJsonValue& value : adjustments) {
        QVERIFY2(!value.toObject().value(QStringLiteral("page")).toArray().isEmpty(),
                 qPrintable(value.toObject().value(QStringLiteral("kind")).toString()));
    }
}

void ControlEditingTest::resizeRunsWithoutADialog()
{
    QVERIFY(succeeded(call(QStringLiteral("run_command"),
                           {{QStringLiteral("id"), QStringLiteral("image.image.size")},
                            {QStringLiteral("params"),
                             QJsonObject{{QStringLiteral("width"), 20},
                                         {QStringLiteral("height"), 10},
                                         {QStringLiteral("constrainProportions"), false}}}})));
    QCOMPARE(view().document_width(), 20);
    QCOMPARE(view().document_height(), 10);
}

void ControlEditingTest::unlistedCommandIsRefused()
{
    QCOMPARE(errorCode(call(QStringLiteral("run_command"),
                            {{QStringLiteral("id"), QStringLiteral("file.save")}})),
             QStringLiteral("refused"));
}

void ControlEditingTest::unlistedKeyIsRefused()
{
    QCOMPARE(errorCode(call(QStringLiteral("run_command"),
                            {{QStringLiteral("id"), QStringLiteral("image.image.size")},
                             {QStringLiteral("params"),
                              QJsonObject{{QStringLiteral("in"), QStringLiteral("/tmp/x")}}}})),
             QStringLiteral("invalid_param"));
}

void ControlEditingTest::oversizedResizeIsRefused()
{
    // Image Size spills to the scratch disks (#1293), so the refusal is pinned
    // to a host with 260 MiB available and no scratch space.
    struct Pin {
        Pin()
        {
            pictura::set_available_memory_for_test(260LL * 1024 * 1024);
            pictura::set_scratch_available_for_test(0);
        }
        ~Pin()
        {
            pictura::set_available_memory_for_test(-1);
            pictura::set_scratch_available_for_test(-1);
        }
    } pin;
    const int width = view().document_width();
    for (const char* id : {"image.image.size", "image.canvas.size"}) {
        QCOMPARE(errorCode(call(QStringLiteral("run_command"),
                                {{QStringLiteral("id"), QString::fromLatin1(id)},
                                 {QStringLiteral("params"),
                                  QJsonObject{{QStringLiteral("width"), 30000},
                                              {QStringLiteral("height"), 30000}}}})),
                 QStringLiteral("invalid_param"));
    }
    QCOMPARE(view().document_width(), width);
}

// [ce_run_orient] [ce_run_orient_key]
void ControlEditingTest::orientationRunsById()
{
    QVERIFY(succeeded(call(QStringLiteral("run_command"),
                           {{QStringLiteral("id"), QStringLiteral("image.image.size")},
                            {QStringLiteral("params"),
                             QJsonObject{{QStringLiteral("width"), 60},
                                         {QStringLiteral("height"), 40},
                                         {QStringLiteral("constrainProportions"), false}}}})));
    QVERIFY(succeeded(call(QStringLiteral("run_command"),
                           {{QStringLiteral("id"), QStringLiteral("image.rotate90cw")}})));
    QCOMPARE(view().document_width(), 40);
    QCOMPARE(view().document_height(), 60);
    QCOMPARE(errorCode(call(QStringLiteral("run_command"),
                            {{QStringLiteral("id"), QStringLiteral("image.flipHorizontal")},
                             {QStringLiteral("params"),
                              QJsonObject{{QStringLiteral("angle"), 90}}}})),
             QStringLiteral("invalid_param"));
    QCOMPARE(view().document_width(), 40);
}

void ControlEditingTest::layerTreeReportsPaths()
{
    QVERIFY(succeeded(call(QStringLiteral("layer_op"),
                           {{QStringLiteral("op"), QStringLiteral("add_group")},
                            {QStringLiteral("index"), 0}})));
    const QJsonObject tree = call(QStringLiteral("layer_tree")).value(QStringLiteral("result")).toObject();
    const QJsonArray layers = tree.value(QStringLiteral("layers")).toArray();
    QVERIFY(layers.size() >= 2);
    for (const QJsonValue& value : layers) {
        QVERIFY(!value.toObject().value(QStringLiteral("path")).toString().isEmpty());
        QVERIFY(value.toObject().contains(QStringLiteral("depth")));
    }
    QVERIFY(call(QStringLiteral("list_layers"))
                .value(QStringLiteral("result")).toObject()
                .contains(QStringLiteral("active")));
}

// [ce_layer_digest]
void ControlEditingTest::layerDigestSeesEveryPixel()
{
    paintCentreSquare();
    const QString path = view().active_layer_path();
    const auto digest = [this](const QString& at) {
        return call(QStringLiteral("layer_digest"), {{QStringLiteral("path"), at}});
    };
    const QJsonObject first = digest(path);
    QVERIFY(succeeded(first));
    const QJsonObject result = first.value(QStringLiteral("result")).toObject();
    const QString value = result.value(QStringLiteral("digest")).toString();
    QCOMPARE(value.size(), 16);
    const auto rect = pictura::transform_api::layerRect(view(), path);
    QVERIFY(rect.valid);
    const QJsonArray reported = result.value(QStringLiteral("rect")).toArray();
    QCOMPARE(reported.size(), 4);
    QCOMPARE(reported.at(0).toInt(), rect.left);
    QCOMPARE(reported.at(1).toInt(), rect.top);
    QCOMPARE(reported.at(2).toInt(), rect.right);
    QCOMPARE(reported.at(3).toInt(), rect.bottom);
    QCOMPARE(digest(path).value(QStringLiteral("result")).toObject()
                 .value(QStringLiteral("digest")).toString(),
             value);

    QVERIFY(succeeded(call(QStringLiteral("selection"),
                           {{QStringLiteral("op"), QStringLiteral("rect")},
                            {QStringLiteral("x"), 12}, {QStringLiteral("y"), 12},
                            {QStringLiteral("w"), 1}, {QStringLiteral("h"), 1}})));
    QVERIFY(succeeded(call(QStringLiteral("run_command"),
                           {{QStringLiteral("id"), QStringLiteral("edit.fill")},
                            {QStringLiteral("params"),
                             QJsonObject{{QStringLiteral("color"), double(0xFFFF0000u)}}}})));
    QVERIFY(digest(path).value(QStringLiteral("result")).toObject()
                .value(QStringLiteral("digest")).toString()
            != value);
    QCOMPARE(errorCode(digest(QStringLiteral("99"))), QStringLiteral("invalid_param"));
}

void ControlEditingTest::activeLayerIsSetByPath()
{
    const QJsonArray layers = call(QStringLiteral("layer_tree"))
                                  .value(QStringLiteral("result")).toObject()
                                  .value(QStringLiteral("layers")).toArray();
    const QString path = layers.last().toObject().value(QStringLiteral("path")).toString();
    QVERIFY(succeeded(call(QStringLiteral("set_active_layer"), {{QStringLiteral("path"), path}})));
    QCOMPARE(view().active_layer_path(), path);
    QCOMPARE(errorCode(call(QStringLiteral("set_active_layer"),
                            {{QStringLiteral("path"), QStringLiteral("99/99")}})),
             QStringLiteral("invalid_param"));
}

void ControlEditingTest::exportLeavesTheDocumentAlone()
{
    const QString path = dir_.filePath(QStringLiteral("export.png"));
    const int index = window_->activeDocumentIndex();
    const QString before = window_->activeFilePath();
    const bool dirty = window_->isDocumentDirty(index);
    const int states = history_api::count(view());
    QVERIFY(succeeded(call(QStringLiteral("export_png"), {{QStringLiteral("path"), path}})));
    QVERIFY(QFile::exists(path));
    QCOMPARE(window_->activeFilePath(), before);
    QCOMPARE(window_->isDocumentDirty(index), dirty);
    QCOMPARE(history_api::count(view()), states);
}

void ControlEditingTest::exportIsGatedLikeASave()
{
    pictura::ControlPolicy policy;
    policy.restricted = true;
    policy.allowEdit = true;
    server_->setPolicy(policy);
    const QString path = dir_.filePath(QStringLiteral("gated.png"));
    ControlClient client(*server_);
    const QJsonObject response =
        client.call(QStringLiteral("export_png"), {{QStringLiteral("path"), path}});
    server_->setPolicy({});
    QCOMPARE(errorCode(response), QStringLiteral("refused"));
    QVERIFY(!QFile::exists(path));
}

void ControlEditingTest::historyListsAndJumps()
{
    const int before = history_api::index(view());
    QVERIFY(succeeded(call(QStringLiteral("adjust_apply"),
                           {{QStringLiteral("kind"), QStringLiteral("posterize")}})));
    const QJsonObject listed = call(QStringLiteral("history"), {{QStringLiteral("op"), QStringLiteral("list")}})
                                   .value(QStringLiteral("result")).toObject();
    QVERIFY(!listed.value(QStringLiteral("states")).toArray().isEmpty());
    QVERIFY(succeeded(call(QStringLiteral("history"), {{QStringLiteral("op"), QStringLiteral("jump")},
                                                       {QStringLiteral("index"), before}})));
    QCOMPARE(history_api::index(view()), before);
    QCOMPARE(server_->tierOf(QStringLiteral("history"), {{QStringLiteral("op"), QStringLiteral("list")}}),
             pictura::ControlTier::Read);
    QCOMPARE(server_->tierOf(QStringLiteral("history"), {{QStringLiteral("op"), QStringLiteral("jump")}}),
             pictura::ControlTier::Edit);
}

void ControlEditingTest::staleUndoIsRefused()
{
    const int old = history_api::index(view());
    QVERIFY(succeeded(call(QStringLiteral("adjust_apply"),
                           {{QStringLiteral("kind"), QStringLiteral("posterize")}})));
    const int current = history_api::index(view());
    QCOMPARE(errorCode(call(QStringLiteral("edit"), {{QStringLiteral("op"), QStringLiteral("undo")},
                                                     {QStringLiteral("expected_index"), old}})),
             QStringLiteral("stale_history"));
    QCOMPARE(history_api::index(view()), current);
    QVERIFY(succeeded(call(QStringLiteral("edit"), {{QStringLiteral("op"), QStringLiteral("undo")},
                                                    {QStringLiteral("expected_index"), current}})));
}

void ControlEditingTest::staleDocumentIsRefused()
{
    ControlClient client(*server_);
    const QJsonObject status = client.call(QStringLiteral("status"));
    const QJsonArray documents = status.value(QStringLiteral("result")).toObject()
                                     .value(QStringLiteral("documents_info")).toArray();
    const int active = window_->activeDocumentIndex();
    const QString id = documents.at(active).toObject().value(QStringLiteral("id")).toString();
    QVERIFY(!id.isEmpty());
    QVERIFY(window_->newDocument(QStringLiteral("Other"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCOMPARE(errorCode(client.call(QStringLiteral("adjust_apply"),
                                   {{QStringLiteral("kind"), QStringLiteral("posterize")},
                                    {QStringLiteral("doc"), id}})),
             QStringLiteral("stale_document"));
}

void ControlEditingTest::eachMutatorAddsOneState_data()
{
    QTest::addColumn<QString>("method");
    QTest::addColumn<QJsonObject>("params");
    const auto kind = [](const char* name) {
        return QJsonObject{{QStringLiteral("kind"), QString::fromLatin1(name)}};
    };
    QTest::newRow("filter") << QStringLiteral("filter") << kind("gaussian-blur");
    QTest::newRow("adjust_apply") << QStringLiteral("adjust_apply") << kind("posterize");
    QTest::newRow("run_command")
        << QStringLiteral("run_command")
        << QJsonObject{{QStringLiteral("id"), QStringLiteral("image.rotate.arbitrary")},
                       {QStringLiteral("params"), QJsonObject{{QStringLiteral("angle"), 15.0}}}};
    QTest::newRow("selection")
        << QStringLiteral("selection")
        << QJsonObject{{QStringLiteral("op"), QStringLiteral("rect")}, {QStringLiteral("x"), 1},
                       {QStringLiteral("y"), 1}, {QStringLiteral("w"), 4}, {QStringLiteral("h"), 4}};
}

void ControlEditingTest::eachMutatorAddsOneState()
{
    QFETCH(QString, method);
    QFETCH(QJsonObject, params);
    const int before = history_api::count(view());
    const QJsonObject response = call(method, params);
    QVERIFY2(succeeded(response),
             qPrintable(QJsonDocument(response).toJson(QJsonDocument::Compact)));
    QCOMPARE(history_api::count(view()), before + 1);
}

void ControlEditingTest::lassoIsOneCall()
{
    const int before = history_api::count(view());
    QJsonArray triangle{QJsonArray{2, 2}, QJsonArray{20, 2}, QJsonArray{2, 20}};
    QVERIFY(succeeded(call(QStringLiteral("selection"), {{QStringLiteral("op"), QStringLiteral("lasso")},
                                                         {QStringLiteral("points"), triangle}})));
    QVERIFY(view().has_selection());
    QCOMPARE(history_api::count(view()), before + 1);

    QJsonArray tooMany;
    for (int i = 0; i <= pictura::ControlServer::kMaxLassoPoints; ++i) {
        tooMany.append(QJsonArray{i % 32, i % 16});
    }
    QCOMPARE(errorCode(call(QStringLiteral("selection"), {{QStringLiteral("op"), QStringLiteral("lasso")},
                                                          {QStringLiteral("points"), tooMany}})),
             QStringLiteral("invalid_param"));
}

// An opaque black 8x8 square in the middle of the active (otherwise empty) layer.
void ControlEditingTest::paintCentreSquare()
{
    QVERIFY(succeeded(call(QStringLiteral("selection"),
                           {{QStringLiteral("op"), QStringLiteral("rect")},
                            {QStringLiteral("x"), 12}, {QStringLiteral("y"), 12},
                            {QStringLiteral("w"), 8}, {QStringLiteral("h"), 8}})));
    QVERIFY(succeeded(call(QStringLiteral("run_command"),
                           {{QStringLiteral("id"), QStringLiteral("edit.fill")},
                            {QStringLiteral("params"),
                             QJsonObject{{QStringLiteral("color"), double(0xFF000000u)}}}})));
    QVERIFY(succeeded(call(QStringLiteral("selection"),
                           {{QStringLiteral("op"), QStringLiteral("deselect")}})));
}

void ControlEditingTest::freeTransformScalesAndRotates()
{
    paintCentreSquare();
    // Just outside the square, inside it once scaled 150 % and turned 15°.
    QCOMPARE(view().sample_argb(11, 16), qRgba(255, 255, 255, 255));
    const int before = history_api::count(view());
    const int index = view().active_layer_path().toInt();
    QVERIFY(succeeded(call(QStringLiteral("layer_op"),
                           {{QStringLiteral("op"), QStringLiteral("free_transform")},
                            {QStringLiteral("index"), index},
                            {QStringLiteral("scale"), 150.0},
                            {QStringLiteral("angle"), 15.0}})));
    QVERIFY(succeeded(call(QStringLiteral("status"))));
    QCOMPARE(history_api::count(view()), before + 1);
    QCOMPARE(view().sample_argb(11, 16), qRgba(0, 0, 0, 255));
    QCOMPARE(view().sample_argb(16, 16), qRgba(0, 0, 0, 255));
    QCOMPARE(errorCode(call(QStringLiteral("layer_op"),
                            {{QStringLiteral("op"), QStringLiteral("free_transform")},
                             {QStringLiteral("index"), index},
                             {QStringLiteral("scale"), 0.0}})),
             QStringLiteral("invalid_param"));
}

void ControlEditingTest::freeTransformIsRefusedDuringASession()
{
    paintCentreSquare();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const QString path = view().active_layer_path();
    QVERIFY(tools->beginFreeTransform(path));
    const int before = history_api::count(view());
    QCOMPARE(errorCode(call(QStringLiteral("layer_op"),
                            {{QStringLiteral("op"), QStringLiteral("free_transform")},
                             {QStringLiteral("index"), path.toInt()},
                             {QStringLiteral("scale"), 150.0}})),
             QStringLiteral("refused"));
    QVERIFY(tools->transformSessionActive());
    QCOMPARE(history_api::count(view()), before);
    tools->cancelFreeTransform();
}

// [ce_key_inactive_window] #1290: a 17000² transform's Return was lost when no
// window was active, because the key went to the main window, not the canvas.
void ControlEditingTest::keyReachesTheCanvasOfAnInactiveWindow()
{
    paintCentreSquare();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->beginFreeTransform(view().active_layer_path()));
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->imageView()->setFocus();
    QWidget other;
    other.show();
    other.activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(&other));
    QCOMPARE(QApplication::focusWidget(), nullptr);
    QVERIFY(succeeded(call(QStringLiteral("key"), {{QStringLiteral("sequence"), QStringLiteral("Escape")}})));
    const bool ended = !tools->transformSessionActive();
    if (!ended) {
        tools->cancelFreeTransform();
    }
    window_->hide();
    QVERIFY(ended);
}

void ControlEditingTest::freeTransformWarpsTheLayer()
{
    paintCentreSquare();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const QString path = view().active_layer_path();
    const int index = path.toInt();
    const auto before = pictura::transform_api::layerRect(view(), path);
    QVERIFY(before.valid);
    const int states = history_api::count(view());
    QCOMPARE(errorCode(call(QStringLiteral("layer_op"),
                            {{QStringLiteral("op"), QStringLiteral("free_transform")},
                             {QStringLiteral("index"), index},
                             {QStringLiteral("warp"), QStringLiteral("arc")},
                             {QStringLiteral("bend"), 150.0}})),
             QStringLiteral("invalid_param"));
    QCOMPARE(errorCode(call(QStringLiteral("layer_op"),
                            {{QStringLiteral("op"), QStringLiteral("free_transform")},
                             {QStringLiteral("index"), index},
                             {QStringLiteral("warp"), QStringLiteral("nope")}})),
             QStringLiteral("refused"));
    QVERIFY(!tools->transformSessionActive());
    QCOMPARE(history_api::count(view()), states);
    QVERIFY(succeeded(call(QStringLiteral("layer_op"),
                           {{QStringLiteral("op"), QStringLiteral("free_transform")},
                            {QStringLiteral("index"), index},
                            {QStringLiteral("warp"), QStringLiteral("arc")},
                            {QStringLiteral("bend"), 50.0}})));
    QVERIFY(succeeded(call(QStringLiteral("status"))));
    QVERIFY(!tools->transformSessionActive());
    QCOMPARE(history_api::count(view()), states + 1);
    const auto after = pictura::transform_api::layerRect(view(), path);
    QVERIFY2(after.top < before.top,
             qPrintable(before.toString() + QStringLiteral(" -> ") + after.toString()));
}

void ControlEditingTest::newDocumentCanHaveABackgroundLayer()
{
    const int before = window_->documentCount();
    QVERIFY(succeeded(call(QStringLiteral("document"),
                           {{QStringLiteral("op"), QStringLiteral("new")},
                            {QStringLiteral("width"), 24}, {QStringLiteral("height"), 16},
                            {QStringLiteral("background_layer"), true}})));
    QCOMPARE(window_->documentCount(), before + 1);
    const QJsonArray layers = call(QStringLiteral("layer_tree"))
                                  .value(QStringLiteral("result")).toObject()
                                  .value(QStringLiteral("layers")).toArray();
    QCOMPARE(layers.size(), 1);
    QCOMPARE(layers.first().toObject().value(QStringLiteral("name")).toString(),
             QStringLiteral("Background"));
    QCOMPARE(view().document_width(), 24);
    QCOMPARE(view().sample_argb(3, 3), qRgba(255, 255, 255, 255));
    QCOMPARE(history_api::count(view()), 1);
    QCOMPARE(errorCode(call(QStringLiteral("document"),
                            {{QStringLiteral("op"), QStringLiteral("new")},
                             {QStringLiteral("width"), 24}, {QStringLiteral("height"), 16},
                             {QStringLiteral("background"), QStringLiteral("transparent")},
                             {QStringLiteral("background_layer"), true}})),
             QStringLiteral("invalid_param"));
}

QTEST_MAIN(ControlEditingTest)
#include "tst_control_editing.moc"
