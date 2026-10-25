// The layer blend-mode controls build from the engine's list (#271): the
// Layers panel combo and Mode filter, the New Layer dialog and the Layer Style
// dialog all show the engine names in engine order, and a combo index is the
// engine index.

#include <QtTest/QtTest>

#include "blend_modes.h"
#include "frame.h"
#include "image_view.h"
#include "layer_new_dialog.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

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

private:
    // A white 40x40 document with layer "1" lifted off the Background and
    // selected in the Layers panel.
    void setupLayer();
    static void verifyNamesAndKeys(const QComboBox* combo, bool keyed);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
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
    panel_->setView(view_);
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->refresh();
    QCoreApplication::processEvents();
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
    verifyNamesAndKeys(panel_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo")), true);
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
    QVERIFY(view_->set_layer_blend(1, QStringLiteral("idiv")));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QString());
    auto* combo = dialog.findChild<QComboBox*>(QStringLiteral("blending.mode"));
    verifyNamesAndKeys(combo, false);
    int burn = -1;
    for (int i = 0; i < pictura::blendModes().size(); ++i) {
        if (pictura::blendModes().at(i).key == QStringLiteral("idiv")) {
            burn = i;
        }
    }
    QCOMPARE(combo->currentIndex(), burn);
    dialog.reject();
}

void BlendModeListTest::comboIndexSetsEngineKey()
{
    setupLayer();
    auto* combo = panel_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
    QVERIFY(combo != nullptr);
    QVERIFY(combo->isEnabled());
    for (const int i : {1, 4, 13, 26}) {
        combo->setCurrentIndex(i);
        QCoreApplication::processEvents();
        QCOMPARE(view_->layer_blend(1), pictura::blendModes().at(i).key);
    }
}

QTEST_MAIN(BlendModeListTest)
#include "tst_blend_mode_list.moc"
