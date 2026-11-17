// Layer > Layer Style and the Layer Style dialog (#109): live edits that OK
// records as one state and Cancel drops, and the Copy / Paste / Clear, Scale
// Effects and Hide / Show All Effects commands. The styled layer is a white
// square lifted off the white Background, so an effect is what changes a pixel.

#include <QtTest/QtTest>

#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "gradient_editor_dialog.h"
#include "image_view.h"
#include "layer_style_contour.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "panels/percent_field.h"
#include "api/layers_bridge.h"

#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtCore/QTimer>
#include <QtCore/QVector>

#include <cmath>
<<<<<<< HEAD
=======
#include <functional>
>>>>>>> c44e6bbe (feat(layer-style): bevel contour, gradient editing, and channel include (#581))
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

class LayerStyleTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void dialogOpensOnItsEffectAndCancelDropsTheEdits();
    void okRecordsOneState();
    void menuCommands();
    void groupBlendingOptions();
    void patternOverlayPicksABuiltInPattern();
    void distanceSliderIsExponential();
    void canvasDragMovesTheShadow();
    void zoomKeysReachTheCanvasBehindADialog();
    void previewIsBounded();
    void refusedValueEmitsNothing();
<<<<<<< HEAD
    void globalLightDialogWritesAndUndoes();
    void globalLightDialogCancelKeepsTheLight();
    void globalAngleFieldFollowsGlobalLight();
    void globalAltitudeFieldFollowsGlobalLight();
    void createLayersCommand();
    void rasterizeLayerStyleCommand();
    void globalLightDialogNoOpAcceptRecordsNothing();
=======
    void contourPresetChangesTheBevelAndCancelDropsIt();
    void glossContourWritesTheEffect();
    void contourCheckboxGatesTheSubControls();
    void gradientEditorWritesTheOverlayStops();
    void gradientEditorCancelKeepsTheGradient();
    void gradientEditorKeepsLocationsStrict();
    void gradientEditorHidesNoiseAndSmoothness();
    void channelIncludeChangesTheCompositeAndCancelRestores();
    void channelRowFollowsTheDocumentMode();
    void gradientEditorPresetStaysOpaque();
    void copyEnabledForChannelOnlyStyle();
>>>>>>> c44e6bbe (feat(layer-style): bevel contour, gradient editing, and channel include (#581))

private:
    // A 10x10 white square at (10, 10) lifted onto layer "1" over a white
    // 40x40 Background, selected in the Layers panel.
    void setupSquare();
    QAction* leaf(const QString& name) const;
    QRgb at(int x, int y) const { return QRgb(view_->sample_argb(x, y)) & 0xffffff; }
    void select(const QStringList& paths);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayerStyleTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayerStyleTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

void LayerStyleTest::setupSquare()
{
    QVERIFY(window_->newDocument(QStringLiteral("Style"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    view_->select_rect(10, 10, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    QCoreApplication::processEvents();
    select({QStringLiteral("1")});
}

QAction* LayerStyleTest::leaf(const QString& name) const
{
    return window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Layer"), QStringLiteral("Layer Style"), name}));
}

void LayerStyleTest::select(const QStringList& paths)
{
    panel_->selectPaths(paths, paths.first());
    window_->registry()->refresh();
}

void LayerStyleTest::dialogOpensOnItsEffectAndCancelDropsTheEdits()
{
    setupSquare();
    const int states = view_->history_count();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("colorOverlay"));
    const int row = dialog.rowKeys().indexOf(QStringLiteral("colorOverlay"));
    QCOMPARE(dialog.currentPage(), row);
    QCOMPARE(dialog.effectList()->item(row)->checkState(), Qt::Checked);
    QCOMPARE(dialog.rowKeys().first(), QString());
    // The overlay defaults to red, drawn live on the canvas.
    QCOMPARE(at(15, 15), QRgb(0xff0000));
    QCOMPARE(at(5, 5), QRgb(0xffffff));

    auto* opacity = dialog.findChild<QSpinBox*>(QStringLiteral("colorOverlay.opacity"));
    QVERIFY(opacity);
    opacity->setValue(0);
    QCOMPARE(at(15, 15), QRgb(0xffffff));


    dialog.reject();
    QCOMPARE(view_->history_count(), states);
    QVERIFY(!pictura::layer_style_has(*view_, QStringLiteral("1")));
    QCOMPARE(at(15, 15), QRgb(0xffffff));
}

void LayerStyleTest::okRecordsOneState()
{
    setupSquare();
    const int states = view_->history_count();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("stroke"));
    auto* size = dialog.findChild<QSpinBox*>(QStringLiteral("stroke.size"));
    QVERIFY(size);
    size->setValue(4);
    size->setValue(2);
    dialog.accept();
    QCOMPARE(view_->history_count(), states + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Layer Style"));
    // A black 2 px stroke outside the square's edge.
    QCOMPARE(at(9, 15), QRgb(0x000000));
    QCOMPARE(at(7, 15), QRgb(0xffffff));
    QCOMPARE(pictura::layer_style_value(*view_, QStringLiteral("1"), QStringLiteral("stroke.size")),
             2.0);

    // Opening and closing without an edit records nothing.
    pictura::LayerStyleDialog idle(view_, QStringLiteral("1"), QString());
    idle.accept();
    QCOMPARE(view_->history_count(), states + 1);
}

void LayerStyleTest::groupBlendingOptions()
{
    QVERIFY(window_->newDocument(QStringLiteral("GroupStyle"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QVERIFY(pictura::layer_style_can_edit(*view_, group));
    QVERIFY(pictura::layer_style_set(*view_, group, QStringLiteral("blending.opacity"), 50.0));
    QCOMPARE(pictura::layer_style_value(*view_, group, QStringLiteral("blending.opacity")), 50.0);
    QVERIFY2(!pictura::layer_style_set(*view_, group, QStringLiteral("dropShadow.on"), 1.0),
             "group effects wait for the group-effects renderer");
    pictura::LayerStyleDialog dialog(view_, group, QString());
    QCOMPARE(dialog.currentPage(), 0);
    QVERIFY(!(dialog.effectList()->item(1)->flags() & Qt::ItemIsEnabled));
    auto* mode = dialog.findChild<QComboBox*>(QStringLiteral("blending.mode"));
    QVERIFY(mode != nullptr);
    QCOMPARE(mode->count(), 27);
    QCOMPARE(mode->currentIndex(), 0);
    QVERIFY(mode->findData(QStringLiteral("pass")) < 0);
}

void LayerStyleTest::menuCommands()
{
    setupSquare();
    QAction* dropShadow = leaf(QStringLiteral("Drop Shadow…"));
    QAction* copy = leaf(QStringLiteral("Copy Layer Style"));
    QAction* paste = leaf(QStringLiteral("Paste Layer Style"));
    QAction* clear = leaf(QStringLiteral("Clear Layer Style"));
    QAction* hide = leaf(QStringLiteral("Hide All Effects"));
    QAction* show = leaf(QStringLiteral("Show All Effects"));
    QAction* globalLight = leaf(QStringLiteral("Global Light…"));
    QVERIFY(dropShadow && copy && paste && clear && hide && show && globalLight);

    QVERIFY(dropShadow->isEnabled());
    QVERIFY(!copy->isEnabled() && !clear->isEnabled() && !hide->isEnabled());
    QVERIFY(globalLight->isEnabled());

    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    window_->registry()->refresh();
    QVERIFY(copy->isEnabled() && clear->isEnabled() && hide->isEnabled());
    QVERIFY(!show->isEnabled());

    hide->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Hide All Effects"));
    QCOMPARE(at(15, 15), QRgb(0xffffff));
    window_->registry()->refresh();
    QVERIFY(show->isEnabled() && !hide->isEnabled());
    show->trigger();
    QCOMPARE(at(15, 15), QRgb(0xff0000));

    // Copy onto a second square, then clear the first.
    copy->trigger();
    view_->select_rect(25, 25, 5, 5, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    QCoreApplication::processEvents();
    // The new square sits above the Background, pushing the first to "2".
    select({QStringLiteral("1")});
    QVERIFY(paste->isEnabled());
    paste->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Paste Layer Style"));
    QCOMPARE(at(27, 27), QRgb(0xff0000));

    select({QStringLiteral("2")});
    clear->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Clear Layer Style"));
    QCOMPARE(at(15, 15), QRgb(0xffffff));
    QCOMPARE(at(27, 27), QRgb(0xff0000));
    QVERIFY(view_->undo());
    QCOMPARE(at(15, 15), QRgb(0xff0000));
}

void LayerStyleTest::patternOverlayPicksABuiltInPattern()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("patternOverlay"));
    const int row = dialog.rowKeys().indexOf(QStringLiteral("patternOverlay"));
    QCOMPARE(dialog.effectList()->item(row)->checkState(), Qt::Checked);
    auto* picker = dialog.findChild<QComboBox*>(QStringLiteral("patternOverlay.pattern"));
    QVERIFY(picker);
    QCOMPARE(picker->count(), 8);
    QCOMPARE(picker->itemText(0), QStringLiteral("Checkerboard"));
    // The checkerboard's dark and light squares now show on the white square.
    QVERIFY(at(11, 11) != QRgb(0xffffff) || at(18, 18) != QRgb(0xffffff));
    picker->setCurrentIndex(1);
    QCOMPARE(pictura::layer_style_value(*view_, QStringLiteral("1"),
                                        QStringLiteral("patternOverlay.pattern")),
             1.0);
    dialog.accept();
    QVERIFY(pictura::layer_style_has(*view_, QStringLiteral("1")));
}

void LayerStyleTest::distanceSliderIsExponential()
{
    using pictura::LayerStyleDialog;
    QCOMPARE(LayerStyleDialog::sliderToDistance(0), 0.0);
    QCOMPARE(LayerStyleDialog::sliderToDistance(1000), 30000.0);
    QVERIFY(qAbs(LayerStyleDialog::sliderToDistance(500) - 61.0) < 0.5);
    for (const int px : {0, 1, 5, 61, 250, 5000, 30000}) {
        const double back =
            LayerStyleDialog::sliderToDistance(LayerStyleDialog::distanceToSlider(px));
        QVERIFY2(qAbs(back - px) <= qMax(0.6, px * 0.013), qPrintable(QString::number(px)));
    }

    setupSquare();
    LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("innerShadow"));
    auto* slider = dialog.findChild<QSlider*>(QStringLiteral("innerShadow.distance.slider"));
    auto* spin = dialog.findChild<QSpinBox*>(QStringLiteral("innerShadow.distance"));
    QVERIFY(slider && spin);
    QCOMPARE(spin->maximum(), 30000);
    slider->setValue(500);
    QCOMPARE(spin->value(), 61);
    QCOMPARE(pictura::layer_style_value(*view_, QStringLiteral("1"),
                                        QStringLiteral("innerShadow.distance")),
             61.0);
    dialog.reject();
}

void LayerStyleTest::canvasDragMovesTheShadow()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.useGlobalLight"), 0.0));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("dropShadow"));
    // Straight down by 8 px: the light comes from 90°.
    QVERIFY(dialog.setShadowOffset(QPointF(0, 8)));
    const auto get = [this](const char* key) {
        return pictura::layer_style_value(*view_, QStringLiteral("1"), QLatin1String(key));
    };
    QCOMPARE(get("dropShadow.angle"), 90.0);
    QCOMPARE(get("dropShadow.distance"), 8.0);
    QCOMPARE(dialog.findChild<QSpinBox*>(QStringLiteral("dropShadow.angle"))->value(), 90);
    QCOMPARE(dialog.findChild<QSpinBox*>(QStringLiteral("dropShadow.distance"))->value(), 8);
    // Up and to the left: the light comes from the lower right (−45°).
    QVERIFY(dialog.setShadowOffset(QPointF(-3, -3)));
    QCOMPARE(get("dropShadow.angle"), -45.0);
    QCOMPARE(get("dropShadow.distance"), 4.0);

    // A real left drag on the canvas, through the dialog's event filter.
    auto* canvas = window_->findChild<pictura::ImageView*>();
    QVERIFY(canvas);
    QVERIFY(dialog.setShadowOffset(QPointF(0, 0)));
    dialog.show();
    const QPoint at = canvas->rect().center();
    const int dy = qMax(1, int(std::lround(10 * canvas->zoom())));
    QTest::mousePress(canvas, Qt::LeftButton, {}, at);
    QTest::mouseMove(canvas, at + QPoint(0, dy));
    QTest::mouseRelease(canvas, Qt::LeftButton, {}, at + QPoint(0, dy));
    QCOMPARE(get("dropShadow.angle"), 90.0);
    QVERIFY(qAbs(get("dropShadow.distance") - 10.0) <= 1.0);

    // Pages without a shadow take no drag.
    dialog.effectList()->setCurrentRow(dialog.rowKeys().indexOf(QStringLiteral("satin")));
    QVERIFY(!dialog.setShadowOffset(QPointF(5, 5)));
    dialog.reject();
}

void LayerStyleTest::zoomKeysReachTheCanvasBehindADialog()
{
    setupSquare();
    auto* canvas = window_->findChild<pictura::ImageView*>();
    QVERIFY(canvas);
    const double before = canvas->zoom();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("dropShadow"));
    QTimer::singleShot(0, &dialog, [&dialog] {
        QTest::keyClick(&dialog, Qt::Key_Equal, Qt::ControlModifier);
        dialog.reject();
    });
    pictura::runDialog(dialog, window_.get());
    QVERIFY2(canvas->zoom() > before, "Ctrl+= zoomed the canvas in");
}

void LayerStyleTest::previewIsBounded()
{
    QVERIFY(window_->newDocument(QStringLiteral("StyleBounded"), 120, 120, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    const QString fixture =
        QFINDTESTDATA("../../../../assets/layers_panel_smoke.psd");
    QVERIFY(!fixture.isEmpty());
    const QString path = view_->place_smart_object(fixture);
    QVERIFY(!path.isEmpty());

    QVERIFY(pictura::layer_style_set(*view_, path, QStringLiteral("dropShadow.on"), 1.0));
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(pictura::layer_style_set(*view_, path, QStringLiteral("dropShadow.size"), 6.0));
    QCOMPARE(damaged.count(), 1);
    const QList<QVariant> args = damaged.takeLast();
    const int w = args.at(2).toInt();
    const int h = args.at(3).toInt();
    QVERIFY(w > 0 && h > 0);
    QVERIFY2(w > 64 && h > 64, "the damaged reach is larger than the 64x64 layer");
    QVERIFY2(w < 120 && h < 120, "a slider tick recomposites the reach, not the document");
}

void LayerStyleTest::refusedValueEmitsNothing()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.on"), 1.0));
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QVERIFY(!pictura::layer_style_set(*view_, QStringLiteral("1"),
                                      QStringLiteral("dropShadow.on"), 1.0));
    QCOMPARE(damaged.count(), 0);
}

<<<<<<< HEAD
void LayerStyleTest::globalLightDialogWritesAndUndoes()
{
    setupSquare();
    const int states = view_->history_count();
    QAction* globalLight = leaf(QStringLiteral("Global Light…"));
    QVERIFY(globalLight && globalLight->isEnabled());
    QTimer::singleShot(0, this, [this] {
        auto* dialog = window_->findChild<QDialog*>(QStringLiteral("globalLightDialog"));
        if (!dialog) {
            dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        }
        if (!dialog) {
            return;
        }
        if (auto* angle = dialog->findChild<QSpinBox*>(QStringLiteral("globalLight.angle"))) {
            angle->setValue(90);
        }
        if (auto* altitude =
                dialog->findChild<QSpinBox*>(QStringLiteral("globalLight.altitude"))) {
            altitude->setValue(45);
        }
        dialog->accept();
    });
    globalLight->trigger();
    QCOMPARE(view_->history_count(), states + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Global Light"));
    QCOMPARE(pictura::layer_style_global_angle(*view_), 90.0);
    QCOMPARE(pictura::layer_style_global_altitude(*view_), 45.0);
    QVERIFY(view_->undo());
    QCOMPARE(pictura::layer_style_global_angle(*view_), 30.0);
}

void LayerStyleTest::globalLightDialogCancelKeepsTheLight()
{
    setupSquare();
    const int states = view_->history_count();
    QAction* globalLight = leaf(QStringLiteral("Global Light…"));
    QTimer::singleShot(0, this, [this] {
        auto* dialog = window_->findChild<QDialog*>(QStringLiteral("globalLightDialog"));
        if (!dialog) {
            dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        }
        if (!dialog) {
            return;
        }
        if (auto* angle = dialog->findChild<QSpinBox*>(QStringLiteral("globalLight.angle"))) {
            angle->setValue(200);
        }
        dialog->reject();
    });
    globalLight->trigger();
    QCOMPARE(view_->history_count(), states);
    QCOMPARE(pictura::layer_style_global_angle(*view_), 30.0);
}

void LayerStyleTest::globalAngleFieldFollowsGlobalLight()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set_global_light(*view_, 90, 45));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow"));
    auto* angle = dialog.findChild<QSpinBox*>(QStringLiteral("dropShadow.angle"));
    auto* global = dialog.findChild<QCheckBox*>(QStringLiteral("dropShadow.useGlobalLight"));
    QVERIFY(angle && global);
    QVERIFY(global->isChecked());
    QVERIFY(!angle->isEnabled());
    QCOMPARE(angle->value(), 90);
    global->setChecked(false);
    QVERIFY(angle->isEnabled());
    QCOMPARE(angle->value(), 120);
}

void LayerStyleTest::globalAltitudeFieldFollowsGlobalLight()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set_global_light(*view_, 30, 60));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("bevel"));
    auto* altitude = dialog.findChild<QSpinBox*>(QStringLiteral("bevel.altitude"));
    auto* global = dialog.findChild<QCheckBox*>(QStringLiteral("bevel.useGlobalLight"));
    QVERIFY(altitude && global);
    QVERIFY(global->isChecked());
    QVERIFY(!altitude->isEnabled());
    QCOMPARE(altitude->value(), 60);
    global->setChecked(false);
    QVERIFY(altitude->isEnabled());
    QCOMPARE(altitude->value(), 30);
}

void LayerStyleTest::createLayersCommand()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.useGlobalLight"), 0.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.angle"), 90.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.distance"), 3.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    window_->registry()->refresh();
    const int states = view_->history_count();
    QAction* create = window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Layer"), QStringLiteral("Layer Style"), QStringLiteral("Create Layers")}));
    QVERIFY(create && create->isEnabled());
    create->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Create Layers"));
    QCOMPARE(view_->history_count(), states + 1);
    // The base moved below the created drop shadow and stays the target.
    QCOMPARE(pictura::layer_selection_anchor(*view_), QStringLiteral("2"));
    QVERIFY(!pictura::layer_style_has(*view_, QStringLiteral("2")));
    QVERIFY(view_->undo());
    QVERIFY(pictura::layer_style_has(*view_, QStringLiteral("1")));
}

void LayerStyleTest::rasterizeLayerStyleCommand()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.useGlobalLight"), 0.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.angle"), 90.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.distance"), 3.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.size"), 0.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    window_->registry()->refresh();
    const QRgb shadowPixel = at(15, 22);
    const int states = view_->history_count();
    QAction* rasterize = window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Layer"), QStringLiteral("Rasterize"), QStringLiteral("Layer Style")}));
    QVERIFY(rasterize && rasterize->isEnabled());
    rasterize->trigger();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Rasterize Layer Style"));
    QCOMPARE(view_->history_count(), states + 1);
    QVERIFY(!pictura::layer_style_has(*view_, QStringLiteral("1")));
    QVERIFY(view_->undo());
    QVERIFY(pictura::layer_style_has(*view_, QStringLiteral("1")));
    QCOMPARE(at(15, 22), shadowPixel);

    // Only disabled effects: the command is disabled.
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("dropShadow.on"), 0.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Disable"));
    window_->registry()->refresh();
    QVERIFY(!rasterize->isEnabled());
}

void LayerStyleTest::globalLightDialogNoOpAcceptRecordsNothing()
{
    setupSquare();
    const int states = view_->history_count();
    QAction* globalLight = leaf(QStringLiteral("Global Light…"));
    QVERIFY(globalLight && globalLight->isEnabled());
    QTimer::singleShot(0, this, [this] {
        auto* dialog = window_->findChild<QDialog*>(QStringLiteral("globalLightDialog"));
        if (!dialog) {
            return;
        }
        dialog->accept();
    });
    globalLight->trigger();
    QCOMPARE(view_->history_count(), states);
}

=======

QVector<QRgb> layerSamples(const std::function<QRgb(int, int)>& at)
{
    QVector<QRgb> out;
    for (int y = 10; y < 20; ++y) {
        for (int x = 10; x < 20; ++x) {
            out.append(at(x, y));
        }
    }
    return out;
}

void LayerStyleTest::contourPresetChangesTheBevelAndCancelDropsIt()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.color"), 0xff0000));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.size"), 4.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.depth"), 500.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Setup"));
    const int states = view_->history_count();
    const auto samples = [this] {
        return layerSamples([this](int x, int y) { return at(x, y); });
    };
    const QVector<QRgb> before = samples();

    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("bevel"));
    auto* swatch =
        dialog.findChild<pictura::ContourSwatch*>(QStringLiteral("bevel.glossContour"));
    QVERIFY(swatch);
    QCOMPARE(swatch->pointsText(), QStringLiteral("0,0 255,255"));
    QTimer::singleShot(0, [] {
        auto* editor = qobject_cast<pictura::ContourEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        if (auto* preset = editor->findChild<QComboBox*>(QStringLiteral("contourPreset"))) {
            preset->setCurrentIndex(2);
        }
        editor->accept();
    });
    swatch->click();
    QCOMPARE(pictura::layer_style_contour(*view_, QStringLiteral("1"),
                                          QStringLiteral("bevel.glossContour")),
             QStringLiteral("0,0 128,255 255,0"));
    QVERIFY(samples() != before);
    dialog.reject();
    QVERIFY(samples() == before);
    QCOMPARE(view_->history_count(), states);
}

void LayerStyleTest::glossContourWritesTheEffect()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.color"), 0xff0000));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.depth"), 500.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Setup"));
    const auto samples = [this] {
        return layerSamples([this](int x, int y) { return at(x, y); });
    };
    const QVector<QRgb> before = samples();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("bevel"));
    auto* swatch =
        dialog.findChild<pictura::ContourSwatch*>(QStringLiteral("bevel.glossContour"));
    QVERIFY(swatch);
    QTimer::singleShot(0, [] {
        auto* editor = qobject_cast<pictura::ContourEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        if (auto* preset = editor->findChild<QComboBox*>(QStringLiteral("contourPreset"))) {
            preset->setCurrentIndex(4);
        }
        editor->accept();
    });
    swatch->click();
    QCOMPARE(pictura::layer_style_contour(*view_, QStringLiteral("1"),
                                          QStringLiteral("bevel.glossContour")),
             QStringLiteral("0,0 32,96 128,160 224,96 255,0"));
    QVERIFY(samples() != before);
    dialog.reject();
}

void LayerStyleTest::contourCheckboxGatesTheSubControls()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.color"), 0xff0000));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Setup"));
    const auto samples = [this] {
        return layerSamples([this](int x, int y) { return at(x, y); });
    };
    const QVector<QRgb> off = samples();
    QVERIFY(pictura::layer_style_set_contour(
        *view_, QStringLiteral("1"), QStringLiteral("bevel.edgeContour"),
        QStringLiteral("0,0 128,220 255,255")));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("bevel.edgeContourOn"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Edge"));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QStringLiteral("bevel"));
    auto* on = dialog.findChild<QCheckBox*>(QStringLiteral("bevel.edgeContourOn"));
    auto* swatch =
        dialog.findChild<pictura::ContourSwatch*>(QStringLiteral("bevel.edgeContour"));
    auto* range = dialog.findChild<QSpinBox*>(QStringLiteral("bevel.edgeRange"));
    auto* antiAlias = dialog.findChild<QCheckBox*>(QStringLiteral("bevel.edgeAntiAlias"));
    QVERIFY(on && swatch && range && antiAlias);
    QVERIFY(on->isChecked());
    QVERIFY(swatch->isEnabled());
    QVERIFY(range->isEnabled());
    QVERIFY(antiAlias->isEnabled());
    on->setChecked(false);
    QVERIFY(!swatch->isEnabled());
    QVERIFY(!range->isEnabled());
    QVERIFY(!antiAlias->isEnabled());
    QVERIFY(samples() == off);
    dialog.reject();
}

void LayerStyleTest::gradientEditorWritesTheOverlayStops()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("gradientOverlay"));
    auto* swatch =
        dialog.findChild<pictura::GradientSwatch*>(QStringLiteral("gradientOverlay.gradient"));
    QVERIFY(swatch);
    const QString before =
        pictura::layer_style_gradient(*view_, QStringLiteral("1"),
                                      QStringLiteral("gradientOverlay.gradient"));
    QCOMPARE(before, QStringLiteral("0:000000 4096:ffffff"));
    const auto samples = [this] {
        return layerSamples([this](int x, int y) { return at(x, y); });
    };
    const QVector<QRgb> beforePixels = samples();
    QTimer::singleShot(0, [] {
        auto* editor = qobject_cast<pictura::GradientEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        editor->bar()->setStopColor(0, QColor(Qt::red));
        editor->bar()->addStop(2048);
        editor->accept();
    });
    dialog.show();
    QTest::mouseClick(swatch, Qt::LeftButton);
    const QString text = pictura::layer_style_gradient(
        *view_, QStringLiteral("1"), QStringLiteral("gradientOverlay.gradient"));
    QCOMPARE(text.split(QLatin1Char(' '), Qt::SkipEmptyParts).size(), 3);
    QVERIFY(text.startsWith(QStringLiteral("0:ff0000 ")));
    QVERIFY(text.endsWith(QStringLiteral("4096:ffffff")));
    QVERIFY(samples() != beforePixels);
    dialog.reject();
}

void LayerStyleTest::gradientEditorCancelKeepsTheGradient()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("gradientOverlay"));
    auto* swatch =
        dialog.findChild<pictura::GradientSwatch*>(QStringLiteral("gradientOverlay.gradient"));
    QVERIFY(swatch);
    const QString before =
        pictura::layer_style_gradient(*view_, QStringLiteral("1"),
                                      QStringLiteral("gradientOverlay.gradient"));
    QTimer::singleShot(0, [] {
        auto* editor = qobject_cast<pictura::GradientEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        editor->bar()->setStopColor(0, QColor(Qt::red));
        editor->reject();
    });
    dialog.show();
    QTest::mouseClick(swatch, Qt::LeftButton);
    QCOMPARE(pictura::layer_style_gradient(*view_, QStringLiteral("1"),
                                           QStringLiteral("gradientOverlay.gradient")),
             before);
    dialog.reject();
}

void LayerStyleTest::gradientEditorKeepsLocationsStrict()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("gradientOverlay"));
    auto* swatch =
        dialog.findChild<pictura::GradientSwatch*>(QStringLiteral("gradientOverlay.gradient"));
    QVERIFY(swatch);
    bool strict = false;
    QTimer::singleShot(0, [&strict] {
        auto* editor = qobject_cast<pictura::GradientEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        auto* bar = editor->bar();
        const int first = bar->stops().at(0).location;
        bar->setStopLocation(1, first);
        strict = bar->stops().at(1).location > bar->stops().at(0).location;
        editor->reject();
    });
    dialog.show();
    QTest::mouseClick(swatch, Qt::LeftButton);
    QVERIFY(strict);
    dialog.reject();
}

void LayerStyleTest::gradientEditorHidesNoiseAndSmoothness()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("gradientOverlay"));
    auto* swatch =
        dialog.findChild<pictura::GradientSwatch*>(QStringLiteral("gradientOverlay.gradient"));
    QVERIFY(swatch);
    bool solidOnly = false;
    QTimer::singleShot(0, [&solidOnly] {
        auto* editor = qobject_cast<pictura::GradientEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        solidOnly = !editor->typeCombo()->isEnabled()
                    && !editor->smoothnessField()->isEnabled();
        editor->reject();
    });
    dialog.show();
    QTest::mouseClick(swatch, Qt::LeftButton);
    QVERIFY(solidOnly);
    dialog.reject();
}

void LayerStyleTest::channelIncludeChangesTheCompositeAndCancelRestores()
{
    setupSquare();
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.on"), 1.0));
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("colorOverlay.color"), 0xff0000));
    pictura::layer_style_commit(*view_, QStringLiteral("Setup"));
    const int states = view_->history_count();
    QCOMPARE(at(15, 15), QRgb(0xff0000));
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"), QString());
    auto* include = dialog.findChild<QCheckBox*>(QStringLiteral("blending.include1"));
    QVERIFY(include);
    QVERIFY(include->isChecked());
    include->setChecked(false);
    QCOMPARE(at(15, 15), QRgb(0xffff00));
    dialog.reject();
    QCOMPARE(at(15, 15), QRgb(0xff0000));
    QCOMPARE(view_->history_count(), states);
}

void LayerStyleTest::channelRowFollowsTheDocumentMode()
{
    QVERIFY(window_->newDocument(QStringLiteral("RgbStyle"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    {
        pictura::LayerStyleDialog dialog(view_, QStringLiteral("0"), QString());
        QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("blending.include0")));
        QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("blending.include1")));
        QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("blending.include2")));
        QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("blending.include3")));
        dialog.reject();
    }
    QVERIFY(window_->newDocument(QStringLiteral("GrayStyle"), 16, 16,
                                 QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    {
        pictura::LayerStyleDialog dialog(view_, QStringLiteral("0"), QString());
        QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("blending.include0")));
        dialog.reject();
    }
    QVERIFY(window_->newDocument(QStringLiteral("CmykStyle"), 16, 16,
                                 QStringLiteral("cmyk"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    {
        pictura::LayerStyleDialog dialog(view_, QStringLiteral("0"), QString());
        QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("blending.include0")));
        dialog.reject();
    }
}

void LayerStyleTest::gradientEditorPresetStaysOpaque()
{
    setupSquare();
    pictura::LayerStyleDialog dialog(view_, QStringLiteral("1"),
                                     QStringLiteral("gradientOverlay"));
    auto* swatch =
        dialog.findChild<pictura::GradientSwatch*>(QStringLiteral("gradientOverlay.gradient"));
    QVERIFY(swatch);
    QTimer::singleShot(0, [] {
        auto* editor = qobject_cast<pictura::GradientEditorDialog*>(
            QApplication::activeModalWidget());
        if (!editor) {
            return;
        }
        editor->choosePreset(1);
        editor->accept();
    });
    dialog.show();
    QTest::mouseClick(swatch, Qt::LeftButton);
    const QString text = pictura::layer_style_gradient(
        *view_, QStringLiteral("1"), QStringLiteral("gradientOverlay.gradient"));
    for (const QString& item : text.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        QCOMPARE(item.section(QLatin1Char(':'), 1).size(), 6);
    }
    dialog.reject();
}

void LayerStyleTest::copyEnabledForChannelOnlyStyle()
{
    setupSquare();
    QAction* copy = leaf(QStringLiteral("Copy Layer Style"));
    QAction* clear = leaf(QStringLiteral("Clear Layer Style"));
    QVERIFY(copy && clear);
    QVERIFY(!copy->isEnabled());
    QVERIFY(!clear->isEnabled());
    QVERIFY(pictura::layer_style_set(*view_, QStringLiteral("1"),
                                     QStringLiteral("blending.include1"), 0.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Channel"));
    window_->registry()->refresh();
    QVERIFY(copy->isEnabled());
    QVERIFY(!clear->isEnabled());
}

>>>>>>> c44e6bbe (feat(layer-style): bevel contour, gradient editing, and channel include (#581))
QTEST_MAIN(LayerStyleTest)
#include "tst_layer_style.moc"
