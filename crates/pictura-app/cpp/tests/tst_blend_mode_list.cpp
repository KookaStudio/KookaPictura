// The layer blend-mode controls build from the engine's list (#271): the
// Layers panel combo and Mode filter, the New Layer dialog and the Layer Style
// dialog all show the engine names in engine order. The offered subset follows
// the document's mode and depth (#589).

#include <QtTest/QtTest>

#include "blend_modes.h"
#include "frame.h"
#include "image_view.h"
#include "layer_new_dialog.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/view_bridge.h"

#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QComboBox>

#include "qt_test_support.h"

class BlendModeListTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void layerCombosFollowEngineList();
    void layerStyleComboIsEngineIndexed();
    void comboIndexSetsEngineKey();
    void layersComboOffersHdrSubset();
    void layersComboFollowsLab();
    void layersComboFollowsIndexed();
    void layersComboRestoresAllModes();
    void layersComboKeepsUnsupportedCurrent();
    void layersComboLeavesGroupPassThrough();
    void newLayerDialogFollowsGate();
    void layerStyleCombosFollowGate();
    void layerStyleKeepsUnsupportedCurrent();

private:
    // A white 40x40 document with layer "1" lifted off the Background and
    // selected in the Layers panel.
    void setupLayer();
    // A 32-bit RGB document with an added layer selected in the panel.
    void setupHdrLayer();
    // The setupLayer document converted to Lab, with layer "1" selected.
    void setupLabLayer();
    // The setupLayer document flattened to Indexed, with layer "0" selected.
    void setupIndexed();
    void selectLayer();
    QString layerPath() const { return QString::number(layerIndex_); }
    QComboBox* layersCombo() const;
    static bool itemEnabled(const QComboBox* combo, int index);
    static QSet<QString> comboKeys(const QComboBox* combo);
    static void verifyNamesAndKeys(const QComboBox* combo, bool keyed);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    int layerIndex_ = 1;
};

void BlendModeListTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QCOMPARE(pictura::blendModes().size(), 27);
}

void BlendModeListTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
    layerIndex_ = 1;
}

void BlendModeListTest::setupLayer()
{
    QVERIFY(window_->newDocument(QStringLiteral("Blend"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    view_->select_rect(10, 10, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    layerIndex_ = 1;
    panel_->setView(view_);
    selectLayer();
}

void BlendModeListTest::setupHdrLayer()
{
    QVERIFY(window_->newDocument(QStringLiteral("BlendHdr"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    QVERIFY(pictura::create_document(*view_, 40, 40, QStringLiteral("rgb"), 32, 0xffffffu, false,
                                     72.0, false));
    QCOMPARE(view_->document_depth_bits(), 32);
    layerIndex_ = view_->add_layer(0);
    QVERIFY(layerIndex_ >= 0);
    panel_->setView(view_);
    selectLayer();
}

void BlendModeListTest::setupLabLayer()
{
    setupLayer();
    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("lab")));
    panel_->setView(view_);
    selectLayer();
}

void BlendModeListTest::setupIndexed()
{
    setupLayer();
    QVERIFY(pictura::convert_to_indexed(*view_, 1, 256, 0, 0));
    QCOMPARE(view_->layer_count(), 1);
    layerIndex_ = 0;
    panel_->setView(view_);
    selectLayer();
}

void BlendModeListTest::selectLayer()
{
    const QString path = layerPath();
    panel_->selectPaths({path}, path);
    panel_->refresh();
    QCoreApplication::processEvents();
}

QComboBox* BlendModeListTest::layersCombo() const
{
    return panel_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
}

bool BlendModeListTest::itemEnabled(const QComboBox* combo, int index)
{
    if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
        if (auto* item = model->item(index)) {
            return item->isEnabled();
        }
    }
    return false;
}

QSet<QString> BlendModeListTest::comboKeys(const QComboBox* combo)
{
    QSet<QString> keys;
    for (int i = 0; i < combo->count(); ++i) {
        keys.insert(combo->itemData(i).toString());
    }
    return keys;
}

void BlendModeListTest::verifyNamesAndKeys(const QComboBox* combo, bool keyed)
{
    QVERIFY(combo != nullptr);
    const QVector<pictura::BlendEntry>& modes = pictura::blendModes();
    QCOMPARE(combo->count(), modes.size());
    for (int i = 0; i < modes.size(); ++i) {
        QCOMPARE(combo->itemText(i), modes.at(i).name);
        if (keyed) {
            QCOMPARE(combo->itemData(i).toString(), modes.at(i).key);
        } else {
            QVERIFY(!combo->itemData(i).isValid());
        }
    }
}

void BlendModeListTest::layerCombosFollowEngineList()
{
    const QVector<pictura::BlendEntry>& modes = pictura::blendModes();
    QCOMPARE(modes.first().key, QStringLiteral("norm"));
    QCOMPARE(modes.first().name, QStringLiteral("Normal"));
    QCOMPARE(modes.last().key, QStringLiteral("lum "));
    QCOMPARE(modes.at(10).name, QStringLiteral("Linear Dodge (Add)"));
    QVERIFY(!modes.first().hasNeutral);
    QVERIFY(modes.at(3).hasNeutral); // Multiply's neutral is white.

    setupLayer();
    verifyNamesAndKeys(layersCombo(), true);
    verifyNamesAndKeys(panel_->findChild<QComboBox*>(QStringLiteral("layersFilterMode")), true);

    pictura::LayerNewDialog dialog(false);
    verifyNamesAndKeys(dialog.findChild<QComboBox*>(QStringLiteral("layerNewMode")), true);
    dialog.setModeForTest(QStringLiteral("norm"));
    QVERIFY(!dialog.neutralEnabledForTest());
    dialog.setModeForTest(QStringLiteral("mul "));
    QVERIFY(dialog.neutralEnabledForTest());
}

void BlendModeListTest::layerStyleComboIsEngineIndexed()
{
    setupLayer();
    QVERIFY(view_->set_layer_blend(layerIndex_, QStringLiteral("idiv")));
    pictura::LayerStyleDialog dialog(view_, layerPath(), QString());
    auto* combo = dialog.findChild<QComboBox*>(QStringLiteral("blending.mode"));
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), pictura::blendModes().size());
    for (int i = 0; i < combo->count(); ++i) {
        QCOMPARE(combo->itemText(i), pictura::blendModes().at(i).name);
        QCOMPARE(combo->itemData(i).toInt(), i);
    }
    const int burn = pictura::blendModeIndex(QStringLiteral("idiv"));
    QVERIFY(burn >= 0);
    QCOMPARE(combo->currentIndex(), burn);
    dialog.reject();
}

void BlendModeListTest::comboIndexSetsEngineKey()
{
    setupLayer();
    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QVERIFY(combo->isEnabled());
    for (const int i : {1, 4, 13, 26}) {
        combo->setCurrentIndex(i);
        QCoreApplication::processEvents();
        QCOMPARE(view_->layer_blend(layerIndex_), pictura::blendModes().at(i).key);
    }
}

void BlendModeListTest::layersComboOffersHdrSubset()
{
    setupHdrLayer();
    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), 15);
    const QSet<QString> keys = comboKeys(combo);
    for (const QString& key :
         {QStringLiteral("norm"), QStringLiteral("diss"), QStringLiteral("dark"),
          QStringLiteral("mul "), QStringLiteral("lite"), QStringLiteral("dkCl"),
          QStringLiteral("lddg"), QStringLiteral("lgCl"), QStringLiteral("diff"),
          QStringLiteral("fsub"), QStringLiteral("fdiv"), QStringLiteral("hue "),
          QStringLiteral("sat "), QStringLiteral("colr"), QStringLiteral("lum ")}) {
        QVERIFY2(keys.contains(key), qPrintable(key));
    }
    for (const QString& key :
         {QStringLiteral("idiv"), QStringLiteral("lbrn"), QStringLiteral("scrn"),
          QStringLiteral("div "), QStringLiteral("over"), QStringLiteral("sLit"),
          QStringLiteral("hLit"), QStringLiteral("vLit"), QStringLiteral("lLit"),
          QStringLiteral("pLit"), QStringLiteral("hMix"), QStringLiteral("smud")}) {
        QVERIFY2(!keys.contains(key), qPrintable(key));
    }
}

void BlendModeListTest::layersComboFollowsLab()
{
    setupLabLayer();
    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), 19);
    const QSet<QString> keys = comboKeys(combo);
    for (const QString& key :
         {QStringLiteral("idiv"), QStringLiteral("div "), QStringLiteral("dark"),
          QStringLiteral("lite"), QStringLiteral("diff"), QStringLiteral("smud"),
          QStringLiteral("fsub"), QStringLiteral("fdiv")}) {
        QVERIFY2(!keys.contains(key), qPrintable(key));
    }
    for (const QString& key :
         {QStringLiteral("norm"), QStringLiteral("scrn"), QStringLiteral("lum ")}) {
        QVERIFY2(keys.contains(key), qPrintable(key));
    }
}

void BlendModeListTest::layersComboFollowsIndexed()
{
    setupIndexed();
    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), 1);
    QCOMPARE(combo->itemData(0).toString(), QStringLiteral("norm"));
    QCOMPARE(combo->itemText(0), QStringLiteral("Normal"));
    QVERIFY(!combo->isEnabled());
}

void BlendModeListTest::layersComboRestoresAllModes()
{
    setupLayer();
    QVERIFY(view_->set_layer_blend(layerIndex_, QStringLiteral("idiv")));
    QCOMPARE(view_->layer_blend(layerIndex_), QStringLiteral("idiv"));
    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("lab")));
    panel_->setView(view_);
    selectLayer();

    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), 20); // 19 allowed + the disabled stored Color Burn
    const int restricted = combo->findData(QStringLiteral("idiv"));
    QVERIFY(restricted >= 0);
    QCOMPARE(combo->currentIndex(), restricted);
    QVERIFY(!itemEnabled(combo, restricted));

    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("rgb")));
    QCOMPARE(view_->layer_blend(layerIndex_), QStringLiteral("idiv"));
    panel_->setView(view_);
    selectLayer();

    QCOMPARE(combo->count(), 27);
    const int restored = combo->findData(QStringLiteral("idiv"));
    QVERIFY(restored >= 0);
    QCOMPARE(combo->currentIndex(), restored);
    QVERIFY(itemEnabled(combo, restored));
}

void BlendModeListTest::layersComboKeepsUnsupportedCurrent()
{
    setupHdrLayer();
    QVERIFY(view_->set_layer_blend(layerIndex_, QStringLiteral("scrn")));
    selectLayer();

    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    QCOMPARE(combo->count(), 16); // 15 allowed + the disabled stored Screen
    const int restricted = combo->findData(QStringLiteral("scrn"));
    QVERIFY(restricted >= 0);
    QCOMPARE(combo->currentIndex(), restricted);
    QVERIFY(!itemEnabled(combo, restricted));
    QCOMPARE(view_->layer_blend(layerIndex_), QStringLiteral("scrn"));

    const int multiply = combo->findData(QStringLiteral("mul "));
    QVERIFY(multiply >= 0);
    combo->setCurrentIndex(multiply);
    QCoreApplication::processEvents();
    QCOMPARE(view_->layer_blend(layerIndex_), QStringLiteral("mul "));
    QCOMPARE(combo->count(), 15);
    QCOMPARE(combo->currentData().toString(), QStringLiteral("mul "));
}

void BlendModeListTest::layersComboLeavesGroupPassThrough()
{
    QVERIFY(window_->newDocument(QStringLiteral("BlendGroup"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QStringList groupPaths;
    groupPaths << group;
    QVERIFY(view_->set_layers_blend(groupPaths, QStringLiteral("pass")) > 0);
    panel_->setView(view_);
    panel_->selectPaths({group}, group);
    panel_->refresh();
    QCoreApplication::processEvents();

    auto* combo = layersCombo();
    QVERIFY(combo != nullptr);
    // Pass Through is group-only and outside the 27-mode table; a dedicated
    // group entry is still pending, so the group keeps the Normal fallback.
    QVERIFY(combo->findData(QStringLiteral("pass")) < 0);
    QCOMPARE(combo->currentIndex(), 0);
    QCOMPARE(combo->currentText(), QStringLiteral("Normal"));
}

void BlendModeListTest::newLayerDialogFollowsGate()
{
    setupLabLayer();
    {
        pictura::LayerNewDialog dialog(false, nullptr, view_);
        auto* mode = dialog.findChild<QComboBox*>(QStringLiteral("layerNewMode"));
        QVERIFY(mode != nullptr);
        QCOMPARE(mode->count(), 19);
        QVERIFY(mode->findData(QStringLiteral("idiv")) < 0);
        QVERIFY(mode->findData(QStringLiteral("scrn")) >= 0);
    }
    {
        pictura::LayerNewDialog groupDialog(true, nullptr, view_);
        auto* mode = groupDialog.findChild<QComboBox*>(QStringLiteral("layerNewMode"));
        QVERIFY(mode != nullptr);
        QCOMPARE(mode->count(), 19);
    }
    {
        pictura::LayerNewDialog noView(false);
        auto* mode = noView.findChild<QComboBox*>(QStringLiteral("layerNewMode"));
        QVERIFY(mode != nullptr);
        QCOMPARE(mode->count(), 27);
    }

    setupHdrLayer();
    pictura::LayerNewDialog hdr(false, nullptr, view_);
    auto* mode = hdr.findChild<QComboBox*>(QStringLiteral("layerNewMode"));
    QVERIFY(mode != nullptr);
    QCOMPARE(mode->count(), 15);
    QVERIFY(mode->findData(QStringLiteral("scrn")) < 0);
}

void BlendModeListTest::layerStyleCombosFollowGate()
{
    setupLabLayer();
    {
        pictura::LayerStyleDialog dialog(view_, layerPath(), QString());
        auto* mode = dialog.findChild<QComboBox*>(QStringLiteral("blending.mode"));
        auto* shadow = dialog.findChild<QComboBox*>(QStringLiteral("dropShadow.mode"));
        QVERIFY(mode && shadow);
        QCOMPARE(mode->count(), 19);
        QCOMPARE(shadow->count(), 19);
        QVERIFY(mode->findData(pictura::blendModeIndex(QStringLiteral("idiv"))) < 0);
        for (int i = 0; i < mode->count(); ++i) {
            const int engine = mode->itemData(i).toInt();
            QVERIFY(engine >= 0);
            QCOMPARE(mode->itemText(i), pictura::blendModes().at(engine).name);
        }
    }

    setupHdrLayer();
    pictura::LayerStyleDialog hdr(view_, layerPath(), QString());
    auto* mode = hdr.findChild<QComboBox*>(QStringLiteral("blending.mode"));
    auto* shadow = hdr.findChild<QComboBox*>(QStringLiteral("dropShadow.mode"));
    QVERIFY(mode && shadow);
    QCOMPARE(mode->count(), 15);
    QCOMPARE(shadow->count(), 15);
    auto* innerGlow = hdr.findChild<QComboBox*>(QStringLiteral("innerGlow.mode"));
    QVERIFY(innerGlow != nullptr);
    QCOMPARE(innerGlow->count(), 16); // 15 allowed + the absent effect's default Screen
    const int glowScreen = innerGlow->findData(pictura::blendModeIndex(QStringLiteral("scrn")));
    QVERIFY(glowScreen >= 0);
    QVERIFY(!itemEnabled(innerGlow, glowScreen));
}

void BlendModeListTest::layerStyleKeepsUnsupportedCurrent()
{
    setupHdrLayer();
    QVERIFY(view_->set_layer_blend(layerIndex_, QStringLiteral("scrn")));

    pictura::LayerStyleDialog dialog(view_, layerPath(), QString());
    auto* mode = dialog.findChild<QComboBox*>(QStringLiteral("blending.mode"));
    QVERIFY(mode != nullptr);
    QCOMPARE(mode->count(), 16); // 15 allowed + the disabled stored Screen

    const int screenEngine = pictura::blendModeIndex(QStringLiteral("scrn"));
    QVERIFY(screenEngine >= 0);
    const int screenItem = mode->findData(screenEngine);
    QVERIFY(screenItem >= 0);
    QCOMPARE(mode->currentIndex(), screenItem);
    QVERIFY(!itemEnabled(mode, screenItem));
    QCOMPARE(pictura::layer_style_value(*view_, layerPath(), QStringLiteral("blending.mode")),
             double(screenEngine));

    const int multiplyEngine = pictura::blendModeIndex(QStringLiteral("mul "));
    QVERIFY(multiplyEngine >= 0);
    const int multiplyItem = mode->findData(multiplyEngine);
    QVERIFY(multiplyItem >= 0);
    mode->setCurrentIndex(multiplyItem);
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layer_style_value(*view_, layerPath(), QStringLiteral("blending.mode")),
             double(multiplyEngine));
    dialog.reject();
}

QTEST_MAIN(BlendModeListTest)
#include "tst_blend_mode_list.moc"
