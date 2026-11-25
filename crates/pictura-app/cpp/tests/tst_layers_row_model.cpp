// The row-action table behind the Layers row menu, constructed from hand-built
// rows without a PicturaMainWindow or a PictureView.

#include <QtTest/QtTest>

#include <QtCore/QVector>

#include "commands.h"
#include "panels/layers_panel_model.h"
#include "panels/layers_row_actions.h"

using namespace pictura;

namespace {

LayerRowNode makeRow(const QString& kind)
{
    LayerRowNode row;
    row.path = QStringLiteral("1");
    row.name = QStringLiteral("Layer 1");
    row.kind = kind;
    return row;
}

const ResolvedRowAction* findAction(const QVector<ResolvedRowAction>& actions, const char* id)
{
    for (const ResolvedRowAction& action : actions) {
        if (action.id == QLatin1String(id)) {
            return &action;
        }
    }
    return nullptr;
}

} // namespace

class LayersRowModelTest : public QObject {
    Q_OBJECT

private slots:
    void pixelRowActionsFromHandBuiltRows();
    void theRowTableCarriesStaticCommandIds();
    void dynamicRowsResolveFromRowFacts();
    void maskRowsExposeTheirSubmenus();
    void unimplementedRowsAreDisabled();
    void absentPathResolvesToEmpty();
};

void LayersRowModelTest::pixelRowActionsFromHandBuiltRows()
{
    LayerRowModel model;
    model.setRows({makeRow(QStringLiteral("pixel"))});

    const QVector<ResolvedRowAction> actions = model.actionsFor(QStringLiteral("1"), {});
    QVERIFY(!actions.isEmpty());
    QCOMPARE(actions.first().id, QStringLiteral("blendingOptions"));
    const ResolvedRowAction* duplicate = findAction(actions, "duplicate");
    QVERIFY(duplicate != nullptr);
    QCOMPARE(duplicate->label, QStringLiteral("Duplicate Layer(s)"));
    QCOMPARE(actions.last().id, QStringLiteral("flatten"));

    QVector<int> blocks;
    for (const ResolvedRowAction& action : actions) {
        if (blocks.isEmpty() || blocks.last() != action.block) {
            blocks.push_back(action.block);
        }
    }
    QCOMPARE(blocks, (QVector<int>{1, 2, 3, 4, 6, 7, 8, 11}));
    QVERIFY(findAction(actions, "editAdjustment") == nullptr);
    QVERIFY(findAction(actions, "rasterizeLayer") == nullptr);
    QVERIFY(findAction(actions, "rasterizeStyle") == nullptr);
    QVERIFY(findAction(actions, "copyShapeAttributes") == nullptr);
}

void LayersRowModelTest::theRowTableCarriesStaticCommandIds()
{
    LayerRowModel model;
    model.setRows({makeRow(QStringLiteral("pixel"))});

    RowFacts facts;
    facts.hasMask = true;
    facts.hasStyle = true;
    facts.canRasterizeStyle = true;
    const QVector<ResolvedRowAction> actions = model.actionsFor(QStringLiteral("1"), facts);

    const ResolvedRowAction* duplicate = findAction(actions, "duplicate");
    QVERIFY(duplicate != nullptr);
    QCOMPARE(duplicate->commandId, QString::fromLatin1(command_ids::LayerDuplicateLayer));
    const ResolvedRowAction* deleteMask = findAction(actions, "deleteLayerMask");
    QVERIFY(deleteMask != nullptr);
    QCOMPARE(deleteMask->commandId, QString::fromLatin1(command_ids::LayerMaskDelete));
    const ResolvedRowAction* convert = findAction(actions, "convertToSmartObject");
    QVERIFY(convert != nullptr);
    QCOMPARE(convert->commandId, QString::fromLatin1(command_ids::LayerSmartObjectConvertTo));
    const ResolvedRowAction* copyStyle = findAction(actions, "copyLayerStyle");
    QVERIFY(copyStyle != nullptr);
    QCOMPARE(copyStyle->commandId, QString::fromLatin1(command_ids::LayerStyleCopyLayerStyle));
    const ResolvedRowAction* merge = findAction(actions, "mergeDown");
    QVERIFY(merge != nullptr);
    QCOMPARE(merge->commandId, QString::fromLatin1(command_ids::LayerMergeLayers));
    const ResolvedRowAction* quickExport = findAction(actions, "quickExport");
    QVERIFY(quickExport != nullptr);
    QCOMPARE(quickExport->commandId, QString::fromLatin1(command_ids::FileQuickExportPng));

    const ResolvedRowAction* rasterizeStyle = findAction(actions, "rasterizeStyle");
    QVERIFY(rasterizeStyle != nullptr);
    QCOMPARE(rasterizeStyle->commandId,
             QString::fromLatin1(command_ids::LayerRasterizeLayerStyle));
    QVERIFY(rasterizeStyle->enabled);
}

void LayersRowModelTest::dynamicRowsResolveFromRowFacts()
{
    LayerRowModel model;
    LayerRowNode smart = makeRow(QStringLiteral("pixel"));
    smart.smartObject = true;
    model.setRows({smart});

    RowFacts facts;
    facts.canRasterizeSmartObject = true;
    QVector<ResolvedRowAction> actions = model.actionsFor(QStringLiteral("1"), facts);
    const ResolvedRowAction* rasterize = findAction(actions, "rasterizeLayer");
    QVERIFY(rasterize != nullptr);
    QCOMPARE(rasterize->commandId,
             QString::fromLatin1(command_ids::LayerRasterizeSmartObject));

    facts = {};
    facts.fillContent = true;
    actions = model.actionsFor(QStringLiteral("1"), facts);
    rasterize = findAction(actions, "rasterizeLayer");
    QVERIFY(rasterize != nullptr);
    QCOMPARE(rasterize->commandId, QString::fromLatin1(command_ids::LayerRasterizeFillContent));

    actions = model.actionsFor(QStringLiteral("1"), {});
    rasterize = findAction(actions, "rasterizeLayer");
    QVERIFY(rasterize != nullptr);
    QCOMPARE(rasterize->commandId, QString::fromLatin1(command_ids::LayerRasterizeLayer));
    const ResolvedRowAction* clipping = findAction(actions, "clippingToggle");
    QVERIFY(clipping != nullptr);
    QCOMPARE(clipping->commandId, QString::fromLatin1(command_ids::LayerCreateClippingMask));
    const ResolvedRowAction* link = findAction(actions, "link");
    QVERIFY(link != nullptr);
    QCOMPARE(link->commandId, QString::fromLatin1(command_ids::LayerLinkLayers));

    facts = {};
    facts.canReleaseClipping = true;
    actions = model.actionsFor(QStringLiteral("1"), facts);
    clipping = findAction(actions, "clippingToggle");
    QVERIFY(clipping != nullptr);
    QCOMPARE(clipping->commandId, QString::fromLatin1(command_ids::LayerReleaseClippingMask));

    smart.linked = true;
    model.setRows({smart});
    actions = model.actionsFor(QStringLiteral("1"), {});
    link = findAction(actions, "link");
    QVERIFY(link != nullptr);
    QCOMPARE(link->commandId, QString::fromLatin1(command_ids::LayerUnlinkLayers));

    LayerRowNode fill = makeRow(QStringLiteral("adjustment"));
    fill.hasAdjustment = true;
    model.setRows({fill});
    facts = {};
    facts.fillContent = true;
    actions = model.actionsFor(QStringLiteral("1"), facts);
    rasterize = findAction(actions, "rasterizeLayer");
    QVERIFY(rasterize != nullptr);
    QCOMPARE(rasterize->commandId, QString::fromLatin1(command_ids::LayerRasterizeFillContent));

    actions = model.actionsFor(QStringLiteral("1"), {});
    QVERIFY(findAction(actions, "rasterizeLayer") == nullptr);
}

void LayersRowModelTest::maskRowsExposeTheirSubmenus()
{
    LayerRowModel model;
    model.setRows({makeRow(QStringLiteral("pixel"))});

    QVector<ResolvedRowAction> actions = model.actionsFor(QStringLiteral("1"), {});
    const ResolvedRowAction* add = findAction(actions, "addLayerMask");
    QVERIFY(add != nullptr);
    QVERIFY(add->submenu);
    QVERIFY(add->commandId.isEmpty());
    QVERIFY(add->enabled);
    const ResolvedRowAction* addVector = findAction(actions, "addVectorMask");
    QVERIFY(addVector != nullptr);
    QVERIFY(addVector->submenu);
    QVERIFY(addVector->enabled);

    RowFacts facts;
    facts.hasMask = true;
    actions = model.actionsFor(QStringLiteral("1"), facts);
    add = findAction(actions, "addLayerMask");
    QVERIFY(add != nullptr);
    QVERIFY(!add->enabled);
    addVector = findAction(actions, "addVectorMask");
    QVERIFY(addVector != nullptr);
    QVERIFY(addVector->enabled);

    facts.hasVectorMask = true;
    actions = model.actionsFor(QStringLiteral("1"), facts);
    addVector = findAction(actions, "addVectorMask");
    QVERIFY(addVector != nullptr);
    QVERIFY(!addVector->enabled);
}

void LayersRowModelTest::unimplementedRowsAreDisabled()
{
    LayerRowModel model;
    model.setRows({makeRow(QStringLiteral("type"))});
    QVector<ResolvedRowAction> actions = model.actionsFor(QStringLiteral("1"), {});
    for (const char* id : {"copyCss", "copySvg"}) {
        const ResolvedRowAction* action = findAction(actions, id);
        QVERIFY2(action != nullptr, id);
        QVERIFY(action->commandId.isEmpty());
        QVERIFY(!action->enabled);
    }

    model.setRows({makeRow(QStringLiteral("adjustment"))});
    actions = model.actionsFor(QStringLiteral("1"), {});
    for (const char* id : {"artboardFromLayers", "frameFromLayers", "convertToFrame"}) {
        const ResolvedRowAction* action = findAction(actions, id);
        QVERIFY2(action != nullptr, id);
        QVERIFY(action->commandId.isEmpty());
        QVERIFY(!action->enabled);
    }

    LayerRowNode smart = makeRow(QStringLiteral("pixel"));
    smart.smartObject = true;
    model.setRows({smart});
    actions = model.actionsFor(QStringLiteral("1"), {});
    for (const char* id : {"relinkToFile", "convertToLinked"}) {
        const ResolvedRowAction* action = findAction(actions, id);
        QVERIFY2(action != nullptr, id);
        QVERIFY(action->commandId.isEmpty());
        QVERIFY(!action->enabled);
    }

}

void LayersRowModelTest::absentPathResolvesToEmpty()
{
    LayerRowModel model;
    model.setRows({makeRow(QStringLiteral("pixel"))});

    QVERIFY(model.rowForPath(QStringLiteral("absent")) == nullptr);
    QVERIFY(model.actionsFor(QStringLiteral("absent"), {}).isEmpty());
    QVERIFY(model.rowForPath(QStringLiteral("1")) != nullptr);
}

#include "qt_test_support.h"

PICTURA_QT_TEST_MAIN(LayersRowModelTest)
#include "tst_layers_row_model.moc"
