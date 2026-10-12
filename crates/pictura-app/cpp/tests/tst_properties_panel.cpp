#include <QtTest/QtTest>
#include "api/layers_api.h"

#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class PropertiesPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void plainDocumentShowsNoProperties();
    void adjustmentLayerShowsItsName();
    void slidersEditLiveAndCommitOnce();
    void groupsCurvesAndFooter();
    void canvasSectionReadsAndResizes();
    void maskSectionAppearsForAMaskedLayer();
    void vectorMaskSectionAppears();

private:
    // A white 16x16 document with one `kind` adjustment layer, selected.
    bool setupAdjustment(const QString& kind);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::PropertiesPanel* panel_ = nullptr;
    pictura::LayersPanel* layers_ = nullptr;
    QString path_;
};

void PropertiesPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PropertiesPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

// No document reads No Properties; a plain layer gets the read-only summary
// (a post-CS6 page ported from photorust, #70).
void PropertiesPanelTest::plainDocumentShowsNoProperties()
{
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    QVERIFY(panel_);
    panel_->setView(nullptr);
    QCOMPARE(panel_->messageForTest(), QStringLiteral("No Properties"));

    const bool created = window_->newDocument(QStringLiteral("Props"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && layers, "properties fixture");
    panel_->setView(view_);
    layers->setView(view_);
    QVERIFY(layers->selectRowForTest(pictura::layers_api::rowAt(*view_, 0).path));
    panel_->refresh();
    QCOMPARE(panel_->messageForTest(), QStringLiteral("Layer Properties"));
    QStringList text;
    for (QLabel* label : panel_->findChildren<QLabel*>()) {
        text << label->text();
    }
    // A new document's single layer is "Layer 0", a pixel layer.
    QVERIFY(text.contains(QStringLiteral("Pixel")));
    QVERIFY(text.contains(QStringLiteral("16 x 16 px")));
    QVERIFY(text.contains(QStringLiteral("Normal")));
    QVERIFY(text.contains(QStringLiteral("100%")));
}

bool PropertiesPanelTest::setupAdjustment(const QString& kind)
{
    if (!window_->newDocument(QStringLiteral("Adjust"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    layers_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_ || !layers_ || !view_->add_adjustment(kind)) {
        return false;
    }
    path_.clear();
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        if (pictura::layers_api::rowAt(*view_, i).hasAdjustment) {
            path_ = pictura::layers_api::rowAt(*view_, i).path;
        }
    }
    panel_->setView(view_);
    layers_->setView(view_);
    if (path_.isEmpty() || !layers_->selectRowForTest(path_)) {
        return false;
    }
    panel_->refresh();
    return panel_->pathForTest() == path_;
}

void PropertiesPanelTest::adjustmentLayerShowsItsName()
{
    const bool created = window_->newDocument(QStringLiteral("Adjust"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_ && layers, "adjustment fixture");
    QVERIFY2(view_->add_adjustment(QStringLiteral("brightness-contrast")), "add adjustment");

    QString path;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        if (pictura::layers_api::rowAt(*view_, i).hasAdjustment) {
            path = pictura::layers_api::rowAt(*view_, i).path;
            break;
        }
    }
    QVERIFY2(!path.isEmpty(), "adjustment row exists");

    panel_->setView(view_);
    layers->setView(view_);
    QVERIFY2(layers->selectRowForTest(path), "adjustment row selected");
    QVERIFY2(panel_->messageForTest().contains(QStringLiteral("Brightness/Contrast")),
             "adjustment kind named");
}

// A slider changes the canvas live, and the gesture is one "Modify … Layer"
// state; undo restores the panel's value and the pixels.
void PropertiesPanelTest::slidersEditLiveAndCommitOnce()
{
    QVERIFY(setupAdjustment(QStringLiteral("hue-saturation")));
    QCOMPARE(panel_->messageForTest(), QStringLiteral("Hue/Saturation"));
    auto* hue = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("hue")));
    auto* lightness = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("lightness")));
    QVERIFY(hue && lightness);
    QCOMPARE(hue->value(), 30);
    const QRgb before = view_->composite_argb(4, 4);
    const int history = view_->history_count();
    lightness->setValue(-50);
    lightness->setValue(-100);
    QCOMPARE(qGray(view_->composite_argb(4, 4)), 0);
    QCOMPARE(view_->history_count(), history);
    panel_->commitForTest();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Modify Hue/Saturation Layer"));

    QVERIFY(view_->undo());
    panel_->refresh();
    QCOMPARE(lightness->value(), 0);
    QCOMPARE(view_->composite_argb(4, 4), before);
}

// Color Balance's tone menu switches the sliders; Reset restores defaults;
// the footer toggles visibility and deletes. (Curves editing is covered by the
// engine's `adjustment_params` tests: `add_adjustment` makes no Curves layer.)
void PropertiesPanelTest::groupsCurvesAndFooter()
{
    QVERIFY(setupAdjustment(QStringLiteral("color-balance")));
    auto* group = qobject_cast<QComboBox*>(panel_->controlForTest(QStringLiteral("group")));
    QWidget* shadows = panel_->controlForTest(QStringLiteral("shadows.cyanRed"));
    QWidget* highlights = panel_->controlForTest(QStringLiteral("highlights.cyanRed"));
    QVERIFY(group && shadows && highlights);
    QCOMPARE(group->count(), 3);
    QVERIFY(shadows->isVisibleTo(panel_) && !highlights->isVisibleTo(panel_));
    group->setCurrentIndex(2);
    QVERIFY(!shadows->isVisibleTo(panel_) && highlights->isVisibleTo(panel_));
    qobject_cast<QSlider*>(highlights)->setValue(60);
    panel_->commitForTest();
    auto* luminosity =
        qobject_cast<QCheckBox*>(panel_->controlForTest(QStringLiteral("preserveLuminosity")));
    QVERIFY(luminosity && luminosity->isChecked());
    luminosity->setChecked(false);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Modify Color Balance Layer"));

    auto* reset = panel_->findChild<QToolButton*>(QStringLiteral("propertiesReset"));
    QVERIFY(reset);
    reset->click();
    QCOMPARE(qobject_cast<QSlider*>(highlights)->value(), 0);
    QVERIFY(luminosity->isChecked());

    auto* visible = panel_->findChild<QToolButton*>(QStringLiteral("propertiesVisible"));
    QVERIFY(visible);
    visible->click();
    int row = -1;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        row = pictura::layers_api::rowAt(*view_, i).path == path_ ? i : row;
    }
    QVERIFY(row >= 0 && !pictura::layers_api::rowAt(*view_, row).visible);
    const int rows = pictura::layers_api::rowCount(*view_);
    panel_->findChild<QToolButton*>(QStringLiteral("propertiesDelete"))->click();
    QCOMPARE(pictura::layers_api::rowCount(*view_), rows - 1);
    window_->closeDocument(window_->activeDocumentIndex(), false);

    // A single-slider page (Threshold) commits under its own name.
    QVERIFY(setupAdjustment(QStringLiteral("threshold")));
    auto* level = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("level")));
    QVERIFY(level);
    QCOMPARE(level->value(), 128);
    level->setValue(250);
    panel_->commitForTest();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Modify Threshold Layer"));
}

// The Canvas section (post-CS6): W/H resize the canvas about its centre (linked
// keeps the aspect), the resolution reads 72 ppi without a ResolutionInfo
// resource, and the Mode / depth menus hand the choice to Image ▸ Mode. With no
// active layer the header reads Document.
void PropertiesPanelTest::canvasSectionReadsAndResizes()
{
    QVERIFY(window_->newDocument(QStringLiteral("Canvas"), 16, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    QVERIFY(view_ && panel_);
    view_->set_active_layer(QString());
    panel_->setView(view_);
    QCOMPARE(panel_->messageForTest(), QStringLiteral("Document"));

    QSpinBox* width = panel_->canvasWidthForTest();
    QSpinBox* height = panel_->canvasHeightForTest();
    QVERIFY(width && height && width->isVisibleTo(panel_));
    QCOMPARE(width->value(), 16);
    QCOMPARE(height->value(), 8);
    QCOMPARE(panel_->resolutionForTest(), QStringLiteral("Resolution: 72 pixels/inch"));
    QCOMPARE(panel_->modeForTest()->currentText(), QStringLiteral("RGB Color"));
    QCOMPARE(panel_->depthForTest()->currentText(), QStringLiteral("8 Bits/Channel"));

    panel_->canvasLinkForTest()->setChecked(true);
    width->setValue(32);
    QCOMPARE(height->value(), 16);
    emit width->editingFinished();
    QCOMPARE(view_->document_width(), 32);
    QCOMPARE(view_->document_height(), 16);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Canvas Size"));

    QSignalSpy modes(panel_, &pictura::PropertiesPanel::modeRequested);
    QSignalSpy depths(panel_, &pictura::PropertiesPanel::depthRequested);
    QComboBox* mode = panel_->modeForTest();
    emit mode->activated(mode->findData(QStringLiteral("lab")));
    QComboBox* depth = panel_->depthForTest();
    emit depth->activated(depth->findData(16));
    QCOMPARE(modes.size(), 1);
    QCOMPARE(modes.at(0).at(0).toString(), QStringLiteral("lab"));
    QCOMPARE(depths.size(), 1);
    QCOMPARE(depths.at(0).at(0).toInt(), 16);
    // The window ran the depth conversion off the panel's request.
    QCOMPARE(view_->document_depth_bits(), 16);
}

// The Mask section appears only for a masked layer, and Delete clears it.
void PropertiesPanelTest::maskSectionAppearsForAMaskedLayer()
{
    QVERIFY(window_->newDocument(QStringLiteral("Masked"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(view_ && panel_ && layers, "mask fixture");
    layers->setView(view_);
    QVERIFY(layers->selectRowForTest(pictura::layers_api::rowAt(*view_, 0).path));
    panel_->setView(view_);
    panel_->refresh();
    QVERIFY2(!panel_->maskSectionVisibleForTest(), "no mask section without a mask");

    QVERIFY(layer_mask_add(*view_, QStringLiteral("reveal-all")));
    panel_->refresh();
    QVERIFY2(panel_->maskSectionVisibleForTest(), "mask section with a mask");

    auto* remove = panel_->findChild<QToolButton*>(QStringLiteral("propertiesMaskDelete"));
    QVERIFY(remove);
    remove->click();
    panel_->refresh();
    QVERIFY2(!panel_->maskSectionVisibleForTest(), "section gone after delete");
    QVERIFY(!pictura::layers_api::rowAt(*view_, 0).hasMask);
}

// The Vector Mask section appears only for a vector-masked layer; Delete clears
// it and Rasterize converts it to a layer mask.
void PropertiesPanelTest::vectorMaskSectionAppears()
{
    QVERIFY(window_->newDocument(QStringLiteral("VectorMasked"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(view_ && panel_ && layers, "vector mask fixture");
    layers->setView(view_);
    QVERIFY(layers->selectRowForTest(pictura::layers_api::rowAt(*view_, 0).path));
    panel_->setView(view_);
    panel_->refresh();
    QVERIFY2(!panel_->vectorMaskSectionVisibleForTest(), "no vector mask section without a mask");

    QVERIFY(vector_mask_add(*view_, QStringLiteral("reveal-all")));
    panel_->refresh();
    QVERIFY2(panel_->vectorMaskSectionVisibleForTest(), "vector mask section with a mask");

    auto* rasterize =
        panel_->findChild<QToolButton*>(QStringLiteral("propertiesVectorMaskRasterize"));
    QVERIFY(rasterize);
    rasterize->click();
    panel_->refresh();
    QVERIFY2(!panel_->vectorMaskSectionVisibleForTest(), "section gone after rasterize");
    QVERIFY(!vector_mask_present(*view_));
    QVERIFY(pictura::layers_api::rowAt(*view_, 0).hasMask);

    QVERIFY(vector_mask_add(*view_, QStringLiteral("hide-all")));
    panel_->refresh();
    QVERIFY2(panel_->vectorMaskSectionVisibleForTest(), "section back with a new mask");
    auto* remove = panel_->findChild<QToolButton*>(QStringLiteral("propertiesVectorMaskDelete"));
    QVERIFY(remove);
    remove->click();
    panel_->refresh();
    QVERIFY2(!panel_->vectorMaskSectionVisibleForTest(), "section gone after delete");
    QVERIFY(!vector_mask_present(*view_));
}

QTEST_MAIN(PropertiesPanelTest)
#include "tst_properties_panel.moc"
