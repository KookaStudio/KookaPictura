// The control server's editing surface: parameters, layer paths, export,
// history, and the guards against stale state.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QDialog>

#include "api/history_api.h"
#include "control_server.h"
#include "control_test_client.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
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
    void layerTreeReportsPaths();
    void activeLayerIsSetByPath();
    void exportLeavesTheDocumentAlone();
    void exportIsGatedLikeASave();
    void historyListsAndJumps();
    void staleUndoIsRefused();
    void staleDocumentIsRefused();
    void eachMutatorAddsOneState_data();
    void eachMutatorAddsOneState();
    void lassoIsOneCall();

private:
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

QTEST_MAIN(ControlEditingTest)
#include "tst_control_editing.moc"
