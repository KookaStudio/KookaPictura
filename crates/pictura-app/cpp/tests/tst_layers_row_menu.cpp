// The Layers row context menu after the CS6 block rebuild: order, separators,
// per-kind rows, the brief Background menu, and the row-action wiring.

#include <QtTest/QtTest>
#include <QtWidgets/QListWidget>

#include "api/layers_api.h"
#include "api/type_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include "frame.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"

class LayersRowMenuTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void rowBlocksInOrder();
    void kindTextLists();
    void backgroundBriefMenu();
    void groupRows();
    void pixelMaskAndClippingRows();
    void linkRows();
    void mergeRows();
    void rasterizeRows();
    void isolationRow();
    void shapeOmitsEditAdjustment();
    void plainPixelOmitsSmartRows();
    void pixelRowFullSet();
    void groupAndAdjustmentOfferExport();
    void typeRowsOfferExport();
    void typeRows();
    void fillRows();
    void groupBlendingOptions();
    void pixelAndSmartRowsOfferExport();
    void unimplementedDisabled();

private:
    bool newDoc(const QString& name);
    QString makeLayer(const QString& name);
    QString makeTypeLayer(const QString& text);
    int indexOf(const QStringList& texts, const char* text) const;
    bool contains(const QStringList& texts, const QString& text) const;
    void refreshPanel();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayersRowMenuTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayersRowMenuTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool LayersRowMenuTest::newDoc(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    return view_ && panel_;
}

QString LayersRowMenuTest::makeLayer(const QString& name)
{
    const QString path = view_->add_layer_in(QString());
    if (!path.isEmpty()) {
        view_->set_layer_name_path(path, name);
    }
    return path;
}

QString LayersRowMenuTest::makeTypeLayer(const QString& text)
{
    pictura::TypeSetting setting{};
    setting.x = 2.0;
    setting.y = 8.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 24.0;
    const QString path = pictura::type_api::commitLayer(
        *view_, text, QStringLiteral("DejaVu Sans"), setting, character,
        pictura::type_api::defaultParagraphSetting());
    refreshPanel();
    return path;
}

int LayersRowMenuTest::indexOf(const QStringList& texts, const char* text) const
{
    return texts.indexOf(QString::fromUtf8(text));
}

bool LayersRowMenuTest::contains(const QStringList& texts, const QString& text) const
{
    return texts.contains(text);
}

void LayersRowMenuTest::refreshPanel()
{
    panel_->refresh();
    QCoreApplication::processEvents();
}

void LayersRowMenuTest::rowBlocksInOrder()
{
    QVERIFY(newDoc(QStringLiteral("Blocks")));
    const QString path = makeLayer(QStringLiteral("Base"));
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")));

    const QStringList texts = panel_->rowMenuTextsForPathForTest(path);
    const int blending = indexOf(texts, "Blending Options…");
    const int duplicate = indexOf(texts, "Duplicate Layer…");
    const int quick = indexOf(texts, "Quick Export as PNG");
    const int exportAs = indexOf(texts, "Export As…");
    const int convert = indexOf(texts, "Convert to Smart Object");
    const int mask = indexOf(texts, "Add Layer Mask");
    const int link = indexOf(texts, "Link Layers");
    const int style = indexOf(texts, "Copy Layer Style");
    const int merge = indexOf(texts, "Merge Down");
    const int label = indexOf(texts, "Color Label");
    QVERIFY(blending >= 0 && duplicate > blending);
    QVERIFY(quick > duplicate && exportAs > quick);
    QVERIFY(convert > exportAs && mask > convert);
    QVERIFY(link > mask && style > link && merge > style && label > merge);
    QVERIFY2(panel_->rowMenuSeparatorCountForPathForTest(path) >= 6,
             "one separator between the non-empty blocks");
    QVERIFY2(!panel_->rowMenuHasAdjacentSeparatorsForPathForTest(path),
             "no empty command block emits a separator");
}

void LayersRowMenuTest::kindTextLists()
{
    QVERIFY(newDoc(QStringLiteral("KindLists")));
    QCOMPARE(panel_->rowMenuTextsForTest(QStringLiteral("pixel")),
             (QStringList{QStringLiteral("Rename"),
                          QStringLiteral("Blending Options…"),
                          QStringLiteral("Duplicate Layer…"),
                          QStringLiteral("Delete Layer"),
                          QStringLiteral("Group from Layers…"),
                          QStringLiteral("New Layer"),
                          QStringLiteral("New Group"),
                          QStringLiteral("Move Layer Up"),
                          QStringLiteral("Move Layer Down"),
                          QStringLiteral("Quick Export as PNG"),
                          QStringLiteral("Export As…"),
                          QStringLiteral("Convert to Smart Object"),
                          QStringLiteral("Add Layer Mask"),
                          QStringLiteral("Delete Layer Mask"),
                          QStringLiteral("Enable Layer Mask"),
                          QStringLiteral("Disable Layer Mask"),
                          QStringLiteral("Add Vector Mask"),
                          QStringLiteral("Delete Vector Mask"),
                          QStringLiteral("Enable Vector Mask"),
                          QStringLiteral("Disable Vector Mask"),
                          QStringLiteral("Create Clipping Mask"),
                          QStringLiteral("Link Layers"),
                          QStringLiteral("Select Linked Layers"),
                          QStringLiteral("Copy Layer Style"),
                          QStringLiteral("Paste Layer Style"),
                          QStringLiteral("Clear Layer Style"),
                          QStringLiteral("Merge Down"),
                          QStringLiteral("Merge Visible"),
                          QStringLiteral("Merge Clipping Mask"),
                          QStringLiteral("Flatten Image"),
                          QStringLiteral("Color Label")}));
    QCOMPARE(panel_->rowMenuTextsForTest(QStringLiteral("group")),
             (QStringList{QStringLiteral("Rename"),
                          QStringLiteral("Blending Options…"),
                          QStringLiteral("Copy CSS"),
                          QStringLiteral("Copy SVG"),
                          QStringLiteral("Duplicate Group…"),
                          QStringLiteral("Delete Group"),
                          QStringLiteral("Group from Layers…"),
                          QStringLiteral("New Layer"),
                          QStringLiteral("New Group"),
                          QStringLiteral("Ungroup Layers"),
                          QStringLiteral("Move Layer Up"),
                          QStringLiteral("Move Layer Down"),
                          QStringLiteral("Quick Export as PNG"),
                          QStringLiteral("Export As…"),
                          QStringLiteral("Artboard from Layers…"),
                          QStringLiteral("Frame from Layers…"),
                          QStringLiteral("Convert to Frame"),
                          QStringLiteral("Convert to Smart Object"),
                          QStringLiteral("Add Layer Mask"),
                          QStringLiteral("Delete Layer Mask"),
                          QStringLiteral("Enable Layer Mask"),
                          QStringLiteral("Disable Layer Mask"),
                          QStringLiteral("Add Vector Mask"),
                          QStringLiteral("Delete Vector Mask"),
                          QStringLiteral("Enable Vector Mask"),
                          QStringLiteral("Disable Vector Mask"),
                          QStringLiteral("Link Layers"),
                          QStringLiteral("Select Linked Layers"),
                          QStringLiteral("Copy Layer Style"),
                          QStringLiteral("Paste Layer Style"),
                          QStringLiteral("Clear Layer Style"),
                          QStringLiteral("Merge Layers"),
                          QStringLiteral("Merge Visible"),
                          QStringLiteral("Merge Clipping Mask"),
                          QStringLiteral("Flatten Image"),
                          QStringLiteral("Color Label")}));
    QCOMPARE(panel_->rowMenuTextsForTest(QStringLiteral("adjustment")),
             (QStringList{QStringLiteral("Rename"),
                          QStringLiteral("Edit Adjustment…"),
                          QStringLiteral("Copy CSS"),
                          QStringLiteral("Copy SVG"),
                          QStringLiteral("Duplicate Layer…"),
                          QStringLiteral("Delete Layer"),
                          QStringLiteral("Group from Layers…"),
                          QStringLiteral("New Layer"),
                          QStringLiteral("New Group"),
                          QStringLiteral("Move Layer Up"),
                          QStringLiteral("Move Layer Down"),
                          QStringLiteral("Quick Export as PNG"),
                          QStringLiteral("Export As…"),
                          QStringLiteral("Artboard from Layers…"),
                          QStringLiteral("Frame from Layers…"),
                          QStringLiteral("Convert to Frame"),
                          QStringLiteral("Convert to Smart Object"),
                          QStringLiteral("Add Layer Mask"),
                          QStringLiteral("Delete Layer Mask"),
                          QStringLiteral("Enable Layer Mask"),
                          QStringLiteral("Disable Layer Mask"),
                          QStringLiteral("Add Vector Mask"),
                          QStringLiteral("Delete Vector Mask"),
                          QStringLiteral("Enable Vector Mask"),
                          QStringLiteral("Disable Vector Mask"),
                          QStringLiteral("Create Clipping Mask"),
                          QStringLiteral("Link Layers"),
                          QStringLiteral("Select Linked Layers"),
                          QStringLiteral("Copy Layer Style"),
                          QStringLiteral("Paste Layer Style"),
                          QStringLiteral("Clear Layer Style"),
                          QStringLiteral("Merge Down"),
                          QStringLiteral("Merge Visible"),
                          QStringLiteral("Merge Clipping Mask"),
                          QStringLiteral("Flatten Image"),
                          QStringLiteral("Color Label")}));
    QCOMPARE(panel_->rowMenuTextsForTest(QStringLiteral("type")),
             (QStringList{QStringLiteral("Rename"),
                          QStringLiteral("Blending Options…"),
                          QStringLiteral("Copy CSS"),
                          QStringLiteral("Copy SVG"),
                          QStringLiteral("Duplicate Layer…"),
                          QStringLiteral("Delete Layer"),
                          QStringLiteral("Group from Layers…"),
                          QStringLiteral("New Layer"),
                          QStringLiteral("New Group"),
                          QStringLiteral("Move Layer Up"),
                          QStringLiteral("Move Layer Down"),
                          QStringLiteral("Quick Export as PNG"),
                          QStringLiteral("Export As…"),
                          QStringLiteral("Artboard from Layers…"),
                          QStringLiteral("Frame from Layers…"),
                          QStringLiteral("Convert to Frame"),
                          QStringLiteral("Convert to Smart Object"),
                          QStringLiteral("Rasterize Type"),
                          QStringLiteral("Link Layers"),
                          QStringLiteral("Select Linked Layers"),
                          QStringLiteral("Copy Layer Style"),
                          QStringLiteral("Paste Layer Style"),
                          QStringLiteral("Clear Layer Style"),
                          QStringLiteral("Merge Visible"),
                          QStringLiteral("Merge Clipping Mask"),
                          QStringLiteral("Flatten Image"),
                          QStringLiteral("Color Label")}));
}

void LayersRowMenuTest::backgroundBriefMenu()
{
    QVERIFY(newDoc(QStringLiteral("Brief")));
    const QStringList texts = panel_->rowMenuTextsForTest(QStringLiteral("background"));
    QCOMPARE(texts,
             (QStringList{QStringLiteral("Layer from Background…"),
                          QStringLiteral("Duplicate Layer…"), QStringLiteral("Delete Layer"),
                          QStringLiteral("Quick Export as PNG"), QStringLiteral("Export As…"),
                          QStringLiteral("Convert to Smart Object")}));
    QVERIFY(!texts.contains(QStringLiteral("Rename")));
    QVERIFY(!texts.contains(QStringLiteral("Blending Options…")));
    QVERIFY(!texts.contains(QStringLiteral("Color Label")));
}

void LayersRowMenuTest::groupRows()
{
    QVERIFY(newDoc(QStringLiteral("GroupRows")));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QVERIFY(!view_->add_layer_in(group).isEmpty());
    refreshPanel();

    const QStringList texts = panel_->rowMenuTextsForPathForTest(group);
    QVERIFY(texts.contains(QStringLiteral("Add Layer Mask")));
    QVERIFY(texts.contains(QStringLiteral("Add Vector Mask")));
    QVERIFY(texts.contains(QStringLiteral("Group from Layers…")));
    QVERIFY(texts.contains(QStringLiteral("Link Layers")));
    QVERIFY(texts.contains(QStringLiteral("Merge Layers")));
    QVERIFY(texts.contains(QStringLiteral("Quick Export as PNG")));
    QVERIFY(texts.contains(QStringLiteral("Convert to Smart Object")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(group,
                                                  QStringLiteral("Convert to Smart Object")));
    QVERIFY(panel_->rowMenuEnabledForPathForTest(group, QStringLiteral("Blending Options…")));
    QVERIFY(texts.contains(QStringLiteral("Ungroup Layers")));
    QVERIFY(!panel_->rowMenuHasAdjacentSeparatorsForPathForTest(group));
}

void LayersRowMenuTest::pixelMaskAndClippingRows()
{
    QVERIFY(newDoc(QStringLiteral("MaskRows")));
    const QString base = makeLayer(QStringLiteral("Base"));
    const QString top = makeLayer(QStringLiteral("Top"));
    QVERIFY(!base.isEmpty() && !top.isEmpty());
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(base),
                     QStringLiteral("Add Vector Mask")));
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(base),
                     QStringLiteral("Create Clipping Mask")));
    QVERIFY(pictura::clipping_create(*view_, QStringList{top}) == 1);
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(top),
                     QStringLiteral("Release Clipping Mask")));
}

void LayersRowMenuTest::linkRows()
{
    QVERIFY(newDoc(QStringLiteral("LinkRows")));
    const QString path = makeLayer(QStringLiteral("Linked"));
    QVERIFY(!path.isEmpty());
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(path),
                     QStringLiteral("Link Layers")));
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(path),
                     QStringLiteral("Select Linked Layers")));
    QVERIFY(view_->link_layers(QStringList{path}, true) == 1);
    refreshPanel();
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(path),
                     QStringLiteral("Unlink Layers")));
}

void LayersRowMenuTest::mergeRows()
{
    QVERIFY(newDoc(QStringLiteral("MergeRows")));
    const QString a = makeLayer(QStringLiteral("A"));
    const QString b = makeLayer(QStringLiteral("B"));
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    pictura::select_layer_path(*view_, a);
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(a),
                     QStringLiteral("Merge Down")));
    panel_->selectPaths({a, b}, b);
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(b),
                     QStringLiteral("Merge Layers")));
}

void LayersRowMenuTest::rasterizeRows()
{
    QVERIFY(newDoc(QStringLiteral("RasterRows")));
    const QString pixel = makeLayer(QStringLiteral("Pixel"));
    QVERIFY(!pixel.isEmpty());
    const QStringList pixelRows = panel_->rowMenuTextsForPathForTest(pixel);
    QVERIFY(!pixelRows.contains(QStringLiteral("Rasterize Type")));
    QVERIFY(!pixelRows.contains(QStringLiteral("Rasterize Shape")));
    QVERIFY(!pixelRows.contains(QStringLiteral("Rasterize Layer")));
    QVERIFY(!panel_->rowMenuHasAdjacentSeparatorsForPathForTest(pixel));

    const QStringList typeRows = panel_->rowMenuTextsForTest(QStringLiteral("type"));
    QVERIFY(typeRows.contains(QStringLiteral("Rasterize Type")));

    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 2;
    spec.y0 = 2;
    spec.x1 = 8;
    spec.y1 = 8;
    const QString shape = pictura::shape_add_layer(*view_, spec, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    refreshPanel();
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(shape),
                     QStringLiteral("Rasterize Shape")));

    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    refreshPanel();
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(fill),
                     QStringLiteral("Rasterize Layer")));

    QVERIFY(!panel_->rowMenuTextsForPathForTest(pixel).contains(
        QStringLiteral("Rasterize Layer Style")));
    QVERIFY(pictura::layer_style_set(*view_, pixel, QStringLiteral("colorOverlay.on"), 1.0));
    const QStringList styled = panel_->rowMenuTextsForPathForTest(pixel);
    QVERIFY(styled.contains(QStringLiteral("Rasterize Layer Style")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(pixel,
                                                  QStringLiteral("Rasterize Layer Style")));
}

void LayersRowMenuTest::isolationRow()
{
    QVERIFY(newDoc(QStringLiteral("Isolation")));
    const QString path = makeLayer(QStringLiteral("Solo"));
    QVERIFY(!path.isEmpty());
    panel_->selectPaths({path}, path);
    panel_->enterIsolation();
    const QStringList texts = panel_->rowMenuTextsForPathForTest(path);
    const int release = indexOf(texts, "Release from Isolation");
    const int style = indexOf(texts, "Copy Layer Style");
    const int merge = indexOf(texts, "Merge Down");
    QVERIFY(release > style && release < merge);
    panel_->releaseIsolation();
}

void LayersRowMenuTest::shapeOmitsEditAdjustment()
{
    QVERIFY(newDoc(QStringLiteral("ShapeEdit")));
    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 2;
    spec.y0 = 2;
    spec.x1 = 8;
    spec.y1 = 8;
    const QString shape = pictura::shape_add_layer(*view_, spec, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    QVERIFY(!panel_->rowMenuTextsForPathForTest(shape).contains(
        QStringLiteral("Edit Adjustment…")));
}

void LayersRowMenuTest::plainPixelOmitsSmartRows()
{
    QVERIFY(newDoc(QStringLiteral("NoSmart")));
    const QString path = makeLayer(QStringLiteral("Plain"));
    const QStringList texts = panel_->rowMenuTextsForPathForTest(path);
    for (const char* row : {"Reset Transform", "Convert to Layers", "New Smart Object via Copy",
                            "Edit Contents", "Replace Contents…", "Export Contents…"}) {
        QVERIFY2(!texts.contains(QString::fromUtf8(row)), row);
    }
}

void LayersRowMenuTest::pixelRowFullSet()
{
    QVERIFY(newDoc(QStringLiteral("FullSet")));
    const QString path = makeLayer(QStringLiteral("Pixel"));
    const QStringList texts = panel_->rowMenuTextsForPathForTest(path);
    for (const char* row : {"Duplicate Layer…", "Delete Layer", "Group from Layers…",
                            "Quick Export as PNG", "Export As…", "Convert to Smart Object",
                            "Add Layer Mask", "Add Vector Mask", "Create Clipping Mask",
                            "Link Layers", "Select Linked Layers", "Copy Layer Style",
                            "Merge Down", "Flatten Image", "Color Label"}) {
        QVERIFY2(texts.contains(QString::fromUtf8(row)), row);
    }
}

void LayersRowMenuTest::groupAndAdjustmentOfferExport()
{
    QVERIFY(newDoc(QStringLiteral("ExportRows")));
    QVERIFY(panel_->rowMenuTextsForTest(QStringLiteral("group"))
                .contains(QStringLiteral("Export As…")));
    QVERIFY(panel_->rowMenuTextsForTest(QStringLiteral("adjustment"))
                .contains(QStringLiteral("Quick Export as PNG")));
}

void LayersRowMenuTest::typeRowsOfferExport()
{
    QVERIFY(newDoc(QStringLiteral("TypeExport")));
    const QStringList texts = panel_->rowMenuTextsForTest(QStringLiteral("type"));
    QVERIFY(texts.contains(QStringLiteral("Export As…")));
    QVERIFY(texts.contains(QStringLiteral("Quick Export as PNG")));
}

void LayersRowMenuTest::typeRows()
{
    QVERIFY(newDoc(QStringLiteral("TypeRows")));
    const QString path = makeTypeLayer(QStringLiteral("Type"));
    QVERIFY(!path.isEmpty());
    const QStringList texts = panel_->rowMenuTextsForPathForTest(path);
    QVERIFY(texts.contains(QStringLiteral("Create Work Path")));
    QVERIFY(texts.contains(QStringLiteral("Convert to Shape")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Create Work Path")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Shape")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Copy CSS")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Copy SVG")));
    QVERIFY(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Smart Object")));
    QVERIFY(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Warp Text…")));
    QVERIFY(texts.contains(QStringLiteral("Horizontal")));
    QVERIFY(texts.contains(QStringLiteral("Vertical")));
    QVERIFY(texts.contains(QStringLiteral("None")));
    QVERIFY(texts.contains(QStringLiteral("Sharp")));
    QVERIFY(texts.contains(QStringLiteral("Convert to Paragraph Text")));

    QVERIFY(panel_->triggerRowMenuActionForPathForTest(path, QStringLiteral("Vertical")));
    QVERIFY(pictura::type_api::layerSetting(*view_, path).vertical);
    QVERIFY(panel_->triggerRowMenuActionForPathForTest(path, QStringLiteral("None")));
    QCOMPARE(pictura::type_api::layerCharacterSetting(*view_, path).anti_alias, 0);
    QVERIFY(panel_->triggerRowMenuActionForPathForTest(
        path, QStringLiteral("Convert to Paragraph Text")));
    QVERIFY(pictura::type_api::isParagraph(*view_, path));
}

void LayersRowMenuTest::fillRows()
{
    QVERIFY(newDoc(QStringLiteral("FillRows")));
    const QString base = makeLayer(QStringLiteral("Base"));
    QVERIFY(!base.isEmpty());
    pictura::select_layer_path(*view_, base);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    refreshPanel();
    const QStringList texts = panel_->rowMenuTextsForPathForTest(fill);
    QVERIFY(texts.contains(QStringLiteral("Duplicate Layer…")));
    QVERIFY(texts.contains(QStringLiteral("Quick Export as PNG")));
    QVERIFY(texts.contains(QStringLiteral("Delete Layer Mask")));
    QVERIFY(texts.contains(QStringLiteral("Add Vector Mask")));
    QVERIFY(texts.contains(QStringLiteral("Create Clipping Mask")));
    QVERIFY(texts.contains(QStringLiteral("Merge Layers")));
    QVERIFY(texts.contains(QStringLiteral("Rasterize Layer")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(fill, QStringLiteral("Copy CSS")));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(fill,
                                                  QStringLiteral("Convert to Smart Object")));
    QVERIFY(!panel_->rowMenuHasAdjacentSeparatorsForPathForTest(fill));
}

void LayersRowMenuTest::groupBlendingOptions()
{
    QVERIFY(newDoc(QStringLiteral("GroupStyle")));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    refreshPanel();
    QVERIFY(panel_->rowMenuEnabledForPathForTest(group, QStringLiteral("Blending Options…")));
    pictura::LayerStyleDialog dialog(view_, group, QString(), nullptr);
    QCOMPARE(dialog.effectList()->count(), 11);
    for (int row = 1; row < dialog.effectList()->count(); ++row) {
        QVERIFY2(!(dialog.effectList()->item(row)->flags() & Qt::ItemIsEnabled),
                 "group effect rows are disabled until group effects render");
    }
}

void LayersRowMenuTest::pixelAndSmartRowsOfferExport()
{
    QVERIFY(newDoc(QStringLiteral("ExportRows")));
    const QString pixel = makeLayer(QStringLiteral("Pixel"));
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(pixel),
                     QStringLiteral("Quick Export as PNG")));
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(pixel),
                     QStringLiteral("Export As…")));
    QVERIFY(view_->convert_to_smart_object(pixel));
    QVERIFY(contains(panel_->rowMenuTextsForPathForTest(pixel),
                     QStringLiteral("Quick Export as PNG")));
}

void LayersRowMenuTest::unimplementedDisabled()
{
    QVERIFY(newDoc(QStringLiteral("Unimplemented")));
    const QString path = makeLayer(QStringLiteral("Styled"));
    QVERIFY(pictura::layer_style_set(*view_, path, QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(!panel_->rowMenuEnabledForPathForTest(path,
                                                  QStringLiteral("Rasterize Layer Style")));
    QVERIFY(panel_->rowMenuToolTipForPathForTest(path,
                                                 QStringLiteral("Rasterize Layer Style"))
                .contains(QStringLiteral("not implemented yet")));
}

QTEST_MAIN(LayersRowMenuTest)
#include "tst_layers_row_menu.moc"
