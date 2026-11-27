#include "tools.h"
#include <QtTest/QtTest>
#include "api/layers_api.h"
#include "api/document_bridge.h"

#include "commands.h"
#include "frame.h"
#include "levels_dialog.h"
#include "selftest_paint_fixture.h"
#include "panels/histogram_panel.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/properties_panel.h"
#include "api/layers_bridge.h"

#include "panels/numeric_field.h"
#include "panels/adjustment_controls.h"
#include "panels/curve_widget.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <QtCore/QCoreApplication>
#include <QtGui/QImage>
#include <QtGui/QAction>

#include "qt_test_support.h"

#include <type_traits>
#include <utility>

template<typename Curve, typename = void>
struct HasDenseCurveApi : std::false_type {};

template<typename Curve>
struct HasDenseCurveApi<Curve, std::void_t<decltype(std::declval<Curve&>().hasDenseCurve())>> : std::true_type {};

template<typename Curve>
void checkPencilIntegration(Curve* curve, QWidget* panel)
{
    if constexpr (HasDenseCurveApi<Curve>::value) {
        auto* pencil = panel->findChild<QToolButton*>(QStringLiteral("curvesPencil"));
        auto* smooth = panel->findChild<QToolButton*>(QStringLiteral("curvesSmooth"));
        QVERIFY(pencil && smooth);
        pencil->click();
        QTest::mousePress(curve, Qt::LeftButton, Qt::NoModifier, curve->rect().center());
        QTest::mouseRelease(curve, Qt::LeftButton, Qt::NoModifier, curve->rect().center());
        QVERIFY(curve->hasDenseCurve());
        QVERIFY(smooth->isEnabled());
        smooth->click();
    } else {
        QVERIFY(!panel->findChild<QToolButton*>(QStringLiteral("curvesPencil")));
    }
}


class PropertiesPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void plainDocumentShowsNoProperties();
    void adjustmentLayerShowsItsName();
    void clipIsDisabledAboveABackground();
    void slidersEditLiveAndCommitOnce();
    void thresholdHistogramShowsLowerLayers();
    void thresholdHistogramRefreshesSourceAndSelection();
    void thresholdHistogramTracksSelectionMove();
    void thresholdHistogramRespectsGroupBoundaries();
    void groupsCurvesAndFooter();
    void curvesPointFields();
    void curvesControlsAndMenu();
    void autoSelectParameter();
    void curvesPencilIntegration();
    void levelsHandlesAndMenu();
    void channelMixerMonochrome();
    void channelMixerCmykGroups();
    void canvasSectionReadsAndResizes();
    void maskSectionAppearsForAMaskedLayer();
    void vectorMaskSectionAppears();
    void maskDensityFeatherAndInvertEdit();
    void vectorMaskDensityAndFeatherEdit();
    void maskRefineEditsStayScoped();
    void footerDeleteUsesTheGate();

private:
    // A white 16x16 document whose layer is selected and shown in the panel.
    bool setupLayer(const QString& name);
    bool addThresholdFromActiveLayer();

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
void PropertiesPanelTest::autoSelectParameter()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    const auto panelName = QStringLiteral("propertiesPanel");
    auto* column = window_->columnForPanel(panelName);
    QVERIFY(column);
    window_->show();
    window_->setScreenMode(pictura::PicturaMainWindow::ScreenMode::Standard);
    window_->setPanelsHidden(false);
    column->setRailMode(false);
    QVERIFY(column->showPanel(panelName, true));
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
    QTRY_VERIFY(panel_->isVisible());
    QMenu menu;
    auto* action = menu.addAction(QStringLiteral("Auto-Select Parameter"));
    action->setCheckable(true);
    action->setData(QStringLiteral("properties.autoParameter"));
    panel_->updatePanelMenu(&menu);
    QVERIFY(action->isEnabled());
    action->trigger();
    auto* black = panel_->controlForTest(QStringLiteral("inputBlack"));
    QVERIFY(black);
    auto* field = black->parentWidget()->findChild<QDoubleSpinBox*>();
    QVERIFY(field);
    QTRY_VERIFY(field->hasFocus() || field->findChild<QLineEdit*>()->hasFocus());
    QVERIFY(setupAdjustment(QStringLiteral("curves")));
    auto* output = panel_->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveOutput"));
    QVERIFY(output);
    QTRY_VERIFY(output->hasFocus() || output->findChild<QLineEdit*>()->hasFocus());
}

void PropertiesPanelTest::curvesPencilIntegration()
{
    QVERIFY(setupAdjustment(QStringLiteral("curves")));
    auto* graph = panel_->findChild<pictura::CurveWidget*>(QStringLiteral("adjustmentCurve"));
    QVERIFY(graph);
    checkPencilIntegration(graph, panel_);
}

void PropertiesPanelTest::levelsHandlesAndMenu()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* histogram = panel_->findChild<pictura::LevelsStrip*>(QStringLiteral("propertiesLevelsHistogram"));
    auto* input = panel_->findChild<pictura::TriangleSlider*>(QStringLiteral("propertiesLevelsInput"));
    auto* output = panel_->findChild<pictura::TriangleSlider*>(QStringLiteral("propertiesLevelsOutput"));
    QVERIFY(histogram && input && output);
    for (int point = 0; point < 3; ++point)
        QVERIFY(panel_->findChild<QToolButton*>(QStringLiteral("propertiesLevelsPoint%1").arg(point)));
    auto* black = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(black);
    QVERIFY(QMetaObject::invokeMethod(input, "valueChanged", Q_ARG(int, 0), Q_ARG(int, 32)));
    QCOMPARE(black->value(), 32);
    QMenu menu;
    auto* clipping = menu.addAction(QStringLiteral("Show Clipping"));
    clipping->setCheckable(true);
    clipping->setData(QStringLiteral("properties.showClipping"));
    panel_->updatePanelMenu(&menu);
    QVERIFY(clipping->isEnabled());
    clipping->trigger();
    panel_->updatePanelMenu(&menu);
    QVERIFY(clipping->isChecked());
}

void PropertiesPanelTest::curvesControlsAndMenu()
{
    QVERIFY(setupAdjustment(QStringLiteral("curves")));
    auto* graph = panel_->findChild<pictura::CurveWidget*>(QStringLiteral("adjustmentCurve"));
    auto* endpoints = panel_->findChild<pictura::TriangleSlider*>(QStringLiteral("curvesEndpoints"));
    QVERIFY(graph && endpoints);
    auto* target = panel_->findChild<QToolButton*>(QStringLiteral("curvesTarget"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(target && tools);
    QVERIFY(!tools->canvasBrushActive());
    target->click();
    QVERIFY(target->isChecked());
    QVERIFY(tools->canvasBrushActive());
    target->click();
    QVERIFY(!tools->canvasBrushActive());
    QVERIFY(panel_->findChild<QToolButton*>(QStringLiteral("curvesBlackPoint")));
    QVERIFY(panel_->findChild<QToolButton*>(QStringLiteral("curvesGrayPoint")));
    QVERIFY(panel_->findChild<QToolButton*>(QStringLiteral("curvesWhitePoint")));
    QVERIFY(graph->selectTone(128));
    auto* input = panel_->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveInput"));
    auto* output = panel_->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveOutput"));
    QVERIFY(input && output && input->isEnabled() && output->isEnabled());
    QCOMPARE(input->value(), 128);
    output->setValue(150);
    QCOMPARE(qRound(graph->points()[graph->selected()].y() * 255), 150);
    endpoints->setValue(0, 20);
    QVERIFY(QMetaObject::invokeMethod(endpoints, "valueChanged", Q_ARG(int, 0), Q_ARG(int, 20)));
    QCOMPARE(qRound(graph->points().first().x() * 255), 20);
    QMenu menu;
    auto* display = menu.addAction(QStringLiteral("Display"));
    display->setData(QStringLiteral("panel.properties.curvesDisplay"));
    connect(display, &QAction::triggered, panel_, [this] {
        panel_->performPanelMenuAction(QStringLiteral("curvesDisplay"));
    });
    auto* clipping = menu.addAction(QStringLiteral("Clipping"));
    clipping->setData(QStringLiteral("properties.showClipping"));
    clipping->setCheckable(true);
    panel_->updatePanelMenu(&menu);
    QVERIFY(display->isEnabled());
    QVERIFY(clipping->isEnabled());
    auto* options = panel_->findChild<QWidget*>(QStringLiteral("curvesDisplayOptions"));
    QVERIFY(options && options->isHidden());
    display->trigger();
    QVERIFY(!options->isHidden());
}

void PropertiesPanelTest::curvesPointFields()
{
    QVERIFY(setupAdjustment(QStringLiteral("curves")));
    auto* input = panel_->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveInput"));
    auto* output = panel_->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveOutput"));
    QVERIFY(input && output);
    QVERIFY(!input->isEnabled());
    QVERIFY(!output->isEnabled());
}

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
    view_->set_presented_view(0, 0, 0, 16, 16);
    panel_->refresh();
    return panel_->pathForTest() == path_;
}

namespace {

pictura::HistogramView* thresholdHistogram(pictura::PropertiesPanel& panel)
{
    return dynamic_cast<pictura::HistogramView*>(
        panel.findChild<QWidget*>(QStringLiteral("thresholdHistogram")));
}

} // namespace

bool PropertiesPanelTest::addThresholdFromActiveLayer()
{
    if (!view_->add_adjustment(QStringLiteral("threshold"))) {
        return false;
    }
    path_ = pictura::layer_selection_anchor(*view_);
    panel_->refresh();
    return !path_.isEmpty() && panel_->pathForTest() == path_;
}

void PropertiesPanelTest::thresholdHistogramShowsLowerLayers()
{
    QVERIFY(setupLayer(QStringLiteral("Threshold histogram")));
    QVERIFY(!view_->add_solid_fill(0xff5a5a5au).isEmpty());
    QVERIFY(addThresholdFromActiveLayer());
    QVERIFY(!view_->add_solid_fill(0xffb4b4b4u).isEmpty());
    window_->selectLayerPath(path_);
    QCOMPARE(panel_->pathForTest(), path_);
    auto* histogram = thresholdHistogram(*panel_);
    auto* slider = panel_->controlForTest(QStringLiteral("level"));
    QVERIFY(histogram && slider);
    auto* layout = qobject_cast<QVBoxLayout*>(slider->parentWidget()->layout());
    QVERIFY(layout && layout->indexOf(histogram) > 0);
    QVERIFY(layout->itemAt(0)->layout());
    QVERIFY(layout->indexOf(histogram) < layout->indexOf(slider));
    QCOMPARE(histogram->binForTest(90), 256u * 255);
    QCOMPARE(histogram->binForTest(0), 0u);
    QCOMPARE(histogram->binForTest(180), 0u);
}

void PropertiesPanelTest::thresholdHistogramRefreshesSourceAndSelection()
{
    QVERIFY(setupLayer(QStringLiteral("Threshold histogram refresh")));
    QVERIFY(view_->select_rect(0, 0, 8, 16, QStringLiteral("new"), 0.0));
    const QString lower = view_->add_solid_fill(0xff5a5a5au);
    QVERIFY(!lower.isEmpty());
    view_->deselect();
    QVERIFY(addThresholdFromActiveLayer());
    auto* histogram = thresholdHistogram(*panel_);
    QVERIFY(histogram);
    QCOMPARE(histogram->binForTest(90), 128u * 255);
    QCOMPARE(histogram->binForTest(255), 128u * 255);
    QCOMPARE(view_->set_layers_visible(QStringList{lower}, false), 1);
    QTRY_COMPARE(histogram->binForTest(255), 256u * 255);
    QCOMPARE(histogram->binForTest(90), 0u);
    QCOMPARE(view_->set_layers_visible(QStringList{lower}, true), 1);
    QTRY_COMPARE(histogram->binForTest(90), 128u * 255);
    QVERIFY(view_->select_rect(0, 0, 8, 16, QStringLiteral("new"), 0.0));
    QTRY_COMPARE(histogram->binForTest(255), 0u);
    QCOMPARE(histogram->binForTest(90), 128u * 255);
}

void PropertiesPanelTest::thresholdHistogramRespectsGroupBoundaries()
{
    QVERIFY(setupLayer(QStringLiteral("Threshold histogram groups")));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QVERIFY(view_->set_layer_blend(group.toInt(), QStringLiteral("pass")));
    QVERIFY(view_->select_rect(0, 0, 8, 16, QStringLiteral("new"), 0.0));
    QVERIFY(view_->add_solid_fill(0xff5a5a5au).startsWith(group + QLatin1Char('/')));
    view_->deselect();
    QVERIFY(addThresholdFromActiveLayer());
    QVERIFY(path_.startsWith(group + QLatin1Char('/')));
    auto* histogram = thresholdHistogram(*panel_);
    QVERIFY(histogram);
    QCOMPARE(histogram->binForTest(90), 128u * 255);
    QCOMPARE(histogram->binForTest(255), 128u * 255);
    QVERIFY(view_->set_layer_blend(group.toInt(), QStringLiteral("norm")));
    QTRY_COMPARE(histogram->binForTest(255), 0u);
    QCOMPARE(histogram->binForTest(90), 128u * 255);
}

void PropertiesPanelTest::thresholdHistogramTracksSelectionMove()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(QColor(90, 90, 90));
    for (int y = 0; y < 64; ++y) {
        for (int x = 32; x < 64; ++x) {
            seed.setPixelColor(x, y, QColor(180, 180, 180));
        }
    }
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("threshold_selection_move"));
    QVERIFY(f.ok());
    view_ = f.view;
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    QVERIFY(panel_);
    panel_->setView(view_);
    QVERIFY(addThresholdFromActiveLayer());
    auto* histogram = thresholdHistogram(*panel_);
    QVERIFY(histogram);
    window_->setActiveTool(pictura::ToolId::Marquee);
    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    options->setFeather(0.0);
    f.canvas->mousePressed(QPointF(4, 4), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(12, 12));
    f.canvas->mouseReleased(QPointF(12, 12));
    QTRY_COMPARE(histogram->binForTest(90), 64u * 255);
    QCOMPARE(histogram->binForTest(180), 0u);
    f.canvas->mousePressed(QPointF(6, 6), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(38, 6));
    QTRY_COMPARE(histogram->binForTest(180), 64u * 255);
    QCOMPARE(histogram->binForTest(90), 0u);
    window_->setActiveTool(pictura::ToolId::EllipticalMarquee);
    QTRY_COMPARE(histogram->binForTest(90), 64u * 255);
    QCOMPARE(histogram->binForTest(180), 0u);
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
    QCoreApplication::processEvents();
    const QImage shown = view_->display_image(0, 0, 0, 16, 16);
    QVERIFY(!shown.isNull());
    QCOMPARE(qGray(shown.pixel(4, 4)), 0);
    QCOMPARE(view_->history_count(), history);
    panel_->commitForTest();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(qGray(view_->composite_argb(4, 4)), 0);
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

bool PropertiesPanelTest::setupLayer(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_ || !layers) {
        return false;
    }
    layers->setView(view_);
    if (!layers->selectRowForTest(pictura::layers_api::rowAt(*view_, 0).path)) {
        return false;
    }
    panel_->setView(view_);
    return true;
}

// The panel's Monochrome check switches a Channel Mixer to its Gray group and
// seeds the 40/40/20 mix; unchecking returns the three RGB groups.
void PropertiesPanelTest::channelMixerMonochrome()
{
    QVERIFY2(setupAdjustment(QStringLiteral("channel-mixer")), "channel mixer fixture");
    auto* group = panel_->findChild<QComboBox*>(QStringLiteral("adjustmentGroup"));
    auto* check = panel_->findChild<QCheckBox*>(QStringLiteral("adjustmentCheck.monochrome"));
    QVERIFY(group && check);
    QCOMPARE(group->count(), 3);
    QVERIFY(!check->isChecked());
    check->click();
    panel_->commitForTest();
    panel_->refresh();
    group = panel_->findChild<QComboBox*>(QStringLiteral("adjustmentGroup"));
    QVERIFY(group);
    QCOMPARE(group->count(), 1);
    QCOMPARE(group->itemText(0), QStringLiteral("Gray"));
    auto* red = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("0.red")));
    QVERIFY(red);
    QCOMPARE(red->value(), 40);
    check = panel_->findChild<QCheckBox*>(QStringLiteral("adjustmentCheck.monochrome"));
    QVERIFY(check);
    check->click();
    panel_->commitForTest();
    panel_->refresh();
    group = panel_->findChild<QComboBox*>(QStringLiteral("adjustmentGroup"));
    QVERIFY(group);
    QCOMPARE(group->count(), 3);
}

// A CMYK Channel Mixer page has the four ink groups and no Monochrome check.
void PropertiesPanelTest::channelMixerCmykGroups()
{
    QVERIFY(window_->newDocument(QStringLiteral("Adjust"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_ && pictura::convert_image_mode(*view_, QStringLiteral("cmyk")));
    QCOMPARE(pictura::image_mode(*view_), QStringLiteral("cmyk"));
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    layers_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_ && layers_ && view_->add_adjustment(QStringLiteral("channel-mixer")));
    QString path;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        if (pictura::layers_api::rowAt(*view_, i).hasAdjustment) {
            path = pictura::layers_api::rowAt(*view_, i).path;
        }
    }
    QVERIFY(!path.isEmpty());
    panel_->setView(view_);
    layers_->setView(view_);
    QVERIFY(layers_->selectRowForTest(path));
    view_->set_presented_view(0, 0, 0, 16, 16);
    panel_->refresh();
    auto* group = panel_->findChild<QComboBox*>(QStringLiteral("adjustmentGroup"));
    QVERIFY(group);
    QCOMPARE(group->count(), 4);
    QCOMPARE(group->itemText(0), QStringLiteral("Cyan"));
    QCOMPARE(group->itemText(3), QStringLiteral("Black"));
    QVERIFY(qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("0.cyan"))));
    QVERIFY(qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("3.black"))));
    QVERIFY(!panel_->findChild<QCheckBox*>(QStringLiteral("adjustmentCheck.monochrome")));
}
void PropertiesPanelTest::maskDensityFeatherAndInvertEdit()
{
    QVERIFY2(setupLayer(QStringLiteral("MaskRefine")), "mask refine fixture");
    QVERIFY(layer_mask_add(*view_, QStringLiteral("hide-all")));
    panel_->refresh();
    auto* density = panel_->findChild<pictura::NumericField*>(QStringLiteral("propertiesMaskDensity"));
    auto* feather = panel_->findChild<pictura::NumericField*>(QStringLiteral("propertiesMaskFeather"));
    auto* invert = panel_->findChild<QToolButton*>(QStringLiteral("propertiesMaskInvert"));
    QVERIFY(density && feather && invert);
    QCOMPARE(density->value(), 100.0);
    QCOMPARE(feather->value(), 0.0);

    const int history = view_->history_count();
    emit density->valueChanged(30.0);
    QCOMPARE(layer_mask_density(*view_), 77);
    QCOMPARE(view_->history_count(), history);
    emit density->valueCommitted(50.0);
    QCOMPARE(layer_mask_density(*view_), 128);
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Mask Density"));

    emit feather->valueCommitted(4.0);
    QCOMPARE(layer_mask_feather(*view_), 4.0);
    QCOMPARE(view_->history_count(), history + 2);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Mask Feather"));
    panel_->refresh();
    QCOMPARE(density->value(), 50.0);
    QCOMPARE(feather->value(), 4.0);

    QVERIFY(view_->undo());
    QCOMPARE(layer_mask_feather(*view_), 0.0);
    QVERIFY(view_->redo());

    invert->click();
    QCOMPARE(view_->history_count(), history + 3);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Invert"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Mask Feather"));
}

// The Vector Mask section's Density and Feather edit the vector mask, each
// commit one state (vmk_props_refine).
void PropertiesPanelTest::vectorMaskDensityAndFeatherEdit()
{
    QVERIFY2(setupLayer(QStringLiteral("VectorRefine")), "vector refine fixture");
    QVERIFY(vector_mask_add(*view_, QStringLiteral("hide-all")));
    panel_->refresh();
    auto* density =
        panel_->findChild<pictura::NumericField*>(QStringLiteral("propertiesVectorMaskDensity"));
    auto* feather =
        panel_->findChild<pictura::NumericField*>(QStringLiteral("propertiesVectorMaskFeather"));
    QVERIFY(density && feather);
    QCOMPARE(density->value(), 100.0);

    const int history = view_->history_count();
    emit density->valueCommitted(50.0);
    emit feather->valueCommitted(4.0);
    QCOMPARE(vector_mask_density(*view_), 128);
    QCOMPARE(vector_mask_feather(*view_), 4.0);
    QCOMPARE(view_->history_count(), history + 2);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Vector Mask Feather"));
    emit feather->valueCommitted(4.0);
    QCOMPARE(view_->history_count(), history + 2);
}

// An uncommitted Density preview is recorded under its own label before
// another control's edit, and committing an untouched Density field whose byte
// has no exact percent changes nothing.
void PropertiesPanelTest::maskRefineEditsStayScoped()
{
    QVERIFY2(setupLayer(QStringLiteral("MaskScope")), "mask scope fixture");
    QVERIFY(layer_mask_add(*view_, QStringLiteral("hide-all")));
    panel_->refresh();
    auto* density = panel_->findChild<pictura::NumericField*>(QStringLiteral("propertiesMaskDensity"));
    auto* invert = panel_->findChild<QToolButton*>(QStringLiteral("propertiesMaskInvert"));
    QVERIFY(density && invert);

    const int history = view_->history_count();
    emit density->valueChanged(30.0);
    invert->click();
    QCOMPARE(view_->history_count(), history + 2);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Invert"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Mask Density"));
    QCOMPARE(layer_mask_density(*view_), 77);
    QVERIFY(view_->redo());

    QVERIFY(layer_mask_set_density(*view_, 76, true));
    panel_->refresh();
    QCOMPARE(density->value(), 30.0);
    const int before = view_->history_count();
    emit density->valueCommitted(density->value());
    QCOMPARE(layer_mask_density(*view_), 76);
    QCOMPARE(view_->history_count(), before);
}

void PropertiesPanelTest::clipIsDisabledAboveABackground()
{
    QVERIFY(window_->newDocument(QStringLiteral("ClipAboveBg"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    layers_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_ && layers_);
    QVERIFY(view_->background_from_layer(QStringLiteral("0")));
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    path_.clear();
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const auto row = pictura::layers_api::rowAt(*view_, i);
        if (row.hasAdjustment) {
            path_ = row.path;
        }
    }
    QVERIFY(!path_.isEmpty());
    panel_->setView(view_);
    layers_->setView(view_);
    QVERIFY(layers_->selectRowForTest(path_));
    panel_->refresh();
    auto* clip = panel_->findChild<QToolButton*>(QStringLiteral("propertiesClip"));
    QVERIFY(clip != nullptr);
    QVERIFY2(!clip->isEnabled(), "clip is disabled above a Background");
}


void PropertiesPanelTest::footerDeleteUsesTheGate()
{
    QVERIFY(setupAdjustment(QStringLiteral("hue-saturation")));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    auto* remove = panel_->findChild<QToolButton*>(QStringLiteral("propertiesDelete"));
    QVERIFY(remove != nullptr);
    panel_->refresh();
    QVERIFY2(remove->isEnabled(), "the footer Delete is enabled for a deletable layer");

    QVERIFY(view_->set_layers_lock(QStringList{path_}, QStringLiteral("all"), true) >= 0);
    panel_->refresh();
    QVERIFY2(!remove->isEnabled(), "the footer Delete greys out with the layer locked");
    const int before = view_->layer_count();
    registry->dispatch(QString::fromLatin1(pictura::command_ids::PanelPropertiesDeleteLayer));
    QCOMPARE(view_->layer_count(), before);

    QVERIFY(view_->set_layers_lock(QStringList{path_}, QStringLiteral("all"), false) >= 0);
    panel_->refresh();
    QVERIFY2(remove->isEnabled(), "unlocking re-enables the footer Delete");
}
PICTURA_QT_TEST_MAIN(PropertiesPanelTest)
#include "tst_properties_panel.moc"
