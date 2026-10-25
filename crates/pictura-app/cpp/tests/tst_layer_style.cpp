// Layer > Layer Style and the Layer Style dialog (#109): live edits that OK
// records as one state and Cancel drops, and the Copy / Paste / Clear, Scale
// Effects and Hide / Show All Effects commands. The styled layer is a white
// square lifted off the white Background, so an effect is what changes a pixel.

#include <QtTest/QtTest>

#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "image_view.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

#include <QtGui/QAction>
#include <QtCore/QTimer>

#include <cmath>
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
    QVERIFY(!globalLight->isEnabled());

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

QTEST_MAIN(LayerStyleTest)
#include "tst_layer_style.moc"
